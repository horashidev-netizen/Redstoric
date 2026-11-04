use std::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

#[derive(Resource, Default)]
pub struct VoxelWorldMap {
    /// Dữ liệu khối để tính toán va chạm vật lý
    pub chunks: HashMap<(i32, i32), ChunkData>,
    /// Entity ID của từng Chunk mesh để despawn khi đi xa
    pub chunk_entities: HashMap<(i32, i32), Entity>,
    /// Danh sách các chunk đang được tính toán ngầm (tránh spawn trùng)
    pub loading_chunks: HashSet<(i32, i32)>,
}

impl VoxelWorldMap {
    /// Lấy độ cao khối cao nhất để người chơi leo đồi / đứng vững
    pub fn get_highest_block(&self, world_x: f32, world_z: f32) -> f32 {
        let block_x = world_x.floor() as i32;
        let block_z = world_z.floor() as i32;

        let chunk_x = block_x.div_euclid(CHUNK_SIZE_X as i32);
        let chunk_z = block_z.div_euclid(CHUNK_SIZE_Z as i32);

        let local_x = block_x.rem_euclid(CHUNK_SIZE_X as i32) as usize;
        let local_z = block_z.rem_euclid(CHUNK_SIZE_Z as i32) as usize;

        if let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) {
            for y in (0..CHUNK_SIZE_Y).rev() {
                if !chunk.get_block(local_x, y, local_z).is_air() {
                    return y as f32 + 1.0;
                }
            }
        }
        0.0
    }
}