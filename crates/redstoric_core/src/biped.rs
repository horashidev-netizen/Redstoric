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
    tail: Quat,
}

fn pose_for(character: &Character, t_run: f32, t: f32) -> Pose {
    use ActionState::*;
    let mut p = Pose { chest_scale: Vec3::ONE, tail: Quat::from_rotation_x(0.25), ..default() };
    match character.state {
        Idle => {
            let breathe = (t * 2.2).sin();
            p.chest_scale = Vec3::new(1.0, 1.0 + breathe * 0.012, 1.0);
            p.arm_l_z = 0.05 + breathe * 0.02;
            p.arm_r_z = -0.05 - breathe * 0.02;
            p.tail = Quat::from_rotation_y((t * 3.0).sin() * 0.18) * Quat::from_rotation_x(0.25);
        }
        Running | Sprinting => {
            let sprint = character.state == Sprinting;
            let (swing, arms, lean, bob) = if sprint { (1.0, 1.0, 0.28, 0.025) } else { (0.65, 0.55, 0.08, 0.0) };
            let s = t_run.sin();
            p.leg_l = s * swing;
            p.leg_r = -s * swing;
            p.arm_l = -s * arms;
            p.arm_r = s * arms;
            p.chest_pitch = -lean;
            p.chest_roll = (t_run * 0.5).sin() * 0.06;
            p.chest_scale = Vec3::new(1.0, 1.0 + (t_run * 2.0).sin().abs() * bob, 1.0);
            p.head_pitch = lean * 0.8 + (t_run * 0.5).sin() * 0.05;
            p.tail = Quat::from_rotation_y((t_run * 1.5).sin() * 0.35) * Quat::from_rotation_x(0.35 + lean);
        }
        Jumping => {
            // Co chân, tay giơ lên, người kéo dãn
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
            // Dang tay giữ thăng bằng, chân duỗi
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
            // Tay chân đổi bên theo nhịp leo, người áp sát vách
            let s = (t * 7.0).sin();
            p.arm_l = -2.5 + s * 0.5;
            p.arm_r = -2.5 - s * 0.5;
            p.leg_l = -0.7 - s * 0.5;
            p.leg_r = -0.7 + s * 0.5;
            p.chest_pitch = 0.12;
            p.head_pitch = -0.3;
        }
        Swimming => {
            // Bơi sải: tay quạt luân phiên, chân đạp
            let s = (t * 5.0).sin();
            p.arm_l = -2.0 + s * 0.9;
            p.arm_r = -2.0 - s * 0.9;
            p.leg_l = s * 0.45;
            p.leg_r = -s * 0.45;
            p.chest_pitch = -0.55;
            p.chest_roll = s * 0.12;
            p.head_pitch = 0.45;
        }
        Attacking(_) => {}
    }
    // Chạm đất: nhún gối, người bè ra
    if character.land_timer > 0.0 {
        let k = (character.land_timer / 0.22).clamp(0.0, 1.0);
        p.chest_scale = Vec3::new(1.0 + 0.1 * k, 1.0 - 0.14 * k, 1.0 + 0.1 * k);
        p.leg_l = 0.5 * k;
        p.leg_r = 0.5 * k;
        p.chest_pitch -= 0.25 * k;
    }
    p
}

/// Hoạt ảnh chạy / chạy nước rút / nhảy / rơi / leo / bơi / chạm đất. Các khớp tiến mượt về tư thế đích.
pub fn biped_walk_animation_system(
    time: Res<Time>,
    character_query: Query<&Character>,
    mut bones: Query<(
        &mut Transform,
        Option<&BoneHead>,
        Option<&BoneChest>,
        Option<&BoneShoulderLeft>,
        Option<&BoneShoulderRight>,
        Option<&BoneLegLeft>,
        Option<&BoneLegRight>,
        Option<&BoneTail>,
    )>,
) {
    let Ok(character) = character_query.single() else { return; };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    // Nhịp bước nhanh hơn khi chạy nước rút
    let t_run = t * if character.state == ActionState::Sprinting { 15.0 } else { 11.0 };
    let pose = pose_for(character, t_run, t);
    let k = (14.0 * dt).min(1.0);
    let attacking_arm = character.attack_timer > 0.0;

    for (mut tf, head, chest, sh_l, sh_r, leg_l, leg_r, tail) in &mut bones {
        if head.is_some() {
            tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.head_pitch), k);
        } else if chest.is_some() {
            let target = Quat::from_rotation_x(pose.chest_pitch) * Quat::from_rotation_z(pose.chest_roll);
            tf.rotation = tf.rotation.slerp(target, k);
            tf.scale = tf.scale.lerp(pose.chest_scale, k);
        } else if sh_l.is_some() {
            tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.arm_l) * Quat::from_rotation_z(pose.arm_l_z), k);
        } else if sh_r.is_some() {
            if !attacking_arm {
                tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.arm_r) * Quat::from_rotation_z(pose.arm_r_z), k);
            }
        } else if leg_l.is_some() {
            tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.leg_l), k);
        } else if leg_r.is_some() {
            tf.rotation = tf.rotation.slerp(Quat::from_rotation_x(pose.leg_r), k);
        } else if tail.is_some() {
            tf.rotation = tf.rotation.slerp(pose.tail, k);
        }
    }
}
