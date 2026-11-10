pub mod animation;
pub mod attack;
pub mod boss;
pub mod hud;
pub mod dummy;
pub mod types;

pub use types::{DummyTarget, Health, HitFlash, Knockback, MeleeAttacker, OnlineMode, PendingBossHits};

use bevy::prelude::*;
use animation::weapon_swing_animation;
use attack::{combat_input_system, melee_hit_detection_system};
use dummy::{dummy_death_system, dummy_hit_flash_system, dummy_knockback_system, spawn_training_dummies};
use crate::dummy::dummy_ground_system;
use boss::{boss_ai_system, boss_death_system, boss_spawn_system, boss_visual_system, player_health_system, BossRegistry};
use hud::{setup_hud, update_hud};

/// Chỉ chạy AI cục bộ khi chơi một mình; khi online, server điều khiển.
fn offline(online: Res<OnlineMode>) -> bool {
    !online.0
}

pub struct CombatAiPlugin;

impl Plugin for CombatAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BossRegistry>()
            .init_resource::<OnlineMode>()
            .init_resource::<PendingBossHits>()
            .add_systems(Startup, (spawn_training_dummies, setup_hud))
            .add_systems(
                Update,
                (
                    boss_spawn_system,
                    boss_ai_system.run_if(offline),
                    boss_visual_system,
                    boss_death_system,
                    player_health_system.run_if(offline),
                    update_hud,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    combat_input_system,
                    melee_hit_detection_system,
                    weapon_swing_animation,
                    dummy_knockback_system,
                    dummy_ground_system,
                    dummy_hit_flash_system,
                    dummy_death_system,
                ),
            );
    }
}