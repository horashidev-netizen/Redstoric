//! Dân làng (NPC) tương tác kiểu Veloren.
//!
//! * Voxel đặt `VillagerSpawn` ở các làng; ở đây biến nó thành NPC: dựng model người từ các mảnh
//!   `figure` (ngoại hình theo hạt giống), gắn tên + nghề.
//! * Bảng tên bay trên đầu NPC (đến 28 khối). Lại gần (<= 3.5 khối) thì hiện gợi ý **E Talk / T Trade**.
//! * **E**: nói chuyện (bấm tiếp để đọc câu kế). **T**: mở cửa hàng, phím **1-4** để mua bằng xu.
//!   **Esc** hoặc bước ra xa để đóng.
//! * Giao dịch hiện chỉ là bộ đếm xu + túi đồ tối giản (chưa có kho đồ thật).
#![allow(unused_imports)]

use bevy::prelude::*;
use redstoric_common::npc::{person_name, Profession};
use redstoric_common::structures::SiteKind;
use redstoric_common::WorldGenerator;
use redstoric_voxel::VillagerSpawn;
use redstoric_voxel::VoxelWorldMap;
use crate::character_creator::{spawn_humanoid, AppearanceCatalog};
use crate::{Character, OrbitCamera};

const START_COINS: u32 = 100;

/// Dân làng đi dạo ngẫu nhiên quanh nhà: đứng nghỉ một lúc rồi đi tới điểm mới trong bán kính `WANDER_RADIUS`.
#[derive(Component)]
pub struct Wander {
    /// Vị trí thế giới của chỗ đứng ban đầu (đặt ở lần chạy đầu tiên, khi GlobalTransform đã sẵn).
    ready: bool,
    pub home: Vec3,
    /// Tâm làng và bán kính đi lại được trong làng.
    center: Vec3,
    roam: f32,
    target: Vec3,
    wait: f32,
    walking: bool,
    phase: f32,
    rng: u32,
}

/// Dân làng đang nổi giận (bị người chơi đánh): không đi dạo, không quay mặt chào; do `redstoric_combat_ai` điều khiển.
#[derive(Component)]
pub struct Hostile;

/// Dân làng đang đánh thú dữ lọt vào làng: không đi dạo, không quay mặt chào (vẫn nói chuyện được).
#[derive(Component)]
pub struct Defending;

/// Kiểu đánh của dân làng khi nổi giận.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcAttack {
    Melee,
    Arrow,
    Fire,
    Holy,
}

/// Vũ khí riêng + chỉ số chiến đấu theo nghề.
#[derive(Clone, Copy, Debug)]
pub struct CombatProfile {
    /// Đường dẫn trong `assets/placeholders/glb/`.
    pub weapon: &'static str,
    pub attack: NpcAttack,
    pub hp: f32,
    pub damage: f32,
    /// Tầm đánh (cận chiến) hoặc tầm bắn.
    pub range: f32,
    pub cooldown: f32,
    pub speed: f32,
}

pub fn combat_profile(job: Profession) -> CombatProfile {
    use NpcAttack::*;
    let p = |weapon, attack, hp, damage, range, cooldown, speed| CombatProfile { weapon, attack, hp, damage, range, cooldown, speed };
    match job {
        Profession::Guard => p("weapon/axe/axe/iron-1h.glb", Melee, 140.0, 14.0, 2.2, 1.1, 4.2),
        Profession::Noble => p("weapon/sword/caladbolg.glb", Melee, 110.0, 12.0, 2.3, 1.0, 3.8),
        Profession::Blacksmith => p("weapon/hammer/forgemaster_hammer.glb", Melee, 120.0, 16.0, 2.2, 1.6, 3.4),
        Profession::Farmer => p("weapon/tool/pitchfork-0.glb", Melee, 80.0, 9.0, 2.6, 1.3, 3.6),
        Profession::Miner => p("weapon/tool/pickaxe_stone.glb", Melee, 100.0, 11.0, 2.1, 1.4, 3.4),
        Profession::Fisherman => p("weapon/spear/tidal_spear.glb", Melee, 85.0, 10.0, 2.8, 1.3, 3.6),
        Profession::Cook => p("weapon/hammer/burnt_drumstick.glb", Melee, 80.0, 8.0, 2.0, 1.0, 3.4),
        Profession::Innkeeper => p("weapon/tool/broom-0.glb", Melee, 80.0, 7.0, 2.4, 1.0, 3.4),
        Profession::Merchant => p("weapon/dagger/dagger_basic-0.glb", Melee, 70.0, 8.0, 1.8, 0.8, 4.0),
        Profession::Bard => p("weapon/tool/wooden_lute.glb", Melee, 70.0, 6.0, 2.0, 1.0, 3.8),
        Profession::Hunter => p("weapon/bow/shortbow/wood.glb", Arrow, 80.0, 10.0, 16.0, 1.8, 3.8),
        Profession::Alchemist => p("weapon/staff/firestaff_starter.glb", Fire, 70.0, 12.0, 14.0, 2.4, 3.2),
        Profession::Scholar => p("weapon/staff/staff/wood.glb", Fire, 60.0, 8.0, 13.0, 2.2, 3.2),
        Profession::Priest => p("weapon/sceptre/crook/wood.glb", Holy, 75.0, 9.0, 13.0, 2.0, 3.2),
    }
}

/// Dân làng đã dựng xong.
#[derive(Component)]
pub struct Villager {
    pub name: String,
    pub job: Profession,
    /// Giọng nữ / nam (theo ngoại hình)
    pub female: bool,
    /// Độ tuổi: 0 trẻ, 1 trung niên, 2 già (quyết định giọng nói và màu tóc)
    pub age: u8,
}

/// Độ tuổi sinh từ hạt giống: ~30% trẻ, 50% trung niên, 20% già. Trưởng làng luôn già; học giả / tu sĩ hay già hơn.
pub fn age_of(seed: u32, job: Profession) -> u8 {
    let r = (seed.wrapping_mul(2654435761) >> 24) % 10;
    match job {
        Profession::Noble => 2,
        Profession::Scholar | Profession::Priest => if r < 5 { 2 } else { 1 },
        _ => match r {
            0..=2 => 0,
            3..=7 => 1,
            _ => 2,
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DialogMode {
    #[default]
    Closed,
    Talk,
    Trade,
}

/// Trạng thái hội thoại / giao dịch hiện tại. Hệ thống chiến đấu đọc `is_open` để không chém khi đang mở.
#[derive(Resource, Default)]
pub struct NpcDialog {
    pub target: Option<Entity>,
    pub mode: DialogMode,
    pub line: usize,
}

impl NpcDialog {
    pub fn is_open(&self) -> bool {
        self.mode != DialogMode::Closed
    }
}

/// Dân làng gần người chơi nhất trong tầm tương tác.
#[derive(Resource, Default)]
struct Nearby(Option<Entity>);

#[derive(Resource)]
pub struct Wallet(pub u32);

/// Túi đồ tối giản: (tên, số lượng).
#[derive(Resource, Default)]
pub struct Inventory(pub Vec<(String, u32)>);

/// Dân làng: mô phỏng (`NpcSimPlugin`) + giao diện / âm thanh (`NpcUiPlugin`). Giữ tên `NpcPlugin` cho nơi gọi cũ.
pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((NpcSimPlugin, NpcUiPlugin));
    }
}

/// Mô phỏng: sinh dân làng, tìm người gần nhất, hội thoại (nhập liệu), đi lại, quay mặt. Chạy trong `GameSet::Sim`.
pub struct NpcSimPlugin;

impl Plugin for NpcSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialog>()
            .init_resource::<Nearby>()
            .init_resource::<Inventory>()
            .insert_resource(Wallet(START_COINS))
            .add_systems(
                Update,
                (
                    init_villagers.run_if(resource_exists::<AppearanceCatalog>),
                    find_nearby,
                    dialog_input,
                    villager_wander,
                    villager_face_player,
                )
                    .chain()
                    .in_set(crate::GameSet::Sim),
            );
    }
}

/// Giao diện: bảng hội thoại, bảng tên trên đầu, đọc thoại thành tiếng. Chạy trong `GameSet::Ui` sau mô phỏng.
pub struct NpcUiPlugin;

impl Plugin for NpcUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dialog_panel).add_systems(
            Update,
            (dialog_voice, update_plates, refresh_dialog_panel)
                .chain()
                .in_set(crate::GameSet::Ui)
                .run_if(redstoric_voxel::preload::world_ready),
        );
    }
}

mod sim;
mod ui;

pub use sim::*;
pub use ui::*;
