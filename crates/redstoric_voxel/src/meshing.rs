use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::Mesh;

use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

pub fn build_greedy_mesh(chunk: &ChunkData) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let dims = [
        CHUNK_SIZE_X as i32,
        CHUNK_SIZE_Y as i32,
        CHUNK_SIZE_Z as i32,
    ];

    for d in 0..3 {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        let mut x = [0i32; 3];
        let mut q = [0i32; 3];
        q[d] = 1;

        let mut mask = vec![None; (dims[u] * dims[v]) as usize];

        x[d] = -1;
        while x[d] < dims[d] {
            let mut n = 0;
            x[v] = 0;
            while x[v] < dims[v] {
                x[u] = 0;
                while x[u] < dims[u] {
                    let b1 = if x[d] >= 0 {
                        chunk.get_block(x[0] as usize, x[1] as usize, x[2] as usize)
                    } else {
                        BlockType::Air
                    };

                    let b2 = if x[d] < dims[d] - 1 {
                        chunk.get_block(
                            (x[0] + q[0]) as usize,
                            (x[1] + q[1]) as usize,
                            (x[2] + q[2]) as usize,
                        )
                    } else {
                        BlockType::Air
                    };

                    mask[n] = match (!b1.is_air(), !b2.is_air()) {
                        (true, false) => Some((b1, 1)),
                        (false, true) => Some((b2, -1)),
                        _ => None,
                    };

                    n += 1;
                    x[u] += 1;
                }
                x[v] += 1;
            }

            x[d] += 1;

            n = 0;
            for j in 0..dims[v] {
                let mut i = 0;
                while i < dims[u] {
                    if let Some((block_type, normal_dir)) = mask[n] {
                        let mut w = 1;
                        while i + w < dims[u] && mask[n + w as usize] == Some((block_type, normal_dir)) {
                            w += 1;
                        }

                        let mut h = 1;
                        'outer: while j + h < dims[v] {
                            for k in 0..w {
                                if mask[n + k as usize + (h * dims[u]) as usize] != Some((block_type, normal_dir)) {
                                    break 'outer;
                                }
                            }
                            h += 1;
                        }

                        let mut du = [0f32; 3];
                        let mut dv = [0f32; 3];
                        du[u] = w as f32;
                        dv[v] = h as f32;

                        let mut base_pos = [0f32; 3];
                        base_pos[d] = x[d] as f32;
                        base_pos[u] = i as f32;
                        base_pos[v] = j as f32;

                        let color = block_type.face_color(d, normal_dir);
                        let mut norm = [0f32; 3];
                        norm[d] = normal_dir as f32;

                        let base_idx = positions.len() as u32;

                        let p0 = base_pos;
                        let p1 = [base_pos[0] + du[0], base_pos[1] + du[1], base_pos[2] + du[2]];
                        let p2 = [base_pos[0] + du[0] + dv[0], base_pos[1] + du[1] + dv[1], base_pos[2] + du[2] + dv[2]];
                        let p3 = [base_pos[0] + dv[0], base_pos[1] + dv[1], base_pos[2] + dv[2]];

                        positions.extend_from_slice(&[p0, p1, p2, p3]);
                        normals.extend_from_slice(&[norm; 4]);
                        colors.extend_from_slice(&[color; 4]);

                        if normal_dir > 0 {
                            indices.extend_from_slice(&[
                                base_idx, base_idx + 1, base_idx + 2,
                                base_idx, base_idx + 2, base_idx + 3,
                            ]);
                        } else {
                            indices.extend_from_slice(&[
                                base_idx, base_idx + 2, base_idx + 1,
                                base_idx, base_idx + 3, base_idx + 2,
                            ]);
                        }

                        for l in 0..h {
                            for k in 0..w {
                                mask[n + k as usize + (l * dims[u]) as usize] = None;
                            }
                        }

                        i += w;
                        n += w as usize;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}