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

#[inline(always)]
fn is_transparent(chunk: &ChunkData, x: i32, y: i32, z: i32) -> bool {
    !is_solid(chunk, x, y, z)
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

pub fn generate_chunk_mesh(chunk: &ChunkData) -> Option<Mesh> {
    let mut builder = MeshBuilder::new();

    for x in 0..CHUNK_SIZE_X as i32 {
        for y in 0..CHUNK_SIZE_Y as i32 {
            for z in 0..CHUNK_SIZE_Z as i32 {
                let current_block = chunk.blocks[x as usize][y as usize][z as usize];
                if current_block == BlockType::Air {
                    continue;
                }

                let base_color = get_block_base_color(current_block);

                // Tạo hạt màu (color jitter) ngẫu nhiên theo tọa độ khối để chống cảm giác bề mặt nhựa phẳng
                let hash = ((x as u32).wrapping_mul(73856093))
                    ^ ((y as u32).wrapping_mul(19349663))
                    ^ ((z as u32).wrapping_mul(83492791));
                let jitter = ((hash % 100) as f32 / 100.0 - 0.5) * 0.06; // Biến thiên +-3%

                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;

                // =============================================================
                // 1. Mặt Trên (+Y / Up)
                // =============================================================
                if is_transparent(chunk, x, y + 1, z) {
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
                if is_transparent(chunk, x, y - 1, z) {
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
                if is_transparent(chunk, x + 1, y, z) {
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
                if is_transparent(chunk, x - 1, y, z) {
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
                if is_transparent(chunk, x, y, z + 1) {
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
                if is_transparent(chunk, x, y, z - 1) {
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