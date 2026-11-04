use noise::{NoiseFn, Perlin};
use bevy::prelude::Resource;
use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

#[derive(Resource, Clone)]
pub struct WorldGenerator {
    perlin: Perlin,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            perlin: Perlin::new(seed),
        }
    }

    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> ChunkData {
        let mut chunk = ChunkData::new();

        for x in 0..CHUNK_SIZE_X {
            for z in 0..CHUNK_SIZE_Z {
                let world_x = (chunk_x * CHUNK_SIZE_X as i32 + x as i32) as f64 * 0.05;
                let world_z = (chunk_z * CHUNK_SIZE_Z as i32 + z as i32) as f64 * 0.05;

                let noise_val = self.perlin.get([world_x, world_z]);
                let height = ((noise_val + 1.0) * 0.5 * 16.0 + 6.0) as usize;
                let height = height.clamp(1, CHUNK_SIZE_Y - 1);

                for y in 0..CHUNK_SIZE_Y {
                    if y > height {
                        chunk.set_block(x, y, z, BlockType::Air);
                    } else if y == height {
                        chunk.set_block(x, y, z, BlockType::Grass);
                    } else if y >= height.saturating_sub(3) {
                        chunk.set_block(x, y, z, BlockType::Dirt);
                    } else {
                        chunk.set_block(x, y, z, BlockType::Stone);
                    }
                }
            }
        }
        chunk
    }
}