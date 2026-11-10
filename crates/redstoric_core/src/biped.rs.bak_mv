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

/// Hoạt ảnh bước chạy và vẫy đuôi chuẩn hệ trục Y-Up tự nhiên của Bevy
pub fn biped_walk_animation_system(
    time: Res<Time>,
    character_query: Query<&Character>,
    mut head_q: Query<&mut Transform, (With<BoneHead>, Without<BoneShoulderLeft>, Without<BoneShoulderRight>, Without<BoneLegLeft>, Without<BoneLegRight>, Without<BoneTail>)>,
    mut shoulder_l_q: Query<&mut Transform, (With<BoneShoulderLeft>, Without<BoneHead>, Without<BoneShoulderRight>, Without<BoneLegLeft>, Without<BoneLegRight>, Without<BoneTail>)>,
    mut shoulder_r_q: Query<&mut Transform, (With<BoneShoulderRight>, Without<BoneHead>, Without<BoneShoulderLeft>, Without<BoneLegLeft>, Without<BoneLegRight>, Without<BoneTail>)>,
    mut leg_l_q: Query<&mut Transform, (With<BoneLegLeft>, Without<BoneHead>, Without<BoneShoulderLeft>, Without<BoneShoulderRight>, Without<BoneLegRight>, Without<BoneTail>)>,
    mut leg_r_q: Query<&mut Transform, (With<BoneLegRight>, Without<BoneHead>, Without<BoneShoulderLeft>, Without<BoneShoulderRight>, Without<BoneLegLeft>, Without<BoneTail>)>,
    mut tail_q: Query<&mut Transform, (With<BoneTail>, Without<BoneHead>, Without<BoneShoulderLeft>, Without<BoneShoulderRight>, Without<BoneLegLeft>, Without<BoneLegRight>)>,
) {
    let Ok(character) = character_query.single() else { return; };
    let t = time.elapsed_secs() * 11.0; // Nhịp bước chân

    match character.state {
        ActionState::Running => {
            let leg_swing = t.sin() * 0.65; // Chân vung góc ~37 độ
            let arm_swing = t.sin() * 0.55; // Tay vung ngược chiều chân
            let tail_wag = (t * 1.5).sin() * 0.35; // Đuôi vẫy nhanh theo nhịp chạy

            // 1. Chân bước tới/lui so le quanh trục X (ngang)
            for mut trans in &mut leg_l_q {
                trans.rotation = Quat::from_rotation_x(leg_swing);
            }
            for mut trans in &mut leg_r_q {
                trans.rotation = Quat::from_rotation_x(-leg_swing);
            }

            // 2. Tay đánh nhịp ngược chiều chân
            for mut trans in &mut shoulder_l_q {
                trans.rotation = Quat::from_rotation_x(-arm_swing);
            }
            if character.attack_timer <= 0.0 {
                for mut trans in &mut shoulder_r_q {
                    trans.rotation = Quat::from_rotation_x(arm_swing);
                }
            }

            // 3. Đuôi sói: Vẫy trái/phải quanh trục Y thẳng đứng, hơi cụp nhẹ xuống quanh trục X
            for mut trans in &mut tail_q {
                trans.rotation = Quat::from_rotation_y(tail_wag) * Quat::from_rotation_x(0.35);
            }

            // 4. Đầu hơi gật nhẹ theo nhịp chạy
            for mut trans in &mut head_q {
                trans.rotation = Quat::from_rotation_x((t * 0.5).sin() * 0.05);
            }
        }
        ActionState::Idle => {
            // Khi đứng yên: Đuôi đung đưa nhẹ nhàng, toàn bộ khớp về vị trí thả lỏng tự nhiên
            let tail_idle = (time.elapsed_secs() * 3.0).sin() * 0.18;

            for mut trans in &mut tail_q {
                trans.rotation = Quat::from_rotation_y(tail_idle) * Quat::from_rotation_x(0.25);
            }
            for mut trans in &mut leg_l_q {
                trans.rotation = Quat::IDENTITY;
            }
            for mut trans in &mut leg_r_q {
                trans.rotation = Quat::IDENTITY;
            }
            for mut trans in &mut head_q {
                trans.rotation = Quat::IDENTITY;
            }
            for mut trans in &mut shoulder_l_q {
                trans.rotation = Quat::IDENTITY;
            }
            if character.attack_timer <= 0.0 {
                for mut trans in &mut shoulder_r_q {
                    trans.rotation = Quat::IDENTITY;
                }
            }
        }
        _ => {}
    }
}