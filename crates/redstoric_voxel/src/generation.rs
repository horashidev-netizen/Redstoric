use bevy::math::{Quat, Vec3};
use bevy::prelude::Resource;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, SuperSimplex};
use crate::block::BlockType;
use crate::structures::Site;
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

    // Rừng: mật độ & cụm loài
    forest_noise: SuperSimplex,
    species_noise: SuperSimplex,

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
            forest_noise: SuperSimplex::new(seed.wrapping_add(606)),
            species_noise: SuperSimplex::new(seed.wrapping_add(707)),
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

        self.generate_trees(&mut chunk, chunk_x, chunk_z, &[]);

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

        // 1b. San phẳng địa hình quanh các khu công trình
        let ox = chunk_x * CHUNK_SIZE_X as i32;
        let oz = chunk_z * CHUNK_SIZE_Z as i32;
        let sites = self.sites_near_chunk(chunk_x, chunk_z);
        self.level_sites(&mut chunk, ox, oz, &sites, &mut surface_map);

        // 2. Rải các đạo cụ/tài nguyên GLB (Props) ngẫu nhiên theo Biome
        for x in 2..CHUNK_SIZE_X - 2 {
            for z in 2..CHUNK_SIZE_Z - 2 {
                let sy = surface_map[x][z];
                if sy <= self.sea_level + 1 || sy >= CHUNK_SIZE_Y - 4 {
                    continue;
                }

                let wx = base_wx + x as f64;
                let wz = base_wz + z as f64;
                if sites.iter().any(|s| s.contains(wx as i32, wz as i32, crate::structures::BLEND)) {
                    continue;
                }
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
                let chance = hash % 10000;

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

        // 2b. Rải sprite (cỏ, hoa, nấm, đá, xương rồng...) theo biome
        self.scatter_sprites(&chunk, ox, oz, &surface_map, &sites, &mut props);

        // 3. Công trình (làng, đấu trường boss) rồi tới cây cối
        self.build_sites(&mut chunk, ox, oz, &sites, &mut props);
        self.generate_trees(&mut chunk, chunk_x, chunk_z, &sites);
        (chunk, props)
    }
}

// =====================================================================
// SINH CÂY (tham khảo Veloren: mật độ theo vùng, nhiều loài, khoảng trống)
// =====================================================================
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ground {
    Grass,
    Desert,
    Snow,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Species {
    Oak,
    BigOak,
    Birch,
    Bush,
    Acacia,
    Pine,
    Dead,
    Cactus,
}

#[inline]
fn tree_hash(a: i32, b: i32, salt: u32) -> u32 {
    let mut h = (a as u32).wrapping_mul(73856093)
        ^ (b as u32).wrapping_mul(83492791)
        ^ salt.wrapping_mul(2654435761);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h
}

impl WorldGenerator {
    /// Loại nền đất tại vị trí (khớp với logic phân tầng vật liệu ở trên).
    fn ground_at(&self, wx: f64, wz: f64, sy: usize) -> Option<Ground> {
        if sy <= self.sea_level + 1 {
            return None;
        }
        let (temp, humid) = self.get_climate(wx, wz);
        if temp < 0.28 || sy >= 45 {
            Some(Ground::Snow)
        } else if temp > 0.68 && humid < 0.35 {
            Some(Ground::Desert)
        } else {
            Some(Ground::Grass)
        }
    }

    /// Ghi 1 block (toạ độ thế giới) nếu nằm trong chunk. Lá chỉ đè lên Air.
    #[inline]
    fn put(chunk: &mut ChunkData, ox: i32, oz: i32, wx: i32, wy: i32, wz: i32, b: BlockType) {
        let lx = wx - ox;
        let lz = wz - oz;
        if lx < 0 || lz < 0 || wy < 0 {
            return;
        }
        let (lx, lz, y) = (lx as usize, lz as usize, wy as usize);
        if lx >= CHUNK_SIZE_X || lz >= CHUNK_SIZE_Z || y >= CHUNK_SIZE_Y {
            return;
        }
        let cur = chunk.get_block(lx, y, lz);
        let is_leaf = matches!(b, BlockType::Leaves | BlockType::LeavesLight | BlockType::PineLeaves);
        let can = if is_leaf {
            cur == BlockType::Air
        } else {
            matches!(cur, BlockType::Air | BlockType::Leaves | BlockType::LeavesLight | BlockType::PineLeaves)
        };
        if can {
            chunk.set_block(lx, y, lz, b);
        }
    }

    fn pick_species(&self, ground: Ground, wx: f64, wz: f64, temp: f32, humid: f32, h: u32) -> Option<Species> {
        let r = h % 100;
        match ground {
            Ground::Desert => Some(Species::Cactus),
            Ground::Snow => {
                if r < 97 { Some(Species::Pine) } else { Some(Species::Dead) }
            }
            Ground::Grass => {
                // Savanna: nóng & hơi khô
                if temp > 0.58 && humid < 0.5 {
                    return Some(if r < 80 { Species::Acacia } else if r < 96 { Species::Bush } else { Species::Dead });
                }
                // Vùng lạnh mát: thông xen sồi
                if temp < 0.38 {
                    return Some(if r < 60 { Species::Pine } else if r < 85 { Species::Birch } else { Species::Oak });
                }
                // Cụm loài thay đổi theo vùng (như "forest lottery" của Veloren)
                let s = (self.species_noise.get([wx * 0.012, wz * 0.012]) as f32 + 1.0) * 0.5;
                if humid < 0.28 {
                    return Some(if r < 12 { Species::Dead } else if r < 45 { Species::Bush } else { Species::Oak });
                }
                Some(if s > 0.62 {
                    // vùng bạch dương
                    if r < 65 { Species::Birch } else if r < 85 { Species::Oak } else { Species::Bush }
                } else if s < 0.32 {
                    // vùng sồi lớn
                    if r < 35 { Species::BigOak } else if r < 80 { Species::Oak } else { Species::Bush }
                } else if r < 12 {
                    Species::BigOak
                } else if r < 25 {
                    Species::Birch
                } else if r < 45 {
                    Species::Bush
                } else {
                    Species::Oak
                })
            }
        }
    }

    /// Sinh toàn bộ cây cho chunk. Cây được đặt theo lưới có lệch ngẫu nhiên
    /// (đảm bảo khoảng cách), duyệt cả ô lân cận để cây vắt qua biên chunk liền mạch.
    pub fn generate_trees(&self, chunk: &mut ChunkData, chunk_x: i32, chunk_z: i32, sites: &[Site]) {
        const CELL: i32 = 8;
        const MARGIN: i32 = 12;
        let ox = chunk_x * CHUNK_SIZE_X as i32;
        let oz = chunk_z * CHUNK_SIZE_Z as i32;

        let c_min_x = (ox - MARGIN).div_euclid(CELL);
        let c_max_x = (ox + CHUNK_SIZE_X as i32 + MARGIN).div_euclid(CELL);
        let c_min_z = (oz - MARGIN).div_euclid(CELL);
        let c_max_z = (oz + CHUNK_SIZE_Z as i32 + MARGIN).div_euclid(CELL);

        for cellx in c_min_x..=c_max_x {
            for cellz in c_min_z..=c_max_z {
                let h = tree_hash(cellx, cellz, 11);
                let tx = cellx * CELL + 1 + (h % (CELL as u32 - 2)) as i32;
                let tz = cellz * CELL + 1 + ((h >> 8) % (CELL as u32 - 2)) as i32;
                // Bỏ qua nhanh nếu tán cây không thể chạm chunk
                if tx < ox - MARGIN || tx >= ox + CHUNK_SIZE_X as i32 + MARGIN
                    || tz < oz - MARGIN || tz >= oz + CHUNK_SIZE_Z as i32 + MARGIN
                {
                    continue;
                }

                // Không mọc cây trong khu làng / đấu trường
                if sites.iter().any(|s| s.contains(tx, tz, crate::structures::BLEND + 3)) {
                    continue;
                }

                let (fx, fz) = (tx as f64, tz as f64);
                let sy = self.get_surface_height(fx, fz);
                let ground = match self.ground_at(fx, fz, sy) {
                    Some(g) => g,
                    None => continue,
                };
                let (temp, humid) = self.get_climate(fx, fz);

                // Mật độ rừng theo vùng: rừng rậm / thưa / bãi trống
                let forest = (self.forest_noise.get([fx * 0.011, fz * 0.011]) as f32 + 1.0) * 0.5;
                let detail = (self.forest_noise.get([fx * 0.07 + 90.0, fz * 0.07]) as f32 + 1.0) * 0.5;
                let mut p = match ground {
                    Ground::Grass => {
                        let base = ((forest - 0.42) * 2.4).clamp(0.0, 0.85);
                        // bãi trống nhỏ trong rừng + vài cây lẻ ngoài đồng
                        let clearing = if detail < 0.30 { 0.15 } else { 1.0 };
                        (base * clearing).max(0.03) * (0.5 + humid)
                    }
                    Ground::Desert => 0.05 * forest,
                    Ground::Snow => ((forest - 0.45) * 1.6).clamp(0.0, 0.5) * if sy > 40 { 0.4 } else { 1.0 },
                };
                p = p.clamp(0.0, 0.9);
                if (tree_hash(cellx, cellz, 29) % 1000) as f32 / 1000.0 >= p {
                    continue;
                }

                let hs = tree_hash(cellx, cellz, 53);
                let sp = match self.pick_species(ground, fx, fz, temp, humid, hs) {
                    Some(s) => s,
                    None => continue,
                };
                self.place_tree(chunk, ox, oz, tx, sy as i32, tz, sp, tree_hash(cellx, cellz, 97));
            }
        }
    }

    /// Sinh cây phân nhánh đệ quy theo thuật toán của Veloren (ProceduralTree):
    /// thân -> cành -> cành con, lá là hình cầu ở đầu các cành cuối.
    #[allow(clippy::too_many_arguments)]
    fn place_tree(&self, chunk: &mut ChunkData, ox: i32, oz: i32, x: i32, sy: i32, z: i32, sp: Species, h: u32) {
        let max_h = CHUNK_SIZE_Y as i32 - sy - 3;
        let rnd = |m: u32, salt: u32| (tree_hash(x, z, salt ^ h) % m) as i32;
        let mut rng = TRng(h | 1);
        let scale = 0.85 + rng.f().powi(2) * 0.5;
        let mut cfg = match sp {
            Species::Oak => TreeCfg {
                trunk_len: 9.0 * scale, trunk_r: 1.3, child_len: 0.62, child_r: 0.6,
                leaf_r: (2.0, 2.6), straight: 0.35, depth: 2, splits: (3.5, 5.5),
                split_range: (0.75, 1.5), lvs: 1.0, wood: BlockType::Wood, leaf: BlockType::Leaves,
            },
            Species::BigOak => TreeCfg {
                trunk_len: 12.0 * scale, trunk_r: 1.8, child_len: 0.65, child_r: 0.6,
                leaf_r: (2.4, 3.2), straight: 0.35, depth: 2, splits: (4.0, 6.0),
                split_range: (0.7, 1.4), lvs: 0.9, wood: BlockType::Wood, leaf: BlockType::Leaves,
            },
            Species::Birch => TreeCfg {
                trunk_len: 10.5 * scale, trunk_r: 1.0, child_len: 0.55, child_r: 0.8,
                leaf_r: (1.5, 2.0), straight: 0.7, depth: 2, splits: (2.5, 4.5),
                split_range: (0.5, 1.2), lvs: 1.4, wood: BlockType::BirchWood, leaf: BlockType::LeavesLight,
            },
            Species::Acacia => TreeCfg {
                trunk_len: 6.5 * scale, trunk_r: 1.2, child_len: 1.0, child_r: 0.7,
                leaf_r: (2.6, 3.4), straight: 0.15, depth: 2, splits: (2.0, 3.5),
                split_range: (0.9, 1.4), lvs: 0.3, wood: BlockType::Wood, leaf: BlockType::LeavesLight,
            },
            Species::Dead => TreeCfg {
                trunk_len: 5.5 * scale, trunk_r: 1.2, child_len: 0.7, child_r: 0.7,
                leaf_r: (0.0, 0.0), straight: 0.45, depth: 2, splits: (2.0, 3.0),
                split_range: (0.75, 1.5), lvs: 1.0, wood: BlockType::Wood, leaf: BlockType::Leaves,
            },
            Species::Bush => TreeCfg {
                trunk_len: 2.0 * scale, trunk_r: 0.8, child_len: 0.8, child_r: 0.8,
                leaf_r: (1.4, 1.9), straight: 0.2, depth: 1, splits: (3.0, 4.5),
                split_range: (0.7, 1.4), lvs: 0.9, wood: BlockType::Wood, leaf: BlockType::Leaves,
            },
            Species::Pine => {
                let th = (7 + rnd(5, 10)).min(max_h - 2);
                if th < 5 { return; }
                for y in 1..=th { Self::put(chunk, ox, oz, x, sy + y, z, BlockType::Wood); }
                let top = sy + th + 1;
                let max_r = 2 + (th >= 10) as i32;
                for y in (sy + 3)..=top {
                    let k = top - y;
                    let r = if k == 0 { 0 } else { ((k + 1) / 2).min(max_r) - (k % 2 == 1 && k > 2) as i32 };
                    for dx in -r..=r {
                        for dz in -r..=r {
                            if dx * dx + dz * dz <= r * r + r / 2 + (r == 0) as i32 {
                                Self::put(chunk, ox, oz, x + dx, y, z + dz, BlockType::PineLeaves);
                            }
                        }
                    }
                }
                return;
            }
            Species::Cactus => {
                // Xương rồng saguaro: thân + 1-2 tay cong lên
                let th = (3 + rnd(4, 12)).min(max_h);
                for y in 1..=th { Self::put(chunk, ox, oz, x, sy + y, z, BlockType::Cactus); }
                if th >= 4 {
                    for (i, (dx, dz)) in [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().enumerate() {
                        if rnd(3, 20 + i as u32) == 0 {
                            let by = sy + 2 + rnd((th - 2).max(1) as u32, 30 + i as u32);
                            let up = 1 + rnd(3, 40 + i as u32);
                            Self::put(chunk, ox, oz, x + dx, by, z + dz, BlockType::Cactus);
                            Self::put(chunk, ox, oz, x + dx * 2, by, z + dz * 2, BlockType::Cactus);
                            for u in 1..=up { Self::put(chunk, ox, oz, x + dx * 2, by + u, z + dz * 2, BlockType::Cactus); }
                        }
                    }
                }
                return;
            }
        };

        // Nếu sát trần thế giới thì thu nhỏ thân để cây không bị cắt cụt ở đỉnh
        let room = CHUNK_SIZE_Y as f32 - 2.0 - (sy as f32 + 1.0);
        let need = cfg.trunk_len * 1.15 + cfg.leaf_r.1 * cfg.lvs.max(1.0);
        if need > room {
            if room < 5.0 { return; }
            cfg.trunk_len *= room / need;
        }

        // Gốc cây hơi chìm xuống đất để không hở; Y là trục đứng.
        let origin = Vec3::new(x as f32 + 0.5, sy as f32 + 0.6, z as f32 + 0.5);
        let dir = Vec3::new(rng.r(), 10.0, rng.r()).normalize();
        let mut branches: Vec<Branch> = Vec::new();
        grow_branch(&mut branches, &cfg, &mut rng, origin, dir, cfg.trunk_len, cfg.trunk_r, cfg.trunk_r * 0.75, 0);

        // Bao quanh toàn bộ cây
        let mut bmin = Vec3::splat(f32::MAX);
        let mut bmax = Vec3::splat(f32::MIN);
        for b in &branches { bmin = bmin.min(b.min); bmax = bmax.max(b.max); }
        let x0 = (bmin.x.floor() as i32).max(ox);
        let x1 = (bmax.x.ceil() as i32).min(ox + CHUNK_SIZE_X as i32 - 1);
        let z0 = (bmin.z.floor() as i32).max(oz);
        let z1 = (bmax.z.ceil() as i32).min(oz + CHUNK_SIZE_Z as i32 - 1);
        let y0 = (bmin.y.floor() as i32).max(sy + 1);
        let y1 = (bmax.y.ceil() as i32).min(CHUNK_SIZE_Y as i32 - 1);
        if x0 > x1 || z0 > z1 || y0 > y1 { return; }

        for wx in x0..=x1 {
            for wz in z0..=z1 {
                for wy in y0..=y1 {
                    let p = Vec3::new(wx as f32 + 0.5, wy as f32 + 0.5, wz as f32 + 0.5);
                    let mut leaf = false;
                    let mut wood = false;
                    for b in &branches {
                        if p.x < b.min.x || p.x > b.max.x || p.y < b.min.y || p.y > b.max.y || p.z < b.min.z || p.z > b.max.z {
                            continue;
                        }
                        let ab = b.b - b.a;
                        let len_sq = ab.length_squared();
                        let t = if len_sq < 1e-4 { 0.0 } else { ((p - b.a).dot(ab) / len_sq).clamp(0.0, 1.0) };
                        let q = b.a + ab * t;
                        let d = p - q;
                        let wr = (b.r0 + (b.r1 - b.r0) * t).max(0.62);
                        if d.length_squared() < wr * wr {
                            wood = true;
                            break;
                        }
                        if b.leaf_r > 0.0 {
                            let dd = Vec3::new(d.x, d.y / b.lvs, d.z);
                            if dd.length_squared() < b.leaf_r * b.leaf_r {
                                leaf = true;
                            }
                        }
                    }
                    if wood {
                        Self::put(chunk, ox, oz, wx, wy, wz, cfg.wood);
                    } else if leaf {
                        Self::put(chunk, ox, oz, wx, wy, wz, cfg.leaf);
                    }
                }
            }
        }
    }
}

struct TreeCfg {
    trunk_len: f32,
    trunk_r: f32,
    child_len: f32,
    child_r: f32,
    leaf_r: (f32, f32),
    straight: f32,
    depth: usize,
    splits: (f32, f32),
    split_range: (f32, f32),
    lvs: f32,
    wood: BlockType,
    leaf: BlockType,
}

struct Branch {
    a: Vec3,
    b: Vec3,
    r0: f32,
    r1: f32,
    leaf_r: f32,
    lvs: f32,
    min: Vec3,
    max: Vec3,
}

/// RNG xorshift nhỏ, tất định theo seed của cây (cây vắt qua chunk luôn giống nhau).
struct TRng(u32);
impl TRng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// [0, 1)
    fn f(&mut self) -> f32 { (self.next() >> 8) as f32 / (1u32 << 24) as f32 }
    /// [-1, 1)
    fn r(&mut self) -> f32 { self.f() * 2.0 - 1.0 }
    fn range(&mut self, a: f32, b: f32) -> f32 { a + (b - a) * self.f() }
}

#[allow(clippy::too_many_arguments)]
fn grow_branch(
    out: &mut Vec<Branch>,
    cfg: &TreeCfg,
    rng: &mut TRng,
    start: Vec3,
    dir: Vec3,
    len: f32,
    r0: f32,
    r1: f32,
    depth: usize,
) {
    let end = start + dir * len;
    let leaf_r = if depth == cfg.depth { rng.range(cfg.leaf_r.0, cfg.leaf_r.1) } else { 0.0 };
    let pad = r0.max(r1).max(leaf_r) + 0.5;
    let pad_y = r0.max(r1).max(leaf_r * cfg.lvs) + 0.5;
    out.push(Branch {
        a: start,
        b: end,
        r0,
        r1,
        leaf_r,
        lvs: cfg.lvs.max(0.1),
        min: start.min(end) - Vec3::new(pad, pad_y, pad),
        max: start.max(end) + Vec3::new(pad, pad_y, pad),
    });

    if depth >= cfg.depth {
        return;
    }
    let splits = rng.range(cfg.splits.0, cfg.splits.1).round().max(1.0) as usize;
    let ab = end - start;
    let len_sq = ab.length_squared().max(1e-4);
    for _ in 0..splits {
        let split_factor = rng.range(cfg.split_range.0, cfg.split_range.1).clamp(0.0, 1.5);
        let tgt = start + ab * split_factor + Vec3::new(rng.r(), rng.r(), rng.r());
        // điểm gần nhất trên thân cha (kẹp trong đoạn)
        let t = ((tgt - start).dot(ab) / len_sq).clamp(0.0, 1.0);
        let bstart = start + ab * t;
        let mut bdir = (tgt - bstart) * (1.0 - cfg.straight) + dir * cfg.straight;
        if bdir.length_squared() < 1e-4 {
            bdir = dir;
        }
        let bdir = bdir.normalize();
        let blen = len * cfg.child_len * (1.0 - (split_factor - 0.5) * 2.0 * 0.0);
        let child_r1 = (r1 * cfg.child_r).max(0.0);
        grow_branch(out, cfg, rng, bstart, bdir, blen, r1, child_r1, depth + 1);
    }
}
