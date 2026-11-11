pub mod animation;
pub mod attack;
pub mod boss;
pub mod boss_models;
pub mod hud;
pub mod dummy;
pub mod fauna;
pub mod types;
pub mod hotbar;
pub mod quest;
pub mod villager_fight;
pub mod ranged;

pub use types::{DummyTarget, Health, HitFlash, Knockback, MeleeAttacker, OnlineMode, PendingBossHits};

use bevy::prelude::*;
use animation::{slash_fx_system, weapon_swing_animation};
use attack::{combat_input_system, melee_hit_detection_system};
use dummy::{dummy_death_system, dummy_hit_flash_system, dummy_knockback_system, spawn_training_dummies};
use crate::dummy::dummy_ground_system;
use boss::{boss_ai_system, boss_death_system, boss_limb_animation, boss_spawn_system, boss_visual_system, player_health_system, BossRegistry};
use hud::{setup_hud, update_hud};
use hotbar::{setup_hotbar, update_hotbar};
use redstoric_voxel::preload::world_ready;
use ranged::{burst_fx_system, projectile_system, ranged_attack_system, setup_fx_assets};

/// Chỉ chạy AI cục bộ khi chơi một mình; khi online, server điều khiển.
fn offline(online: Res<OnlineMode>) -> bool {
    !online.0
}

/// Mô phỏng chiến đấu: AI thú / boss / dân làng, nhiệm vụ, đạn, sát thương. Mọi system nằm trong `GameSet::Sim`.
/// Phần này không dựng giao diện 2D (HUD, hotbar, bong bóng thoại): các thứ đó thuộc `CombatAiUiPlugin`.
/// Mục tiêu: sau này tách `Sim` ra crate riêng để server chạy không cần cửa sổ hay GPU.
pub struct CombatAiSimPlugin;

impl Plugin for CombatAiSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(fauna::FaunaPlugin)
            .init_resource::<BossRegistry>()
            .init_resource::<OnlineMode>()
            .init_resource::<PendingBossHits>()
            .init_resource::<quest::QuestLog>()
            .add_systems(Startup, (spawn_training_dummies, setup_fx_assets, villager_fight::setup_villager_fx))
            .add_systems(
                Update,
                (quest::quest_accept_system, quest::quest_progress_system, quest::quest_marker_system)
                    .chain()
                    .in_set(redstoric_core::GameSet::Sim),
            )
            .add_systems(
                Update,
                (
                    villager_fight::villager_fighter_init,
                    villager_fight::villager_link_parts,
                    villager_fight::villager_hit_system,
                    villager_fight::villager_defend_system,
                    villager_fight::villager_combat_ai,
                    villager_fight::villager_shot_system,
                    villager_fight::villager_weapon_visibility,
                )
                    .chain()
                    .in_set(redstoric_core::GameSet::Sim),
            )
            .add_systems(
                Update,
                (
                    boss_spawn_system,
                    boss_ai_system.run_if(offline),
                    boss_visual_system,
                    boss_limb_animation,
                    boss_death_system,
                    player_health_system.run_if(offline),
                )
                    .chain()
                    .in_set(redstoric_core::GameSet::Sim),
            )
            .add_systems(
                Update,
                (
                    combat_input_system,
                    melee_hit_detection_system,
                    weapon_swing_animation,
                    slash_fx_system,
                    ranged_attack_system,
                    projectile_system,
                    burst_fx_system,
                    dummy_knockback_system,
                    dummy_ground_system,
                    dummy_hit_flash_system,
                    dummy_death_system,
                )
                    .in_set(redstoric_core::GameSet::Sim),
            );
    }
}

/// Giao diện của chiến đấu / nhiệm vụ: thanh máu, hotbar, bảng nhiệm vụ, bong bóng thoại của dân làng.
/// Chạy trong `GameSet::Ui` (sau mô phỏng) và chỉ khi thế giới đã tải xong, giống các system Sim trước đây.
pub struct CombatAiUiPlugin;

impl Plugin for CombatAiUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_hud, setup_hotbar, quest::setup_quest_hud)).add_systems(
            Update,
            (update_hud, update_hotbar, quest::quest_hud_system, villager_fight::villager_speech_system)
                .in_set(redstoric_core::GameSet::Ui)
                .run_if(world_ready),
        );
    }
}

/// Toàn bộ chiến đấu: mô phỏng + giao diện (giữ nguyên tên cũ để `src/lib.rs` không phải đổi).
pub struct CombatAiPlugin;

impl Plugin for CombatAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CombatAiSimPlugin, CombatAiUiPlugin));
    }
}
