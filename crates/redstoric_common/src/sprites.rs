//! Thư viện sprite (cây cỏ, hoa, nấm, đá, đồ nội thất, trạm chế tạo, mộ...) trong
//! `assets/placeholders/glb/sprite` và hệ thống rải sprite theo vùng sinh thái.
//!
//! * Danh mục được quét từ đĩa một lần (lười, thread-safe): mỗi thư mục con là một "nhóm"
//!   (`grass`, `flowers`, `furniture`...). Thêm file `.glb` mới là tự có trong danh mục.
//! * Mỗi sprite có một độ cao nâng riêng (đọc từ bounding box trong file glb) để đáy model
//!   chạm đúng mặt đất, bất kể sprite cao hay thấp.
//! * `scatter_sprites` rải cỏ / hoa / nấm / xương rồng... theo biome (đồng cỏ, savanna, sa mạc,
//!   tuyết, bãi biển), giống lớp `scatter` của Veloren.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bevy_math::{Quat, Vec3};

use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use crate::generation::WorldGenerator;
use crate::structures::{hash2, Site, BLEND};
use crate::{PropSpawnData, WorldPropKind};

const SPRITE_DIR: &str = "placeholders/glb/sprite";

pub struct SpriteDef {
    pub name: String,
    pub path: &'static str,
    pub scale: f32,
    disk: PathBuf,
    y_offset: OnceLock<f32>,
}

impl SpriteDef {
    /// Khoảng nâng so với mặt đất để đáy model chạm đất (đơn vị thế giới).
    pub fn y_offset(&self) -> f32 {
        *self.y_offset.get_or_init(|| -read_min_y(&self.disk) * self.scale)
    }
}

struct Catalog {
    defs: Vec<SpriteDef>,
    by_cat: HashMap<String, Vec<u16>>,
    by_name: HashMap<String, u16>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(build_catalog)
}

pub(crate) fn def(id: u16) -> &'static SpriteDef {
    &catalog().defs[id as usize]
}

fn category_scale(top: &str) -> f32 {
    match top {
        "grass" | "flowers" | "ferns" | "mushrooms" | "savanna_bush" | "snow_bush" | "leafy_plant" => 0.08,
        "cacti" | "rocks" | "mineral" => 0.09,
        _ => 0.07,
    }
}

fn find_sprite_dir() -> Option<PathBuf> {
    [
        PathBuf::from("assets").join(SPRITE_DIR),
        PathBuf::from("../../assets").join(SPRITE_DIR),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(SPRITE_DIR),
    ]
    .into_iter()
    .find(|p| p.is_dir())
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let child_rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if path.is_dir() {
            walk(&path, &child_rel, out);
        } else if name.ends_with(".glb") {
            out.push((child_rel, path));
        }
    }
}

/// Danh mục sprite dựng sẵn (do `cargo run --bin dump_manifests` sinh ra) dùng khi không có thư mục
/// asset trên đĩa, ví dụ trong APK Android: mỗi phần tử là `[đường dẫn, độ cao đáy model]`.
const EMBEDDED_MANIFEST: &str = include_str!("../../../assets_manifest/sprite_catalog.json");

/// Xuất danh mục sprite hiện có trên đĩa thành JSON (xem `EMBEDDED_MANIFEST`).
pub fn dump_manifest_json() -> String {
    let Some(base) = find_sprite_dir() else { return "[]".to_string() };
    let mut files = Vec::new();
    walk(&base, "", &mut files);
    let list: Vec<serde_json::Value> =
        files.iter().map(|(rel, disk)| serde_json::json!([rel, read_min_y(disk)])).collect();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
}

fn build_catalog() -> Catalog {
    let mut catalog = Catalog { defs: Vec::new(), by_cat: HashMap::new(), by_name: HashMap::new() };
    // (đường dẫn tương đối, file trên đĩa, độ cao đáy đã biết)
    let mut files: Vec<(String, PathBuf, Option<f32>)> = Vec::new();
    if let Some(base) = find_sprite_dir() {
        let mut found = Vec::new();
        walk(&base, "", &mut found);
        files.extend(found.into_iter().map(|(rel, disk)| (rel, disk, None)));
    } else if let Ok(list) = serde_json::from_str::<Vec<(String, f32)>>(EMBEDDED_MANIFEST) {
        files.extend(list.into_iter().map(|(rel, min_y)| (rel, PathBuf::new(), Some(min_y))));
    }
    if files.is_empty() {
        eprintln!("Không tìm thấy thư mục {SPRITE_DIR} hay danh mục dựng sẵn; sẽ không có sprite");
        return catalog;
    }
    for (rel, disk, known_min_y) in files {
        if catalog.defs.len() >= u16::MAX as usize {
            break;
        }
        let stem = rel.trim_end_matches(".glb").to_string();
        let cat = match rel.rfind('/') {
            Some(i) => rel[..i].to_string(),
            None => String::new(),
        };
        let top = cat.split('/').next().unwrap_or("").to_string();
        let path: &'static str = Box::leak(format!("{SPRITE_DIR}/{rel}").into_boxed_str());
        let id = catalog.defs.len() as u16;
        let scale = category_scale(&top);
        let y_offset = OnceLock::new();
        if let Some(min_y) = known_min_y {
            let _ = y_offset.set(-min_y * scale);
        }
        catalog.defs.push(SpriteDef { name: stem.clone(), path, scale, disk, y_offset });
        catalog.by_cat.entry(cat).or_default().push(id);
        catalog.by_name.insert(stem, id);
    }
    catalog
}

/// Độ cao đáy (trục Y) của model, đọc từ accessor POSITION trong phần JSON của file glb.
fn read_min_y(path: &Path) -> f32 {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else { return 0.0 };
    let mut head = [0u8; 20];
    if file.read_exact(&mut head).is_err() {
        return 0.0;
    }
    let len = u32::from_le_bytes([head[12], head[13], head[14], head[15]]) as usize;
    let mut buf = vec![0u8; len];
    if file.read_exact(&mut buf).is_err() {
        return 0.0;
    }
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&buf) else { return 0.0 };
    let mut min_y = f64::MAX;
    if let Some(meshes) = json["meshes"].as_array() {
        for mesh in meshes {
            if let Some(prims) = mesh["primitives"].as_array() {
                for prim in prims {
                    if let Some(idx) = prim["attributes"]["POSITION"].as_u64() {
                        if let Some(v) = json["accessors"][idx as usize]["min"][1].as_f64() {
                            min_y = min_y.min(v);
                        }
                    }
                }
            }
        }
    }
    if min_y == f64::MAX { 0.0 } else { min_y as f32 }
}

// ---------------------------------------------------------------------------------------------
// API tra cứu
// ---------------------------------------------------------------------------------------------

/// Sprite theo tên, vd. `"crafting_station/anvil"` (không có đuôi `.glb`).
pub fn sprite(name: &str) -> Option<WorldPropKind> {
    catalog().by_name.get(name).map(|id| WorldPropKind::Sprite(*id))
}

/// Chọn một sprite trong nhóm theo hash.
pub fn pick(category: &str, h: u32) -> Option<WorldPropKind> {
    let ids = catalog().by_cat.get(category)?;
    if ids.is_empty() {
        return None;
    }
    Some(WorldPropKind::Sprite(ids[h as usize % ids.len()]))
}

/// Như `pick` nhưng chỉ xét sprite có tên chứa `needle`.
pub fn pick_where(category: &str, needle: &str, h: u32) -> Option<WorldPropKind> {
    let cat = catalog();
    let ids: Vec<u16> = cat
        .by_cat
        .get(category)?
        .iter()
        .copied()
        .filter(|id| cat.defs[*id as usize].name.contains(needle))
        .collect();
    if ids.is_empty() {
        return None;
    }
    Some(WorldPropKind::Sprite(ids[h as usize % ids.len()]))
}

// ---------------------------------------------------------------------------------------------
// Rải sprite theo biome
// ---------------------------------------------------------------------------------------------

/// (nhóm sprite, xác suất trên 10 000 cột đất)
type Table = &'static [(&'static str, u32)];

const GRASSLAND: Table = &[
    ("grass", 150), ("flowers", 40), ("ferns", 15), ("mushrooms", 6),
    ("rocks", 8), ("blueberry", 5), ("twigs", 6),
];
const FOREST_DAMP: Table = &[
    ("grass", 120), ("flowers", 30), ("ferns", 50), ("mushrooms", 25),
    ("rocks", 8), ("blueberry", 6), ("twigs", 8), ("leafy_plant", 12),
];
const SAVANNA: Table = &[
    ("savanna_bush", 35), ("grass", 110), ("dead_bush", 6), ("rocks", 8), ("bone", 2),
];
const DESERT: Table = &[
    ("cacti", 16), ("dead_bush", 18), ("dead_plant", 12), ("rocks", 8), ("bone", 3),
];
const SNOW: Table = &[
    ("snow_bush", 28), ("snow_pebbles", 36), ("rocks", 5), ("crystal", 6), ("twigs", 4),
];
const BEACH: Table = &[("seashells", 25), ("reed", 10)];

impl WorldGenerator {
    pub(crate) fn scatter_sprites(
        &self,
        chunk: &ChunkData,
        ox: i32,
        oz: i32,
        surface_map: &[[usize; CHUNK_SIZE_Z]; CHUNK_SIZE_X],
        sites: &[Site],
        props: &mut Vec<PropSpawnData>,
    ) {
        const MAX_TOTAL: u32 = 260; // tổng xác suất lớn nhất của mọi bảng, để loại nhanh
        let sea = self.sea_level;
        for x in 0..CHUNK_SIZE_X {
            for z in 0..CHUNK_SIZE_Z {
                let (wx, wz) = (ox + x as i32, oz + z as i32);
                let h = hash2(wx, wz, 5151);
                let roll = h % 10_000;
                if roll >= MAX_TOTAL {
                    continue;
                }
                let sy = surface_map[x][z];
                if sy + 4 >= CHUNK_SIZE_Y || sites.iter().any(|s| s.contains(wx, wz, BLEND)) {
                    continue;
                }
                let (temp, humid) = self.get_climate(wx as f64, wz as f64);
                let table: Table = match chunk.get_block(x, sy, z) {
                    BlockType::Grass => {
                        if temp > 0.58 && humid < 0.5 {
                            SAVANNA
                        } else if humid > 0.5 {
                            FOREST_DAMP
                        } else {
                            GRASSLAND
                        }
                    }
                    BlockType::Sand => {
                        if sy > sea && sy <= sea + 2 {
                            BEACH
                        } else if temp > 0.68 && humid < 0.35 {
                            DESERT
                        } else {
                            continue;
                        }
                    }
                    BlockType::Snow => SNOW,
                    _ => continue,
                };

                let mut acc = 0;
                let mut chosen = None;
                for (cat, weight) in table {
                    acc += weight;
                    if roll < acc {
                        chosen = Some(*cat);
                        break;
                    }
                }
                let Some(cat) = chosen else { continue };
                let Some(kind) = pick(cat, h >> 12) else { continue };

                let jx = ((h >> 4) % 100) as f32 / 100.0 * 0.6 - 0.3;
                let jz = ((h >> 9) % 100) as f32 / 100.0 * 0.6 - 0.3;
                props.push(PropSpawnData {
                    kind,
                    world_pos: Vec3::new(wx as f32 + 0.5 + jx, sy as f32 + 1.0, wz as f32 + 0.5 + jz),
                    rotation: Quat::from_rotation_y(((h >> 16) % 360) as f32 * std::f32::consts::PI / 180.0),
                });
            }
        }
    }
}
