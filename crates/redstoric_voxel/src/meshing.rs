use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

pub use generate_chunk_mesh as build_greedy_mesh;

pub struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Thêm 1 Quad gồm 4 đỉnh, mỗi đỉnh nhận một màu riêng (cho phép gán Ambient Occlusion từng góc)
    fn add_quad_with_ao(
        &mut self,
        v0: [f32; 3],
        v1: [f32; 3],
        v2: [f32; 3],
        v3: [f32; 3],
        normal: [f32; 3],
        c0: [f32; 4],
        c1: [f32; 4],
        c2: [f32; 4],
        c3: [f32; 4],
    ) {
        let base_index = self.positions.len() as u32;

        self.positions.extend([v0, v1, v2, v3]);
        self.normals.extend([normal; 4]);
        self.uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        self.colors.extend([c0, c1, c2, c3]);

        // Nếu góc bị che gập nếp chéo, flip index để tránh méo bóng AO
        self.indices.extend([
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    pub fn build(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));

        Some(mesh)
    }
}

/// Màu Linear sRGB cơ bản cho từng khối
fn get_block_base_color(block: BlockType) -> [f32; 4] {
    match block {
        BlockType::Grass => [0.20, 0.58, 0.18, 1.0],      // Xanh cỏ
        BlockType::Dirt => [0.38, 0.24, 0.14, 1.0],       // Nâu đất
        BlockType::Stone => [0.48, 0.48, 0.50, 1.0],      // Xám đá
        BlockType::CliffStone => [0.32, 0.32, 0.35, 1.0], // Đá vách núi
        BlockType::Sand => [0.80, 0.74, 0.52, 1.0],       // Cát
        BlockType::Snow => [0.92, 0.94, 0.97, 1.0],       // Tuyết
        BlockType::Water => [0.15, 0.45, 0.85, 0.75],     // Nước
        BlockType::Wood => [0.36, 0.25, 0.14, 1.0],       // Gỗ sồi
        BlockType::BirchWood => [0.86, 0.85, 0.80, 1.0],  // Gỗ bạch dương
        BlockType::Cactus => [0.18, 0.52, 0.24, 1.0],     // Xương rồng
        BlockType::Leaves => [0.16, 0.50, 0.14, 1.0],     // Lá sồi
        BlockType::LeavesLight => [0.48, 0.64, 0.20, 1.0],// Lá sáng (bạch dương/acacia)
        BlockType::PineLeaves => [0.08, 0.32, 0.20, 1.0], // Lá thông
        BlockType::Planks => [0.62, 0.46, 0.27, 1.0],     // Ván gỗ
        BlockType::Cobble => [0.55, 0.55, 0.56, 1.0],     // Đá lát
        BlockType::RoofTile => [0.65, 0.22, 0.18, 1.0],   // Ngói đỏ
        BlockType::Bed => [0.75, 0.20, 0.25, 1.0],        // Giường
        BlockType::Obsidian => [0.12, 0.10, 0.18, 1.0],   // Hắc diện thạch
        BlockType::Farmland => [0.30, 0.19, 0.11, 1.0],   // Đất ruộng
        BlockType::Wheat => [0.85, 0.72, 0.22, 1.0],      // Lúa mì chín
        BlockType::Rice => [0.55, 0.80, 0.30, 1.0],       // Lúa non
        BlockType::Air => [0.0, 0.0, 0.0, 0.0],
    }
}

#[inline(always)]
fn is_solid(chunk: &ChunkData, x: i32, y: i32, z: i32) -> bool {
    if x < 0 || x >= CHUNK_SIZE_X as i32
        || y < 0 || y >= CHUNK_SIZE_Y as i32
        || z < 0 || z >= CHUNK_SIZE_Z as i32
    {
        return false;
    }
    !matches!(chunk.blocks[x as usize][y as usize][z as usize], BlockType::Air | BlockType::Water)
}

/// Một mặt của khối `current` có cần vẽ không, dựa trên khối kề bên tại (x, y, z)?
/// - Khối đặc: vẽ khi bên cạnh là không khí hoặc nước.
/// - Nước: chỉ vẽ khi bên cạnh là không khí (bỏ các mặt nằm giữa hai khối nước).
/// - Dưới đáy chunk (y < 0): không vẽ vì không bao giờ nhìn thấy.
/// - Ngoài biên chunk theo X/Z hoặc phía trên: vẽ (chưa biết chunk kề bên có gì).
#[inline(always)]
fn face_visible(chunk: &ChunkData, current: BlockType, x: i32, y: i32, z: i32) -> bool {
    if y < 0 {
        return false;
    }
    if x < 0
        || x >= CHUNK_SIZE_X as i32
        || y >= CHUNK_SIZE_Y as i32
        || z < 0
        || z >= CHUNK_SIZE_Z as i32
    {
        return true;
    }
    let neighbor = chunk.blocks[x as usize][y as usize][z as usize];
    if current == BlockType::Water {
        neighbor == BlockType::Air
    } else {
        matches!(neighbor, BlockType::Air | BlockType::Water)
    }
}

/// Thuật toán Vertex Ambient Occlusion (AO): tính mức độ bị che khuất của 1 góc đỉnh
#[inline(always)]
fn vertex_ao(side1: bool, side2: bool, corner: bool) -> f32 {
    if side1 && side2 {
        0.48 // Góc kẹt giữa 2 khối vuông: tối nhất
    } else {
        let count = (side1 as u8) + (side2 as u8) + (corner as u8);
        match count {
            3 => 0.48,
            2 => 0.65,
            1 => 0.82,
            _ => 1.0, // Thoáng hoàn toàn: sáng 100%
        }
    }
}

/// Helper nhân độ sáng mặt, biến thiên hạt (jitter) và bóng AO vào màu đỉnh
#[inline(always)]
fn apply_shading(base: [f32; 4], face_light: f32, jitter: f32, ao: f32) -> [f32; 4] {
    let factor = face_light * ao;
    [
        ((base[0] + jitter) * factor).clamp(0.0, 1.0),
        ((base[1] + jitter) * factor).clamp(0.0, 1.0),
        ((base[2] + jitter) * factor).clamp(0.0, 1.0),
        base[3],
    ]
}


/// Nhiễu giá trị 2D (value noise) mượt, theo toạ độ thế giới, trả về 0..1.
fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    fn h(ix: i32, iz: i32, seed: u32) -> f32 {
        let mut v = (ix as u32).wrapping_mul(374761393)
            ^ (iz as u32).wrapping_mul(668265263)
            ^ seed.wrapping_mul(2246822519);
        v = (v ^ (v >> 13)).wrapping_mul(1274126177);
        v ^= v >> 16;
        (v & 0xffff) as f32 / 65535.0
    }
    let (ix, iz) = (x.floor() as i32, z.floor() as i32);
    let (fx, fz) = (x - ix as f32, z - iz as f32);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let a = h(ix, iz, seed) + (h(ix + 1, iz, seed) - h(ix, iz, seed)) * sx;
    let b = h(ix, iz + 1, seed) + (h(ix + 1, iz + 1, seed) - h(ix, iz + 1, seed)) * sx;
    a + (b - a) * sz
}

/// Biến thiên màu theo vùng (tần số thấp): cỏ/lá chuyển giữa vàng-úa và xanh lam-đậm,
/// cát/đất/đá sáng tối theo mảng lớn. Liên tục qua biên chunk vì dùng toạ độ thế giới.
fn region_tint(block: BlockType, base: [f32; 4], wx: i32, wz: i32) -> [f32; 4] {
    let (x, z) = (wx as f32, wz as f32);
    let big = value_noise(x / 48.0, z / 48.0, 11) - 0.5;   // mảng lớn
    let mid = value_noise(x / 14.0, z / 14.0, 23) - 0.5;   // mảng vừa
    let n = big * 0.7 + mid * 0.3; // ~ -0.5..0.5
    let [r, g, b, a] = base;
    match block {
        BlockType::Grass | BlockType::Leaves | BlockType::LeavesLight | BlockType::Rice => {
            // n>0: ngả vàng/úa (thêm đỏ, bớt lam); n<0: xanh lục đậm lạnh
            [
                (r + n * 0.22).clamp(0.0, 1.0),
                (g + n * 0.04 - n.abs() * 0.04).clamp(0.0, 1.0),
                (b - n * 0.10).clamp(0.0, 1.0),
                a,
            ]
        }
        BlockType::PineLeaves => [r, (g + n * 0.06).clamp(0.0, 1.0), (b + n * 0.05).clamp(0.0, 1.0), a],
        BlockType::Sand | BlockType::Dirt | BlockType::Farmland => {
            let k = 1.0 + n * 0.30;
            [(r * k).clamp(0.0, 1.0), (g * k).clamp(0.0, 1.0), (b * k * (1.0 - n * 0.1)).clamp(0.0, 1.0), a]
        }
        BlockType::Stone | BlockType::CliffStone | BlockType::Cobble => {
            let k = 1.0 + n * 0.35;
            [(r * k).clamp(0.0, 1.0), (g * k).clamp(0.0, 1.0), (b * k * (1.0 + n * 0.1)).clamp(0.0, 1.0), a]
        }
        BlockType::Snow => {
            let k = 1.0 + n * 0.06;
            [(r * k).clamp(0.0, 1.0), (g * k).clamp(0.0, 1.0), b, a]
        }
        _ => base,
    }
}

pub fn generate_chunk_mesh(chunk: &ChunkData, cx: i32, cz: i32) -> Option<Mesh> {
    let mut builder = MeshBuilder::new();

    for x in 0..CHUNK_SIZE_X as i32 {
        for y in 0..CHUNK_SIZE_Y as i32 {
            for z in 0..CHUNK_SIZE_Z as i32 {
                let current_block = chunk.blocks[x as usize][y as usize][z as usize];
                // Nước dựng riêng (xem `generate_water_mesh`) để có vật liệu bóng, trong suốt
                if current_block == BlockType::Air || current_block == BlockType::Water {
                    continue;
                }

                // Toạ độ thế giới: hash/nhiễu theo đó để không lặp lại giữa các chunk
                let wx = cx * CHUNK_SIZE_X as i32 + x;
                let wz = cz * CHUNK_SIZE_Z as i32 + z;
                let base_color = region_tint(current_block, get_block_base_color(current_block), wx, wz);

                // Tạo hạt màu (color jitter) ngẫu nhiên theo tọa độ khối để chống cảm giác bề mặt nhựa phẳng
                let hash = ((wx as u32).wrapping_mul(73856093))
                    ^ ((y as u32).wrapping_mul(19349663))
                    ^ ((wz as u32).wrapping_mul(83492791));
                let hash = (hash ^ (hash >> 13)).wrapping_mul(0x5bd1e995);
                let jitter_amp = match current_block {
                    BlockType::Leaves | BlockType::LeavesLight | BlockType::PineLeaves => 0.20, // lá: nhiều sắc độ
                    BlockType::Wood | BlockType::BirchWood => 0.10,
                    BlockType::Wheat | BlockType::Rice => 0.14,
                    _ => 0.06,
                };
                let jitter = ((hash % 100) as f32 / 100.0 - 0.5) * jitter_amp;

                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;

                // =============================================================
                // 1. Mặt Trên (+Y / Up)
                // =============================================================
                if face_visible(chunk, current_block, x, y + 1, z) {
                    let s_left  = is_solid(chunk, x - 1, y + 1, z);
                    let s_right = is_solid(chunk, x + 1, y + 1, z);
                    let s_back  = is_solid(chunk, x, y + 1, z - 1);
                    let s_front = is_solid(chunk, x, y + 1, z + 1);

                    let c_bl = is_solid(chunk, x - 1, y + 1, z - 1);
                    let c_fl = is_solid(chunk, x - 1, y + 1, z + 1);
                    let c_fr = is_solid(chunk, x + 1, y + 1, z + 1);
                    let c_br = is_solid(chunk, x + 1, y + 1, z - 1);

                    let ao0 = vertex_ao(s_left, s_back, c_bl);
                    let ao1 = vertex_ao(s_left, s_front, c_fl);
                    let ao2 = vertex_ao(s_right, s_front, c_fr);
                    let ao3 = vertex_ao(s_right, s_back, c_br);

                    let light = 1.0;
                    builder.add_quad_with_ao(
                        [fx, fy + 1.0, fz],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz],
                        [0.0, 1.0, 0.0],
                        apply_shading(base_color, light, jitter, ao0),
                        apply_shading(base_color, light, jitter, ao1),
                        apply_shading(base_color, light, jitter, ao2),
                        apply_shading(base_color, light, jitter, ao3),
                    );
                }

                // =============================================================
                // 2. Mặt Dưới (-Y / Down)
                // =============================================================
                if face_visible(chunk, current_block, x, y - 1, z) {
                    let light = 0.55;
                    let c = apply_shading(base_color, light, jitter, 1.0);
                    builder.add_quad_with_ao(
                        [fx, fy, fz + 1.0],
                        [fx, fy, fz],
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy, fz + 1.0],
                        [0.0, -1.0, 0.0],
                        c, c, c, c,
                    );
                }

                // =============================================================
                // 3. Mặt Phải (+X / Right)
                // =============================================================
                if face_visible(chunk, current_block, x + 1, y, z) {
                    let s_up    = is_solid(chunk, x + 1, y + 1, z);
                    let s_down  = is_solid(chunk, x + 1, y - 1, z);
                    let s_back  = is_solid(chunk, x + 1, y, z - 1);
                    let s_front = is_solid(chunk, x + 1, y, z + 1);

                    let ao0 = vertex_ao(s_down, s_back, is_solid(chunk, x + 1, y - 1, z - 1));
                    let ao1 = vertex_ao(s_up, s_back, is_solid(chunk, x + 1, y + 1, z - 1));
                    let ao2 = vertex_ao(s_up, s_front, is_solid(chunk, x + 1, y + 1, z + 1));
                    let ao3 = vertex_ao(s_down, s_front, is_solid(chunk, x + 1, y - 1, z + 1));

                    let light = 0.75;
                    builder.add_quad_with_ao(
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                        [1.0, 0.0, 0.0],
                        apply_shading(base_color, light, jitter, ao0),
                        apply_shading(base_color, light, jitter, ao1),
                        apply_shading(base_color, light, jitter, ao2),
                        apply_shading(base_color, light, jitter, ao3),
                    );
                }

                // =============================================================
                // 4. Mặt Trái (-X / Left)
                // =============================================================
                if face_visible(chunk, current_block, x - 1, y, z) {
                    let s_up    = is_solid(chunk, x - 1, y + 1, z);
                    let s_down  = is_solid(chunk, x - 1, y - 1, z);
                    let s_back  = is_solid(chunk, x - 1, y, z - 1);
                    let s_front = is_solid(chunk, x - 1, y, z + 1);

                    let ao0 = vertex_ao(s_down, s_front, is_solid(chunk, x - 1, y - 1, z + 1));
                    let ao1 = vertex_ao(s_up, s_front, is_solid(chunk, x - 1, y + 1, z + 1));
                    let ao2 = vertex_ao(s_up, s_back, is_solid(chunk, x - 1, y + 1, z - 1));
                    let ao3 = vertex_ao(s_down, s_back, is_solid(chunk, x - 1, y - 1, z - 1));

                    let light = 0.65;
                    builder.add_quad_with_ao(
                        [fx, fy, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx, fy + 1.0, fz],
                        [fx, fy, fz],
                        [-1.0, 0.0, 0.0],
                        apply_shading(base_color, light, jitter, ao0),
                        apply_shading(base_color, light, jitter, ao1),
                        apply_shading(base_color, light, jitter, ao2),
                        apply_shading(base_color, light, jitter, ao3),
                    );
                }

                // =============================================================
                // 5. Mặt Trước (+Z / Front)
                // =============================================================
                if face_visible(chunk, current_block, x, y, z + 1) {
                    let s_up    = is_solid(chunk, x, y + 1, z + 1);
                    let s_down  = is_solid(chunk, x, y - 1, z + 1);
                    let s_left  = is_solid(chunk, x - 1, y, z + 1);
                    let s_right = is_solid(chunk, x + 1, y, z + 1);

                    let ao0 = vertex_ao(s_down, s_left, is_solid(chunk, x - 1, y - 1, z + 1));
                    let ao1 = vertex_ao(s_down, s_right, is_solid(chunk, x + 1, y - 1, z + 1));
                    let ao2 = vertex_ao(s_up, s_right, is_solid(chunk, x + 1, y + 1, z + 1));
                    let ao3 = vertex_ao(s_up, s_left, is_solid(chunk, x - 1, y + 1, z + 1));

                    let light = 0.85;
                    builder.add_quad_with_ao(
                        [fx, fy, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                        [0.0, 0.0, 1.0],
                        apply_shading(base_color, light, jitter, ao0),
                        apply_shading(base_color, light, jitter, ao1),
                        apply_shading(base_color, light, jitter, ao2),
                        apply_shading(base_color, light, jitter, ao3),
                    );
                }

                // =============================================================
                // 6. Mặt Sau (-Z / Back)
                // =============================================================
                if face_visible(chunk, current_block, x, y, z - 1) {
                    let s_up    = is_solid(chunk, x, y + 1, z - 1);
                    let s_down  = is_solid(chunk, x, y - 1, z - 1);
                    let s_left  = is_solid(chunk, x - 1, y, z - 1);
                    let s_right = is_solid(chunk, x + 1, y, z - 1);

                    let ao0 = vertex_ao(s_down, s_right, is_solid(chunk, x + 1, y - 1, z - 1));
                    let ao1 = vertex_ao(s_down, s_left, is_solid(chunk, x - 1, y - 1, z - 1));
                    let ao2 = vertex_ao(s_up, s_left, is_solid(chunk, x - 1, y + 1, z - 1));
                    let ao3 = vertex_ao(s_up, s_right, is_solid(chunk, x + 1, y + 1, z - 1));

                    let light = 0.80;
                    builder.add_quad_with_ao(
                        [fx + 1.0, fy, fz],
                        [fx, fy, fz],
                        [fx, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [0.0, 0.0, -1.0],
                        apply_shading(base_color, light, jitter, ao0),
                        apply_shading(base_color, light, jitter, ao1),
                        apply_shading(base_color, light, jitter, ao2),
                        apply_shading(base_color, light, jitter, ao3),
                    );
                }
            }
        }
    }

    builder.build()
}

/// Mặt nước của một chunk: chỉ các mặt trên tiếp giáp không khí, nằm thấp hơn đỉnh khối một chút.
/// UV theo toạ độ thế giới để normal map gợn sóng liền mạch giữa các chunk.
/// Màu đỉnh mã hoá độ sâu: nước nông trong và xanh ngọc, càng sâu càng đục và xanh đậm,
/// nên nhìn xuyên được xuống đáy ở chỗ nông và không nhìn thấy gì ở chỗ sâu.
pub fn generate_water_mesh(chunk: &ChunkData, cx: i32, cz: i32) -> Option<Mesh> {
    const N: usize = CHUNK_SIZE_X;
    const UV_SCALE: f32 = 1.0 / 6.0;
    let (ox, oz) = ((cx * CHUNK_SIZE_X as i32) as f32, (cz * CHUNK_SIZE_Z as i32) as f32);

    // Mỗi cột: độ cao mặt nước và độ sâu (số khối nước liên tiếp bên dưới)
    let mut top: Vec<(usize, usize, usize, f32)> = Vec::new();
    let mut depth = [[0.0f32; N]; N];
    for x in 0..N {
        for z in 0..N {
            for y in 0..CHUNK_SIZE_Y {
                if chunk.blocks[x][y][z] != BlockType::Water {
                    continue;
                }
                if y + 1 < CHUNK_SIZE_Y && chunk.blocks[x][y + 1][z] != BlockType::Air {
                    continue;
                }
                let mut d = 0usize;
                while d <= y && chunk.blocks[x][y - d][z] == BlockType::Water {
                    d += 1;
                }
                depth[x][z] = d as f32;
                top.push((x, y, z, d as f32));
            }
        }
    }
    if top.is_empty() {
        return None;
    }

    // Độ sâu tại đỉnh = trung bình 4 ô quanh đỉnh (ô không phải nước tính 0) nên bờ nước trong dần
    let vertex_depth = |vx: usize, vz: usize| -> f32 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for (dx, dz) in [(0usize, 0usize), (1, 0), (0, 1), (1, 1)] {
            if vx + dx >= 1 && vz + dz >= 1 && vx + dx - 1 < N && vz + dz - 1 < N {
                sum += depth[vx + dx - 1][vz + dz - 1];
                n += 1.0;
            }
        }
        if n > 0.0 { sum / n } else { 0.0 }
    };
    let color_for = |d: f32| -> [f32; 4] {
        let t = (d / 9.0).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        let shallow = [0.42, 0.80, 0.82];
        let deep = [0.015, 0.10, 0.26];
        let a = 0.16 + 0.80 * t;
        [
            shallow[0] + (deep[0] - shallow[0]) * t,
            shallow[1] + (deep[1] - shallow[1]) * t,
            shallow[2] + (deep[2] - shallow[2]) * t,
            a,
        ]
    };

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (x, y, z, _) in top {
        let fy = y as f32 + 0.9;
        let base = positions.len() as u32;
        for (dx, dz) in [(0usize, 0usize), (0, 1), (1, 1), (1, 0)] {
            let (px, pz) = ((x + dx) as f32, (z + dz) as f32);
            positions.push([px, fy, pz]);
            uvs.push([(ox + px) * UV_SCALE, (oz + pz) * UV_SCALE]);
            colors.push(color_for(vertex_depth(x + dx, z + dz)));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let n = positions.len();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1.0, 0.0, 0.0, 1.0]; n]);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}
