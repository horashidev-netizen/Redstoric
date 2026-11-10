pub mod cave;
pub mod climate;
pub mod surface;

use crate::block::BlockType;
use cave::CaveGenerator;
use climate::{Biome, ClimateGenerator};
use surface::SurfaceGenerator;

pub const CHUNK_SIZE_X: usize = 16;
pub const CHUNK_SIZE_Y: usize = 16;
pub const CHUNK_SIZE_Z: usize = 128;

pub struct WorldGenerator {
    surface: SurfaceGenerator,
    climate: ClimateGenerator,
    cave: CaveGenerator,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            surface: SurfaceGenerator::new(seed),
            climate: ClimateGenerator::new(seed),
            cave: CaveGenerator::new(seed),
        }
    }

    /// Điền dữ liệu block cho 1 Chunk tại tọa độ (chunk_x, chunk_y)
    pub fn generate_chunk(
        &self,
        chunk_x: i32,
        chunk_y: i32,
        blocks: &mut [BlockType; CHUNK_SIZE_X * CHUNK_SIZE_Y * CHUNK_SIZE_Z],
    ) {
        let base_wx = (chunk_x * CHUNK_SIZE_X as i32) as f64;
        let base_wy = (chunk_y * CHUNK_SIZE_Y as i32) as f64;

        for x in 0..CHUNK_SIZE_X {
            for y in 0..CHUNK_SIZE_Y {
                let wx = base_wx + x as f64;
                let wy = base_wy + y as f64;

                let surface_z = self.surface.sample_height(wx, wy);

                for z in 0..CHUNK_SIZE_Z {
                    let wz = z as f64;
                    let idx = x + (y * CHUNK_SIZE_X) + (z * CHUNK_SIZE_X * CHUNK_SIZE_Y);

                    // 1. Phía trên mặt đất
                    if wz > surface_z {
                        if wz <= self.surface.sea_level {
                            blocks[idx] = BlockType::Water;
                        } else {
                            blocks[idx] = BlockType::Air;
                        }
                        continue;
                    }

                    // 2. Kiểm tra hang động ngầm
                    if self.cave.is_cave(wx, wy, wz, surface_z) {
                        blocks[idx] = BlockType::Air;
                        continue;
                    }

                    // 3. Phân tầng bề mặt dựa trên Biome
                    let (temp, humid) = self.climate.sample_climate(wx, wy, wz);
                    let biome = self.climate.get_biome(temp, humid, wz, surface_z, self.surface.sea_level);
                    let depth = surface_z - wz;

                    blocks[idx] = match biome {
                        Biome::Desert => {
                            if depth < 4.0 { BlockType::Sand } else { BlockType::Stone }
                        }
                        Biome::SnowMountain => {
                            if depth < 1.0 { BlockType::Snow } else { BlockType::Stone }
                        }
                        Biome::Ocean => {
                            if depth < 3.0 { BlockType::Sand } else { BlockType::Stone }
                        }
                        Biome::Forest | Biome::Savanna => {
                            if depth < 1.0 {
                                BlockType::Grass
                            } else if depth < 4.0 {
                                BlockType::Dirt
                            } else {
                                BlockType::Stone
                            }
                        }
                    };
                }
            }
        }
    }
}