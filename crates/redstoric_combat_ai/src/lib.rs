pub mod animation;
pub mod attack;
pub mod dummy;
pub mod types;

pub use types::{DummyTarget, Health, HitFlash, Knockback, MeleeAttacker};

use bevy::prelude::*;
use animation::weapon_swing_animation;
use attack::{combat_input_system, melee_hit_detection_system};
use dummy::{dummy_death_system, dummy_hit_flash_system, dummy_knockback_system, spawn_training_dummies};
use crate::dummy::dummy_ground_system;

pub struct CombatAiPlugin;

impl Plugin for CombatAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_training_dummies)
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