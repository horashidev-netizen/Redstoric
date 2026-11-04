use bevy::prelude::*;
use redstoric_core::{ActionState, Character};
use crate::types::{DummyTarget, Health, HitFlash, Knockback, MeleeAttacker};

pub const ATTACK_DURATION: f32 = 0.35;

/// Đọc chuột trái và bắt đầu chu kỳ vung kiếm
pub fn combat_input_system(
    time: Res<Time>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut Character, &mut MeleeAttacker)>,
) {
    let dt = time.delta_secs();

    for (mut character, mut attacker) in &mut query {
        if character.attack_timer > 0.0 {
            character.attack_timer -= dt;
            if character.attack_timer <= 0.0 {
                character.attack_timer = 0.0;
                character.state = ActionState::Idle;
                attacker.has_hit = false;
            }
        }

        if mouse_button.just_pressed(MouseButton::Left) && character.attack_timer <= 0.0 {
            character.state = ActionState::Attacking(1);
            character.attack_timer = ATTACK_DURATION;
            attacker.has_hit = false; // Reset trạng thái để nhát chém mới có thể gây sát thương
        }
    }
}

/// Thuật toán quét Hitbox hình nón phía trước mặt nhân vật
pub fn melee_hit_detection_system(
    mut attacker_query: Query<(&Transform, &Character, &mut MeleeAttacker)>,
    mut target_query: Query<(Entity, &Transform, &mut Health, Option<&mut Knockback>, Option<&mut HitFlash>), With<DummyTarget>>,
    mut commands: Commands,
) {
    for (player_transform, character, mut attacker) in &mut attacker_query {
        if let ActionState::Attacking(_) = character.state {
            let progress = 1.0 - (character.attack_timer / ATTACK_DURATION);

            // Quét trúng đòn ở giữa nhát chém (từ 30% đến 80% animation)
            if progress >= 0.3 && progress <= 0.8 && !attacker.has_hit {
                let player_pos = player_transform.translation;
                let player_forward = player_transform.forward();

                for (target_entity, target_transform, mut health, knockback_opt, hit_flash_opt) in &mut target_query {
                    let target_pos = target_transform.translation;
                    let to_target = target_pos - player_pos;
                    let distance = to_target.length();

                    // 1. Kiểm tra khoảng cách vũ khí
                    if distance <= attacker.range {
                        let dir_to_target = to_target.normalize_or_zero();
                        let dot = player_forward.dot(dir_to_target);

                        // 2. Kiểm tra góc nhìn phía trước (~140 độ hình quạt)
                        if dot > 0.3 {
                            attacker.has_hit = true;

                            // Trừ máu
                            health.current = (health.current - attacker.damage).max(0.0);
                            info!("Bù nhìn bị chém! Máu còn: {}/{}", health.current, health.max);

                            // Lực đẩy lùi Knockback (hướng ra xa + hơi văng lên trời)
                            let kb_dir = (dir_to_target + Vec3::Y * 0.4).normalize();
                            let kb_velocity = kb_dir * 12.0;

                            if let Some(mut kb) = knockback_opt {
                                kb.velocity = kb_velocity;
                            } else {
                                commands.entity(target_entity).insert(Knockback { velocity: kb_velocity });
                            }

                            // Chớp đỏ phản hồi trúng đòn
                            if let Some(mut flash) = hit_flash_opt {
                                flash.timer = 0.15;
                            } else {
                                commands.entity(target_entity).insert(HitFlash { timer: 0.15 });
                            }
                        }
                    }
                }
            }
        }
    }
}