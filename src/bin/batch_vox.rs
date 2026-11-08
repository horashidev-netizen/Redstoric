use dot_vox::{Color, DotVoxData, Model};
use serde_json::json;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Vertex tối ưu 16 bytes:
/// - pos: [f32; 3] = 12 bytes
/// - color: [u8; 4] = 4 bytes (R, G, B, A)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AlignedVoxelVertex {
    pub pos: [f32; 3],
    pub color: [u8; 4],
}

pub struct ProcessedMesh {
    pub vertices: Vec<AlignedVoxelVertex>,
    pub indices: Vec<u32>,
    pub min_pos: [f32; 3],
    pub max_pos: [f32; 3],
}

fn main() {
    println!("=== REDSTORIC VOX TO GLB CONVERTER (16-BYTE GPU ALIGNED) ===");

    let input_dir = PathBuf::from("assets/placeholders/voxygen/voxel");
    let output_dir = PathBuf::from("assets/placeholders/glb");

    if !input_dir.exists() {
        eprintln!("Không tìm thấy thư mục: {}", input_dir.display());
        return;
    }

    let mut count = 0;
    for entry in WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "vox") {
            let rel_path = match path.strip_prefix(&input_dir) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let mut out_path = output_dir.join(rel_path);
            out_path.set_extension("glb");

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            match convert_single_vox(path, &out_path) {
                Ok(_) => {
                    println!(" [OK] {}", rel_path.display());
                    count += 1;
                }
                Err(err) => {
                    eprintln!(" [LỖI] {}: {}", rel_path.display(), err);
                }
            }
        }
    }

    println!("\nHoàn tất! Đã convert chuẩn xác {} file .glb.", count);
}

fn convert_single_vox(vox_path: &Path, glb_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data: DotVoxData = dot_vox::load(vox_path.to_str().ok_or("Đường dẫn không hợp lệ")?)?;

    if data.models.is_empty() {
        return Err("Model rỗng".into());
    }

    let model = data.models.first().ok_or("Model rỗng")?;
    let palette = if data.palette.is_empty() {
        &dot_vox::DEFAULT_PALETTE[..]
    } else {
        &data.palette[..]
    };

    let mesh = build_aligned_mesh(model, palette)?;
    if mesh.indices.is_empty() {
        return Err("Không có mặt voxel lộ ra".into());
    }

    export_glb(&mesh, glb_path)?;
    Ok(())
}

fn build_aligned_mesh(model: &Model, palette: &[Color]) -> Result<ProcessedMesh, Box<dyn std::error::Error>> {
    let sx = model.size.x as usize;
    let sy = model.size.y as usize;
    let sz = model.size.z as usize;

    let mut grid = vec![None; sx * sy * sz];
    for v in &model.voxels {
        let idx = (v.x as usize) + (v.y as usize) * sx + (v.z as usize) * sx * sy;
        grid[idx] = Some(v.i);
    }

    let is_solid = |vx: i32, vy: i32, vz: i32| -> bool {
        if vx < 0 || vx >= sx as i32 || vy < 0 || vy >= sy as i32 || vz < 0 || vz >= sz as i32 {
            false
        } else {
            let idx = (vx as usize) + (vy as usize) * sx + (vz as usize) * sx * sy;
            grid[idx].is_some()
        }
    };

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let mut min_pos = [f32::MAX; 3];
    let mut max_pos = [f32::MIN; 3];

    let cx = model.size.x as f32 / 2.0;
    let cy = model.size.y as f32 / 2.0;
    let cz = model.size.z as f32 / 2.0;

    for v in &model.voxels {
        let vx = v.x as i32;
        let vy = v.y as i32;
        let vz = v.z as i32;

        let col = &palette[v.i as usize];
        let color_rgba = [col.r, col.g, col.b, 255];

        let x0 = vx as f32 - cx;
        let x1 = x0 + 1.0;

        let y0 = vz as f32 - cz;
        let y1 = y0 + 1.0;

        let z1 = -(vy as f32 - cy);
        let z0 = z1 - 1.0;

        let faces: [(bool, [[f32; 3]; 4]); 6] = [
            // 1. Mặt Trên (+Y)
            (
                !is_solid(vx, vy, vz + 1),
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
            ),
            // 2. Mặt Dưới (-Y)
            (
                !is_solid(vx, vy, vz - 1),
                [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            ),
            // 3. Mặt Phải (+X)
            (
                !is_solid(vx + 1, vy, vz),
                [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
            ),
            // 4. Mặt Trái (-X)
            (
                !is_solid(vx - 1, vy, vz),
                [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
            ),
            // 5. Mặt Trước (-Z)
            (
                !is_solid(vx, vy + 1, vz),
                [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
            ),
            // 6. Mặt Sau (+Z)
            (
                !is_solid(vx, vy - 1, vz),
                [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            ),
        ];

        for (exposed, corners) in faces {
            if exposed {
                let base_idx = vertices.len() as u32;
                for pos in corners {
                    vertices.push(AlignedVoxelVertex {
                        pos,
                        color: color_rgba,
                    });

                    min_pos[0] = min_pos[0].min(pos[0]);
                    min_pos[1] = min_pos[1].min(pos[1]);
                    min_pos[2] = min_pos[2].min(pos[2]);

                    max_pos[0] = max_pos[0].max(pos[0]);
                    max_pos[1] = max_pos[1].max(pos[1]);
                    max_pos[2] = max_pos[2].max(pos[2]);
                }
                indices.extend_from_slice(&[
                    base_idx,
                    base_idx + 1,
                    base_idx + 2,
                    base_idx,
                    base_idx + 2,
                    base_idx + 3,
                ]);
            }
        }
    }

    Ok(ProcessedMesh {
        vertices,
        indices,
        min_pos,
        max_pos,
    })
}

fn export_glb(mesh: &ProcessedMesh, out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut bin_buffer = Vec::new();

    // Buffer 0: Các đỉnh xen kẽ (16 bytes/đỉnh)
    // 0..12 bytes: POSITION (3x f32)
    // 12..16 bytes: COLOR_0 (4x u8)
    let v_offset = bin_buffer.len();
    for v in &mesh.vertices {
        for coord in v.pos {
            bin_buffer.extend_from_slice(&coord.to_le_bytes());
        }
        bin_buffer.extend_from_slice(&v.color);
    }
    let v_len = bin_buffer.len() - v_offset;

    // Buffer 1: Indices
    let idx_offset = bin_buffer.len();
    for &i in &mesh.indices {
        bin_buffer.extend_from_slice(&i.to_le_bytes());
    }
    let idx_len = bin_buffer.len() - idx_offset;

    while bin_buffer.len() % 4 != 0 {
        bin_buffer.push(0);
    }

    let gltf_json = json!({
        "asset": { "version": "2.0", "generator": "redstoric_batch_vox" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "VoxelModel" }],
        "materials": [{
            "name": "VoxelMaterial",
            "pbrMetallicRoughness": {
                "roughnessFactor": 0.85,
                "metallicFactor": 0.05
            },
            "doubleSided": true
        }],
        "meshes": [{
            "primitives": [{
                "attributes": {
                    "POSITION": 0,
                    "COLOR_0": 1
                },
                "indices": 2,
                "material": 0,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126, // FLOAT (Chuẩn bắt buộc của Bevy POSITION)
                "count": mesh.vertices.len(),
                "type": "VEC3",
                "min": mesh.min_pos,
                "max": mesh.max_pos
            },
            {
                "bufferView": 0,
                "byteOffset": 12,
                "componentType": 5121, // UNSIGNED_BYTE (Nén màu RGBA)
                "count": mesh.vertices.len(),
                "type": "VEC4",
                "normalized": true
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5125, // UNSIGNED_INT
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": v_offset,
                "byteLength": v_len,
                "byteStride": 16,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": idx_offset,
                "byteLength": idx_len,
                "target": 34963
            }
        ],
        "buffers": [{ "byteLength": bin_buffer.len() }]
    });

    let mut json_bytes = serde_json::to_vec(&gltf_json)?;
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_buffer.len();

    let file = File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"glTF")?;
    writer.write_all(&2u32.to_le_bytes())?;
    writer.write_all(&(total_length as u32).to_le_bytes())?;

    writer.write_all(&(json_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(b"JSON")?;
    writer.write_all(&json_bytes)?;

    writer.write_all(&(bin_buffer.len() as u32).to_le_bytes())?;
    writer.write_all(b"BIN\0")?;
    writer.write_all(&bin_buffer)?;

    writer.flush()?;
    Ok(())
}