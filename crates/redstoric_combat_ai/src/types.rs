use bevy::prelude::*;

/// Máu của thực thể
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

/// Hiệu ứng chớp đỏ khi bị chém trúng
#[derive(Component)]
pub struct HitFlash {
    pub timer: f32,
}

/// Lực đẩy lùi khi dính đòn
#[derive(Component)]
pub struct Knockback {
    pub velocity: Vec3,
}

/// Đánh dấu là Bù nhìn tập đánh
#[derive(Component)]
pub struct DummyTarget;

/// Khả năng cận chiến của Player
#[derive(Component)]
pub struct MeleeAttacker {
    pub damage: f32,
    pub range: f32,
    pub has_hit: bool,
}

impl Default for MeleeAttacker {
    fn default() -> Self {
        Self {
            damage: 25.0,
            range: 2.8, // Tầm chém 2.8m
            has_hit: false,
        }
    }
}