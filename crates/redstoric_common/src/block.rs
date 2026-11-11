#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum BlockType {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Water,
    Sand,
    Snow,
    CliffStone,
    Wood,
    BirchWood,
    Cactus,
    Leaves,
    LeavesLight,
    PineLeaves,
    Planks,
    Cobble,
    RoofTile,
    Bed,
    Obsidian,
    Farmland,
    Wheat,
    Rice,
    /// Tường trát vữa (daub) của nhà kiểu Veloren
    Plaster,
    /// Gỗ dầm sẫm màu (khung nhà, xà ngang)
    DarkWood,
    RoofTileBlue,
    RoofTileGreen,
    /// Mái rơm
    Thatch,
}

impl BlockType {
    #[inline(always)]
    pub fn is_air(&self) -> bool {
        matches!(self, BlockType::Air)
    }

    /// Lá cây và cây trồng: không cản đường đi và không tính là mặt đất.
    #[inline(always)]
    pub fn is_foliage(&self) -> bool {
        matches!(
            self,
            BlockType::Leaves | BlockType::LeavesLight | BlockType::PineLeaves | BlockType::Wheat | BlockType::Rice
        )
    }

    /// Trả về màu sắc [R, G, B, A] tùy theo hướng mặt (trên, dưới hay vách đứng)
    pub fn face_color(&self, axis: usize, normal_dir: i32) -> [f32; 4] {
        match (axis, normal_dir) {
            (1, 1) => [0.30, 0.72, 0.35, 1.0],  // Mặt trên (+Y): Cỏ xanh
            (1, -1) => [0.42, 0.28, 0.16, 1.0], // Mặt đáy (-Y): Đất tối màu
            _ => match self {                    // Vách đứng (+-X, +-Z)
                BlockType::Grass | BlockType::Dirt => [0.48, 0.32, 0.20, 1.0], // Đất nâu
                BlockType::Stone => [0.55, 0.55, 0.58, 1.0],                   // Đá xám
                BlockType::Air => [0.0, 0.0, 0.0, 0.0],
                BlockType::Water => [0.2, 0.5, 0.85, 0.7], // Xanh nước biển (hơi trong suốt)
                BlockType::Sand  => [0.86, 0.79, 0.59, 1.0], // Vàng cát sa mạc
                BlockType::Snow  => [0.95, 0.96, 0.98, 1.0], // Trắng tuyết
                BlockType::CliffStone => [0.42, 0.42, 0.44, 1.0],
                BlockType::Wood => [0.36, 0.25, 0.14, 1.0],
                BlockType::BirchWood => [0.85, 0.84, 0.80, 1.0],
                BlockType::Cactus => [0.20, 0.50, 0.22, 1.0],
                BlockType::Leaves => [0.18, 0.50, 0.16, 1.0],
                BlockType::LeavesLight => [0.45, 0.62, 0.20, 1.0],
                BlockType::PineLeaves => [0.10, 0.34, 0.20, 1.0],
                BlockType::Planks => [0.62, 0.46, 0.27, 1.0],
                BlockType::Cobble => [0.55, 0.55, 0.56, 1.0],
                BlockType::RoofTile => [0.65, 0.22, 0.18, 1.0],
                BlockType::Bed => [0.75, 0.20, 0.25, 1.0],
                BlockType::Obsidian => [0.12, 0.10, 0.18, 1.0],
                BlockType::Farmland => [0.30, 0.19, 0.11, 1.0],
                BlockType::Wheat => [0.85, 0.72, 0.22, 1.0],
                BlockType::Rice => [0.55, 0.80, 0.30, 1.0],
                BlockType::Plaster => [0.86, 0.80, 0.68, 1.0],
                BlockType::DarkWood => [0.24, 0.15, 0.08, 1.0],
                BlockType::RoofTileBlue => [0.22, 0.32, 0.50, 1.0],
                BlockType::RoofTileGreen => [0.26, 0.42, 0.24, 1.0],
                BlockType::Thatch => [0.72, 0.58, 0.30, 1.0],
            },
        }
    }
}