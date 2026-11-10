use bevy::prelude::*;
use redstoric_core::weapons::{Loadout, WeaponKind};
use redstoric_core::{ActionState, Character};
use crate::types::{DummyTarget, Health, HitFlash, Knockback, MeleeAttacker, OnlineMode, PendingBossHits};

/// Đọc chuột trái và bắt đầu một đòn theo vũ khí đang cầm
pub fn combat_input_system(
    time: Res<Time>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    loadout: Res<Loadout>,
    menu: Option<Res<redstoric_core::graphics_menu::GraphicsMenu>>,
    creator: Option<Res<redstoric_core::character_creator::CreatorMenu>>,
    dialog: Option<Res<redstoric_core::npc::NpcDialog>>,
    mut query: Query<(&mut Character, &mut MeleeAttacker)>,
) {
    let dt = time.delta_secs();
    // Đang mở menu cài đặt thì click chuột là để bấm nút, không phải để đánh
    let menu_open = menu.map(|m| m.open).unwrap_or(false)
        || creator.map(|m| m.open).unwrap_or(false)
        || dialog.map(|d| d.is_open()).unwrap_or(false);
    let def = loadout.current();

    for (mut character, mut attacker) in &mut query {
        if character.attack_timer > 0.0 {
            character.attack_timer -= dt;
            if character.attack_timer <= 0.0 {
                character.attack_timer = 0.0;
                character.state = ActionState::Idle;
                attacker.has_hit = false;
            }
        }

        if mouse_button.just_pressed(MouseButton::Left) && !menu_open && character.attack_timer <= 0.0 {
            character.state = ActionState::Attacking(1);
            character.attack_len = def.duration;
            character.attack_timer = def.duration;
            attacker.damage = def.damage;
            attacker.range = def.range;
            attacker.has_hit = false; // Reset để nhát mới có thể gây sát thương / bắn đạn
        }
    }
}

/// Gây sát thương lên một mục tiêu: trừ máu, đẩy lùi, chớp đỏ. Boss khi online do server tính nên chỉ báo đòn.
pub fn apply_hit(
    commands: &mut Commands,
    online: bool,
    pending: &mut PendingBossHits,
    entity: Entity,
    health: &mut Health,
    knockback: Option<&mut Knockback>,
    flash: Option<&mut HitFlash>,
    boss: Option<&crate::boss::Boss>,
    dir: Vec3,
    damage: f32,
    push: f32,
) {
    if let (true, Some(boss)) = (online, boss) {
        pending.0.push(boss.key);
    } else {
        health.current = (health.current - damage).max(0.0);
        bevy::log::info!("Trúng đòn! Máu mục tiêu còn: {}/{}", health.current, health.max);
        let kb_velocity = (dir + Vec3::Y * 0.4).normalize_or_zero() * push;
        match knockback {
            Some(kb) => kb.velocity = kb_velocity,
            None => {
                commands.entity(entity).insert(Knockback { velocity: kb_velocity });
            }
        }
    }
    match flash {
        Some(f) => f.timer = 0.15,
        None => {
            commands.entity(entity).insert(HitFlash { timer: 0.15 });
        }
    }
}

/// Đòn cận chiến. Kiếm: hình quạt phía trước; rìu: vòng quanh người; búa: nện xuống vùng phía trước.
/// Cung / trượng bắn đạn trong `ranged.rs`.
pub fn melee_hit_detection_system(
    loadout: Res<Loadout>,
    mut attacker_query: Query<(&Transform, &Character, &mut MeleeAttacker)>,
    mut target_query: Query<(Entity, &Transform, &mut Health, Option<&mut Knockback>, Option<&mut HitFlash>, Option<&crate::boss::Boss>), Or<(With<DummyTarget>, With<crate::boss::Boss>, With<crate::fauna::Animal>)>>,
    mut commands: Commands,
    online: Res<OnlineMode>,
    mut pending_hits: ResMut<PendingBossHits>,
) {
    let def = loadout.current();
    if !matches!(def.kind, WeaponKind::Sword | WeaponKind::Axe | WeaponKind::Hammer) {
        return;
    }
    for (player_transform, character, mut attacker) in &mut attacker_query {
        let ActionState::Attacking(_) = character.state else { continue };
        let progress = 1.0 - (character.attack_timer / character.attack_len.max(0.05));
        if progress < def.release || attacker.has_hit {
            continue;
        }
        attacker.has_hit = true; // mỗi nhát chỉ quét một lần, trúng mọi mục tiêu trong vùng

        let player_pos = player_transform.translation;
        let forward = *player_transform.forward();
        // Tâm vùng đánh và bán kính
        let (centre, radius, cone, push) = match def.kind {
            WeaponKind::Sword => (player_pos, attacker.range, Some(0.3), 12.0),
            WeaponKind::Axe => (player_pos, attacker.range, None, 14.0),
            _ => (player_pos + forward * 2.0, attacker.range, None, 20.0), // búa
        };

        for (entity, tf, mut health, mut kb, mut flash, boss) in &mut target_query {
            let to_target = tf.translation - centre;
            if to_target.length() > radius {
                continue;
            }
            let from_player = tf.translation - player_pos;
            let dir = from_player.normalize_or_zero();
            if let Some(min_dot) = cone {
                if forward.dot(dir) <= min_dot {
                    continue;
                }
            }
            apply_hit(
                &mut commands, online.0, &mut pending_hits, entity, &mut health,
                kb.as_deref_mut(), flash.as_deref_mut(), boss, dir, attacker.damage, push,
            );
        }
    }
}
