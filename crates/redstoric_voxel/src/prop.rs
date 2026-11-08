use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldPropKind {
    // Thân gỗ
    WoodLog,
    EldwoodLog,
    HardwoodLog,
    FrostwoodLog,
    Bamboo,

    // Rương & Thùng
    Chest,
    Crate,

    // Cây cối, hoa cỏ, thảm thực vật
    BlueFlower,
    RedFlower,
    Sunflower,
    Bush,

    /// Điểm xuất hiện của boss (không có model; sinh ra `BossSpawnPoint`)
    BossSpawn,

    /// Sprite trong danh mục `sprites` (chỉ số trong danh mục). `world_pos` là điểm chạm đất.
    Sprite(u16),
}

impl WorldPropKind {
    pub fn glb_asset_path(&self) -> &'static str {
        match self {
            WorldPropKind::WoodLog => "placeholders/glb/item/wood/wood.glb",
            WorldPropKind::EldwoodLog => "placeholders/glb/item/wood/eldwood.glb",
            WorldPropKind::HardwoodLog => "placeholders/glb/item/wood/hardwood.glb",
            WorldPropKind::FrostwoodLog => "placeholders/glb/item/wood/frostwood.glb",
            WorldPropKind::Bamboo => "placeholders/glb/item/wood/bamboo.glb",
            WorldPropKind::Chest => "placeholders/glb/sprite/chests/chest.glb",
            WorldPropKind::Crate => "placeholders/glb/sprite/furniture/crate-0.glb",
            WorldPropKind::BlueFlower => "placeholders/glb/sprite/flowers/flower_blue_1.glb",
            WorldPropKind::RedFlower => "placeholders/glb/sprite/flowers/flower_red_1.glb",
            WorldPropKind::Sunflower => "placeholders/glb/sprite/flowers/sunflower_1.glb",
            WorldPropKind::Bush => "placeholders/glb/sprite/dead_bush/1.glb",
            WorldPropKind::BossSpawn => "",
            WorldPropKind::Sprite(id) => crate::sprites::def(*id).path,
        }
    }

    pub fn scale(&self) -> Vec3 {
        match self {
            WorldPropKind::Sprite(id) => Vec3::splat(crate::sprites::def(*id).scale),
            WorldPropKind::Chest | WorldPropKind::Crate => Vec3::splat(0.065),
            WorldPropKind::BlueFlower | WorldPropKind::RedFlower => Vec3::splat(0.075),
            WorldPropKind::Sunflower => Vec3::splat(0.085),
            _ => Vec3::splat(0.055),
        }
    }
}

impl WorldPropKind {
    /// Độ nâng cộng thêm vào `world_pos.y` khi spawn (chỉ dùng cho sprite, để đáy chạm đất).
    pub fn ground_offset(&self) -> f32 {
        match self {
            WorldPropKind::Sprite(id) => crate::sprites::def(*id).y_offset(),
            _ => 0.0,
        }
    }
}

/// Dữ liệu prop sinh ra từ World Generator
#[derive(Debug, Clone)]
pub struct PropSpawnData {
    pub kind: WorldPropKind,
    pub world_pos: Vec3,
    pub rotation: Quat,
}

#[derive(Component)]
pub struct WorldProp;

/// Điểm spawn boss nằm ở tâm đấu trường. `home` là toạ độ thế giới tuyệt đối.
#[derive(Component)]
pub struct BossSpawnPoint {
    pub home: Vec3,
}
