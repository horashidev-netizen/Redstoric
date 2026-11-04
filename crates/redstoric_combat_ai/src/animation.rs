use bevy::prelude::*;
use redstoric_core::{ActionState, Character, WeaponHand};
use crate::attack::ATTACK_DURATION;

pub fn weapon_swing_animation(
    character_query: Query<&Character>,
    mut weapon_query: Query<&mut Transform, With<WeaponHand>>,
) {
    let Ok(character) = character_query.single() else { return; };
    let Ok(mut weapon_transform) = weapon_query.single_mut() else { return; };

    if let ActionState::Attacking(_) = character.state {
        let progress = (1.0 - (character.attack_timer / ATTACK_DURATION)).clamp(0.0, 1.0);

        // Vung một nhát chém mạnh mẽ từ trên phải quét xuống trước ngực sang trái
        let slash_start = Quat::from_euler(EulerRot::YXZ, -0.6, -0.9, 0.5);
        let slash_end = Quat::from_euler(EulerRot::YXZ, 1.2, 0.6, -0.5);

        let smooth_t = progress * progress * (3.0 - 2.0 * progress);
        weapon_transform.rotation = slash_start.slerp(slash_end, smooth_t);
    } else {
        // Tư thế nghỉ: Cầm kiếm chúc nhẹ ra phía trước, mũi kiếm hướng thẳng theo tầm nhìn
        weapon_transform.rotation = Quat::from_rotation_x(-0.35) * Quat::from_rotation_z(0.15);
    }
}