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
}

impl BlockType {
    #[inline(always)]
    pub fn is_air(&self) -> bool {
        matches!(self, BlockType::Air)
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
                BlockType::CliffStone => [0.42, 0.42, 0.44, 1.0]
            },
        }
    }
}