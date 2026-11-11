//! Boss ở trung tâm đấu trường: chờ đến khi người chơi lại gần, đuổi theo, báo hiệu rồi vung đòn.
//!
//! Điểm spawn do bộ sinh thế giới tạo (`BossSpawnPoint`). Boss là entity độc lập (không nằm trong
//! chunk) nên không bị dỡ khi chunk ở xa; `BossRegistry` đảm bảo mỗi đấu trường chỉ có một boss
//! và boss đã bị hạ thì không xuất hiện lại.

use std::collections::HashSet;

use bevy::prelude::*;
use redstoric_common::bosses::BossKind;
use redstoric_common::structures::SiteKind;
use redstoric_common::WorldGenerator;
use redstoric_core::character_creator::MODEL_SCALE;
use redstoric_core::Character;
use redstoric_voxel::{BossSpawnPoint, VoxelWorldMap};

use crate::boss_models::{BossModel, Limb, CYCLOPS, MINOTAUR, OGRE, TROLL_MOUNTAIN, YETI};

use crate::types::{Health, HitFlash, Knockback};

// Số liệu cân bằng lấy từ `redstoric_common::rules` để khớp với server.
use redstoric_common::rules::boss as rules;
const BOSS_HEALTH: f32 = rules::HEALTH;
const AGGRO_RANGE: f32 = rules::AGGRO_RANGE;
const LEASH_RANGE: f32 = rules::LEASH_RANGE;
const SPEED: f32 = rules::SPEED;
const ATTACK_RANGE: f32 = rules::ATTACK_RANGE;
const ATTACK_DAMAGE: f32 = rules::ATTACK_DAMAGE;
const WINDUP: f32 = rules::WINDUP;
const RECOVER: f32 = rules::RECOVER;
pub const DEATH_TIME: f32 = rules::DEATH_TIME;

const BODY_COLOR: Color = Color::srgb(0.30, 0.10, 0.12);
const WINDUP_COLOR: Color = Color::srgb(0.95, 0.45, 0.10);
const FLASH_COLOR: Color = Color::srgb(1.0, 0.1, 0.1);

#[derive(Resource, Default)]
pub struct BossRegistry {
    alive: HashSet<IVec2>,
    defeated: HashSet<IVec2>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BossState {
    Idle,
    Chase,
    Windup(f32),
    Recover(f32),
    Dying(f32),
}

#[derive(Component)]
pub struct Boss {
    pub home: Vec3,
    pub key: IVec2,
    pub state: BossState,
    pub kind: BossKind,
}

/// Trạng thái hoạt ảnh đi/đánh của boss dùng model glb.
#[derive(Component)]
pub struct BossAnim {
    pub phase: f32,
    pub last: Vec3,
    pub speed: f32,
}

/// Khớp chi (tay/chân) của boss glb; hoạt ảnh xoay khớp này.
#[derive(Component)]
pub struct BossLimb {
    pub owner: Entity,
    pub limb: Limb,
}

fn model_of(kind: BossKind) -> Option<&'static BossModel> {
    match kind {
        BossKind::Guardian => None,
        BossKind::Ogre => Some(&OGRE),
        BossKind::Minotaur => Some(&MINOTAUR),
        BossKind::Yeti => Some(&YETI),
        BossKind::TrollMountain => Some(&TROLL_MOUNTAIN),
        BossKind::Cyclops => Some(&CYCLOPS),
    }
}

/// Loại boss của đấu trường chứa điểm `home` (theo bậc trong `WorldPlan`).
fn kind_at(generator: Option<&WorldGenerator>, home: Vec3) -> BossKind {
    let Some(g) = generator else {
        return BossKind::Guardian;
    };
    let (x, z) = (home.x.floor() as i32, home.z.floor() as i32);
    g.sites_in_area(x - 12, z - 12, x + 12, z + 12)
        .into_iter()
        .find(|s| s.kind == SiteKind::BossArena && s.dist(x, z) <= 12.0)
        .map(|s| BossKind::for_tier(s.tier, s.seed))
        .unwrap_or(BossKind::Guardian)
}

#[derive(Component)]
pub struct BossVisual {
    material: Handle<StandardMaterial>,
}

pub fn boss_spawn_system(
    mut commands: Commands,
    mut registry: ResMut<BossRegistry>,
    spawn_points: Query<&BossSpawnPoint, Added<BossSpawnPoint>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    generator: Option<Res<WorldGenerator>>,
) {
    for point in &spawn_points {
        let home = point.home;
        let key = IVec2::new(home.x.floor() as i32, home.z.floor() as i32);
        if registry.alive.contains(&key) || registry.defeated.contains(&key) {
            continue;
        }
        registry.alive.insert(key);
        let kind = kind_at(generator.as_deref(), home);

        // Boss người khổng lồ dựng từ các mảnh glb của Veloren
        if let Some(model) = model_of(kind) {
            spawn_glb_boss(&mut commands, &assets, home, key, kind, model);
            continue;
        }

        let body_mat = materials.add(StandardMaterial {
            base_color: BODY_COLOR,
            perceptual_roughness: 0.85,
            ..default()
        });
        let eye_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.2),
            emissive: LinearRgba::rgb(5.0, 3.5, 0.5),
            ..default()
        });

        let torso = meshes.add(Cuboid::new(2.0, 2.2, 1.3));
        let head = meshes.add(Cuboid::new(1.2, 1.1, 1.1));
        let arm = meshes.add(Cuboid::new(0.7, 2.0, 0.7));
        let leg = meshes.add(Cuboid::new(0.8, 1.3, 0.9));
        let horn = meshes.add(Cuboid::new(0.2, 0.7, 0.2));
        let eye = meshes.add(Cuboid::new(0.25, 0.18, 0.1));

        commands
            .spawn((
                Transform::from_translation(home),
                Visibility::default(),
                Boss { home, key, state: BossState::Idle, kind },
                BossVisual { material: body_mat.clone() },
                Health::new(BOSS_HEALTH * kind.hp_mult()),
                Name::new(kind.name()),
            ))
            .with_children(|b| {
                let m = |mesh: &Handle<Mesh>, x: f32, y: f32, z: f32| {
                    (Mesh3d(mesh.clone()), MeshMaterial3d(body_mat.clone()), Transform::from_xyz(x, y, z))
                };
                b.spawn(m(&torso, 0.0, 2.3, 0.0));
                b.spawn(m(&head, 0.0, 3.9, 0.0));
                b.spawn(m(&arm, -1.4, 2.3, 0.0));
                b.spawn(m(&arm, 1.4, 2.3, 0.0));
                b.spawn(m(&leg, -0.55, 0.65, 0.0));
                b.spawn(m(&leg, 0.55, 0.65, 0.0));
                b.spawn(m(&horn, -0.45, 4.7, 0.0));
                b.spawn(m(&horn, 0.45, 4.7, 0.0));
                for x in [-0.3, 0.3] {
                    b.spawn((
                        Mesh3d(eye.clone()),
                        MeshMaterial3d(eye_mat.clone()),
                        Transform::from_xyz(x, 3.95, -0.58),
                    ));
                }
            });
    }
}

fn spawn_glb_boss(
    commands: &mut Commands,
    assets: &AssetServer,
    home: Vec3,
    key: IVec2,
    kind: BossKind,
    model: &BossModel,
) {
    let root = commands
        .spawn((
            Transform::from_translation(home),
            Visibility::default(),
            Boss { home, key, state: BossState::Idle, kind },
            BossAnim { phase: 0.0, last: home, speed: 0.0 },
            Health::new(BOSS_HEALTH * kind.hp_mult()),
            Name::new(kind.name()),
        ))
        .id();
    let scale = MODEL_SCALE * kind.scale();
    let body = commands.spawn((Transform::from_scale(Vec3::splat(scale)), Visibility::default())).id();
    commands.entity(root).add_child(body);

    // Khớp tay/chân: thứ tự khớp với `BossModel::pivots`
    let limbs = [Limb::ArmL, Limb::ArmR, Limb::LegL, Limb::LegR];
    let mut pivot_ids: Vec<Entity> = Vec::with_capacity(4);
    for (i, limb) in limbs.iter().enumerate() {
        let id = commands
            .spawn((
                Transform::from_translation(Vec3::from(model.pivots[i])),
                Visibility::default(),
                BossLimb { owner: root, limb: *limb },
            ))
            .id();
        commands.entity(body).add_child(id);
        pivot_ids.push(id);
    }
    for part in model.parts {
        let parent = match part.limb {
            Limb::Body => body,
            Limb::ArmL => pivot_ids[0],
            Limb::ArmR => pivot_ids[1],
            Limb::LegL => pivot_ids[2],
            Limb::LegR => pivot_ids[3],
        };
        let handle = assets.load(GltfAssetLabel::Scene(0).from_asset(format!("placeholders/glb/npc/{}", part.glb)));
        commands.entity(parent).with_children(|c| {
            c.spawn((WorldAssetRoot(handle), Transform::from_translation(Vec3::from(part.pos))));
        });
    }
}

/// Hoạt ảnh boss glb: đi (tay chân đảo nhau), giơ tay lấy đà, bổ xuống khi nghỉ; dọn Knockback/HitFlash.
pub fn boss_limb_animation(
    time: Res<Time>,
    mut commands: Commands,
    mut bosses: Query<(Entity, &Boss, &Transform, &mut BossAnim, Option<&mut HitFlash>, Option<&Knockback>)>,
    mut limbs: Query<(&BossLimb, &mut Transform), Without<Boss>>,
) {
    let dt = time.delta_secs().max(1e-4);
    for (entity, boss, tf, mut anim, flash, knockback) in &mut bosses {
        if knockback.is_some() {
            commands.entity(entity).remove::<Knockback>();
        }
        if let Some(mut flash) = flash {
            flash.timer -= dt;
            if flash.timer <= 0.0 {
                commands.entity(entity).remove::<HitFlash>();
            }
        }
        let moved = Vec3::new(tf.translation.x - anim.last.x, 0.0, tf.translation.z - anim.last.z).length();
        anim.last = tf.translation;
        let speed = (moved / dt).min(12.0);
        anim.speed += (speed - anim.speed) * (dt * 8.0).min(1.0);
        anim.phase += anim.speed * dt * 0.9;
        let swing = (anim.speed / 4.0).min(1.0) * 0.65;
        let walk = anim.phase.sin() * swing;

        let windup_total = WINDUP * boss.kind.windup_mult();
        let arm_pitch = match boss.state {
            BossState::Windup(t) => -(1.0 - (t / windup_total).clamp(0.0, 1.0)) * 2.4,
            BossState::Recover(t) => 0.7 * (t / RECOVER).clamp(0.0, 1.0),
            _ => 0.0,
        };
        let attacking = matches!(boss.state, BossState::Windup(_) | BossState::Recover(_));

        for (limb, mut ltf) in &mut limbs {
            if limb.owner != entity {
                continue;
            }
            let pitch = match limb.limb {
                Limb::ArmL => if attacking { arm_pitch } else { -walk },
                Limb::ArmR => if attacking { arm_pitch } else { walk },
                Limb::LegL => walk,
                Limb::LegR => -walk,
                Limb::Body => 0.0,
            };
            ltf.rotation = Quat::from_rotation_x(pitch);
        }
    }
}

pub fn boss_ai_system(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    mut player: Query<(&Transform, &mut Health, &Character), Without<Boss>>,
    mut bosses: Query<(&mut Boss, &mut Transform, &mut Health), Without<Character>>,
) {
    let Some(map) = map else { return };
    let Ok((player_tf, mut player_health, pch)) = player.single_mut() else { return };
    let dodging = pch.rolling();
    let dt = time.delta_secs();

    for (mut boss, mut tf, mut health) in &mut bosses {
        if matches!(boss.state, BossState::Dying(_)) {
            continue;
        }
        if health.current <= 0.0 {
            boss.state = BossState::Dying(DEATH_TIME);
            continue;
        }
        let kind = boss.kind;
        let atk_range = ATTACK_RANGE * kind.range_mult();
        let pos = tf.translation;
        let chunk = (
            (pos.x / 16.0).floor() as i32,
            (pos.z / 16.0).floor() as i32,
        );
        if !map.chunks.contains_key(&chunk) {
            continue; // chunk chưa tải: đứng yên, tránh rơi xuống hư không
        }

        let to_player = Vec3::new(player_tf.translation.x - pos.x, 0.0, player_tf.translation.z - pos.z);
        let dist = to_player.length();
        let player_from_home = Vec3::new(
            player_tf.translation.x - boss.home.x,
            0.0,
            player_tf.translation.z - boss.home.z,
        )
        .length();

        boss.state = match boss.state {
            BossState::Idle => {
                if dist < AGGRO_RANGE && player_from_home < LEASH_RANGE {
                    BossState::Chase
                } else {
                    BossState::Idle
                }
            }
            BossState::Chase => {
                if player_from_home >= LEASH_RANGE {
                    BossState::Idle
                } else if dist <= atk_range - 0.4 {
                    BossState::Windup(WINDUP * kind.windup_mult())
                } else {
                    BossState::Chase
                }
            }
            BossState::Windup(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    if dist <= atk_range + 0.6 && !dodging {
                        player_health.current = (player_health.current - ATTACK_DAMAGE * kind.damage_mult()).max(0.0);
                    }
                    BossState::Recover(RECOVER)
                } else {
                    BossState::Windup(t)
                }
            }
            BossState::Recover(t) => {
                let t = t - dt;
                if t <= 0.0 { BossState::Chase } else { BossState::Recover(t) }
            }
            BossState::Dying(t) => BossState::Dying(t),
        };

        // Quay mặt về phía người chơi khi đang giao chiến
        if !matches!(boss.state, BossState::Idle) && dist > 0.01 {
            let d = to_player / dist;
            tf.rotation = Quat::from_rotation_y((-d.x).atan2(-d.z));
        }

        // Di chuyển: đuổi người chơi, hoặc quay về tâm khi rảnh
        let to_home = Vec3::new(boss.home.x - pos.x, 0.0, boss.home.z - pos.z);
        let (dir, speed) = match boss.state {
            BossState::Chase => (to_player.normalize_or_zero(), SPEED * kind.speed_mult()),
            BossState::Idle if to_home.length() > 1.5 => (to_home.normalize_or_zero(), SPEED * kind.speed_mult() * 0.6),
            _ => (Vec3::ZERO, 0.0),
        };
        if dir != Vec3::ZERO {
            let nx = pos.x + dir.x * speed * dt;
            let nz = pos.z + dir.z * speed * dt;
            let (h, blocked) = map.get_walk_height(nx, nz, pos.y);
            if !blocked && h - pos.y <= 1.25 {
                tf.translation = Vec3::new(nx, h, nz);
            }
        } else {
            tf.translation.y = map.get_walk_height(pos.x, pos.z, pos.y).0;
        }

        // Hồi máu khi không có ai để đánh
        if boss.state == BossState::Idle {
            health.current = (health.current + 15.0 * dt).min(health.max);
        }
    }
}

/// Đổi màu theo trạng thái (báo hiệu đòn đánh, chớp đỏ khi trúng đòn) và bỏ knockback.
pub fn boss_visual_system(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bosses: Query<(Entity, &Boss, &BossVisual, Option<&mut HitFlash>, Option<&Knockback>)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, boss, visual, flash, knockback) in &mut bosses {
        if knockback.is_some() {
            commands.entity(entity).remove::<Knockback>();
        }
        let mut color = if matches!(boss.state, BossState::Windup(_)) { WINDUP_COLOR } else { BODY_COLOR };
        if let Some(mut flash) = flash {
            flash.timer -= dt;
            if flash.timer > 0.0 {
                color = FLASH_COLOR;
            } else {
                commands.entity(entity).remove::<HitFlash>();
            }
        }
        if let Some(mut mat) = materials.get_mut(&visual.material) {
            if mat.base_color != color {
                mat.base_color = color;
            }
        }
    }
}

pub fn boss_death_system(
    time: Res<Time>,
    mut registry: ResMut<BossRegistry>,
    mut bosses: Query<(Entity, &mut Boss, &mut Transform)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut boss, mut tf) in &mut bosses {
        if let BossState::Dying(t) = boss.state {
            let t = t - dt;
            if t <= 0.0 {
                registry.alive.remove(&boss.key);
                registry.defeated.insert(boss.key);
                commands.entity(entity).despawn();
                bevy::log::info!("Boss đã bị hạ!");
            } else {
                boss.state = BossState::Dying(t);
                let fall = (1.0 - t / DEATH_TIME).clamp(0.0, 1.0);
                tf.rotation = Quat::from_rotation_x(-fall * std::f32::consts::FRAC_PI_2);
            }
        }
    }
}

/// Người chơi hồi máu chậm; hết máu thì hồi sinh ở điểm xuất phát.
pub fn player_health_system(
    time: Res<Time>,
    mut player: Query<(&mut Health, &mut Transform, &mut Character)>,
) {
    let dt = time.delta_secs();
    for (mut health, mut tf, mut character) in &mut player {
        if health.current <= 0.0 {
            health.current = health.max;
            tf.translation = Vec3::new(0.0, 40.0, 0.0);
            character.velocity_y = 0.0;
            bevy::log::info!("Bạn đã gục ngã và hồi sinh ở điểm xuất phát");
        } else if health.current < health.max {
            health.current = (health.current + 1.5 * dt).min(health.max);
        }
    }
}
