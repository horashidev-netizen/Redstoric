//! Thành phần ECS của prop (chỉ client dùng). Dữ liệu thuần nằm ở `redstoric_common::prop`.
use bevy::prelude::*;

pub use redstoric_common::prop::{PropSpawnData, WorldPropKind};

#[derive(Component)]
pub struct WorldProp;

/// Điểm spawn boss nằm ở tâm đấu trường. `home` là toạ độ thế giới tuyệt đối.
#[derive(Component)]
pub struct BossSpawnPoint {
    pub home: Vec3,
}

/// Điểm đứng của một dân làng. Voxel chỉ đặt entity này; `redstoric_core::npc` biến nó thành
/// NPC hoàn chỉnh (model, bảng tên, hội thoại). `job` là chỉ số `redstoric_common::npc::Profession`.
#[derive(Component)]
pub struct VillagerSpawn {
    pub seed: u32,
    pub job: u8,
}

/// Đèn điểm của đèn đường/đèn lồng/nến: chỉ sáng ban đêm. `intensity` là độ sáng tối đa.
#[derive(Component)]
pub struct NightLamp {
    pub intensity: f32,
}
