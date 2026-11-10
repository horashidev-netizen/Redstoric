//! Phần client: kết nối tới server, gửi vị trí / đòn đánh, hiển thị người chơi khác và boss
//! theo trạng thái server gửi về.
//!
//! Chỉ kết nối khi có tham số `--connect <ip[:cổng]>` (hoặc biến môi trường `REDSTORIC_SERVER`).
//! Không có thì game chạy offline như cũ và không tạo luồng mạng nào.
//!
//! Khi vào server thành công, `OnlineMode` bật: AI boss và hồi máu cục bộ tắt, máu người chơi và
//! boss lấy từ server. Mất kết nối thì tự quay về chơi offline.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};

use bevy::prelude::*;
use bevy_quinnet::client::certificate::CertificateVerificationMode;
use bevy_quinnet::client::connection::{
    ClientAddrConfiguration, ConnectionEvent, ConnectionFailedEvent, ConnectionLostEvent,
};
use bevy_quinnet::client::{
    ClientConnectionConfiguration, ClientConnectionConfigurationDefaultables, QuinnetClient, QuinnetClientPlugin,
};
use redstoric_combat_ai::boss::{Boss, BossState, DEATH_TIME};
use redstoric_combat_ai::ranged::{spawn_remote_fx, BurstFx, FxAssets, FxKind, ProjKind, Projectile, RemoteFx};
use redstoric_combat_ai::{Health, OnlineMode, PendingBossHits};
use redstoric_core::character_creator::CharacterAppearance;
use redstoric_core::daynight::{EnvSync, TimeOfDay};
use redstoric_core::weapons::{Loadout, WEAPONS};
use redstoric_core::weather::Weather;
use redstoric_core::Character;
use redstoric_voxel::{VoxelWorldMap, WORLD_SEED};

use crate::protocol::*;

const SEND_INTERVAL: f32 = 0.05;
/// Tốc độ bám theo vị trí server (càng lớn càng sát nhưng càng giật).
const SMOOTHING: f32 = 15.0;

/// Cấu hình kết nối, đọc từ tham số dòng lệnh / biến môi trường.
#[derive(Resource, Clone, Debug)]
pub struct NetConfig {
    pub server: Option<SocketAddr>,
    pub name: String,
}

impl NetConfig {
    /// `--connect 192.168.1.5:6000 --name Tam`, hoặc `REDSTORIC_SERVER` / `REDSTORIC_NAME`.
    pub fn from_args() -> Self {
        // Điện thoại không có dòng lệnh: địa chỉ server được "nướng" vào APK lúc build bằng
        // `REDSTORIC_DEFAULT_SERVER=ip:6000 cargo apk build` (xem README).
        let mut server = std::env::var("REDSTORIC_SERVER").ok().or_else(|| option_env!("REDSTORIC_DEFAULT_SERVER").map(String::from));
        let mut name = std::env::var("REDSTORIC_NAME").ok().or_else(|| option_env!("REDSTORIC_DEFAULT_NAME").map(String::from));
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--connect" => server = args.next(),
                "--name" => name = args.next(),
                _ => {}
            }
        }
        let server = server.and_then(|text| {
            let text = if text.contains(':') { text } else { format!("{text}:{DEFAULT_PORT}") };
            match text.to_socket_addrs() {
                Ok(mut addrs) => addrs.find(|a| a.is_ipv4()),
                Err(err) => {
                    bevy::log::warn!("Địa chỉ server '{text}' không hợp lệ: {err}");
                    None
                }
            }
        });
        Self { server, name: name.unwrap_or_else(|| "Player".to_string()) }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum NetStatus {
    #[default]
    Offline,
    Connecting,
    Handshaking,
    Online,
    Failed(String),
}

#[derive(Resource, Default)]
pub struct NetClient {
    pub status: NetStatus,
    pub my_id: Option<u64>,
    remote: HashMap<u64, Entity>,
}

/// Người chơi khác, vị trí đích do server báo.
#[derive(Component)]
pub struct RemotePlayer {
    pub id: u64,
    target: Vec3,
    yaw: f32,
    initialized: bool,
    class: u8,
    attacking: bool,
}

/// Thanh vũ khí trên tay người chơi từ xa: màu theo chức nghiệp, vung khi đang đánh.
#[derive(Component)]
struct RemoteWeapon {
    material: Handle<StandardMaterial>,
}

/// Vị trí đích (mặt phẳng XZ) của boss, lấy từ server; độ cao do client dò theo địa hình.
#[derive(Component)]
pub struct NetBossTarget {
    pos: Vec2,
    yaw: f32,
}

#[derive(Component)]
struct NetStatusText;

pub struct NetClientPlugin;

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        let config = NetConfig::from_args();
        // Không kết nối thì không khởi tạo client QUIC (tránh tạo luồng tokio thừa)
        let offline = config.server.is_none();
        app.add_plugins(QuinnetClientPlugin { initialize_later: offline })
            .insert_resource(config)
            .init_resource::<NetClient>()
            .add_systems(Startup, (connect_on_start, setup_status_ui).run_if(resource_exists::<QuinnetClient>))
            .add_systems(
                Update,
                (
                    handle_connection_events,
                    handle_server_messages,
                    smooth_remote_entities,
                    send_state,
                    send_boss_hits,
                    send_fx,
                    update_remote_weapons,
                    send_appearance,
                    update_status_ui,
                )
                    .chain()
                    .run_if(resource_exists::<QuinnetClient>),
            );
    }
}

fn connect_on_start(config: Res<NetConfig>, mut client: ResMut<QuinnetClient>, mut net: ResMut<NetClient>) {
    let Some(server) = config.server else { return };
    let result = client.open_connection(ClientConnectionConfiguration {
        addr_config: ClientAddrConfiguration::from_addrs(server, SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0)),
        // Mã hoá TLS 1.3 nhưng chưa xác thực server (chấp nhận chứng chỉ tự ký). Ổn cho LAN / bạn bè.
        cert_mode: CertificateVerificationMode::SkipVerification,
        defaultables: ClientConnectionConfigurationDefaultables {
            send_channels_cfg: channels(),
            ..Default::default()
        },
    });
    match result {
        Ok(_) => {
            net.status = NetStatus::Connecting;
            info!("Đang kết nối tới server {server} ...");
        }
        Err(err) => {
            net.status = NetStatus::Failed(format!("{err:?}"));
            error!("Không mở được kết nối: {err:?}");
        }
    }
}

fn reset_session(
    commands: &mut Commands,
    net: &mut NetClient,
    online: &mut OnlineMode,
    status: NetStatus,
) {
    for (_, entity) in net.remote.drain() {
        commands.entity(entity).despawn();
    }
    net.my_id = None;
    net.status = status;
    online.0 = false;
}

fn handle_connection_events(
    mut commands: Commands,
    mut connected: MessageReader<ConnectionEvent>,
    mut failed: MessageReader<ConnectionFailedEvent>,
    mut lost: MessageReader<ConnectionLostEvent>,
    mut client: ResMut<QuinnetClient>,
    mut net: ResMut<NetClient>,
    mut online: ResMut<OnlineMode>,
    mut env_sync: ResMut<EnvSync>,
    config: Res<NetConfig>,
    appearance: Option<Res<CharacterAppearance>>,
) {
    for _ in connected.read() {
        net.status = NetStatus::Handshaking;
        let appearance = appearance
            .as_deref()
            .and_then(|a| serde_json::to_string(a).ok())
            .unwrap_or_default();
        if let Some(conn) = client.get_connection_mut() {
            conn.try_send_message_on(
                CH_RELIABLE,
                ClientMsg::Hello { version: PROTOCOL_VERSION, name: config.name.clone(), appearance },
            );
        }
    }
    for ev in failed.read() {
        warn!("Kết nối thất bại: {:?}", ev.err);
        reset_session(&mut commands, &mut net, &mut online, NetStatus::Failed(format!("{:?}", ev.err)));
        env_sync.active = false;
    }
    for _ in lost.read() {
        warn!("Mất kết nối tới server, chuyển về chơi offline");
        reset_session(&mut commands, &mut net, &mut online, NetStatus::Offline);
        env_sync.active = false;
    }
}

fn boss_key(key: [i32; 2]) -> IVec2 {
    IVec2::new(key[0], key[1])
}

fn handle_server_messages(
    mut commands: Commands,
    mut client: ResMut<QuinnetClient>,
    mut net: ResMut<NetClient>,
    mut online: ResMut<OnlineMode>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut player: Query<(&mut Transform, &mut Character, &mut Health)>,
    mut bosses: Query<(Entity, &mut Boss, &mut Health, Option<&mut NetBossTarget>), Without<Character>>,
    mut remotes: Query<&mut RemotePlayer>,
    mut tod: ResMut<TimeOfDay>,
    mut weather: ResMut<Weather>,
    mut env_sync: ResMut<EnvSync>,
    fx_assets: Option<Res<FxAssets>>,
) {
    let mut inbox: Vec<ServerMsg> = Vec::new();
    {
        let Some(conn) = client.get_connection_mut() else { return };
        for channel in [CH_RELIABLE, CH_UNRELIABLE] {
            while let Some(msg) = conn.try_receive_message_on::<ServerMsg, u8>(channel) {
                inbox.push(msg);
            }
        }
    }

    for msg in inbox {
        match msg {
            ServerMsg::Welcome { your_id, seed } => {
                net.my_id = Some(your_id);
                net.status = NetStatus::Online;
                online.0 = true;
                env_sync.active = true;
                info!("Đã vào server, id của bạn là #{your_id}");
                if seed != WORLD_SEED {
                    warn!("Hạt giống thế giới khác server ({seed} != {WORLD_SEED}): địa hình sẽ không khớp!");
                }
            }
            ServerMsg::Rejected { reason } => {
                warn!("Server từ chối: {reason}");
                net.status = NetStatus::Failed(reason);
                if let Some(conn) = client.get_connection_mut() {
                    conn.try_disconnect();
                }
            }
            ServerMsg::PlayerJoined { id, name, .. } => {
                if Some(id) == net.my_id || net.remote.contains_key(&id) {
                    continue;
                }
                info!("{name} (#{id}) đã vào");
                let entity = spawn_remote(&mut commands, &mut meshes, &mut materials, id, &name);
                net.remote.insert(id, entity);
            }
            ServerMsg::PlayerLeft { id } => {
                if let Some(entity) = net.remote.remove(&id) {
                    commands.entity(entity).despawn();
                }
            }
            // Ngoại hình người khác đã nhận nhưng hiện chưa dùng: người chơi từ xa còn là mô hình tạm.
            ServerMsg::PlayerAppearance { .. } => {}
            ServerMsg::Env { hour, rain } => {
                tod.hour = hour.rem_euclid(24.0);
                weather.target = rain.clamp(0.0, 1.0);
            }
            ServerMsg::Fx { id, kind, pos, dir } => {
                if Some(id) == net.my_id {
                    continue;
                }
                if let Some(assets) = fx_assets.as_ref() {
                    spawn_remote_fx(&mut commands, &mut materials, assets, kind, Vec3::from(pos), Vec3::from(dir).normalize_or_zero());
                }
            }
            ServerMsg::Teleport { pos } => {
                if let Ok((mut tf, mut character, _)) = player.single_mut() {
                    tf.translation = Vec3::from(pos);
                    character.velocity_y = 0.0;
                }
            }
            ServerMsg::BossDown { boss } => {
                let key = boss_key(boss);
                for (_, mut b, _, _) in &mut bosses {
                    if b.key == key && !matches!(b.state, BossState::Dying(_)) {
                        b.state = BossState::Dying(DEATH_TIME);
                    }
                }
            }
            ServerMsg::Snapshot { players, bosses: boss_list, hp, max_hp } => {
                if let Ok((_, _, mut health)) = player.single_mut() {
                    health.current = hp;
                    health.max = max_hp;
                }
                for (id, state) in players {
                    let Some(entity) = net.remote.get(&id) else { continue };
                    if let Ok(mut remote) = remotes.get_mut(*entity) {
                        remote.target = Vec3::from(state.pos);
                        remote.yaw = state.yaw;
                        remote.class = state.class;
                        remote.attacking = state.attacking;
                    }
                }
                for data in boss_list {
                    let key = boss_key(data.key);
                    for (entity, mut boss, mut health, target) in &mut bosses {
                        if boss.key != key {
                            continue;
                        }
                        health.current = data.hp;
                        health.max = data.max_hp;
                        boss.state = match data.phase {
                            BossPhase::Idle => BossState::Idle,
                            BossPhase::Chase => BossState::Chase,
                            BossPhase::Windup => BossState::Windup(0.3),
                            BossPhase::Recover => BossState::Recover(0.3),
                            BossPhase::Dying => match boss.state {
                                BossState::Dying(t) => BossState::Dying(t),
                                _ => BossState::Dying(DEATH_TIME),
                            },
                        };
                        let new_target = Vec2::new(data.x, data.z);
                        match target {
                            Some(mut t) => {
                                t.pos = new_target;
                                t.yaw = data.yaw;
                            }
                            None => {
                                commands.entity(entity).insert(NetBossTarget { pos: new_target, yaw: data.yaw });
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Mô hình tạm cho người chơi khác: khối hộp màu riêng theo id, có "mũi" chỉ hướng mặt (-Z).
fn spawn_remote(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: u64,
    name: &str,
) -> Entity {
    let color = Color::hsl((id as f32 * 47.0) % 360.0, 0.65, 0.55);
    let body = materials.add(StandardMaterial { base_color: color, perceptual_roughness: 0.8, ..default() });
    let legs = meshes.add(Cuboid::new(0.5, 0.7, 0.4));
    let torso = meshes.add(Cuboid::new(0.8, 0.8, 0.45));
    let head = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
    let nose = meshes.add(Cuboid::new(0.12, 0.12, 0.12));
    commands
        .spawn((
            Transform::from_xyz(0.0, -1000.0, 0.0), // ẩn cho tới khi nhận vị trí đầu tiên
            Visibility::default(),
            RemotePlayer { id, target: Vec3::new(0.0, -1000.0, 0.0), yaw: 0.0, initialized: false, class: 0, attacking: false },
            Name::new(format!("Remote {name}")),
        ))
        .with_children(|p| {
            let part = |mesh: &Handle<Mesh>, y: f32, z: f32| {
                (Mesh3d(mesh.clone()), MeshMaterial3d(body.clone()), Transform::from_xyz(0.0, y, z))
            };
            p.spawn(part(&legs, 0.35, 0.0));
            p.spawn(part(&torso, 1.1, 0.0));
            p.spawn(part(&head, 1.75, 0.0));
            p.spawn(part(&nose, 1.75, -0.3));
            // Vũ khí cầm ở tay phải (thanh dài, đổi màu theo chức nghiệp)
            let wmat = materials.add(StandardMaterial { base_color: Color::WHITE, unlit: true, ..default() });
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 1.1))),
                MeshMaterial3d(wmat.clone()),
                Transform::from_xyz(0.55, 1.0, -0.4),
                RemoteWeapon { material: wmat },
            ));
        })
        .id()
}

fn smooth_remote_entities(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    mut remotes: Query<(&mut Transform, &mut RemotePlayer), Without<Boss>>,
    mut bosses: Query<(&mut Transform, &NetBossTarget), With<Boss>>,
) {
    let dt = time.delta_secs();
    let k = 1.0 - (-SMOOTHING * dt).exp();

    for (mut tf, mut remote) in &mut remotes {
        // Chưa nhận snapshot nào thì chưa hiện; lần đầu hoặc lệch quá xa (dịch chuyển) thì nhảy thẳng tới
        if remote.target.y < -500.0 {
            continue;
        }
        if !remote.initialized || tf.translation.distance(remote.target) > 20.0 {
            tf.translation = remote.target;
            remote.initialized = true;
        } else {
            tf.translation = tf.translation.lerp(remote.target, k);
        }
        tf.rotation = tf.rotation.slerp(Quat::from_rotation_y(remote.yaw), k);
    }

    for (mut tf, target) in &mut bosses {
        let pos = tf.translation;
        let nx = pos.x + (target.pos.x - pos.x) * k;
        let nz = pos.z + (target.pos.y - pos.z) * k;
        let mut ny = pos.y;
        if let Some(map) = &map {
            let chunk = ((nx / 16.0).floor() as i32, (nz / 16.0).floor() as i32);
            if map.chunks.contains_key(&chunk) {
                ny = map.get_walk_height(nx, nz, pos.y).0;
            }
        }
        tf.translation = Vec3::new(nx, ny, nz);
        tf.rotation = tf.rotation.slerp(Quat::from_rotation_y(target.yaw), k);
    }
}

fn send_state(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut last: Local<Option<Vec3>>,
    net: Res<NetClient>,
    mut client: ResMut<QuinnetClient>,
    loadout: Res<Loadout>,
    player: Query<(&Transform, &Character)>,
) {
    if net.status != NetStatus::Online {
        return;
    }
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = SEND_INTERVAL;
    let Ok((tf, ch)) = player.single() else { return };
    let attacking = matches!(ch.state, redstoric_core::ActionState::Attacking(_));
    let pos = tf.translation;
    let previous: Option<Vec3> = *last;
    let moving = previous.map(|l| Vec2::new(pos.x - l.x, pos.z - l.z).length() > 0.02).unwrap_or(false);
    *last = Some(pos);
    let yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
    if let Some(conn) = client.get_connection_mut() {
        conn.try_send_message_on(
            CH_UNRELIABLE,
            ClientMsg::State(PlayerState { pos: pos.to_array(), yaw, moving, class: loadout.selected as u8, attacking }),
        );
    }
}

fn send_boss_hits(
    net: Res<NetClient>,
    loadout: Res<Loadout>,
    mut pending: ResMut<PendingBossHits>,
    mut client: ResMut<QuinnetClient>,
) {
    if pending.0.is_empty() {
        return;
    }
    if net.status != NetStatus::Online {
        pending.0.clear();
        return;
    }
    if let Some(conn) = client.get_connection_mut() {
        for key in pending.0.drain(..) {
            conn.try_send_message_on(CH_RELIABLE, ClientMsg::Attack { boss: [key.x, key.y], class: loadout.selected as u8 });
        }
    }
}

/// Gửi lên server các đạn / hiệu ứng vũ khí do chính mình vừa tạo để người khác thấy.
fn send_fx(
    net: Res<NetClient>,
    mut client: ResMut<QuinnetClient>,
    projectiles: Query<(&Transform, &Projectile), (Added<Projectile>, Without<RemoteFx>)>,
    bursts: Query<(&Transform, &BurstFx), (Added<BurstFx>, Without<RemoteFx>)>,
) {
    if net.status != NetStatus::Online {
        return;
    }
    let Some(conn) = client.get_connection_mut() else { return };
    for (tf, pr) in &projectiles {
        let kind = match pr.kind {
            ProjKind::Arrow => 0,
            ProjKind::Fireball => 1,
            ProjKind::HealBolt => 2,
        };
        conn.try_send_message_on(
            CH_RELIABLE,
            ClientMsg::Fire { kind, pos: tf.translation.to_array(), dir: tf.forward().as_vec3().to_array() },
        );
    }
    for (tf, fx) in &bursts {
        let kind = match fx.kind {
            FxKind::Slam => 3,
            FxKind::Spin => 4,
            _ => continue,
        };
        conn.try_send_message_on(
            CH_RELIABLE,
            ClientMsg::Fire { kind, pos: tf.translation.to_array(), dir: [0.0, 0.0, -1.0] },
        );
    }
}

/// Đổi màu thanh vũ khí theo chức nghiệp và vung khi người chơi đó đang đánh.
fn update_remote_weapons(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    remotes: Query<(&RemotePlayer, &Children)>,
    mut weapons: Query<(&RemoteWeapon, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (remote, children) in &remotes {
        let def = &WEAPONS[(remote.class as usize).min(WEAPONS.len() - 1)];
        let (r, g, b) = def.color;
        for child in children.iter().map(|c| c.clone()) {
            let Ok((weapon, mut tf)) = weapons.get_mut(child) else { continue };
            if let Some(mut m) = materials.get_mut(&weapon.material) {
                m.base_color = Color::srgb(r, g, b);
            }
            tf.rotation = if remote.attacking {
                Quat::from_rotation_x(-1.0 + (t * 16.0).sin() * 0.9)
            } else {
                Quat::from_rotation_x(0.35)
            };
        }
    }
}

fn send_appearance(
    net: Res<NetClient>,
    appearance: Option<Res<CharacterAppearance>>,
    mut client: ResMut<QuinnetClient>,
) {
    let Some(appearance) = appearance else { return };
    if net.status != NetStatus::Online || !appearance.is_changed() || appearance.is_added() {
        return;
    }
    if let (Ok(json), Some(conn)) = (serde_json::to_string(&*appearance), client.get_connection_mut()) {
        conn.try_send_message_on(CH_RELIABLE, ClientMsg::Appearance(json));
    }
}

fn setup_status_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        children![(
            Text::new(""),
            TextFont { font_size: bevy::text::FontSize::Px(14.0), ..default() },
            TextColor(Color::WHITE),
            NetStatusText,
        )],
    ));
}

fn update_status_ui(net: Res<NetClient>, config: Res<NetConfig>, mut text: Query<&mut Text, With<NetStatusText>>) {
    let Ok(mut text) = text.single_mut() else { return };
    let server = config.server.map(|s| s.to_string()).unwrap_or_default();
    let line = match &net.status {
        NetStatus::Offline => "Offline".to_string(),
        NetStatus::Connecting => format!("Đang kết nối {server} ..."),
        NetStatus::Handshaking => "Đang bắt tay với server ...".to_string(),
        NetStatus::Online => format!("Online ({server}) - {} người chơi", net.remote.len() + 1),
        NetStatus::Failed(reason) => format!("Lỗi mạng: {reason}"),
    };
    if **text != line {
        **text = line;
    }
}
