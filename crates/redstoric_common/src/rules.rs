//! Hằng số luật chơi, **nguồn sự thật duy nhất** cho cả chế độ offline (client) và server.
//! Muốn cân bằng lại game, chỉ cần sửa ở đây.

/// Thông số boss.
pub mod boss {
    pub const HEALTH: f32 = 500.0;
    /// Khoảng cách boss bắt đầu đuổi người chơi.
    pub const AGGRO_RANGE: f32 = 22.0;
    /// Quá xa điểm nhà thì boss bỏ cuộc và quay về.
    pub const LEASH_RANGE: f32 = 50.0;
    pub const SPEED: f32 = 3.6;
    pub const ATTACK_RANGE: f32 = 3.2;
    pub const ATTACK_DAMAGE: f32 = 20.0;
    /// Thời gian lấy đà trước khi đánh (giây).
    pub const WINDUP: f32 = 0.7;
    /// Thời gian nghỉ sau khi đánh (giây).
    pub const RECOVER: f32 = 1.1;
    /// Thời gian hoạt ảnh chết (giây).
    pub const DEATH_TIME: f32 = 2.5;
    /// Bán kính di chuyển tối đa quanh tâm đấu trường (server mô phỏng trên mặt phẳng XZ).
    pub const ARENA_LIMIT: f32 = 13.0;
    /// Tốc độ hồi máu khi không có ai trong tầm (máu/giây).
    pub const REGEN: f32 = 15.0;
}

/// Thông số người chơi.
pub mod player {
    pub const MAX_HP: f32 = 100.0;
    /// Sát thương mỗi đòn kiếm lên boss.
    pub const DAMAGE: f32 = 25.0;
    /// Tầm đánh tối đa server chấp nhận (rộng hơn tầm client để bù độ trễ).
    pub const REACH: f32 = 7.0;
    pub const ATTACK_COOLDOWN: f32 = 0.3;
    pub const REGEN: f32 = 1.5;
    /// Tốc độ tối đa hợp lệ (m/s); vượt quá thì server coi là gian lận/teleport.
    pub const MAX_SPEED: f32 = 40.0;
    pub const RESPAWN_POINT: [f32; 3] = [0.0, 40.0, 0.0];
}

/// Động vật hoang dã (cắn của thú dữ, hồi chiêu...).
pub mod fauna {
    /// Thời gian lấy đà trước khi cắn (giây).
    pub const BITE_WINDUP: f32 = 0.45;
    /// Nghỉ sau mỗi cú cắn (giây).
    pub const BITE_RECOVER: f32 = 1.0;
    pub const BITE_RANGE: f32 = 1.9;
    /// Số giây tiếp tục chạy sau khi hết thấy mối đe doạ (`FLEE_DURATION` của Veloren).
    pub const FLEE_DURATION: f32 = 3.0;
    /// Tốc độ chạy trốn theo tỉ lệ tốc độ chạy (Veloren dùng 0.65, ở đây tăng để thú đỡ dễ bị bắt).
    pub const MAX_FLEE_SPEED: f32 = 0.85;
    /// Bán kính tối đa quanh nhà khi lang thang (`MAX_PATROL_DIST`).
    pub const MAX_PATROL_DIST: f32 = 50.0;
    /// Thời gian thể xác còn lại sau khi chết (giây).
    pub const DEATH_TIME: f32 = 1.8;
}

/// Chỉ số vũ khí mà server cần để kiểm tra đòn đánh (theo chức nghiệp).
/// Bảng đầy đủ (mô hình, màu...) nằm ở `redstoric_core::weapons::WEAPONS`; test bên đó bảo đảm hai bảng khớp nhau.
pub mod weapons {
    #[derive(Clone, Copy, Debug)]
    pub struct Stats {
        pub duration: f32,
        pub damage: f32,
        pub range: f32,
    }

    pub const STATS: [Stats; 6] = [
        Stats { duration: 0.35, damage: 25.0, range: 3.0 },
        Stats { duration: 0.60, damage: 22.0, range: 3.3 },
        Stats { duration: 0.85, damage: 45.0, range: 3.8 },
        Stats { duration: 0.60, damage: 24.0, range: 55.0 },
        Stats { duration: 0.75, damage: 32.0, range: 45.0 },
        Stats { duration: 0.50, damage: 14.0, range: 40.0 },
    ];
}
