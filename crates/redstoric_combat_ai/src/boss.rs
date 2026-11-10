//! Boss ở trung tâm đấu trường: chờ đến khi người chơi lại gần, đuổi theo, báo hiệu rồi vung đòn.
//!
//! Điểm spawn do bộ sinh thế giới tạo (`BossSpawnPoint`). Boss là entity độc lập (không nằm trong
//! chunk) nên không bị dỡ khi chunk ở xa; `BossRegistry` đảm bảo mỗi đấu trường chỉ có một boss
//! và boss đã bị hạ thì không xuất hiện lại.

use std::collections::HashSet;

use bevy::prelude::*;
use redstoric_core::Character;
use redstoric_voxel::{BossSpawnPoint, VoxelWorldMap};

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
) {
    for point in &spawn_points {
        let home = point.home;
        let key = IVec2::new(home.x.floor() as i32, home.z.floor() as i32);
        if registry.alive.contains(&key) || registry.defeated.contains(&key) {
            continue;
        }
        registry.alive.insert(key);

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
                Boss { home, key, state: BossState::Idle },
                BossVisual { material: body_mat.clone() },
                Health::new(BOSS_HEALTH),
                Name::new("Obsidian Guardian"),
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

pub fn boss_ai_system(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    mut player: Query<(&Transform, &mut Health), (With<Character>, Without<Boss>)>,
    mut bosses: Query<(&mut Boss, &mut Transform, &mut Health), Without<Character>>,
) {
    let Some(map) = map else { return };
    let Ok((player_tf, mut player_health)) = player.single_mut() else { return };
    let dt = time.delta_secs();

    for (mut boss, mut tf, mut health) in &mut bosses {
        if matches!(boss.state, BossState::Dying(_)) {
            continue;
        }
        if health.current <= 0.0 {
            boss.state = BossState::Dying(DEATH_TIME);
            continue;
        }
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
                } else if dist <= ATTACK_RANGE - 0.4 {
                    BossState::Windup(WINDUP)
                } else {
                    BossState::Chase
                }
            }
            BossState::Windup(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    if dist <= ATTACK_RANGE + 0.6 {
                        player_health.current = (player_health.current - ATTACK_DAMAGE).max(0.0);
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
            BossState::Chase => (to_player.normalize_or_zero(), SPEED),
            BossState::Idle if to_home.length() > 1.5 => (to_home.normalize_or_zero(), SPEED * 0.6),
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
