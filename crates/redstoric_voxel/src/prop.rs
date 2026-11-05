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
        }
    }

    pub fn scale(&self) -> Vec3 {
        match self {
            WorldPropKind::Chest | WorldPropKind::Crate => Vec3::splat(0.065),
            WorldPropKind::BlueFlower | WorldPropKind::RedFlower => Vec3::splat(0.075),
            WorldPropKind::Sunflower => Vec3::splat(0.085),
            _ => Vec3::splat(0.055),
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