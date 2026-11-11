use bevy::prelude::*;
use crate::{ActionState, Character};

#[derive(Component)]
pub struct BoneHead;

#[derive(Component)]
pub struct BoneJaw;

#[derive(Component)]
pub struct BoneChest;

#[derive(Component)]
pub struct BoneShoulderLeft;

#[derive(Component)]
pub struct BoneShoulderRight;

#[derive(Component)]
pub struct BoneHandLeft;

#[derive(Component)]
pub struct BoneHandRight;

#[derive(Component)]
pub struct BoneLegLeft;

#[derive(Component)]
pub struct BoneLegRight;

#[derive(Component)]
pub struct BoneTail;

/// Tư thế đích của từng khớp, mỗi frame tiến dần về đó để chuyển trạng thái mượt.

/// Khớp của bộ model người. Gắn cho **cả người chơi lẫn dân làng** (các `Bone*` ở trên chỉ người chơi có,
/// vì hệ thống vũ khí/đòn đánh của người chơi dùng chúng).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Limb {
    /// Gốc thân (chứa toàn bộ model): nghiêng người, xoay khi lăn.
    Body,
    Chest,
    Head,
    ShoulderL,
    ShoulderR,
    LegL,
    LegR,
    Tail,
}

/// Bộ điều khiển hoạt ảnh trên entity gốc của một nhân vật (người chơi hoặc NPC).
/// Tốc độ và độ nghiêng được tính từ chuyển động thật của entity nên mọi nhân vật dùng chung một bộ hoạt ảnh.
#[derive(Component, Default)]
pub struct AnimDriver {
    phase: f32,
    last_pos: Option<Vec3>,
    last_yaw: f32,
    speed: f32,
    tilt: f32,
    limbs: Vec<(Entity, Limb)>,
    linked: bool,
}

#[derive(Default)]
struct Pose {
    leg_l: f32,
    leg_r: f32,
    arm_l: f32,
    arm_r: f32,
    arm_l_z: f32,
    arm_r_z: f32,
    chest_pitch: f32,
    chest_roll: f32,
    chest_scale: Vec3,
    head_pitch: f32,
    head_yaw: f32,
    tail: Quat,
    /// Xoay cả thân quanh trục X (lăn), radian
    body_spin: f32,
    /// Hạ thân (lăn / nhún)
    body_drop: f32,
}

struct PoseInput {
    state: ActionState,
    /// Tốc độ ngang thật (khối/giây)
    speed: f32,
    land_timer: f32,
    roll: f32,
    tilt: f32,
}

fn pose_for(i: &PoseInput, t_run: f32, t: f32) -> Pose {
    use ActionState::*;
    let mut p = Pose { chest_scale: Vec3::ONE, tail: Quat::from_rotation_x(0.25), ..default() };
    match i.state {
        Idle | Attacking(_) => {
            let breathe = (t * 2.2).sin();
            p.chest_scale = Vec3::new(1.0, 1.0 + breathe * 0.012, 1.0);
            p.arm_l_z = 0.05 + breathe * 0.02;
            p.arm_r_z = -0.05 - breathe * 0.02;
            p.tail = Quat::from_rotation_y((t * 3.0).sin() * 0.18) * Quat::from_rotation_x(0.25);
        }
        Running | Sprinting => {
            let sprint = i.state == Sprinting;
            // Đi chậm (NPC đi dạo) thì vung tay chân ít hơn, như `speednorm` của Veloren
            let walk = (i.speed / 6.0).clamp(0.35, 1.0);
            let (swing, arms, lean, bob) =
                if sprint { (1.0, 1.0, 0.28, 0.025) } else { (0.65 * walk, 0.55 * walk, 0.08 * walk, 0.0) };
            let s = t_run.sin();
            p.leg_l = s * swing;
            p.leg_r = -s * swing;
            p.arm_l = -s * arms;
            p.arm_r = s * arms;
            p.chest_pitch = -lean;
            p.chest_roll = (t_run * 0.5).sin() * 0.06 * walk;
            p.chest_scale = Vec3::new(1.0, 1.0 + (t_run * 2.0).sin().abs() * bob, 1.0);
            p.head_pitch = lean * 0.8 + (t_run * 0.5).sin() * 0.05;
            p.tail = Quat::from_rotation_y((t_run * 1.5).sin() * 0.35) * Quat::from_rotation_x(0.35 + lean);
            // Nghiêng vào cua (Veloren: chest rotation_z(tilt * -0.6), head nhìn theo hướng rẽ)
            p.chest_roll += i.tilt * 0.35;
            p.head_yaw = i.tilt * 1.2;
            // Chân dạng ra phía ngoài cua
            p.arm_l_z = 0.08 + i.tilt.max(0.0) * 0.6;
            p.arm_r_z = -0.08 + i.tilt.min(0.0) * 0.6;
        }
        Jumping => {
            p.leg_l = -0.9;
            p.leg_r = 0.3;
            p.arm_l = -2.3;
            p.arm_r = -2.3;
            p.arm_l_z = 0.35;
            p.arm_r_z = -0.35;
            p.chest_pitch = 0.05;
            p.chest_scale = Vec3::new(0.95, 1.08, 0.95);
            p.head_pitch = -0.15;
        }
        Falling => {
            p.leg_l = 0.35;
            p.leg_r = -0.25;
            p.arm_l = -1.2;
            p.arm_r = -1.2;
            p.arm_l_z = 0.9;
            p.arm_r_z = -0.9;
            p.chest_scale = Vec3::new(0.97, 1.04, 0.97);
            p.chest_roll = (t * 6.0).sin() * 0.05;
        }
        Climbing => {
            let s = (t * 7.0).sin();
            p.arm_l = -2.5 + s * 0.5;
            p.arm_r = -2.5 - s * 0.5;
            p.leg_l = -0.7 - s * 0.5;
            p.leg_r = -0.7 + s * 0.5;
            p.chest_pitch = 0.12;
            p.head_pitch = -0.3;
        }
        Swimming => {
            let s = (t * 5.0).sin();
            p.arm_l = -2.0 + s * 0.9;
            p.arm_r = -2.0 - s * 0.9;
            p.leg_l = s * 0.45;
            p.leg_r = -s * 0.45;
            p.chest_pitch = -0.55;
            p.chest_roll = s * 0.12;
            p.head_pitch = 0.45;
        }
        Rolling => {
            // Theo `roll.rs` của Veloren: co người (buildup), xoay tròn 1 vòng (movement), duỗi ra (recover)
            let r = i.roll;
            let tuck = if r < 0.15 { r / 0.15 } else if r > 0.85 { (1.0 - r) / 0.15 } else { 1.0 };
            let spin = ((r - 0.1) / 0.75).clamp(0.0, 1.0);
            let spin = spin * spin * (3.0 - 2.0 * spin);
            p.body_spin = -std::f32::consts::TAU * spin;
            p.body_drop = 0.35 * tuck;
            p.leg_l = -1.4 * tuck;
            p.leg_r = -1.2 * tuck;
            p.arm_l = -1.7 * tuck;
            p.arm_r = -1.7 * tuck;
            p.arm_l_z = 0.25 * tuck;
            p.arm_r_z = -0.25 * tuck;
            p.chest_pitch = -0.45 * tuck;
            p.head_pitch = -0.4 * tuck;
            p.chest_scale = Vec3::new(1.0, 1.0 - 0.08 * tuck, 1.0);
        }
    }
    if i.land_timer > 0.0 {
        let k = (i.land_timer / 0.22).clamp(0.0, 1.0);
        p.chest_scale = Vec3::new(1.0 + 0.1 * k, 1.0 - 0.14 * k, 1.0 + 0.1 * k);
        p.leg_l = 0.5 * k;
        p.leg_r = 0.5 * k;
        p.chest_pitch -= 0.25 * k;
    }
    p
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

/// NPC xa hơn khoảng này không chạy hoạt ảnh.
const ANIM_RANGE: f32 = 70.0;

/// Tâm xoay khi lăn (toạ độ của entity gốc, ~ hông).
const ROLL_PIVOT: Vec3 = Vec3::new(0.0, 0.75, 0.0);

/// Hoạt ảnh chung cho mọi nhân vật dựng bằng `spawn_humanoid`: đứng / đi / chạy / chạy nước rút / nhảy / rơi /
/// leo / bơi / lăn / chạm đất, cộng **nghiêng người khi rẽ** (tính từ tốc độ xoay hướng như `tilt` của Veloren).
/// Người chơi lấy trạng thái từ `Character`; NPC suy ra trạng thái từ tốc độ di chuyển thật.
pub fn biped_animation_system(
    time: Res<Time>,
    children: Query<&Children>,
    limb_q: Query<&Limb>,
    player_q: Query<&GlobalTransform, With<Character>>,
    mut roots: Query<(
        Entity,
        &GlobalTransform,
        &mut AnimDriver,
        Option<&Character>,
        Option<&crate::npc::Hostile>,
        Option<&crate::npc::Defending>,
    )>,
    mut tfs: Query<&mut Transform, With<Limb>>,
) {
    let dt = time.delta_secs().max(1e-4);
    let t = time.elapsed_secs();
    let k = (14.0 * dt).min(1.0);

    let ppos = player_q.single().ok().map(|p| p.translation());

    for (root, gt, mut d, character, hostile, defending) in &mut roots {
        // LOD: NPC ở xa (ngoài tầm sương mù) không cần hoạt ảnh
        if character.is_none() {
            if let Some(p) = ppos {
                if p.distance_squared(gt.translation()) > ANIM_RANGE * ANIM_RANGE {
                    d.last_pos = None;
                    continue;
                }
            }
        }
        if !d.linked {
            d.limbs = children
                .iter_descendants(root)
                .filter_map(|e| limb_q.get(e).ok().map(|l| (e, *l)))
                .collect();
            d.linked = !d.limbs.is_empty();
            if !d.linked {
                continue;
            }
        }

        // Tốc độ ngang & tốc độ xoay hướng thật
        let pos = gt.translation();
        let (_, rot, _) = gt.to_scale_rotation_translation();
        let yaw = rot.to_euler(EulerRot::YXZ).0;
        let inst = d.last_pos.map_or(0.0, |lp| Vec2::new(pos.x - lp.x, pos.z - lp.z).length() / dt);
        let inst = if inst > 60.0 { d.speed } else { inst }; // dịch chuyển tức thời (hồi sinh, tải chunk)
        d.speed += (inst - d.speed) * (dt * 10.0).min(1.0);
        let yaw_rate = if d.last_pos.is_some() { wrap_angle(yaw - d.last_yaw) / dt } else { 0.0 };
        d.last_pos = Some(pos);
        d.last_yaw = yaw;
        let moving_k = (d.speed / 4.0).clamp(0.0, 1.0);
        let tilt_target = (yaw_rate * 0.06).clamp(-0.4, 0.4) * moving_k;
        d.tilt += (tilt_target - d.tilt) * (dt * 8.0).min(1.0);

        let (state, land_timer, roll, arm_busy) = match character {
            Some(c) => (c.state, c.land_timer, c.roll_progress(), c.attack_timer > 0.0),
            None => {
                let s = if d.speed > 3.0 {
                    ActionState::Sprinting
                } else if d.speed > 0.3 {
                    ActionState::Running
                } else {
                    ActionState::Idle
                };
                (s, 0.0, 0.0, hostile.is_some() || defending.is_some())
            }
        };
        let rate = if state == ActionState::Sprinting { 15.0 } else { (4.0 + d.speed * 0.7).clamp(4.0, 11.0) };
        d.phase += dt * rate;
        let input = PoseInput { state, speed: d.speed, land_timer, roll, tilt: d.tilt };
        let pose = pose_for(&input, d.phase, t);

        let mut broken = false;
        for &(e, limb) in d.limbs.iter() {
            let Ok(mut tf) = tfs.get_mut(e) else {
                broken = true;
                continue;
            };
            match limb {
                Limb::Body => {
                    // Nghiêng cả người vào cua (quanh bàn chân) + xoay tròn quanh hông khi lăn
                    let lean = Quat::from_rotation_z(d.tilt * 0.6);
                    let spin = Quat::from_rotation_x(pose.body_spin);
                    let target_rot = lean * spin;
                    let offset = lean * (ROLL_PIVOT - spin * ROLL_PIVOT) - Vec3::Y * pose.body_drop;
                    if state == ActionState::Rolling {
                        // Xoay nhanh: đặt trực tiếp để không bị nội suy "cắt góc"
                        tf.rotation = target_rot;
                        tf.translation = offset;
                    } else {
                        tf.rotation = tf.rotation.slerp(target_rot, k);
                        tf.translation = tf.translation.lerp(offset, k);
                    }
                }
                Limb::Head => {
                    let target = Quat::from_rotation_y(pose.head_yaw) * Quat::from_rotation_x(pose.head_pitch);
                    tf.rotation = tf.rotation.slerp(target, k);
                }
                Limb::Chest => {
                    let target = Quat::from_rotation_x(pose.chest_pitch) * Quat::from_rotation_z(pose.chest_roll);
                    tf.rotation = tf.rotation.slerp(target, k);
                    tf.scale = tf.scale.lerp(pose.chest_scale, k);
                }
                Limb::ShoulderL => {
                    let target = Quat::from_rotation_x(pose.arm_l) * Quat::from_rotation_z(pose.arm_l_z);
                    tf.rotation = tf.rotation.slerp(target, k);
                }
                Limb::ShoulderR => {
                    if !arm_busy {
                        let target = Quat::from_rotation_x(pose.arm_r) * Quat::from_rotation_z(pose.arm_r_z);
                        tf.rotation = tf.rotation.slerp(target, k);
                    }
                }
                Limb::LegL => tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.leg_l), k),
                Limb::LegR => tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.leg_r), k),
                Limb::Tail => tf.rotation = tf.rotation.slerp(pose.tail, k),
            }
        }
        if broken {
            // Model vừa được dựng lại (đổi ngoại hình): tìm lại khớp ở khung sau
            d.linked = false;
        }
    }
}
