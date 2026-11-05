use bevy::math::{Quat, Vec3};
use bevy::prelude::Resource;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, SuperSimplex};
use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use crate::{PropSpawnData, WorldPropKind};

#[derive(Resource, Clone)]
pub struct WorldGenerator {
    // Độ cao lục địa & đồi núi
    continental_noise: Fbm<Perlin>,
    mountain_noise: SuperSimplex,
    detail_noise: Perlin,

    // Khí hậu (Nhiệt độ & Độ ẩm)
    temperature_noise: SuperSimplex,
    humidity_noise: SuperSimplex,

    // Hang động 3D
    cave_noise: SuperSimplex,

    pub sea_level: usize,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        let continental_noise = Fbm::<Perlin>::new(seed)
            .set_octaves(4)
            .set_frequency(0.005)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        Self {
            continental_noise,
            mountain_noise: SuperSimplex::new(seed.wrapping_add(101)),
            detail_noise: Perlin::new(seed.wrapping_add(202)),
            temperature_noise: SuperSimplex::new(seed.wrapping_add(303)),
            humidity_noise: SuperSimplex::new(seed.wrapping_add(404)),
            cave_noise: SuperSimplex::new(seed.wrapping_add(505)),
            sea_level: 16,
        }
    }

    /// Tính độ cao bề mặt kết hợp nhiều tầng Noise
    #[inline]
    pub fn get_surface_height(&self, wx: f64, wz: f64) -> usize {
        let base = self.continental_noise.get([wx * 0.004, wz * 0.004]);
        let mtn = self.mountain_noise.get([wx * 0.012, wz * 0.012]).max(0.0).powi(2);
        let detail = self.detail_noise.get([wx * 0.04, wz * 0.04]) * 0.2;

        let height = (self.sea_level as f64 + (base * 18.0) + (mtn * 22.0) + (detail * 4.0) + 6.0) as isize;
        height.clamp(3, (CHUNK_SIZE_Y - 8) as isize) as usize
    }

    /// Lấy chỉ số nhiệt độ và độ ẩm
    #[inline]
    pub fn get_climate(&self, wx: f64, wz: f64) -> (f32, f32) {
        let t = (self.temperature_noise.get([wx * 0.0015, wz * 0.0015]) as f32 + 1.0) * 0.5;
        let h = (self.humidity_noise.get([wx * 0.0015, wz * 0.0015]) as f32 + 1.0) * 0.5;
        (t.clamp(0.0, 1.0), h.clamp(0.0, 1.0))
    }

    /// Hang động 3D ngầm
    #[inline]
    pub fn is_cave(&self, wx: f64, wy: f64, wz: f64, surface_y: usize) -> bool {
        if wy as usize >= surface_y.saturating_sub(3) || wy <= 3.0 {
            return false;
        }
        let density = self.cave_noise.get([wx * 0.04, wy * 0.04, wz * 0.04]).abs();
        density < 0.055
    }

    /// Hàm sinh toàn bộ khối trong một Chunk
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> ChunkData {
        let mut chunk = ChunkData::new();
        let base_wx = (chunk_x * CHUNK_SIZE_X as i32) as f64;
        let base_wz = (chunk_z * CHUNK_SIZE_Z as i32) as f64;

        // Lưu độ cao bề mặt 2D của chunk để tạo cây cối sau khi rải đất đá
        let mut surface_map = [[0usize; CHUNK_SIZE_Z]; CHUNK_SIZE_X];

        for x in 0..CHUNK_SIZE_X {
            for z in 0..CHUNK_SIZE_Z {
                let wx = base_wx + x as f64;
                let wz = base_wz + z as f64;

                let surface_y = self.get_surface_height(wx, wz);
                surface_map[x][z] = surface_y;
                let (temp, humid) = self.get_climate(wx, wz);

                for y in 0..CHUNK_SIZE_Y {
                    let wy = y as f64;

                    // 1. Phía trên mặt đất
                    if y > surface_y {
                        if y <= self.sea_level {
                            chunk.set_block(x, y, z, BlockType::Water);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Air);
                        }
                        continue;
                    }

                    // 2. Hang động ngầm 3D
                    if self.is_cave(wx, wy, wz, surface_y) {
                        chunk.set_block(x, y, z, BlockType::Air);
                        continue;
                    }

                    // 3. Phân tầng vật liệu Biome
                    let depth = surface_y - y;

                    if surface_y <= self.sea_level + 1 {
                        // Vùng bãi biển/đáy nước
                        if depth <= 2 {
                            chunk.set_block(x, y, z, BlockType::Sand);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else if temp < 0.28 || surface_y >= 45 {
                        // Vùng núi băng tuyết
                        if depth == 0 {
                            chunk.set_block(x, y, z, BlockType::Snow);
                        } else if depth <= 2 {
                            chunk.set_block(x, y, z, BlockType::CliffStone);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else if temp > 0.68 && humid < 0.35 {
                        // Sa mạc khô cằn
                        if depth <= 4 {
                            chunk.set_block(x, y, z, BlockType::Sand);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else {
                        // Đồng bằng & rừng xanh mát
                        if depth == 0 {
                            chunk.set_block(x, y, z, BlockType::Grass);
                        } else if depth <= 3 {
                            chunk.set_block(x, y, z, BlockType::Dirt);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    }
                }
            }
        }

        // 4. Sinh cây cối ngẫu nhiên trên mặt cỏ (Cây Voxel hình quả sồi)
        for x in 2..CHUNK_SIZE_X.saturating_sub(2) {
            for z in 2..CHUNK_SIZE_Z.saturating_sub(2) {
                let sy = surface_map[x][z];
                if sy > self.sea_level + 1 && sy + 7 < CHUNK_SIZE_Y {
                    if chunk.get_block(x, sy, z) == BlockType::Grass {
                        // Tỷ lệ sinh cây dựa trên độ băm vị trí
                        let hash_tree = (chunk_x as u32)
                            .wrapping_mul(73856093)
                            ^ (chunk_z as u32).wrapping_mul(83492791)
                            ^ (x as u32).wrapping_mul(19349663);
                        let hash = hash_tree;

                        if hash % 100 < 3 {
                            // Thân cây (3 block)
                            for ty in 1..=3 {
                                chunk.set_block(x, sy + ty, z, BlockType::Dirt);
                            }
                            // Tán lá hình cầu nhỏ (lá cây gán tạm khối Grass)
                            for lx in x.saturating_sub(1)..=x + 1 {
                                for lz in z.saturating_sub(1)..=z + 1 {
                                    chunk.set_block(lx, sy + 4, lz, BlockType::Grass);
                                    chunk.set_block(lx, sy + 5, lz, BlockType::Grass);
                                }
                            }
                            chunk.set_block(x, sy + 6, z, BlockType::Grass);
                        }
                    }
                }
            }
        }

        chunk
    }
    pub fn generate_chunk_with_props(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> (ChunkData, Vec<PropSpawnData>) {
        let mut chunk = ChunkData::new();
        let mut props = Vec::new();

        let base_wx = (chunk_x * CHUNK_SIZE_X as i32) as f64;
        let base_wz = (chunk_z * CHUNK_SIZE_Z as i32) as f64;
        let mut surface_map = [[0usize; CHUNK_SIZE_Z]; CHUNK_SIZE_X];

        // 1. Điền thể tích Voxel Terrain (Đất, đá, cát, tuyết)
        for x in 0..CHUNK_SIZE_X {
            for z in 0..CHUNK_SIZE_Z {
                let wx = base_wx + x as f64;
                let wz = base_wz + z as f64;

                let surface_y = self.get_surface_height(wx, wz);
                surface_map[x][z] = surface_y;
                let (temp, humid) = self.get_climate(wx, wz);

                for y in 0..CHUNK_SIZE_Y {
                    let wy = y as f64;

                    if y > surface_y {
                        if y <= self.sea_level {
                            chunk.set_block(x, y, z, BlockType::Water);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Air);
                        }
                        continue;
                    }

                    if self.is_cave(wx, wy, wz, surface_y) {
                        chunk.set_block(x, y, z, BlockType::Air);
                        continue;
                    }

                    let depth = surface_y - y;

                    if surface_y <= self.sea_level + 1 {
                        if depth <= 2 {
                            chunk.set_block(x, y, z, BlockType::Sand);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else if temp < 0.28 || surface_y >= 45 {
                        if depth == 0 {
                            chunk.set_block(x, y, z, BlockType::Snow);
                        } else if depth <= 2 {
                            chunk.set_block(x, y, z, BlockType::CliffStone);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else if temp > 0.68 && humid < 0.35 {
                        if depth <= 4 {
                            chunk.set_block(x, y, z, BlockType::Sand);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    } else {
                        if depth == 0 {
                            chunk.set_block(x, y, z, BlockType::Grass);
                        } else if depth <= 3 {
                            chunk.set_block(x, y, z, BlockType::Dirt);
                        } else {
                            chunk.set_block(x, y, z, BlockType::Stone);
                        }
                    }
                }
            }
        }

        // 2. Rải các đạo cụ/tài nguyên GLB (Props) ngẫu nhiên theo Biome
        for x in 2..CHUNK_SIZE_X - 2 {
            for z in 2..CHUNK_SIZE_Z - 2 {
                let sy = surface_map[x][z];
                if sy <= self.sea_level + 1 || sy >= CHUNK_SIZE_Y - 4 {
                    continue;
                }

                let wx = base_wx + x as f64;
                let wz = base_wz + z as f64;
                let (temp, _humid) = self.get_climate(wx, wz);

                // Hash vị trí phân bố đều
                let cx = chunk_x as u32;
                let cz = chunk_z as u32;
                let lx = x as u32;
                let lz = z as u32;

                let hash = cx.wrapping_mul(73856093)
                    ^ cz.wrapping_mul(83492791)
                    ^ lx.wrapping_mul(19349663)
                    ^ lz.wrapping_mul(3999971);

                let chance = hash % 10000;                let chance = hash % 10000;

                // GIẢM MẬT ĐỘ (chỉ ~0.4% cơ hội thay vì 3.2% như trước)
                let prop_kind = if chance < 8 {
                    // Rất hiếm khi có rương/thùng (0.08%)
                    if chance < 3 {
                        Some(WorldPropKind::Chest)
                    } else {
                        Some(WorldPropKind::Crate)
                    }
                } else if chance < 40 {
                    // Khúc gỗ rải rác tự nhiên theo Biome (0.32%)
                    if temp < 0.28 {
                        Some(WorldPropKind::FrostwoodLog)
                    } else if temp > 0.65 {
                        Some(WorldPropKind::HardwoodLog)
                    } else if sy > 32 {
                        Some(WorldPropKind::EldwoodLog)
                    } else {
                        Some(WorldPropKind::WoodLog)
                    }
                } else {
                    None
                };

                if let Some(kind) = prop_kind {
                    let angle = ((hash % 360) as f32).to_radians();

                    // NÂNG ĐỘ CAO Y:
                    // Mặt cỏ nằm ở sy. Khối voxel cỏ chiếm từ sy đến sy + 1.0.
                    // Đặt tâm mô hình ở sy + 1.35 đến 1.5 để đáy rương chạm khít mặt cỏ thay vì lún xuống đất
                    let y_offset = match kind {
                        WorldPropKind::Chest | WorldPropKind::Crate => 1.45,
                        _ => 1.30, // Khúc gỗ tròn nhỏ hơn
                    };

                    let world_pos = Vec3::new(
                        wx as f32 + 0.5,
                        sy as f32 + y_offset,
                        wz as f32 + 0.5,
                    );

                    props.push(PropSpawnData {
                        kind,
                        world_pos,
                        rotation: Quat::from_rotation_y(angle),
                    });
                }
            }
        }

        // 3. Sinh cây Voxel tự nhiên trên mặt cỏ (Đồng bằng & Rừng)
        for x in 2..CHUNK_SIZE_X - 2 {
            for z in 2..CHUNK_SIZE_Z - 2 {
                let sy = surface_map[x][z];

                // Chỉ mọc trên mặt đất khô ráo và không vượt quá trần chunk
                if sy > self.sea_level + 1 && sy + 8 < CHUNK_SIZE_Y {
                    let current_surface_block = chunk.get_block(x, sy, z);

                    if current_surface_block == BlockType::Grass {
                        // Dùng wrapping_mul chống overflow
                        let cx = chunk_x as u32;
                        let cz = chunk_z as u32;
                        let lx = x as u32;
                        let lz = z as u32;

                        let hash_tree = cx.wrapping_mul(73856093)
                            ^ cz.wrapping_mul(83492791)
                            ^ lx.wrapping_mul(19349663)
                            ^ lz.wrapping_mul(3999971);

                        // TĂNG TỶ LỆ MỌC CÂY: 2.5% mỗi block bề mặt
                        if hash_tree % 100 < 3 {
                            // Chiều cao thân cây ngẫu nhiên 3 đến 5 block
                            let trunk_height = 3 + (hash_tree as usize % 3);

                            // 1. Thân cây gỗ (Block Dirt/Wood)
                            for ty in 1..=trunk_height {
                                chunk.set_block(x, sy + ty, z, BlockType::Dirt);
                            }

                            // 2. Tán lá cây (Block Grass xanh mát) dạng hình tán cầu
                            let top_y = sy + trunk_height;

                            // Lớp lá rộng tầng dưới
                            for lx in x.saturating_sub(2)..=x + 2 {
                                for lz in z.saturating_sub(2)..=z + 2 {
                                    if (lx as isize - x as isize).abs() + (lz as isize - z as isize).abs() <= 3 {
                                        chunk.set_block(lx, top_y, lz, BlockType::Grass);
                                        chunk.set_block(lx, top_y + 1, lz, BlockType::Grass);
                                    }
                                }
                            }

                            // Lớp lá đỉnh ngọn cây
                            for lx in x.saturating_sub(1)..=x + 1 {
                                for lz in z.saturating_sub(1)..=z + 1 {
                                    chunk.set_block(lx, top_y + 2, lz, BlockType::Grass);
                                }
                            }
                            chunk.set_block(x, top_y + 3, z, BlockType::Grass);
                        }
                    }
                }
            }
        }
        (chunk, props)
    }
}