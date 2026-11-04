use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Z};
use crate::generation::WorldGenerator;
use crate::map::VoxelWorldMap;
use crate::meshing::build_greedy_mesh;

/// Đánh dấu Entity là tâm điểm để hệ thống stream chunk xung quanh (thường gắn vào Player)
#[derive(Component, Default)]
pub struct VoxelViewer;

/// Khoảng cách tải chunk xung quanh người chơi (Load Radius)
const LOAD_DISTANCE: i32 = 4;
/// Khoảng cách dỡ bỏ chunk (Unload Radius) - Hysteresis chống lag kiểu Veloren
const UNLOAD_DISTANCE: i32 = 6;

#[derive(Resource)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

/// Component chứa tác vụ tính toán Noise & Mesh ngầm trên CPU Worker Thread
#[derive(Component)]
pub struct ChunkGenTask(pub Task<(i32, i32, ChunkData, Mesh)>);

/// Khởi tạo Material địa hình lúc khởi động game
pub fn setup_voxel_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        perceptual_roughness: 0.85,
        reflectance: 0.15,
        ..default()
    });
    commands.insert_resource(TerrainMaterial(material));
}

/// Hệ thống quét tọa độ người chơi: Sinh chunk mới và xóa chunk ở quá xa
pub fn chunk_streaming_system(
    mut commands: Commands,
    generator: Res<WorldGenerator>,
    mut voxel_map: ResMut<VoxelWorldMap>,
    // Lấy vị trí của bất kỳ entity nào có gắn VoxelViewer
    viewer_query: Query<&Transform, With<VoxelViewer>>,
) {
    let Ok(viewer_transform) = viewer_query.single() else { return; };

    let player_cx = (viewer_transform.translation.x / CHUNK_SIZE_X as f32).floor() as i32;
    let player_cz = (viewer_transform.translation.z / CHUNK_SIZE_Z as f32).floor() as i32;

    let pool = AsyncComputeTaskPool::get();

    // 1. Quét tìm các chunk cần sinh trong bán kính LOAD_DISTANCE
    let mut chunks_to_load = Vec::new();
    for dx in -LOAD_DISTANCE..=LOAD_DISTANCE {
        for dz in -LOAD_DISTANCE..=LOAD_DISTANCE {
            if dx * dx + dz * dz <= LOAD_DISTANCE * LOAD_DISTANCE {
                let coord = (player_cx + dx, player_cz + dz);
                if !voxel_map.chunks.contains_key(&coord) && !voxel_map.loading_chunks.contains(&coord) {
                    let dist_sq = dx * dx + dz * dz;
                    chunks_to_load.push((dist_sq, coord));
                }
            }
        }
    }

    // Ưu tiên tải từ gần đến xa
    chunks_to_load.sort_by_key(|(dist, _)| *dist);

    // Bắn tác vụ sang các luồng worker chạy ngầm
    for (_, coord) in chunks_to_load {
        voxel_map.loading_chunks.insert(coord);
        // Đổi tên biến sang world_gen để tránh trùng từ khóa dành riêng `gen` của Rust 2024
        let world_gen = generator.clone();

        let task = pool.spawn(async move {
            let chunk = world_gen.generate_chunk(coord.0, coord.1);
            let mesh = build_greedy_mesh(&chunk);
            (coord.0, coord.1, chunk, mesh)
        });

        commands.spawn(ChunkGenTask(task));
    }

    // 2. Dỡ bỏ (Despawn) các chunk nằm ngoài bán kính UNLOAD_DISTANCE
    let mut chunks_to_remove = Vec::new();
    for &coord in voxel_map.chunk_entities.keys() {
        let dx = coord.0 - player_cx;
        let dz = coord.1 - player_cz;
        if dx * dx + dz * dz > UNLOAD_DISTANCE * UNLOAD_DISTANCE {
            chunks_to_remove.push(coord);
        }
    }

    for coord in chunks_to_remove {
        if let Some(entity) = voxel_map.chunk_entities.remove(&coord) {
            commands.entity(entity).despawn();
        }
        voxel_map.chunks.remove(&coord);
    }
}

/// Thu gom kết quả từ Worker Threads và render lên màn hình
pub fn handle_chunk_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ChunkGenTask)>,
    mut voxel_map: ResMut<VoxelWorldMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TerrainMaterial>,
) {
    for (task_entity, mut task) in &mut tasks {
        if let Some((cx, cz, chunk, mesh)) = block_on(future::poll_once(&mut task.0)) {
            voxel_map.chunks.insert((cx, cz), chunk);
            voxel_map.loading_chunks.remove(&(cx, cz));

            let entity = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.0.clone()),
                Transform::from_xyz(
                    (cx * CHUNK_SIZE_X as i32) as f32,
                    0.0,
                    (cz * CHUNK_SIZE_Z as i32) as f32,
                ),
            )).id();

            voxel_map.chunk_entities.insert((cx, cz), entity);
            commands.entity(task_entity).despawn();
        }
    }
}