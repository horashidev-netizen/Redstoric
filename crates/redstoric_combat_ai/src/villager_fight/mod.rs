//! Dân làng tự vệ. Người chơi chém/bắn trúng một dân làng thì mọi dân làng trong bán kính
//! `ALARM_RADIUS` quanh đó nổi giận (`Hostile`), rút **vũ khí riêng của nghề** (`combat_profile`) và đánh lại:
//! nghề cận chiến (Guard rìu, Blacksmith búa, Farmer chĩa, Miner cuốc, Fisherman giáo, ...) đuổi theo và chém,
//! Hunter bắn tên, Alchemist/Scholar ném cầu lửa, Priest bắn tia thánh.
//!
//! Người chơi chạy xa (`CALM_DIST`) đủ lâu, hoặc dân làng bị kéo quá xa nhà (`LEASH`), thì họ nguôi giận,
//! cất vũ khí và hồi đầy máu. Dân làng bị hạ thì ngã xuống rồi biến mất (chunk tải lại sẽ sinh lại).
//! Mỗi lần làm cả làng nổi giận: -1 uy tín nhiệm vụ với làng đó.
//!
//! Thú dữ (sói...) bị chặn ở rìa làng (`fauna::Terrain::village_blocked`). Con nào lọt vào gần thì dân làng
//! cũng rút vũ khí ra đánh (`Defending`), và hô vài câu thoại (bong bóng chữ trên đầu, `Speech`), ví dụ
//! "Hang on, let me give you a hand!" khi người chơi đang ở gần.
#![allow(unused_imports)]

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use redstoric_common::structures::SiteKind;
use redstoric_common::WorldGenerator;
use redstoric_core::character_creator::{NpcArmRight, NpcWeapon};
use redstoric_common::fauna::Alignment;
use redstoric_core::npc::{combat_profile, CombatProfile, Defending, Hostile, NpcAttack, Villager};
use redstoric_core::OrbitCamera;
use redstoric_core::weapons::{Loadout, WeaponKind};
use redstoric_core::{ActionState, Character};
use redstoric_voxel::{BlockType, VoxelWorldMap};
use crate::fauna::{Agent, Animal, Mood};
use crate::quest::QuestLog;
use crate::ranged::{ProjKind, Projectile};
use crate::types::{Health, HitFlash, Knockback, MeleeAttacker};

#[derive(Clone, Copy, PartialEq, Debug)]
enum FightState {
    Idle,
    Chase,
    Windup(f32),
    Recover(f32),
    Dead(f32),
}

#[derive(Component)]
pub struct Fighter {
    profile: CombatProfile,
    pub hp: f32,
    state: FightState,
    cooldown: f32,
    calm_t: f32,
    yaw: f32,
    home: Option<Vec3>,
    arm: Option<Entity>,
    weapons: Vec<Entity>,
    linked: bool,
    knock: Vec3,
    /// Thú dữ đang đánh (khi không nổi giận với người chơi)
    pub target: Option<Entity>,
    scan_t: f32,
}

impl Fighter {
    pub fn max_hp(&self) -> f32 {
        self.profile.hp
    }
    pub fn is_dead(&self) -> bool {
        matches!(self.state, FightState::Dead(_))
    }
}

/// Đạn của dân làng (chỉ trúng người chơi).
#[derive(Component)]
pub struct VillagerShot {
    /// true: bắn người chơi; false: bắn thú dữ
    at_player: bool,
    vel: Vec3,
    gravity: f32,
    damage: f32,
    life: f32,
}

#[derive(Resource)]
pub struct VillagerFx {
    arrow: Handle<Mesh>,
    orb: Handle<Mesh>,
    arrow_mat: Handle<StandardMaterial>,
    fire_mat: Handle<StandardMaterial>,
    holy_mat: Handle<StandardMaterial>,
}

mod sim;
mod speech;

pub use sim::*;
pub use speech::*;
