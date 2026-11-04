use dot_vox::{Color, DotVoxData, Model};
use serde_json::json;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    println!("=== REDSTORIC VOX TO GLB BATCH CONVERTER (CHUẨN HỆ TRỤC BEVY/glTF) ===");

    // 1. Đường dẫn thư mục voxel gốc của Veloren
    let input_dir = PathBuf::from("/Users/tamnguyen/Downloads/veloren-master/assets/voxygen/voxel");
    // 2. Đường dẫn xuất file glb của Redstoric
    let output_dir = PathBuf::from("/Users/tamnguyen/RustroverProjects/Redstoric/assets/placeholders/glb");

    if !input_dir.exists() {
        eprintln!("Không tìm thấy thư mục: {}", input_dir.display());
        return;
    }

    println!("Nguồn: {}", input_dir.display());
    println!("Đích:  {}\n", output_dir.display());

    let mut count = 0;
    for entry in WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "vox") {
            let rel_path = path.strip_prefix(&input_dir).unwrap();
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
    let data: DotVoxData = dot_vox::load(vox_path.to_str().unwrap())?;

    if data.models.is_empty() {
        return Err("Model rỗng".into());
    }

    let model = &data.models[0];
    let palette = if data.palette.is_empty() {
        &dot_vox::DEFAULT_PALETTE[..]
    } else {
        &data.palette[..]
    };

    let mesh = build_mesh_aligned(model, palette);
    if mesh.indices.is_empty() {
        return Err("Không có mặt voxel lộ ra".into());
    }

    export_glb(&mesh, glb_path)?;
    Ok(())
}

struct RawVoxelMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    min: [f32; 3],
    max: [f32; 3],
}

/// Dựng lưới 3D với hệ trục glTF chuẩn:
/// X_gltf = X_vox (Ngang)
/// Y_gltf = Z_vox (Cao)
/// Z_gltf = -Y_vox (Trước/Sau - Đảm bảo mặt trước nhìn đúng -Z của Bevy)
fn build_mesh_aligned(model: &Model, palette: &[Color]) -> RawVoxelMesh {
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

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    // Căn tâm: X và Y đưa về giữa, Z đưa về đáy (để chân chạm đất chuẩn)
    let cx = model.size.x as f32 / 2.0;
    let cy = model.size.y as f32 / 2.0;
    let cz = model.size.z as f32 / 2.0;

    for v in &model.voxels {
        let vx = v.x as i32;
        let vy = v.y as i32;
        let vz = v.z as i32;

        let col = &palette[v.i as usize];
        let color_rgba = [
            (col.r as f32 / 255.0).powf(2.2), // Chuẩn sRGB sang Linear cho Bevy PBR
            (col.g as f32 / 255.0).powf(2.2),
            (col.b as f32 / 255.0).powf(2.2),
            1.0,
        ];

        // Tọa độ góc tối thiểu của khối voxel trong glTF
        // vx -> X
        // vz -> Y (Chiều cao)
        // vy -> Z (vy tiến về phía trước trong MagicaVoxel tương ứng với -Z tiến về trước trong glTF)
        let x0 = vx as f32 - cx;
        let x1 = x0 + 1.0;

        let y0 = vz as f32 - cz;
        let y1 = y0 + 1.0;

        let z1 = -(vy as f32 - cy);       // Mặt sau
        let z0 = z1 - 1.0;                 // Mặt trước (-Z)

        // 6 mặt với thứ tự đỉnh Counter-Clockwise (CCW) chuẩn hướng mặt ra ngoài
        let faces: [(bool, [f32; 3], [[f32; 3]; 4]); 6] = [
            // 1. Mặt Trên (+Y glTF = +Z vox)
            (
                !is_solid(vx, vy, vz + 1),
                [0.0, 1.0, 0.0],
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
            ),
            // 2. Mặt Dưới (-Y glTF = -Z vox)
            (
                !is_solid(vx, vy, vz - 1),
                [0.0, -1.0, 0.0],
                [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
            ),
            // 3. Mặt Phải (+X glTF = +X vox)
            (
                !is_solid(vx + 1, vy, vz),
                [1.0, 0.0, 0.0],
                [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
            ),
            // 4. Mặt Trái (-X glTF = -X vox)
            (
                !is_solid(vx - 1, vy, vz),
                [-1.0, 0.0, 0.0],
                [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
            ),
            // 5. Mặt Trước (-Z glTF = +Y vox) -> Mặt sói nhìn về hướng này!
            (
                !is_solid(vx, vy + 1, vz),
                [0.0, 0.0, -1.0],
                [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
            ),
            // 6. Mặt Sau (+Z glTF = -Y vox) -> Sau gáy
            (
                !is_solid(vx, vy - 1, vz),
                [0.0, 0.0, 1.0],
                [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            ),
        ];

        for (exposed, normal, corners) in faces {
            if exposed {
                let base = positions.len() as u32;
                for p in corners {
                    positions.push(p);
                    normals.push(normal);
                    colors.push(color_rgba);

                    min[0] = min[0].min(p[0]);
                    min[1] = min[1].min(p[1]);
                    min[2] = min[2].min(p[2]);

                    max[0] = max[0].max(p[0]);
                    max[1] = max[1].max(p[1]);
                    max[2] = max[2].max(p[2]);
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
    }

    RawVoxelMesh {
        positions,
        normals,
        colors,
        indices,
        min,
        max,
    }
}

fn export_glb(mesh: &RawVoxelMesh, out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut bin_buffer = Vec::new();

    let pos_offset = bin_buffer.len();
    for p in &mesh.positions {
        for &coord in p {
            bin_buffer.extend_from_slice(&coord.to_le_bytes());
        }
    }
    let pos_len = bin_buffer.len() - pos_offset;

    let norm_offset = bin_buffer.len();
    for n in &mesh.normals {
        for &coord in n {
            bin_buffer.extend_from_slice(&coord.to_le_bytes());
        }
    }
    let norm_len = bin_buffer.len() - norm_offset;

    let col_offset = bin_buffer.len();
    for c in &mesh.colors {
        for &channel in c {
            bin_buffer.extend_from_slice(&channel.to_le_bytes());
        }
    }
    let col_len = bin_buffer.len() - col_offset;

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
            "name": "VoxelPbrMaterial",
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
                    "NORMAL": 1,
                    "COLOR_0": 2
                },
                "indices": 3,
                "material": 0,
                "mode": 4
            }]
        }],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": mesh.positions.len(),
                "type": "VEC3",
                "min": mesh.min,
                "max": mesh.max
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": mesh.normals.len(),
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5126,
                "count": mesh.colors.len(),
                "type": "VEC4"
            },
            {
                "bufferView": 3,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": pos_offset, "byteLength": pos_len, "target": 34962 },
            { "buffer": 0, "byteOffset": norm_offset, "byteLength": norm_len, "target": 34962 },
            { "buffer": 0, "byteOffset": col_offset, "byteLength": col_len, "target": 34962 },
            { "buffer": 0, "byteOffset": idx_offset, "byteLength": idx_len, "target": 34963 }
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