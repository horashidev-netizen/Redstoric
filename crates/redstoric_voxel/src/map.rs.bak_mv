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
    pub fn get_highest_block(&self, world_x: f32, world_z: f32) -> f32 {
        let block_x = world_x.floor() as i32;
        let block_z = world_z.floor() as i32;

        let chunk_x = block_x.div_euclid(CHUNK_SIZE_X as i32);
        let chunk_z = block_z.div_euclid(CHUNK_SIZE_Z as i32);

        let local_x = block_x.rem_euclid(CHUNK_SIZE_X as i32) as usize;
        let local_z = block_z.rem_euclid(CHUNK_SIZE_Z as i32) as usize;

        if let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) {
            // Quét từ trên đỉnh chunk xuống đáy để tìm khối đất/cỏ đầu tiên
            for y in (0..CHUNK_SIZE_Y).rev() {
                let b = chunk.get_block(local_x, y, local_z);
                if !b.is_air() && !b.is_foliage() {
                    return y as f32 + 1.0;
                }
            }
        }
        0.0
    }

    /// Độ cao sàn để đi bộ tại (x, z) cho nhân vật đang đứng ở độ cao `feet_y`.
    /// Bỏ qua lá cây; trả về (độ_cao_sàn, bị_chặn). `bị_chặn` = true khi có khối đặc
    /// (vách đất, thân cây, cành...) nằm ở 2 lớp ngay trên bậc bước lên được.
    pub fn get_walk_height(&self, world_x: f32, world_z: f32, feet_y: f32) -> (f32, bool) {
        let block_x = world_x.floor() as i32;
        let block_z = world_z.floor() as i32;
        let chunk_x = block_x.div_euclid(CHUNK_SIZE_X as i32);
        let chunk_z = block_z.div_euclid(CHUNK_SIZE_Z as i32);
        let lx = block_x.rem_euclid(CHUNK_SIZE_X as i32) as usize;
        let lz = block_z.rem_euclid(CHUNK_SIZE_Z as i32) as usize;

        let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) else {
            return (0.0, false);
        };
        let top = CHUNK_SIZE_Y as i32 - 1;
        let base = ((feet_y + 0.25).floor() as i32).clamp(0, top);

        let mut floor_h = 0.0;
        for y in (0..=base).rev() {
            let b = chunk.get_block(lx, y as usize, lz);
            if !b.is_air() && !b.is_foliage() {
                floor_h = y as f32 + 1.0;
                break;
            }
        }
        let mut blocked = false;
        for dy in 1..=2 {
            let y = base + dy;
            if y <= top {
                let b = chunk.get_block(lx, y as usize, lz);
                if !b.is_air() && !b.is_foliage() && b != crate::block::BlockType::Water {
                    blocked = true;
                }
            }
        }
        (floor_h, blocked)
    }
}