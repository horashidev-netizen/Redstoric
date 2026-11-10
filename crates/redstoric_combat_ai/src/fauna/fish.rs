//! Cá nhỏ bơi trong hồ và sông. Chuyển động 3D kiểu `fish_small` của Veloren: bơi lang thang, quanh
//! quẩn gần điểm nhà, quay đầu khi gặp bờ, hoảng sợ lao đi khi người chơi lại gần.
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::prelude::*;
use redstoric_common::fauna::{FishKind, FishSpawn};
use redstoric_common::BlockType;
use redstoric_core::character_creator::MODEL_SCALE;
use redstoric_core::Character;
use redstoric_voxel::VoxelWorldMap;

use super::model::{fish_layout_of, PartRole};
use super::{rnd, yaw_of};

pub const MAX_FISH: usize = 36;
/// Cá nhỏ hơn thú trên cạn
const FISH_SCALE: f32 = 0.7;

#[derive(Component)]
pub struct Fish {
    home: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    target_speed: f32,
    speed_t: f32,
    flee_t: f32,
    phase: f32,
    rng: u32,
}

#[derive(Component)]
pub struct FishBone {
    owner: Entity,
    role: PartRole,
    rest: Vec3,
}

pub fn spawn_fish(commands: &mut Commands, assets: &AssetServer, f: FishSpawn, _chunk: IVec2) -> Entity {
    let mut rng = f.seed | 1;
    let yaw = rnd(&mut rng) * TAU;
    let pos = Vec3::new(f.x, f.y, f.z);
    let root = commands
        .spawn((
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
            Fish {
                home: pos,
                yaw,
                pitch: 0.0,
                speed: 0.8,
                target_speed: 0.8,
                speed_t: 0.0,
                flee_t: 0.0,
                phase: rnd(&mut rng) * TAU,
                rng,
            },
        ))
        .id();
    let model = commands
        .spawn((Transform::from_scale(Vec3::splat(MODEL_SCALE * FISH_SCALE)), Visibility::default()))
        .id();
    commands.entity(root).add_child(model);
    let mut ids: Vec<Entity> = Vec::new();
    for part in fish_layout_of(f.kind) {
        let rest = Vec3::from(part.bone);
        let bone = commands
            .spawn((Transform::from_translation(rest), Visibility::default(), FishBone { owner: root, role: part.role, rest }))
            .id();
        let parent = part.parent.map(|i| ids[i]).unwrap_or(model);
        commands.entity(parent).add_child(bone);
        let handle = assets.load(GltfAssetLabel::Scene(0).from_asset(format!("placeholders/glb/npc/{}", part.glb)));
        commands.entity(bone).with_children(|c| {
            c.spawn((WorldAssetRoot(handle), Transform::from_translation(Vec3::from(part.center))));
        });
        ids.push(bone);
    }
    let _ = FishKind::Clownfish;
    root
}

pub fn fish_ai_system(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    player: Query<&Transform, (With<Character>, Without<Fish>)>,
    mut fishes: Query<(&mut Fish, &mut Transform), Without<Character>>,
) {
    let Some(map) = map else { return; };
    let dt = time.delta_secs().min(0.1);
    let ppos = player.single().map(|t| t.translation).ok();
    let is_water = |p: Vec3| map.block_at(p.x, p.y, p.z) == Some(BlockType::Water);

    for (mut f, mut tf) in &mut fishes {
        let mut p = tf.translation;

        // Hoảng sợ khi người chơi lại gần: quay đầu bỏ chạy
        if let Some(pp) = ppos {
            let d = p - pp;
            if d.length() < 6.0 {
                f.flee_t = 1.5;
                f.yaw = yaw_of(Vec2::new(d.x, d.z));
            }
        }
        f.flee_t = (f.flee_t - dt).max(0.0);

        // Lang thang: đổi hướng nhẹ, đổi tốc độ mỗi vài giây, bơi về nhà nếu đi xa
        f.speed_t -= dt;
        if f.speed_t <= 0.0 {
            f.speed_t = 1.5 + rnd(&mut f.rng) * 2.5;
            f.target_speed = 0.5 + rnd(&mut f.rng) * 1.1;
        }
        let jitter = (rnd(&mut f.rng) - 0.5) * 2.6 * dt;
        f.yaw += jitter;
        let to_home = Vec2::new(f.home.x - p.x, f.home.z - p.z);
        if to_home.length() > 7.0 && f.flee_t <= 0.0 {
            let want = yaw_of(to_home);
            let diff = (want - f.yaw + PI).rem_euclid(TAU) - PI;
            f.yaw += diff * (1.5 * dt).min(1.0);
        }
        let target = if f.flee_t > 0.0 { 4.2 } else { f.target_speed };
        f.speed += (target - f.speed) * (4.0 * dt).min(1.0);

        // Độ sâu: bơi về độ sâu của đàn
        let vy = ((f.home.y - p.y) * 0.6).clamp(-0.7, 0.7);
        let fwd = Vec3::new(-f.yaw.sin(), 0.0, -f.yaw.cos());
        let next = p + fwd * f.speed * dt + Vec3::Y * vy * dt;
        let ahead = p + fwd * 0.9;
        if is_water(next) && is_water(ahead) {
            p = next;
        } else {
            // Gặp bờ: quay ngang
            f.yaw += FRAC_PI_2 + rnd(&mut f.rng);
            f.speed *= 0.5;
        }
        f.pitch += (-(vy / f.speed.max(0.3)).clamp(-0.5, 0.5) * 0.5 - f.pitch) * (4.0 * dt).min(1.0);
        f.phase += (f.speed * 4.0 + 2.0) * dt;
        tf.translation = p;
        tf.rotation = Quat::from_rotation_y(f.yaw) * Quat::from_rotation_x(f.pitch);
    }
}

pub fn fish_anim_system(time: Res<Time>, fishes: Query<&Fish>, mut bones: Query<(&FishBone, &mut Transform)>) {
    let k = (14.0 * time.delta_secs()).min(1.0);
    for (bone, mut tf) in &mut bones {
        let Ok(f) = fishes.get(bone.owner) else { continue; };
        let amp = 0.35 + f.speed * 0.12;
        let s = f.phase.sin();
        let rot = match bone.role {
            PartRole::Body => Quat::from_rotation_y(s * -0.08),
            PartRole::Tail => Quat::from_rotation_y(s * amp),
            PartRole::FinL => Quat::from_rotation_z(0.35 + (f.phase * 0.5).sin() * 0.3),
            PartRole::FinR => Quat::from_rotation_z(-0.35 - (f.phase * 0.5).sin() * 0.3),
            _ => Quat::IDENTITY,
        };
        tf.translation = bone.rest;
        tf.rotation = tf.rotation.slerp(rot, k);
    }
}
