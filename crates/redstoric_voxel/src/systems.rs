use bevy::prelude::*;
use crate::prop::{PropSpawnData, WorldProp};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Z};
use crate::map::VoxelWorldMap;
use crate::meshing::build_greedy_mesh;
use crate::worldgen::CHUNK_SIZE_Y;
use crate::WorldGenerator;
/// Đánh dấu Entity là tâm điểm để hệ thống stream chunk xung quanh (thường gắn vào Player)
#[derive(Component, Default)]
pub struct VoxelViewer;

/// Khoảng cách tải chunk xung quanh người chơi (Load Radius)
const LOAD_DISTANCE: i32 = 7;
/// Khoảng cách dỡ bỏ chunk (Unload Radius) - Hysteresis chống lag kiểu Veloren
const UNLOAD_DISTANCE: i32 = 9;
#[derive(Component)]
pub struct ChunkComponent {
    pub coord: IVec2,
}
#[derive(Resource)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

/// Component chứa tác vụ tính toán Noise & Mesh ngầm trên CPU Worker Thread
#[derive(Component)]
pub struct ChunkGenTask(pub Task<(i32, i32, ChunkData, Mesh, Vec<PropSpawnData>)>);
/// Khởi tạo Material địa hình lúc khởi động game
pub fn setup_voxel_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE, // Giữ màu trắng để nhân 1:1 với màu đỉnh (Vertex Color)
        perceptual_roughness: 0.9,
        reflectance: 0.05,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
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
    for (_, coord) in chunks_to_load.into_iter().take(2) {
        voxel_map.loading_chunks.insert(coord);
        let world_gen = generator.clone();

        let task = pool.spawn(async move {
            let (chunk, props) = world_gen.generate_chunk_with_props(coord.0, coord.1);
            let mesh = build_greedy_mesh(&chunk).unwrap();
            (coord.0, coord.1, chunk, mesh, props)
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
    material: Option<Res<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let Some(material) = material else { return; };

    for (task_entity, mut task) in &mut tasks {
        if let Some((cx, cz, chunk, mesh, props)) = block_on(future::poll_once(&mut task.0)) {
            voxel_map.chunks.insert((cx, cz), chunk);
            voxel_map.loading_chunks.remove(&(cx, cz));

            // 1. Spawn Chunk Voxel Terrain
            let chunk_entity = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.0.clone()),
                Transform::from_xyz(
                    (cx * CHUNK_SIZE_X as i32) as f32,
                    0.0,
                    (cz * CHUNK_SIZE_Z as i32) as f32,
                ),
            )).id();

            // 2. Spawn các Prop (Rương, Khúc gỗ Eldwood/Hardwood) làm con của chunk
            commands.entity(chunk_entity).with_children(|parent| {
                for prop in props {
                    let scene_handle = asset_server.load(
                        GltfAssetLabel::Scene(0).from_asset(prop.kind.glb_asset_path())
                    );

                    parent.spawn((
                        WorldAssetRoot(scene_handle),
                        Transform::from_translation(prop.world_pos - Vec3::new((cx * CHUNK_SIZE_X as i32) as f32, 0.0, (cz * CHUNK_SIZE_Z as i32) as f32))
                            .with_rotation(prop.rotation)
                            .with_scale(prop.kind.scale()),
                        WorldProp,
                    ));
                }
            });

            voxel_map.chunk_entities.insert((cx, cz), chunk_entity);
            commands.entity(task_entity).despawn();
        }
    }
}
pub fn spawn_chunks_around_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_gen: Res<WorldGenerator>,
    existing_chunks: Query<&ChunkComponent>,
) {
    // Test trước một cụm 3x3 chunks quanh tọa độ (0, 0)
    let render_distance = 2;

    for cx in -render_distance..=render_distance {
        for cy in -render_distance..=render_distance {
            let coord = IVec2::new(cx, cy);

            // Bỏ qua nếu chunk này đã được spawn
            if existing_chunks.iter().any(|c| c.coord == coord) {
                continue;
            }

            // 1. Sinh dữ liệu voxel cho chunk
            let chunk_data = world_gen.generate_chunk(cx, cy);
            // 2. Chuyển mảng khối thành Mesh 3D
            if let Some(mesh) = crate::meshing::generate_chunk_mesh(&chunk_data) {
                let world_pos = Vec3::new(
                    (cx * CHUNK_SIZE_X as i32) as f32,
                    0.0,
                    (cy * CHUNK_SIZE_Y as i32) as f32,
                );

                // 3. Spawn Mesh ra Scene (Bevy 0.15+ dùng Mesh3d & MeshMaterial3d)
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.9,
                        ..default()
                    })),
                    Transform::from_translation(world_pos),
                    ChunkComponent { coord },
                ));
            }
        }
    }
}