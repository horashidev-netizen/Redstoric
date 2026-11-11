//! Các loại boss và chỉ số riêng của từng loại. Loại boss do **bậc** của đấu trường quyết định
//! (xem `worldplan`), nên server và client tự chọn cùng một loại mà không cần gửi qua mạng.
//!
//! Hệ số nhân áp lên chỉ số gốc ở `rules::boss` (máu, sát thương, tốc độ, tầm đánh, thời gian lấy đà).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BossKind {
    /// Boss khối đen gốc của game (bậc 1).
    Guardian,
    Ogre,
    Minotaur,
    Yeti,
    TrollMountain,
    Cyclops,
}

impl BossKind {
    pub const ALL: [BossKind; 6] = [
        BossKind::Guardian,
        BossKind::Ogre,
        BossKind::Minotaur,
        BossKind::Yeti,
        BossKind::TrollMountain,
        BossKind::Cyclops,
    ];

    /// Loại boss của đấu trường bậc `tier` (1..=5); `seed` chọn giữa hai loại ở bậc 3.
    pub fn for_tier(tier: u8, seed: u32) -> Self {
        match tier {
            0 | 1 => BossKind::Guardian,
            2 => BossKind::Ogre,
            3 => {
                if seed % 2 == 0 {
                    BossKind::Minotaur
                } else {
                    BossKind::Yeti
                }
            }
            4 => BossKind::TrollMountain,
            _ => BossKind::Cyclops,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BossKind::Guardian => "Obsidian Guardian",
            BossKind::Ogre => "Ogre Brute",
            BossKind::Minotaur => "Minotaur Warlord",
            BossKind::Yeti => "Frost Yeti",
            BossKind::TrollMountain => "Mountain Troll",
            BossKind::Cyclops => "Cyclops Titan",
        }
    }

    pub fn hp_mult(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 1.6,
            BossKind::Minotaur => 2.3,
            BossKind::Yeti => 2.5,
            BossKind::TrollMountain => 3.4,
            BossKind::Cyclops => 4.6,
        }
    }

    pub fn damage_mult(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 1.3,
            BossKind::Minotaur => 1.7,
            BossKind::Yeti => 1.5,
            BossKind::TrollMountain => 2.1,
            BossKind::Cyclops => 2.7,
        }
    }

    pub fn speed_mult(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 0.85,
            BossKind::Minotaur => 1.2,
            BossKind::Yeti => 1.05,
            BossKind::TrollMountain => 0.8,
            BossKind::Cyclops => 0.7,
        }
    }

    pub fn range_mult(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 1.2,
            BossKind::Minotaur => 1.15,
            BossKind::Yeti => 1.25,
            BossKind::TrollMountain => 1.4,
            BossKind::Cyclops => 1.6,
        }
    }

    /// Thời gian lấy đà (đòn nặng thì lâu hơn: người chơi có thêm thời gian né).
    pub fn windup_mult(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 1.2,
            BossKind::Minotaur => 0.85,
            BossKind::Yeti => 1.0,
            BossKind::TrollMountain => 1.3,
            BossKind::Cyclops => 1.5,
        }
    }

    /// Hệ số kích thước model (chỉ hình ảnh).
    pub fn scale(self) -> f32 {
        match self {
            BossKind::Guardian => 1.0,
            BossKind::Ogre => 1.0,
            BossKind::Minotaur => 1.0,
            BossKind::Yeti => 1.15,
            BossKind::TrollMountain => 1.2,
            BossKind::Cyclops => 1.4,
        }
    }
}
