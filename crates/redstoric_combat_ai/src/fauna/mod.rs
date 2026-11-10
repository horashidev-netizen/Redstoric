//! Động vật hoang dã, theo cách Veloren làm:
//! - **Spawn** (`redstoric_common::fauna`): gói thú theo vùng sinh thái + thời điểm trong ngày + cỡ bầy
//!   (`world/src/layer/wildlife.rs`), sinh khi chunk vào tầm và xoá khi đi xa.
//! - **AI** (`server/agent`): lang thang kiểu random-walk có `bearing` kéo về nhà (`idle` trong
//!   `action_nodes.rs`, chạy 30 tick/giây), dừng khi gặp vách/vực/nước, nhận biết người chơi qua
//!   `sight_dist`/`listen_dist` với thanh `awareness`, rồi Alert → chạy trốn (`flee`) hoặc đuổi cắn (sói),
//!   cả bầy cùng báo động, bỏ chạy khi máu dưới `flee_health`.
//! - **Hoạt ảnh** (`voxygen/anim/quadruped_*`): chu kỳ chân `lab`/`tempo`, nhún thân, đầu cúi gặm cỏ...
//!
//! Động vật chỉ tồn tại cục bộ trên mỗi client (không đồng bộ qua mạng); khi chơi online thú dữ không
//! gây sát thương vì máu người chơi do server quản lý.
mod fish;
mod layout;
pub mod model;

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use redstoric_common::fauna::{chunk_fauna, hash32, Alignment, DayPeriod, Psyche, Species};
use redstoric_common::rules::fauna as rules;
use redstoric_common::{chunk_in_world, BlockType, WorldGenerator, CHUNK_SIZE_X, CHUNK_SIZE_Z, WORLD_SEED};
use redstoric_core::character_creator::MODEL_SCALE;
use redstoric_core::daynight::TimeOfDay;
use redstoric_core::{ActionState, Character};
use redstoric_voxel::VoxelWorldMap;

use crate::types::{Health, HitFlash, Knockback, OnlineMode};
use model::{layout_of, PartRole};

/// Chunk trong bán kính này (Chebyshev) thì có thú.
const SPAWN_CHUNKS: i32 = 5;
/// Ra khỏi bán kính này thì thú của chunk bị xoá.
const DESPAWN_CHUNKS: i32 = 7;
const MAX_ANIMALS: usize = 45;
const CHUNKS_PER_TICK: usize = 3;
/// Tick của AI rảnh (Veloren chạy 30 TPS).
const TICK: f32 = 1.0 / 30.0;

#[derive(Component)]
pub struct Animal {
    pub species: Species,
    pub chunk: IVec2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Idle,
    Graze,
    Alert,
    Flee,
    Chase,
    Windup,
    Recover,
    Dead,
}

/// Bộ não con vật.
#[derive(Component)]
pub struct Agent {
    pub psyche: Psyche,
    pub home: Vec3,
    pub bearing: Vec2,
    pub awareness: f32,
    pub mood: Mood,
    pub state_t: f32,
    pub graze_len: f32,
    pub flee_timer: f32,
    pub lost_t: f32,
    pub threat: Option<Vec3>,
    pub tick_acc: f32,
    pub rng: u32,
    pub pack: u32,
    pub last_hp: f32,
    pub hurt: f32,
    /// Tốc độ ngang hiện tại và pha bước chân (cho hoạt ảnh)
    pub speed: f32,
    pub phase: f32,
    /// Góc đầu quay về phía người chơi, so với thân
    pub look: f32,
    pub yaw: f32,
}

impl Agent {
    fn new(species: Species, home: Vec3, seed: u32, pack: u32) -> Self {
        Self {
            psyche: species.psyche(),
            home,
            bearing: Vec2::ZERO,
            awareness: 0.0,
            mood: Mood::Idle,
            state_t: 0.0,
            graze_len: 5.0,
            flee_timer: 0.0,
            lost_t: 0.0,
            threat: None,
            tick_acc: 0.0,
            rng: seed | 1,
            pack,
            last_hp: species.max_health(),
            hurt: 0.0,
            speed: 0.0,
            phase: (seed & 0xff) as f32,
            look: 0.0,
            yaw: 0.0,
        }
    }
}

/// Khớp xương gắn vào model, hoạt ảnh điều khiển vị trí/xoay của nó.
#[derive(Component)]
pub struct AnimalBone {
    owner: Entity,
    species: Species,
    role: PartRole,
    rest: Vec3,
    seed: f32,
}

/// Các chunk đã sinh thú và danh sách thú của chunk đó.
#[derive(Resource, Default)]
pub struct FaunaChunks(HashMap<IVec2, Vec<Entity>>);

pub struct FaunaPlugin;

impl Plugin for FaunaPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("REDSTORIC_NO_FAUNA").is_some() {
            return;
        }
        app.init_resource::<FaunaChunks>().add_systems(
            Update,
            (fauna_spawn_system, fauna_ai_system, fauna_anim_system, fish::fish_ai_system, fish::fish_anim_system)
                .chain()
                .run_if(redstoric_voxel::preload::world_ready),
        );
    }
}

// ─────────────────────────────────────────────────────────────
// Tiện ích
// ─────────────────────────────────────────────────────────────

pub(super) fn rnd(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s >> 8) as f32 / 16_777_216.0
}

fn wrap_angle(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// Góc yaw để mặt (hướng -Z) nhìn theo `d` trên mặt phẳng XZ.
pub(super) fn yaw_of(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

struct Terrain<'a> {
    map: Option<&'a VoxelWorldMap>,
    wg: &'a WorldGenerator,
}

impl Terrain<'_> {
    fn ground_y(&self, x: f32, z: f32, y: f32) -> f32 {
        match self.map {
            Some(m) => m.get_walk_height(x, z, y).0,
            None => self.wg.get_surface_height(x as f64, z as f64) as f32 + 1.0,
        }
    }

    /// Độ cao sàn nếu bước tới (x, z) được: không có vách, vực sâu hay nước.
    fn walkable(&self, x: f32, z: f32, from_y: f32) -> Option<f32> {
        match self.map {
            Some(m) => {
                m.block_at(x, from_y.max(1.0), z)?;
                let (g, blocked) = m.get_walk_height(x, z, from_y);
                if blocked || g - from_y > 1.25 || from_y - g > 3.5 {
                    return None;
                }
                if m.block_at(x, g + 0.5, z) == Some(BlockType::Water) {
                    return None;
                }
                Some(g)
            }
            None => {
                let g = self.wg.get_surface_height(x as f64, z as f64);
                if g <= self.wg.sea_level + 1 {
                    return None;
                }
                let g = g as f32 + 1.0;
                ((g - from_y).abs() <= 3.0).then_some(g)
            }
        }
    }
}

/// Đi một bước theo `dir`; thử lệch hướng nếu bị chặn. Trả về hướng đã đi.
fn step(ter: &Terrain, p: &mut Vec3, dir: Vec2, speed: f32, dt: f32, tries: &[f32]) -> Option<Vec2> {
    for &a in tries {
        let d = Vec2::from_angle(a).rotate(dir);
        let n = Vec2::new(p.x, p.z) + d * speed * dt;
        if ter.walkable(n.x, n.y, p.y).is_some() {
            p.x = n.x;
            p.z = n.y;
            return Some(d);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────
// Spawn
// ─────────────────────────────────────────────────────────────

fn spawn_animal(
    commands: &mut Commands,
    assets: &AssetServer,
    species: Species,
    pos: Vec3,
    seed: u32,
    pack: u32,
    chunk: IVec2,
) -> Entity {
    let mut rng = seed | 1;
    let yaw = rnd(&mut rng) * TAU;
    let mut agent = Agent::new(species, pos, seed, pack);
    agent.yaw = yaw;
    let root = commands
        .spawn((
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            Animal { species, chunk },
            Health::new(species.max_health()),
            agent,
        ))
        .id();
    let model = commands
        .spawn((Transform::from_scale(Vec3::splat(MODEL_SCALE * species.scale())), Visibility::default()))
        .id();
    commands.entity(root).add_child(model);

    let seedf = (seed & 0xffff) as f32 * 0.01;
    let mut ids: Vec<Entity> = Vec::new();
    for part in layout_of(species) {
        let rest = Vec3::from(part.bone);
        let bone = commands
            .spawn((
                Transform::from_translation(rest),
                Visibility::default(),
                AnimalBone { owner: root, species, role: part.role, rest, seed: seedf },
            ))
            .id();
        let parent = part.parent.map(|i| ids[i]).unwrap_or(model);
        commands.entity(parent).add_child(bone);
        let handle = assets.load(GltfAssetLabel::Scene(0).from_asset(format!("placeholders/glb/npc/{}", part.glb)));
        commands.entity(bone).with_children(|c| {
            c.spawn((WorldAssetRoot(handle), Transform::from_translation(Vec3::from(part.center))));
        });
        ids.push(bone);
    }
    root
}

fn fauna_spawn_system(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    wg: Res<WorldGenerator>,
    tod: Option<Res<TimeOfDay>>,
    mut chunks: ResMut<FaunaChunks>,
    player: Query<&Transform, With<Character>>,
    animals: Query<(), With<Animal>>,
    fish_q: Query<(), With<fish::Fish>>,
    mut timer: Local<f32>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.5;
    let Ok(ptf) = player.single() else { return; };
    let pp = ptf.translation;
    let pc = IVec2::new(
        (pp.x / CHUNK_SIZE_X as f32).floor() as i32,
        (pp.z / CHUNK_SIZE_Z as f32).floor() as i32,
    );

    // Xoá thú của chunk đã ở xa
    let far: Vec<IVec2> = chunks
        .0
        .keys()
        .copied()
        .filter(|c| (c.x - pc.x).abs().max((c.y - pc.y).abs()) > DESPAWN_CHUNKS)
        .collect();
    for c in far {
        if let Some(list) = chunks.0.remove(&c) {
            for e in list {
                commands.entity(e).try_despawn();
            }
        }
    }

    let mut count = animals.iter().count();
    let mut fish_count = fish_q.iter().count();
    if count >= MAX_ANIMALS && fish_count >= fish::MAX_FISH {
        return;
    }
    let period = DayPeriod::from_hour(tod.map(|t| t.hour).unwrap_or(9.0));

    // Chunk gần người chơi được xét trước
    let mut todo: Vec<(i32, IVec2)> = Vec::new();
    for dx in -SPAWN_CHUNKS..=SPAWN_CHUNKS {
        for dz in -SPAWN_CHUNKS..=SPAWN_CHUNKS {
            let c = pc + IVec2::new(dx, dz);
            if chunks.0.contains_key(&c) || !chunk_in_world(c.x, c.y) {
                continue;
            }
            todo.push((dx * dx + dz * dz, c));
        }
    }
    todo.sort_by_key(|(d, _)| *d);

    for (_, c) in todo.into_iter().take(CHUNKS_PER_TICK) {
        let mut list = Vec::new();
        let pack = hash32((c.x as u32).wrapping_mul(73_856_093) ^ (c.y as u32).wrapping_mul(19_349_663));
        for s in chunk_fauna(&wg, WORLD_SEED, c.x, c.y, period) {
            if count >= MAX_ANIMALS {
                break;
            }
            // Không sinh sát mặt người chơi
            if Vec2::new(s.x - pp.x, s.z - pp.z).length() < 12.0 {
                continue;
            }
            let y = wg.get_surface_height(s.x as f64, s.z as f64) as f32 + 1.0;
            list.push(spawn_animal(&mut commands, &assets, s.species, Vec3::new(s.x, y, s.z), s.seed, pack, c));
            count += 1;
        }
        for f in redstoric_common::fauna::chunk_fish(&wg, WORLD_SEED, c.x, c.y) {
            if fish_count >= fish::MAX_FISH {
                break;
            }
            list.push(fish::spawn_fish(&mut commands, &assets, f, c));
            fish_count += 1;
        }
        chunks.0.insert(c, list);
    }
}

// ─────────────────────────────────────────────────────────────
// AI
// ─────────────────────────────────────────────────────────────

fn fauna_ai_system(
    time: Res<Time>,
    online: Res<OnlineMode>,
    wg: Res<WorldGenerator>,
    map: Option<Res<VoxelWorldMap>>,
    mut commands: Commands,
    mut player: Query<(&Transform, &Character, &mut Health), Without<Animal>>,
    mut animals: Query<
        (Entity, &Animal, &mut Agent, &mut Transform, &Health, Option<&mut Knockback>, Option<&mut HitFlash>),
        Without<Character>,
    >,
) {
    let dt = time.delta_secs().min(0.1);
    let Ok((ptf, pchar, mut phealth)) = player.single_mut() else { return; };
    let ppos = ptf.translation;
    let ter = Terrain { map: map.as_deref(), wg: &*wg };
    let mut pack_alerts: Vec<(u32, Vec3)> = Vec::new();

    for (entity, animal, mut ag, mut tf, health, kb, flash) in &mut animals {
        let sp = animal.species;
        let psy = ag.psyche;
        let mut p = tf.translation;

        // ── Đã chết: ngã xuống rồi biến mất
        if ag.mood == Mood::Dead {
            ag.state_t += dt;
            ag.speed = 0.0;
            let roll = (ag.state_t / 0.35).min(1.0) * 1.45;
            tf.rotation = Quat::from_rotation_y(ag.yaw) * Quat::from_rotation_z(roll);
            if ag.state_t > rules::DEATH_TIME {
                commands.entity(entity).try_despawn();
            }
            continue;
        }
        if health.current <= 0.0 {
            ag.mood = Mood::Dead;
            ag.state_t = 0.0;
            ag.yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
            continue;
        }
        let hp_frac = health.current / sp.max_health();

        // ── Bị đánh: nhận biết ngay và phản ứng
        if health.current < ag.last_hp - 0.01 {
            ag.awareness = 1.0;
            ag.threat = Some(ppos);
            ag.flee_timer = rules::FLEE_DURATION * 1.5;
            ag.lost_t = 0.0;
            ag.hurt = 0.25;
            ag.mood = if sp.alignment() == Alignment::Enemy && hp_frac >= psy.flee_health {
                Mood::Chase
            } else {
                Mood::Flee
            };
        }
        ag.last_hp = health.current;
        ag.hurt = (ag.hurt - dt).max(0.0);

        if let Some(mut f) = flash {
            f.timer -= dt;
            if f.timer <= 0.0 {
                commands.entity(entity).remove::<HitFlash>();
            }
        }
        if let Some(mut k) = kb {
            p.x += k.velocity.x * dt;
            p.z += k.velocity.z * dt;
            k.velocity *= (1.0 - 6.0 * dt).max(0.0);
            if k.velocity.length_squared() < 0.05 {
                commands.entity(entity).remove::<Knockback>();
            }
        }

        // ── Nhận biết người chơi (sight / listen + awareness)
        let to = ppos - p;
        let dist = to.length();
        let dir2 = Vec2::new(to.x, to.z).normalize_or_zero();
        let fwd3 = tf.forward();
        let fwd = Vec2::new(fwd3.x, fwd3.z).normalize_or_zero();
        let scale = if sp.alignment() == Alignment::Enemy { 1.0 } else { 0.7 };
        let sight = psy.sight_dist * psy.aggro_range_multiplier * scale;
        let quiet = match pchar.state {
            ActionState::Idle => 0.35,
            ActionState::Sprinting => 1.5,
            _ => 1.0,
        };
        let listen = psy.listen_dist * psy.aggro_range_multiplier * scale * quiet;
        let seen = dist < sight && fwd.dot(dir2) > -0.35;
        let heard = dist < listen;
        if seen || heard {
            let range = if seen { sight } else { listen };
            ag.awareness = (ag.awareness + dt * (0.9 + 2.2 * (1.0 - dist / range).clamp(0.0, 1.0))).min(1.2);
            ag.threat = Some(ppos);
            ag.lost_t = 0.0;
        } else {
            ag.awareness = (ag.awareness - 0.18 * dt).max(0.0);
            ag.lost_t += dt;
        }

        // ── Chuyển trạng thái
        let react = |ag: &mut Agent| {
            ag.flee_timer = rules::FLEE_DURATION;
            ag.state_t = 0.0;
            ag.mood = if sp.alignment() == Alignment::Enemy && hp_frac >= psy.flee_health {
                Mood::Chase
            } else {
                Mood::Flee
            };
        };
        let mood = ag.mood;
        match mood {
            Mood::Idle | Mood::Graze => {
                if ag.awareness >= 1.0 {
                    react(&mut *ag);
                } else if ag.awareness >= 0.4 {
                    ag.mood = Mood::Alert;
                    ag.state_t = 0.0;
                }
            }
            Mood::Alert => {
                if ag.awareness >= 1.0 {
                    react(&mut *ag);
                } else if ag.awareness < 0.15 {
                    ag.mood = Mood::Idle;
                }
            }
            Mood::Flee => {
                if seen || heard {
                    ag.flee_timer = rules::FLEE_DURATION;
                } else {
                    ag.flee_timer -= dt;
                }
                if ag.flee_timer <= 0.0 {
                    ag.mood = Mood::Idle;
                    ag.awareness = 0.0;
                }
            }
            Mood::Chase | Mood::Windup | Mood::Recover => {
                let home_far = Vec2::new(p.x - ag.home.x, p.z - ag.home.z).length() > rules::MAX_PATROL_DIST * 2.5;
                if hp_frac < psy.flee_health {
                    react(&mut *ag);
                } else if ag.lost_t > 4.0 || dist > sight * 2.0 || home_far {
                    ag.mood = Mood::Idle;
                    ag.awareness = 0.0;
                }
            }
            Mood::Dead => {}
        }

        // ── Hành vi theo trạng thái
        let mut moved_speed = 0.0;
        let mut face: Option<Vec2> = None;
        let mood = ag.mood;
        match mood {
            Mood::Idle | Mood::Graze => {
                if ag.mood == Mood::Graze {
                    ag.state_t += dt;
                    if ag.state_t >= ag.graze_len {
                        ag.mood = Mood::Idle;
                    }
                }
                // Lang thang kiểu Veloren: bearing random-walk + kéo về nhà, 30 tick/giây
                ag.tick_acc += dt;
                let mut ticks = 0;
                while ag.tick_acc >= TICK && ticks < 4 {
                    ag.tick_acc -= TICK;
                    ticks += 1;
                    let diff = Vec2::new(rnd(&mut ag.rng) - 0.5, rnd(&mut ag.rng) - 0.5);
                    let wf = psy.idle_wander_factor.max(0.0).sqrt() * psy.aggro_range_multiplier.max(0.0).sqrt();
                    let b = ag.bearing;
                    ag.bearing += (diff * 0.1 - b * 0.01) * wf;
                    let to_home = Vec2::new(ag.home.x - p.x, ag.home.z - p.z);
                    ag.bearing += to_home / (0.01 + rules::MAX_PATROL_DIST * psy.idle_wander_factor)
                        * 0.015
                        * psy.idle_wander_factor;
                    // Sắp đụng vách / vực / nước thì dừng lại
                    if ag.bearing.length() > 0.01 {
                        let ahead = Vec2::new(p.x, p.z) + ag.bearing.normalize() * 3.0;
                        if ter.walkable(ahead.x, ahead.y, p.y).is_none() {
                            ag.bearing *= 0.1;
                        }
                    }
                    // Thỉnh thoảng dừng lại gặm cỏ / nghỉ
                    if ag.mood == Mood::Idle && rnd(&mut ag.rng) < 0.0035 {
                        ag.mood = Mood::Graze;
                        ag.state_t = 0.0;
                        ag.graze_len = 4.0 + rnd(&mut ag.rng) * 6.0;
                    }
                }
                if ag.mood == Mood::Idle && ag.bearing.length() > 0.5 {
                    let spd = sp.walk_speed() * ag.bearing.length().min(1.0);
                    let dir = ag.bearing.normalize();
                    if let Some(d) = step(&ter, &mut p, dir, spd, dt, &[0.0, 0.6, -0.6]) {
                        moved_speed = spd;
                        face = Some(d);
                    } else {
                        ag.bearing *= 0.1;
                    }
                }
            }
            Mood::Alert => {
                face = Some(dir2);
            }
            Mood::Flee => {
                let from = ag.threat.unwrap_or(ppos);
                let away = Vec2::new(p.x - from.x, p.z - from.z).normalize_or(Vec2::X);
                let spd = sp.run_speed() * rules::MAX_FLEE_SPEED;
                let tries = [0.0, 0.5, -0.5, 1.0, -1.0, 1.7, -1.7, 2.5];
                if let Some(d) = step(&ter, &mut p, away, spd, dt, &tries) {
                    moved_speed = spd;
                    face = Some(d);
                } else {
                    face = Some(away);
                }
            }
            Mood::Chase => {
                face = Some(dir2);
                if dist <= rules::BITE_RANGE {
                    ag.mood = Mood::Windup;
                    ag.state_t = 0.0;
                } else {
                    let spd = sp.run_speed() * 0.9;
                    if let Some(d) = step(&ter, &mut p, dir2, spd, dt, &[0.0, 0.5, -0.5, 1.0, -1.0]) {
                        moved_speed = spd;
                        face = Some(d);
                    }
                }
            }
            Mood::Windup => {
                face = Some(dir2);
                ag.state_t += dt;
                if ag.state_t >= rules::BITE_WINDUP {
                    if dist <= rules::BITE_RANGE + 0.9 && !online.0 {
                        phealth.current = (phealth.current - sp.bite_damage()).max(0.0);
                    }
                    ag.mood = Mood::Recover;
                    ag.state_t = 0.0;
                }
            }
            Mood::Recover => {
                face = Some(dir2);
                ag.state_t += dt;
                if ag.state_t >= rules::BITE_RECOVER {
                    ag.mood = Mood::Chase;
                }
            }
            Mood::Dead => {}
        }

        // Cả bầy báo động khi một con bắt đầu săn
        if ag.mood == Mood::Chase {
            pack_alerts.push((ag.pack, p));
        }

        // ── Xoay mặt, bám sát mặt đất, ghi hoạt ảnh
        let cur_yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
        if let Some(d) = face {
            if d.length_squared() > 1e-4 {
                let k = (if matches!(ag.mood, Mood::Flee | Mood::Chase) { 10.0 } else { 6.0 } * dt).min(1.0);
                tf.rotation = tf.rotation.slerp(Quat::from_rotation_y(yaw_of(d)), k);
            }
        }
        let want_look = wrap_angle(yaw_of(dir2) - cur_yaw).clamp(-0.9, 0.9);
        let want_look = if ag.awareness > 0.3 { want_look } else { 0.0 };
        ag.look += (want_look - ag.look) * (6.0 * dt).min(1.0);
        ag.yaw = cur_yaw;

        let gy = ter.ground_y(p.x, p.z, p.y);
        p.y = if gy > p.y { (p.y + 10.0 * dt).min(gy) } else { (p.y - 16.0 * dt).max(gy) };
        tf.translation = p;

        ag.speed += (moved_speed - ag.speed) * (10.0 * dt).min(1.0);
        if ag.speed > 0.05 {
            let floor = 1.5;
            ag.phase += ag.speed.max(floor) * dt;
        }
    }

    // Báo động cho các thành viên khác trong bầy
    if !pack_alerts.is_empty() {
        for (_, animal, mut ag, tf, _, _, _) in &mut animals {
            if animal.species.alignment() != Alignment::Enemy {
                continue;
            }
            if matches!(ag.mood, Mood::Idle | Mood::Graze | Mood::Alert)
                && pack_alerts
                    .iter()
                    .any(|(id, pos)| *id == ag.pack && pos.distance(tf.translation) < 30.0)
            {
                ag.awareness = 1.0;
                ag.mood = Mood::Chase;
                ag.threat = Some(ppos);
                ag.flee_timer = rules::FLEE_DURATION;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Hoạt ảnh
// ─────────────────────────────────────────────────────────────

fn fauna_anim_system(
    time: Res<Time>,
    agents: Query<&Agent>,
    mut bones: Query<(&AnimalBone, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let k = (14.0 * dt).min(1.0);
    let now = time.elapsed_secs();

    for (bone, mut tf) in &mut bones {
        let Ok(ag) = agents.get(bone.owner) else { continue; };
        let sp = bone.species;
        let small = matches!(sp, Species::Pig | Species::Rabbit);
        let t = now + bone.seed;

        // Chu kỳ chạy như `quadruped_*/run.rs` của Veloren
        let lab = 0.62;
        let tempo = sp.tempo();
        let sn = (ag.speed / 12.0).min(1.0).powf(0.4);
        let a = ag.phase * lab * tempo;
        let short = (a + PI).sin() * sn;
        let shortalt = (a + PI * 0.5).sin() * sn;
        let footvert = a.sin() * sn;
        let footvertt = (a + PI * 0.4).sin() * sn;
        let footvertf = (a + PI * 0.3).sin() * sn;
        let footverttf = (a + PI * 0.7).sin() * sn;
        let (spring, maximize) = match sp {
            Species::Pig => (0.6, 0.9),
            Species::Rabbit => (1.3, 1.2),
            Species::Deer => (0.9, 0.75),
            Species::Wolf => (0.8, 0.8),
        };
        let graze_pitch = match sp {
            Species::Rabbit => -0.5,
            _ => -0.65,
        };

        let mut off = Vec3::ZERO;
        let mut rot = Quat::IDENTITY;
        let mut scale = Vec3::ONE;

        match bone.role {
            PartRole::Body => {
                off.y = (2.0 * sn * spring + shortalt * 3.0 * spring) * if small { 1.0 } else { 0.5 };
                rot = Quat::from_rotation_x(short * 0.2 * spring * 0.4);
                if sn < 0.1 {
                    let b = (t * 2.2).sin() * 0.012;
                    scale = Vec3::new(1.0, 1.0 + b, 1.0);
                }
                if ag.hurt > 0.0 {
                    scale *= Vec3::new(1.0 + ag.hurt * 0.5, 1.0 - ag.hurt * 0.5, 1.0 + ag.hurt * 0.5);
                }
                if matches!(ag.mood, Mood::Windup) {
                    // Lấy đà: người rướn ra sau
                    rot = Quat::from_rotation_x(0.12);
                }
            }
            PartRole::Rear => {
                rot = Quat::from_rotation_y(short * 0.05);
            }
            PartRole::Neck => {
                if !small {
                    let pitch = match ag.mood {
                        Mood::Graze => graze_pitch * 0.7,
                        Mood::Alert => 0.15,
                        Mood::Windup => 0.2,
                        _ => short * 0.08,
                    };
                    rot = Quat::from_rotation_x(pitch);
                }
            }
            PartRole::Head => {
                let pitch = match ag.mood {
                    Mood::Graze => graze_pitch * if small { 1.0 } else { 0.6 },
                    Mood::Alert => 0.15,
                    Mood::Windup => 0.35,
                    Mood::Recover => -0.25,
                    _ => short * 0.12 + (t * 0.7).sin() * 0.03,
                };
                rot = Quat::from_rotation_y(ag.look) * Quat::from_rotation_x(pitch);
            }
            PartRole::Jaw => {
                let open = match ag.mood {
                    Mood::Windup => -0.6,
                    Mood::Recover => -0.15,
                    Mood::Graze => (t * 6.0).sin().max(0.0) * -0.15,
                    _ => -0.04,
                };
                rot = Quat::from_rotation_x(open);
            }
            PartRole::Ears => {
                let back = if matches!(ag.mood, Mood::Chase | Mood::Windup | Mood::Flee) { -0.35 } else { 0.0 };
                rot = Quat::from_rotation_x(back + (t * 2.0).sin() * 0.04);
            }
            PartRole::Tail => {
                let panic = if matches!(ag.mood, Mood::Flee) { 1.8 } else { 1.0 };
                rot = Quat::from_rotation_y((t * 3.0).sin() * 0.2 * panic + short * 0.3)
                    * Quat::from_rotation_x(0.1);
            }
            PartRole::LegFL => {
                rot = Quat::from_rotation_x(0.2 * sn + maximize * footverttf * 0.65);
                off.y = (footvertf * -1.5).max(-1.0);
            }
            PartRole::LegFR => {
                rot = Quat::from_rotation_x(0.2 * sn + maximize * footvertt * 0.65);
                off.y = (footvert * -1.5).max(-1.0);
            }
            PartRole::LegBL => {
                rot = Quat::from_rotation_x(-0.25 * sn + maximize * footvertt * -0.8);
                off.y = (footvert * 1.5).max(-1.0);
            }
            PartRole::LegBR => {
                rot = Quat::from_rotation_x(-0.25 * sn + maximize * footverttf * -0.8);
                off.y = (footvertf * 1.5).max(-1.0);
            }
            PartRole::FootFL => rot = Quat::from_rotation_x((-footverttf * 0.5).max(0.0)),
            PartRole::FootFR => rot = Quat::from_rotation_x((-footvertt * 0.5).max(0.0)),
            PartRole::FootBL => rot = Quat::from_rotation_x((footvertt * 0.6).max(0.0)),
            PartRole::FootBR => rot = Quat::from_rotation_x((footverttf * 0.6).max(0.0)),
            PartRole::FinL | PartRole::FinR => {}
        }

        tf.translation = tf.translation.lerp(bone.rest + off, k);
        tf.rotation = tf.rotation.slerp(rot, k);
        tf.scale = tf.scale.lerp(scale, k);
    }
}
