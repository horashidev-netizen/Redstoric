pub mod block;
pub mod chunk;
pub mod generation;
pub mod map;
pub mod meshing;
pub mod systems;
pub mod worldgen;
pub mod prop;
pub use prop::{WorldProp, WorldPropKind, PropSpawnData};

pub use block::BlockType;
pub use chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
pub use generation::WorldGenerator;
pub use map::VoxelWorldMap;
pub use systems::VoxelViewer; // <-- Thêm dòng này

use bevy::prelude::*;
use systems::{chunk_streaming_system, handle_chunk_tasks, setup_voxel_world};

pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorldMap>()
            .insert_resource(WorldGenerator::new(1337))
            .add_systems(Startup, setup_voxel_world)
            .add_systems(Update, (chunk_streaming_system, handle_chunk_tasks));
    }
}