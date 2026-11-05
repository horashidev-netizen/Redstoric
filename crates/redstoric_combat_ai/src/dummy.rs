use bevy::prelude::*;
use redstoric_voxel::VoxelWorldMap;
use crate::types::{DummyTarget, Health, HitFlash, Knockback};

/// Sinh 3 con bù nhìn đứng ở phía trước vị trí spawn của Player
pub fn spawn_training_dummies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dummy_positions = [
        Vec3::new(3.0, 25.0, -5.0),
        Vec3::new(-4.0, 25.0, -6.0),
        Vec3::new(0.0, 25.0, -8.0),
    ];

    let dummy_mesh = meshes.add(Cuboid::new(0.9, 2.0, 0.9));

    for pos in dummy_positions {
        let dummy_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.80, 0.55, 0.30), // Màu gỗ sáng
            perceptual_roughness: 0.85,
            ..default()
        });

        commands.spawn((
            Mesh3d(dummy_mesh.clone()),
            MeshMaterial3d(dummy_mat),
            Transform::from_translation(pos),
            DummyTarget,
            Health::new(100.0),
            Name::new("Training Dummy"),
        ));
    }
}

/// Hệ thống giữ cho Bù nhìn luôn đứng trên bề mặt địa hình Voxel (chống bị chôn dưới lòng đất)
pub fn dummy_ground_system(
    voxel_map: Option<Res<VoxelWorldMap>>,
    mut query: Query<&mut Transform, With<DummyTarget>>,
) {
    let Some(ref map) = voxel_map else { return; };

    for mut transform in &mut query {
        let ground_y = map.get_highest_block(transform.translation.x, transform.translation.z);
        // Khi map đã load được chunk ở tọa độ đó (> 0.0)
        if ground_y > 0.0 {
            transform.translation.y = ground_y + 1.0; // Ghim thẳng xuống mặt cỏ
        }
    }
}
/// Xử lý văng lùi (Knockback) khi dính chém
pub fn dummy_knockback_system(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Knockback), With<DummyTarget>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut kb) in &mut query {
        transform.translation += kb.velocity * dt;
        kb.velocity = kb.velocity.lerp(Vec3::ZERO, (8.0 * dt).min(1.0));

        if kb.velocity.length_squared() < 0.05 {
            commands.entity(entity).remove::<Knockback>();
        }
    }
}

/// Chớp đỏ khi trúng đòn rồi trả về màu gỗ
pub fn dummy_hit_flash_system(
    time: Res<Time>,
    mut query: Query<(Entity, &mut HitFlash, &MeshMaterial3d<StandardMaterial>), With<DummyTarget>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();

    for (entity, mut flash, mat_handle) in &mut query {
        flash.timer -= dt;

        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            if flash.timer > 0.0 {
                mat.base_color = Color::srgb(1.0, 0.1, 0.1); // Chớp đỏ tươi
            } else {
                mat.base_color = Color::srgb(0.80, 0.55, 0.30); // Về màu gỗ
                commands.entity(entity).remove::<HitFlash>();
            }
        }
    }
}

/// Khi hết máu, Bù nhìn ngã gục xuống đồi
pub fn dummy_death_system(
    mut query: Query<(&Health, &mut Transform), (With<DummyTarget>, Changed<Health>)>,
) {
    for (health, mut transform) in &mut query {
        if health.current <= 0.0 {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        }
    }
}