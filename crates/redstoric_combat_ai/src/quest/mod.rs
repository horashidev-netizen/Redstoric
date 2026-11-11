//! Nhiệm vụ của dân làng (tham khảo `rtsim/src/data/quest.rs` của Veloren).
//!
//! * Mỗi **nghề** có bộ nhiệm vụ riêng (`templates`), kèm lời thoại, vật phẩm nhiệm vụ và quà thưởng
//!   lấy từ hàng của chính nghề đó. Người giao việc là một dân làng cụ thể (theo tên).
//! * Đứng gần dân làng (<= 3.5 khối) bấm **Q** để nhận việc; làm xong mục tiêu thì quay lại đúng người đó
//!   bấm **Q** để trả việc (Courier thì xong ngay khi tới nơi). Bấm **X** hai lần để bỏ việc.
//! * Loại việc: Hunt (theo loài), Fetch (nhặt vật phẩm phát sáng), Patrol (đi qua 3 cột mốc quanh làng),
//!   Scout (do thám dungeon), Courier (đưa thư tới làng khác), SlayBoss (hạ boss).
//! * Mỗi lần trả việc +1 **uy tín** với làng: thưởng tăng 10%/điểm (tối đa +100%), và từ 2 uy tín
//!   làng mới giao việc diệt boss (riêng Guard và Village Elder giao luôn).
//!
//! Tiến độ hiện chỉ lưu trong phiên chơi (chưa ghi ra file).
#![allow(unused_imports)]

use std::collections::{HashMap, HashSet};
use bevy::prelude::*;
use redstoric_common::bosses::BossKind;
use redstoric_common::fauna::Species;
use redstoric_common::npc::Profession;
use redstoric_common::structures::{Site, SiteKind};
use redstoric_common::WorldGenerator;
use redstoric_core::npc::{Hostile, Inventory, NpcDialog, Villager, Wallet};
use redstoric_core::voice::VoiceQueue;
use redstoric_core::Character;
use redstoric_voxel::VoxelWorldMap;
use crate::boss::{Boss, BossState};
use crate::fauna::{Agent, Animal, Mood};

/// Tầm bấm Q để nhận/trả việc.
const OFFER_RANGE: f32 = 3.5;

/// Tới gần tâm làng nhận thư trong bán kính này thì hoàn thành Courier.
const ARRIVE_RANGE: f32 = 28.0;

/// Tới gần tâm dungeon trong bán kính này thì hoàn thành Scout.
const SCOUT_RANGE: f32 = 40.0;

/// Tầm nhặt vật phẩm Fetch.
const PICKUP_RANGE: f32 = 2.2;

/// Tầm chạm cột mốc Patrol.
const PATROL_RANGE: f32 = 4.0;

/// Boss chết trong bán kính này quanh tâm dungeon thì tính là dungeon đó.
const BOSS_SITE_RADIUS: f32 = 60.0;

/// Thú chết trong bán kính này quanh làng giao việc thì tính cho Hunt.
const HUNT_RADIUS: f32 = 220.0;

/// Thời gian (giây) thông báo hiện trên nhật ký.
const MESSAGE_TIME: f32 = 9.0;

/// Bấm X lần hai trong khoảng này thì bỏ việc.
const ABANDON_WINDOW: f32 = 3.0;

/// Uy tín cần để làng giao việc diệt boss.
const SLAY_REP: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestGoal {
    /// Hạ `need` con thú (`species` = None: loài nào cũng được) quanh khu `site`.
    Hunt { site: u32, species: Option<Species>, need: u32 },
    /// Hạ boss ở dungeon `site`.
    SlayBoss { site: u32 },
    /// Đi tới làng `to`.
    Courier { to: u32 },
    /// Nhặt `need` vật phẩm rơi quanh điểm (x, z).
    Fetch { x: i32, z: i32, need: u32 },
    /// Tới gần dungeon `site`.
    Scout { site: u32 },
    /// Chạm `need` cột mốc quanh làng `site`.
    Patrol { site: u32, need: u32 },
}

#[derive(Clone)]
pub struct Quest {
    /// Làng giao việc (SiteId).
    pub giver: u32,
    pub giver_name: String,
    pub job: Profession,
    pub title: &'static str,
    pub intro: String,
    /// Câu mẫu (khoá tìm file giọng nói)
    pub voice: &'static str,
    /// Vật phẩm nhặt được (Fetch), rỗng nếu không có.
    pub item: &'static str,
    pub goal: QuestGoal,
    pub progress: u32,
    /// Đã xong mục tiêu, chờ quay về trả việc.
    pub done: bool,
    pub reward: u32,
    pub reward_item: Option<&'static str>,
}

#[derive(Resource, Default)]
pub struct QuestLog {
    pub active: Option<Quest>,
    pub completed: u32,
    /// Các dungeon đã bị hạ boss trong phiên (không giao SlayBoss lặp lại).
    pub slain_sites: HashSet<u32>,
    /// Uy tín theo làng (SiteId -> số việc đã trả).
    pub reputation: HashMap<u32, u32>,
    /// Số nhiệm vụ đã nhận (để đổi loại việc cho đa dạng).
    pub issued: u32,
    pub message: String,
    pub message_t: f32,
    abandon_t: f32,
}

impl QuestLog {
    pub(crate) fn say(&mut self, text: String) {
        self.message = text;
        self.message_t = MESSAGE_TIME;
    }
    pub fn rep(&self, site: u32) -> u32 {
        self.reputation.get(&site).copied().unwrap_or(0)
    }
}

mod helpers;
mod markers;
mod sim;
mod templates;
mod ui;

pub use helpers::*;
pub use markers::*;
pub use sim::*;
pub use templates::*;
pub use ui::*;
