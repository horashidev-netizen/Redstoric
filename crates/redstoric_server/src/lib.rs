//! Server chuyên dụng (authoritative): giữ danh sách người chơi, chạy AI boss, kiểm tra mọi
//! thông tin client gửi lên. Không cần cửa sổ hay GPU; chạy bằng binary `redstoric_server`.
//!
//! Server không tải voxel. Vị trí boss chỉ tính trên mặt phẳng XZ và bị giới hạn trong sân đấu
//! (tường đấu trường chặn boss ở client cũ, ở đây là bán kính `ARENA_LIMIT`); độ cao của boss do
//! client tự dò theo địa hình cục bộ.

use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_quinnet::server::certificate::CertificateRetrievalMode;
use bevy_quinnet::server::{
    ConnectionEvent, ConnectionLostEvent, EndpointAddrConfiguration, QuinnetServer, QuinnetServerPlugin,
    ServerEndpointConfiguration, ServerEndpointConfigurationDefaultables,
};
use redstoric_common::bosses::BossKind;
use redstoric_common::structures::SiteKind;
use redstoric_common::{WorldGenerator, WORLD_SEED};
use redstoric_store::{PlayerRecord, Store, WorldRecord};

use redstoric_protocol::*;

// --- Luật chơi do server quyết định: nguồn chung nằm ở `redstoric_common::rules` ---
use redstoric_common::rules::{boss as rb, player as rp};
const BOSS_HEALTH: f32 = rb::HEALTH;
const AGGRO_RANGE: f32 = rb::AGGRO_RANGE;
const LEASH_RANGE: f32 = rb::LEASH_RANGE;
/// Boss không đi ra khỏi bán kính này quanh tâm đấu trường (tường đấu trường nằm ở 15.5).
const ARENA_LIMIT: f32 = rb::ARENA_LIMIT;
const BOSS_SPEED: f32 = rb::SPEED;
const ATTACK_RANGE: f32 = rb::ATTACK_RANGE;
const ATTACK_DAMAGE: f32 = rb::ATTACK_DAMAGE;
const WINDUP: f32 = rb::WINDUP;
const RECOVER: f32 = rb::RECOVER;
const DEATH_TIME: f32 = rb::DEATH_TIME;
const BOSS_REGEN: f32 = rb::REGEN;

const PLAYER_DAMAGE: f32 = rp::DAMAGE;
/// Tầm đánh tối đa chấp nhận được (tầm chém 2.8 + bán kính boss + bù độ trễ mạng).
const PLAYER_REACH: f32 = rp::REACH;
const ATTACK_COOLDOWN: f32 = rp::ATTACK_COOLDOWN;
const PLAYER_REGEN: f32 = rp::REGEN;
/// Tốc độ ngang tối đa chấp nhận (nhân vật chạy 10 m/s, chừa dư cho knockback và độ trễ).
const MAX_SPEED: f32 = rp::MAX_SPEED;
const HELLO_TIMEOUT: f32 = 10.0;
const BOSS_ACTIVATE_RADIUS: i32 = 200;
const SNAPSHOT_BOSS_RADIUS: f32 = 300.0;
const MAX_MESSAGES_PER_CLIENT_PER_TICK: usize = 64;

#[derive(Resource)]
pub struct ServerConfig {
    pub port: u16,
}

/// Kho dữ liệu trên đĩa (tài khoản, nhân vật, thế giới).
#[derive(Resource)]
struct ServerStore(Store);

/// Tự lưu mọi người chơi và thế giới mỗi chừng này giây.
const AUTOSAVE_INTERVAL: f32 = 30.0;

struct Player {
    /// Mã tài khoản (sha256 của khoá), dùng làm tên file lưu.
    account: String,
    progress: String,
    logins: u32,
    name: String,
    appearance: String,
    state: PlayerState,
    has_state: bool,
    hp: f32,
    last_state_t: f32,
    last_attack_t: f32,
    last_chat_t: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Chase,
    Windup(f32),
    Recover(f32),
    Dying(f32),
}

struct ServerBoss {
    home: Vec2,
    pos: Vec2,
    yaw: f32,
    hp: f32,
    phase: Phase,
    /// Loại boss (quyết định chỉ số), suy ra từ bậc của đấu trường.
    kind: BossKind,
}

#[derive(Resource, Default)]
pub struct ServerWorld {
    players: HashMap<u64, Player>,
    /// Client đã kết nối nhưng chưa gửi `Hello` (id -> thời điểm kết nối).
    pending: HashMap<u64, f32>,
    bosses: HashMap<[i32; 2], ServerBoss>,
    defeated: HashSet<[i32; 2]>,
    env: ServerEnv,
}

/// Đồng hồ và thời tiết của cả thế giới (mọi người chơi thấy cùng một giờ và cùng một cơn mưa).
struct ServerEnv {
    hour: f32,
    rain: f32,
    rain_timer: f32,
    rng: u32,
    send_timer: f32,
}

impl Default for ServerEnv {
    fn default() -> Self {
        Self { hour: 9.0, rain: 0.0, rain_timer: 120.0, rng: 0x2545_f491, send_timer: 0.0 }
    }
}

/// Một ngày dài 600 giây thực, như client offline.
const DAY_LENGTH: f32 = 600.0;
/// Người ở xa hơn mức này không nhận hiệu ứng vũ khí của nhau.
const FX_RADIUS: f32 = 90.0;

pub struct NetServerPlugin {
    pub port: u16,
    /// Thư mục lưu dữ liệu (tài khoản, nhân vật, thế giới).
    pub data_dir: PathBuf,
}

impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        let store = Store::open(&self.data_dir).unwrap_or_else(|err| panic!("Không mở được thư mục dữ liệu {:?}: {err}", self.data_dir));
        let saved = store.load_world();
        info!("Dữ liệu server ở {:?}: {} boss đã hạ, giờ {:.1}", store.dir(), saved.defeated.len(), saved.hour);
        let mut world = ServerWorld::default();
        world.defeated = saved.defeated.iter().copied().collect();
        world.env.hour = saved.hour;
        app.add_plugins(QuinnetServerPlugin::default())
            .insert_resource(ServerStore(store))
            .insert_resource(ServerConfig { port: self.port })
            .insert_resource(world)
            .insert_resource(WorldGenerator::with_plan(WORLD_SEED))
            .insert_resource(Time::<Fixed>::from_hz(20.0))
            .add_systems(Startup, start_listening)
            .add_systems(Update, (handle_connections, receive_messages, spawn_bosses).chain())
            .add_systems(FixedUpdate, (simulate, advance_env, send_snapshots).chain())
            .add_systems(Update, autosave);
    }
}

fn start_listening(mut server: ResMut<QuinnetServer>, config: Res<ServerConfig>) {
    let result = server.start_endpoint(ServerEndpointConfiguration {
        // IPv4 cho chắc: trên Windows socket IPv6 mặc định không nhận kết nối IPv4.
        addr_config: EndpointAddrConfiguration::from_ip(Ipv4Addr::UNSPECIFIED, config.port),
        cert_mode: CertificateRetrievalMode::LoadFromFileOrGenerateSelfSigned {
            cert_file: "server_cert.pem".to_string(),
            key_file: "server_key.pem".to_string(),
            save_on_disk: true,
            server_hostname: "localhost".to_string(),
        },
        defaultables: ServerEndpointConfigurationDefaultables {
            send_channels_cfg: channels(),
            ..Default::default()
        },
    });
    match result {
        Ok(_) => info!("Redstoric server đang lắng nghe QUIC/UDP trên cổng {}", config.port),
        Err(err) => error!("Không mở được cổng {}: {err}", config.port),
    }
}

fn sanitize_name(raw: &str, id: u64) -> String {
    let name: String = raw.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    let name = name.trim().to_string();
    if name.is_empty() { format!("Player{id}") } else { name }
}

fn sanitize_appearance(raw: String) -> String {
    if raw.len() > MAX_APPEARANCE_BYTES { String::new() } else { raw }
}

fn record_of(p: &Player) -> PlayerRecord {
    PlayerRecord {
        name: p.name.clone(),
        appearance: p.appearance.clone(),
        pos: p.state.pos,
        class: p.state.class,
        hp: p.hp,
        progress: p.progress.clone(),
        logins: p.logins,
        last_seen: redstoric_store::now_secs(),
    }
}

fn save_world(store: &Store, world: &ServerWorld) {
    let mut defeated: Vec<[i32; 2]> = world.defeated.iter().copied().collect();
    defeated.sort();
    if let Err(err) = store.save_world(&WorldRecord { defeated, hour: world.env.hour }) {
        error!("Không lưu được thế giới: {err}");
    }
}

fn save_player(store: &Store, p: &Player) {
    if let Err(err) = store.save_player(&p.account, &record_of(p)) {
        error!("Không lưu được {}: {err}", p.name);
    }
}

/// Định kỳ ghi mọi người chơi đang online và thế giới ra đĩa.
fn autosave(time: Res<Time>, mut timer: Local<f32>, world: Res<ServerWorld>, store: Res<ServerStore>) {
    *timer += time.delta_secs();
    if *timer < AUTOSAVE_INTERVAL {
        return;
    }
    *timer = 0.0;
    for p in world.players.values() {
        save_player(&store.0, p);
    }
    save_world(&store.0, &world);
}

fn handle_connections(
    store: Res<ServerStore>,
    time: Res<Time>,
    mut connected: MessageReader<ConnectionEvent>,
    mut lost: MessageReader<ConnectionLostEvent>,
    mut world: ResMut<ServerWorld>,
    mut server: ResMut<QuinnetServer>,
) {
    let now = time.elapsed_secs();
    for ev in connected.read() {
        info!("Client {} vừa kết nối, chờ Hello", ev.id);
        world.pending.insert(ev.id, now);
    }
    let Some(ep) = server.get_endpoint_mut() else { return };
    for ev in lost.read() {
        world.pending.remove(&ev.id);
        if let Some(p) = world.players.remove(&ev.id) {
            info!("{} (#{}) đã thoát", p.name, ev.id);
            save_player(&store.0, &p);
            ep.try_broadcast_message_on(CH_RELIABLE, ServerMsg::PlayerLeft { id: ev.id });
        }
    }
    // Kết nối nhưng không chào hỏi thì đá ra, tránh chiếm chỗ
    let stale: Vec<u64> = world
        .pending
        .iter()
        .filter(|(_, t)| now - **t > HELLO_TIMEOUT)
        .map(|(id, _)| *id)
        .collect();
    for id in stale {
        world.pending.remove(&id);
        ep.try_disconnect_client(id);
    }
}

fn receive_messages(store: Res<ServerStore>, time: Res<Time>, mut server: ResMut<QuinnetServer>, mut world: ResMut<ServerWorld>) {
    let Some(ep) = server.get_endpoint_mut() else { return };
    let now = time.elapsed_secs();

    let mut inbox: Vec<(u64, ClientMsg)> = Vec::new();
    for id in ep.clients() {
        for channel in [CH_RELIABLE, CH_UNRELIABLE] {
            let mut count = 0;
            while let Some(msg) = ep.try_receive_message_from::<ClientMsg, u8>(id, channel) {
                inbox.push((id, msg));
                count += 1;
                if count >= MAX_MESSAGES_PER_CLIENT_PER_TICK {
                    break;
                }
            }
        }
    }

    for (id, msg) in inbox {
        match msg {
            ClientMsg::Hello { version, name, appearance, key } => {
                if world.players.contains_key(&id) {
                    continue;
                }
                if version != PROTOCOL_VERSION {
                    ep.try_send_message_on(
                        id,
                        CH_RELIABLE,
                        ServerMsg::Rejected {
                            reason: format!("Sai phiên bản giao thức (server {PROTOCOL_VERSION}, client {version})"),
                        },
                    );
                    ep.try_disconnect_client(id);
                    continue;
                }
                if world.players.len() >= MAX_PLAYERS {
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Rejected { reason: "Server đã đầy".to_string() });
                    ep.try_disconnect_client(id);
                    continue;
                }
                let Some(account) = redstoric_store::account_id(&key) else {
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Rejected { reason: "Khoá tài khoản không hợp lệ".to_string() });
                    ep.try_disconnect_client(id);
                    continue;
                };
                if world.players.values().any(|p| p.account == account) {
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Rejected { reason: "Tài khoản này đang đăng nhập ở nơi khác".to_string() });
                    ep.try_disconnect_client(id);
                    continue;
                }
                world.pending.remove(&id);
                let saved = store.0.load_player(&account);
                let name = sanitize_name(&name, id);
                let appearance = sanitize_appearance(appearance);
                info!("{name} (#{id}) vào server");

                let (start_pos, class, hp, progress, logins) = match &saved {
                    Some(r) if r.pos.iter().all(|v| v.is_finite()) => (
                        r.pos,
                        r.class,
                        if r.hp > 0.0 { r.hp.min(PLAYER_MAX_HP) } else { PLAYER_MAX_HP },
                        r.progress.clone(),
                        r.logins,
                    ),
                    _ => ([0.0, 25.0, 0.0], 0, PLAYER_MAX_HP, String::new(), 0),
                };
                ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Welcome { your_id: id, seed: WORLD_SEED, progress: progress.clone() });
                if saved.is_some() {
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Teleport { pos: start_pos });
                }
                for boss in &world.defeated {
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::BossDown { boss: *boss });
                }
                ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Env { hour: world.env.hour, rain: world.env.rain });
                // Báo cho người mới biết những ai đang online, và báo cho người cũ biết có người mới
                for (other_id, other) in &world.players {
                    ep.try_send_message_on(
                        id,
                        CH_RELIABLE,
                        ServerMsg::PlayerJoined {
                            id: *other_id,
                            name: other.name.clone(),
                            appearance: other.appearance.clone(),
                        },
                    );
                }
                let others: Vec<u64> = world.players.keys().copied().collect();
                ep.try_send_group_message_on(
                    others.iter(),
                    CH_RELIABLE,
                    ServerMsg::PlayerJoined { id, name: name.clone(), appearance: appearance.clone() },
                );
                world.players.insert(
                    id,
                    Player {
                        account,
                        progress,
                        logins: logins + 1,
                        name,
                        appearance,
                        state: PlayerState { pos: start_pos, class, ..Default::default() },
                        has_state: false,
                        hp,
                        last_state_t: now,
                        last_attack_t: f32::MIN,
                        last_chat_t: f32::MIN,
                    },
                );
            }

            ClientMsg::Progress(progress) => {
                let Some(p) = world.players.get_mut(&id) else { continue };
                if progress.len() <= redstoric_store::MAX_PROGRESS_BYTES && serde_json::from_str::<serde_json::Value>(&progress).is_ok() {
                    p.progress = progress;
                }
            }

            ClientMsg::Appearance(appearance) => {
                let appearance = sanitize_appearance(appearance);
                let Some(p) = world.players.get_mut(&id) else { continue };
                p.appearance = appearance.clone();
                let others: Vec<u64> = world.players.keys().copied().filter(|o| *o != id).collect();
                ep.try_send_group_message_on(
                    others.iter(),
                    CH_RELIABLE,
                    ServerMsg::PlayerAppearance { id, appearance },
                );
            }

            ClientMsg::State(state) => {
                let Some(p) = world.players.get_mut(&id) else { continue };
                if !state.is_valid() {
                    continue;
                }
                // Giữ trong biên thế giới hữu hạn
                let mut state = state;
                state.pos[0] = state.pos[0].clamp(-redstoric_common::WORLD_LIMIT, redstoric_common::WORLD_LIMIT);
                state.pos[2] = state.pos[2].clamp(-redstoric_common::WORLD_LIMIT, redstoric_common::WORLD_LIMIT);
                if !p.has_state {
                    p.state = state;
                    p.has_state = true;
                    p.last_state_t = now;
                    continue;
                }
                let dt = (now - p.last_state_t).clamp(0.05, 1.0);
                let dx = state.pos[0] - p.state.pos[0];
                let dz = state.pos[2] - p.state.pos[2];
                if (dx * dx + dz * dz).sqrt() > MAX_SPEED * dt + 3.0 {
                    // Di chuyển quá nhanh (lag lớn hoặc gian lận): kéo về vị trí cuối hợp lệ
                    warn!("{} (#{id}) di chuyển bất thường, đã kéo lại", p.name);
                    ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Teleport { pos: p.state.pos });
                    p.last_state_t = now;
                } else {
                    p.state = state;
                    p.last_state_t = now;
                }
            }

            ClientMsg::Attack { boss, class } => {
                let Some(p) = world.players.get_mut(&id) else { continue };
                // Sát thương, tầm và nhịp đánh theo chức nghiệp (bảng WEAPONS dùng chung với client)
                let weapons = &redstoric_common::rules::weapons::STATS;
                let def = &weapons[(class as usize).min(weapons.len() - 1)];
                let cooldown = (def.duration * 0.6).min(ATTACK_COOLDOWN.max(0.2));
                if p.hp <= 0.0 || now - p.last_attack_t < cooldown {
                    continue;
                }
                let player_xz = Vec2::new(p.state.pos[0], p.state.pos[2]);
                p.last_attack_t = now;
                let Some(b) = world.bosses.get_mut(&boss) else { continue };
                // Cận chiến: tầm chém cộng bán kính boss và độ trễ; đạn: tầm bay tối đa
                let reach = if def.range > 20.0 { def.range + 3.0 } else { def.range + PLAYER_REACH - 2.8 };
                if matches!(b.phase, Phase::Dying(_)) || b.pos.distance(player_xz) > reach {
                    continue;
                }
                b.hp = (b.hp - def.damage.max(PLAYER_DAMAGE * 0.2)).max(0.0);
            }

            ClientMsg::Fire { kind, pos, dir } => {
                if !world.players.contains_key(&id) || kind > 4 {
                    continue;
                }
                if !pos.iter().chain(dir.iter()).all(|v| v.is_finite() && v.abs() < 1.0e6) {
                    continue;
                }
                let origin = Vec2::new(pos[0], pos[2]);
                let near: Vec<u64> = world
                    .players
                    .iter()
                    .filter(|(other, p)| **other != id && Vec2::new(p.state.pos[0], p.state.pos[2]).distance(origin) < FX_RADIUS)
                    .map(|(other, _)| *other)
                    .collect();
                ep.try_send_group_message_on(near.iter(), CH_RELIABLE, ServerMsg::Fx { id, kind, pos, dir });
            }

            ClientMsg::Chat(text) => {
                let Some(p) = world.players.get_mut(&id) else {
                    continue;
                };
                // Chống spam: tối đa ~2 tin mỗi giây
                if now - p.last_chat_t < 0.5 {
                    continue;
                }
                let text = sanitize_chat(&text);
                if text.is_empty() {
                    continue;
                }
                p.last_chat_t = now;
                let name = p.name.clone();
                info!("[chat] {name}: {text}");
                let ids: Vec<u64> = world.players.keys().copied().collect();
                ep.try_send_group_message_on(ids.iter(), CH_RELIABLE, ServerMsg::Chat { id, name, text });
            }
        }
    }
}

/// Mỗi giây quét quanh người chơi, thấy đấu trường nào chưa có boss (và chưa bị hạ) thì tạo boss.
fn spawn_bosses(
    time: Res<Time>,
    mut timer: Local<f32>,
    generator: Res<WorldGenerator>,
    mut world: ResMut<ServerWorld>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 1.0;

    let centers: Vec<(i32, i32)> = world
        .players
        .values()
        .map(|p| (p.state.pos[0] as i32, p.state.pos[2] as i32))
        .collect();
    for (cx, cz) in centers {
        let r = BOSS_ACTIVATE_RADIUS;
        for site in generator.sites_in_area(cx - r, cz - r, cx + r, cz + r) {
            if !matches!(site.kind, SiteKind::BossArena) {
                continue;
            }
            let key = [site.x, site.z];
            if world.bosses.contains_key(&key) || world.defeated.contains(&key) {
                continue;
            }
            let home = Vec2::new(site.x as f32 + 0.5, site.z as f32 + 0.5);
            let kind = BossKind::for_tier(site.tier, site.seed);
            info!("Boss xuất hiện tại đấu trường {:?}", key);
            world.bosses.insert(
                key,
                ServerBoss { home, pos: home, yaw: 0.0, hp: BOSS_HEALTH * kind.hp_mult(), phase: Phase::Idle, kind },
            );
        }
    }
}

/// AI boss và hồi máu, chạy 20 lần/giây.
fn simulate(store: Res<ServerStore>, time: Res<Time>, mut world: ResMut<ServerWorld>, mut server: ResMut<QuinnetServer>) {
    let dt = time.delta_secs();
    let world = &mut *world;

    let targets: Vec<(u64, Vec2)> = world
        .players
        .iter()
        .filter(|(_, p)| p.hp > 0.0)
        .map(|(id, p)| (*id, Vec2::new(p.state.pos[0], p.state.pos[2])))
        .collect();

    let mut damage: Vec<(u64, f32)> = Vec::new();
    let mut downed: Vec<[i32; 2]> = Vec::new();

    for (key, boss) in world.bosses.iter_mut() {
        if let Phase::Dying(t) = boss.phase {
            let t = t - dt;
            if t <= 0.0 {
                downed.push(*key);
            } else {
                boss.phase = Phase::Dying(t);
            }
            continue;
        }
        if boss.hp <= 0.0 {
            boss.phase = Phase::Dying(DEATH_TIME);
            continue;
        }

        let nearest = targets
            .iter()
            .filter(|(_, p)| p.distance(boss.home) < LEASH_RANGE)
            .min_by(|a, b| a.1.distance(boss.pos).total_cmp(&b.1.distance(boss.pos)))
            .map(|(_, p)| *p);
        let dist = nearest.map(|p| p.distance(boss.pos)).unwrap_or(f32::MAX);

        let kind = boss.kind;
        let atk_range = ATTACK_RANGE * kind.range_mult();
        let atk_damage = ATTACK_DAMAGE * kind.damage_mult();
        let boss_speed = BOSS_SPEED * kind.speed_mult();
        boss.phase = match boss.phase {
            Phase::Idle => {
                if nearest.is_some() && dist < AGGRO_RANGE { Phase::Chase } else { Phase::Idle }
            }
            Phase::Chase => {
                if nearest.is_none() {
                    Phase::Idle
                } else if dist <= atk_range - 0.4 {
                    Phase::Windup(WINDUP * kind.windup_mult())
                } else {
                    Phase::Chase
                }
            }
            Phase::Windup(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    for (id, p) in &targets {
                        if p.distance(boss.pos) <= atk_range + 0.6 {
                            damage.push((*id, atk_damage));
                        }
                    }
                    Phase::Recover(RECOVER)
                } else {
                    Phase::Windup(t)
                }
            }
            Phase::Recover(t) => {
                let t = t - dt;
                if t <= 0.0 { Phase::Chase } else { Phase::Recover(t) }
            }
            Phase::Dying(t) => Phase::Dying(t),
        };

        if let Some(p) = nearest {
            if boss.phase != Phase::Idle && dist > 0.01 {
                let d = (p - boss.pos) / dist;
                boss.yaw = (-d.x).atan2(-d.y);
            }
        }

        let to_home = boss.home - boss.pos;
        let step = match boss.phase {
            Phase::Chase => nearest.map(|p| (p - boss.pos).normalize_or_zero() * boss_speed * dt),
            Phase::Idle if to_home.length() > 1.5 => Some(to_home.normalize_or_zero() * boss_speed * 0.6 * dt),
            _ => None,
        };
        if let Some(step) = step {
            boss.pos += step;
            let off = boss.pos - boss.home;
            if off.length() > ARENA_LIMIT {
                boss.pos = boss.home + off.normalize() * ARENA_LIMIT;
            }
        }

        if boss.phase == Phase::Idle {
            boss.hp = (boss.hp + BOSS_REGEN * dt).min(BOSS_HEALTH * kind.hp_mult());
        }
    }

    // Sát thương lên người chơi, và hồi sinh nếu gục
    let mut respawned: Vec<u64> = Vec::new();
    for (id, amount) in damage {
        if let Some(p) = world.players.get_mut(&id) {
            p.hp -= amount;
            if p.hp <= 0.0 {
                p.hp = PLAYER_MAX_HP;
                p.state.pos = RESPAWN_POINT;
                respawned.push(id);
            }
        }
    }
    for p in world.players.values_mut() {
        if p.hp < PLAYER_MAX_HP {
            p.hp = (p.hp + PLAYER_REGEN * dt).min(PLAYER_MAX_HP);
        }
    }

    let Some(ep) = server.get_endpoint_mut() else { return };
    for id in respawned {
        info!("Người chơi #{id} gục và hồi sinh");
        ep.try_send_message_on(id, CH_RELIABLE, ServerMsg::Teleport { pos: RESPAWN_POINT });
    }
    for key in downed {
        world.bosses.remove(&key);
        world.defeated.insert(key);
        info!("Boss {:?} đã bị hạ", key);
        ep.try_broadcast_message_on(CH_RELIABLE, ServerMsg::BossDown { boss: key });
        save_world(&store.0, world);
    }
}

fn send_snapshots(world: Res<ServerWorld>, mut server: ResMut<QuinnetServer>) {
    let Some(ep) = server.get_endpoint_mut() else { return };
    if world.players.is_empty() {
        return;
    }

    let states: Vec<(u64, PlayerState)> = world.players.iter().map(|(id, p)| (*id, p.state)).collect();
    let bosses: Vec<BossNet> = world
        .bosses
        .iter()
        .map(|(key, b)| BossNet {
            key: *key,
            x: b.pos.x,
            z: b.pos.y,
            yaw: b.yaw,
            hp: b.hp,
            max_hp: BOSS_HEALTH * b.kind.hp_mult(),
            phase: match b.phase {
                Phase::Idle => BossPhase::Idle,
                Phase::Chase => BossPhase::Chase,
                Phase::Windup(_) => BossPhase::Windup,
                Phase::Recover(_) => BossPhase::Recover,
                Phase::Dying(_) => BossPhase::Dying,
            },
        })
        .collect();

    for (id, me) in &world.players {
        let me_xz = Vec2::new(me.state.pos[0], me.state.pos[2]);
        let msg = ServerMsg::Snapshot {
            players: states.iter().filter(|(other, _)| other != id).cloned().collect(),
            // Giới hạn số boss để gói luôn nằm gọn trong một datagram
            bosses: bosses
                .iter()
                .filter(|b| Vec2::new(b.x, b.z).distance(me_xz) < SNAPSHOT_BOSS_RADIUS)
                .take(10)
                .cloned()
                .collect(),
            hp: me.hp,
            max_hp: PLAYER_MAX_HP,
        };
        ep.try_send_message_on(*id, CH_UNRELIABLE, msg);
    }
}

/// Đồng hồ thế giới và lịch mưa; phát cho mọi người mỗi giây.
fn advance_env(time: Res<Time>, mut world: ResMut<ServerWorld>, mut server: ResMut<QuinnetServer>) {
    let dt = time.delta_secs();
    let world = &mut *world;
    let env = &mut world.env;
    env.hour = (env.hour + dt / DAY_LENGTH * 24.0).rem_euclid(24.0);

    env.rain_timer -= dt;
    if env.rain_timer <= 0.0 {
        let mut r = env.rng;
        let mut next = || {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            (r >> 8) as f32 / 16_777_216.0
        };
        // Đang mưa kéo dài 1–3 phút, trời quang 2–5 phút
        let (rain, dur) = if env.rain > 0.5 { (0.0, 120.0 + next() * 120.0) } else { (1.0, 60.0 + next() * 120.0) };
        env.rng = r;
        env.rain = rain;
        env.rain_timer = dur;
        env.send_timer = 0.0; // báo ngay
    }

    env.send_timer -= dt;
    if env.send_timer > 0.0 || world.players.is_empty() {
        return;
    }
    world.env.send_timer = 1.0;
    let Some(ep) = server.get_endpoint_mut() else { return };
    let msg = ServerMsg::Env { hour: world.env.hour, rain: world.env.rain };
    let ids: Vec<u64> = world.players.keys().copied().collect();
    ep.try_send_group_message_on(ids.iter(), CH_RELIABLE, msg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use redstoric_protocol::{MAX_APPEARANCE_BYTES, MAX_NAME_CHARS};

    #[test]
    fn name_is_trimmed_and_stripped_of_control_chars() {
        assert_eq!(sanitize_name("Bob", 7), "Bob");
        assert_eq!(sanitize_name("Bob\n", 7), "Bob");
        assert_eq!(sanitize_name("B\u{7}ob", 7), "Bob");
    }

    #[test]
    fn name_is_length_limited() {
        let long = "a".repeat(MAX_NAME_CHARS * 5);
        assert_eq!(sanitize_name(&long, 1).chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn empty_name_falls_back_to_player_id() {
        assert_eq!(sanitize_name("", 42), "Player42");
        assert_eq!(sanitize_name("   ", 42), "Player42");
        assert_eq!(sanitize_name("\n\t", 5), "Player5");
    }

    #[test]
    fn oversized_appearance_is_dropped() {
        let ok = "x".repeat(MAX_APPEARANCE_BYTES);
        assert_eq!(sanitize_appearance(ok.clone()), ok);
        assert!(sanitize_appearance("x".repeat(MAX_APPEARANCE_BYTES + 1)).is_empty());
        assert!(sanitize_appearance(String::new()).is_empty());
    }
}
