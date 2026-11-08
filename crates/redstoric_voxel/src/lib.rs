pub mod block;
pub mod chunk;
pub mod generation;
pub mod map;
pub mod meshing;
pub mod systems;
pub mod worldgen;
pub mod prop;
pub mod sprites;
pub mod structures;
pub use prop::{BossSpawnPoint, WorldProp, WorldPropKind, PropSpawnData};

pub use block::BlockType;
pub use chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
pub use generation::WorldGenerator;
pub use map::VoxelWorldMap;
pub use systems::{VoxelStreamSettings, VoxelViewer};

use bevy::prelude::*;
use systems::{chunk_streaming_system, handle_chunk_tasks, setup_voxel_world, spawn_chunk_props};

/// Hạt giống thế giới. Server và mọi client phải dùng cùng giá trị để chunk sinh giống nhau.
pub const WORLD_SEED: u32 = 1337;

pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorldMap>()
            .init_resource::<VoxelStreamSettings>()
            .insert_resource(WorldGenerator::new(WORLD_SEED))
            .add_systems(Startup, setup_voxel_world)
            .add_systems(Update, (chunk_streaming_system, handle_chunk_tasks, spawn_chunk_props));
    }
}