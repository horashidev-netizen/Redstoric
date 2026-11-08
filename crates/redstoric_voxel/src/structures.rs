//! Cấu trúc sinh tự động theo vùng: **làng nhà ở** và **đấu trường boss**.
//!
//! Thế giới chia thành các vùng `REGION x REGION` khối. Mỗi vùng có thể có tối đa một khu
//! (quyết định bằng hash cố định theo toạ độ vùng, nên luôn giống nhau với cùng một seed).
//! Khu được đặt ở nơi đất khô, tương đối bằng phẳng; địa hình quanh khu được san phẳng
//! để nhà và đấu trường đứng vững. Mọi thứ đều là hàm thuần của toạ độ thế giới nên các chunk
//! kề nhau tự khớp mà không cần dữ liệu chung.

use bevy::math::{Quat, Vec3};

use crate::block::BlockType;
use crate::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use crate::generation::WorldGenerator;
use crate::sprites::{pick, pick_where, sprite};
use crate::{PropSpawnData, WorldPropKind};

/// Kích thước một vùng (khối).
pub const REGION: i32 = 192;
/// Khoảng cách tối thiểu từ tâm khu tới rìa vùng (để khu không bị cắt bởi vùng khác).
const MARGIN: i32 = 56;
/// Bề rộng dải chuyển tiếp địa hình ngoài bán kính khu.
pub const BLEND: i32 = 6;
const MAX_FOOTPRINT: i32 = 52;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SiteKind {
    Village,
    BossArena,
}

#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub kind: SiteKind,
    pub x: i32,
    pub z: i32,
    /// Độ cao mặt đất (chỉ số khối trên cùng) sau khi san phẳng.
    pub base_y: i32,
    /// Bán kính vùng phẳng.
    pub radius: i32,
    pub seed: u32,
}

impl Site {
    #[inline]
    pub fn dist(&self, wx: i32, wz: i32) -> f32 {
        let dx = (wx - self.x) as f32;
        let dz = (wz - self.z) as f32;
        (dx * dx + dz * dz).sqrt()
    }

    /// Điểm có nằm trong vùng phẳng (+ biên `margin`) không.
    #[inline]
    pub fn contains(&self, wx: i32, wz: i32, margin: i32) -> bool {
        self.dist(wx, wz) <= (self.radius + margin) as f32
    }
}

pub(crate) fn hash2(a: i32, b: i32, salt: u32) -> u32 {
    let mut h = (a as u32).wrapping_mul(73856093)
        ^ (b as u32).wrapping_mul(83492791)
        ^ salt.wrapping_mul(2654435761);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h
}

/// RNG xorshift nhỏ.
struct SRng(u32);
impl SRng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

#[inline]
fn set_world(chunk: &mut ChunkData, ox: i32, oz: i32, wx: i32, wy: i32, wz: i32, b: BlockType) {
    let lx = wx - ox;
    let lz = wz - oz;
    if lx < 0 || lz < 0 || wy < 0 {
        return;
    }
    let (lx, lz, y) = (lx as usize, lz as usize, wy as usize);
    if lx >= CHUNK_SIZE_X || lz >= CHUNK_SIZE_Z || y >= CHUNK_SIZE_Y {
        return;
    }
    chunk.set_block(lx, y, lz, b);
}

impl WorldGenerator {
    /// Khu của một vùng (nếu có).
    pub fn site_for_region(&self, rx: i32, rz: i32) -> Option<Site> {
        let h = hash2(rx, rz, 901);
        let kind = match h % 100 {
            0..=37 => SiteKind::Village,
            38..=57 => SiteKind::BossArena,
            _ => return None,
        };
        let span = (REGION - 2 * MARGIN) as u32;
        let x = rx * REGION + MARGIN + ((h >> 8) % span) as i32;
        let z = rz * REGION + MARGIN + ((h >> 16) % span) as i32;
        let radius = match kind {
            SiteKind::Village => 44,
            SiteKind::BossArena => 24,
        };

        let sea = self.sea_level as i32;
        let centre = self.get_surface_height(x as f64, z as f64) as i32;
        let (temp, _) = self.get_climate(x as f64, z as f64);
        if centre <= sea + 3 || centre > 36 || temp < 0.28 {
            return None;
        }

        // Địa hình phải tương đối bằng phẳng quanh tâm
        let mut lo = centre;
        let mut hi = centre;
        let mut sum = centre;
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let sx = x + (a.cos() * radius as f32 * 0.7).round() as i32;
            let sz = z + (a.sin() * radius as f32 * 0.7).round() as i32;
            let hh = self.get_surface_height(sx as f64, sz as f64) as i32;
            if hh <= sea + 2 {
                return None;
            }
            lo = lo.min(hh);
            hi = hi.max(hh);
            sum += hh;
        }
        if hi - lo > 8 {
            return None;
        }
        let base_y = (sum as f32 / 9.0).round() as i32;
        if base_y <= sea + 2 || base_y + 12 >= CHUNK_SIZE_Y as i32 {
            return None;
        }

        Some(Site { kind, x, z, base_y, radius, seed: hash2(rx, rz, 77) | 1 })
    }

    /// Các khu có phần (kể cả dải chuyển tiếp) chạm vào hình chữ nhật cho trước.
    pub fn sites_in_area(&self, min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> Vec<Site> {
        let r0x = (min_x - MAX_FOOTPRINT).div_euclid(REGION);
        let r1x = (max_x + MAX_FOOTPRINT).div_euclid(REGION);
        let r0z = (min_z - MAX_FOOTPRINT).div_euclid(REGION);
        let r1z = (max_z + MAX_FOOTPRINT).div_euclid(REGION);
        let mut out = Vec::new();
        for rx in r0x..=r1x {
            for rz in r0z..=r1z {
                if let Some(s) = self.site_for_region(rx, rz) {
                    let f = s.radius + BLEND;
                    if s.x + f >= min_x && s.x - f <= max_x && s.z + f >= min_z && s.z - f <= max_z {
                        out.push(s);
                    }
                }
            }
        }
        out
    }

    pub fn sites_near_chunk(&self, chunk_x: i32, chunk_z: i32) -> Vec<Site> {
        let ox = chunk_x * CHUNK_SIZE_X as i32;
        let oz = chunk_z * CHUNK_SIZE_Z as i32;
        self.sites_in_area(ox, oz, ox + CHUNK_SIZE_X as i32 - 1, oz + CHUNK_SIZE_Z as i32 - 1)
    }

    /// San phẳng địa hình quanh các khu và cập nhật `surface_map`.
    pub(crate) fn level_sites(
        &self,
        chunk: &mut ChunkData,
        ox: i32,
        oz: i32,
        sites: &[Site],
        surface_map: &mut [[usize; CHUNK_SIZE_Z]; CHUNK_SIZE_X],
    ) {
        if sites.is_empty() {
            return;
        }
        let sea = self.sea_level;
        for x in 0..CHUNK_SIZE_X {
            for z in 0..CHUNK_SIZE_Z {
                let (wx, wz) = (ox + x as i32, oz + z as i32);
                let orig = surface_map[x][z];
                for site in sites {
                    let d = site.dist(wx, wz);
                    let target = if d <= site.radius as f32 {
                        site.base_y as usize
                    } else if d <= (site.radius + BLEND) as f32 {
                        let t = (d - site.radius as f32) / BLEND as f32;
                        let s = t * t * (3.0 - 2.0 * t);
                        (site.base_y as f32 + (orig as f32 - site.base_y as f32) * s).round() as usize
                    } else {
                        continue;
                    };
                    if target != orig {
                        let mut top = chunk.get_block(x, orig, z);
                        if matches!(top, BlockType::Water | BlockType::Air) {
                            top = BlockType::Sand;
                        }
                        if target > orig {
                            for y in orig..target {
                                let b = if y + 3 >= target { BlockType::Dirt } else { BlockType::Stone };
                                chunk.set_block(x, y, z, b);
                            }
                        } else {
                            for y in (target + 1)..CHUNK_SIZE_Y {
                                chunk.set_block(x, y, z, if y <= sea { BlockType::Water } else { BlockType::Air });
                            }
                        }
                        chunk.set_block(x, target, z, top);
                        surface_map[x][z] = target;
                    }
                    break;
                }
            }
        }
    }

    /// Xây công trình của các khu vào chunk và thêm prop (rương, điểm spawn boss).
    pub(crate) fn build_sites(
        &self,
        chunk: &mut ChunkData,
        ox: i32,
        oz: i32,
        sites: &[Site],
        props: &mut Vec<PropSpawnData>,
    ) {
        for site in sites {
            match site.kind {
                SiteKind::Village => build_village(chunk, ox, oz, site, props),
                SiteKind::BossArena => build_boss_arena(chunk, ox, oz, site, props),
            }
        }
    }
}

fn in_chunk(ox: i32, oz: i32, wx: i32, wz: i32) -> bool {
    wx >= ox && wz >= oz && wx < ox + CHUNK_SIZE_X as i32 && wz < oz + CHUNK_SIZE_Z as i32
}

/// Thêm một sprite tại toạ độ thế giới (x, y, z) nếu điểm đó nằm trong chunk đang sinh.
#[allow(clippy::too_many_arguments)]
fn add_sprite(
    props: &mut Vec<PropSpawnData>,
    ox: i32,
    oz: i32,
    kind: Option<WorldPropKind>,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
) {
    let Some(kind) = kind else { return };
    if !in_chunk(ox, oz, x.floor() as i32, z.floor() as i32) {
        return;
    }
    props.push(PropSpawnData { kind, world_pos: Vec3::new(x, y, z), rotation: Quat::from_rotation_y(yaw) });
}

// ---------------------------------------------------------------------------------------------
// Làng
// ---------------------------------------------------------------------------------------------

struct House {
    x0: i32,
    x1: i32,
    z0: i32,
    z1: i32,
    /// 0: +X, 1: -X, 2: +Z, 3: -Z (cạnh có cửa, hướng về quảng trường)
    door: u8,
}

fn village_houses(site: &Site) -> Vec<House> {
    let mut rng = SRng(site.seed);
    let n = 4 + rng.below(3) as i32;
    let step = std::f32::consts::TAU / n as f32;
    let mut out = Vec::new();
    for i in 0..n {
        let a = (i as f32 + (rng.f() - 0.5) * 0.3) * step;
        let r = 17.0 + rng.f() * 5.0;
        let hx = site.x + (a.cos() * r).round() as i32;
        let hz = site.z + (a.sin() * r).round() as i32;
        let w = if rng.below(2) == 0 { 7 } else { 9 };
        let d = 7;
        let dx = site.x - hx;
        let dz = site.z - hz;
        let door = if dx.abs() >= dz.abs() {
            if dx > 0 { 0 } else { 1 }
        } else if dz > 0 {
            2
        } else {
            3
        };
        out.push(House { x0: hx - w / 2, x1: hx + w / 2, z0: hz - d / 2, z1: hz + d / 2, door });
    }
    out
}

fn build_village(chunk: &mut ChunkData, ox: i32, oz: i32, site: &Site, props: &mut Vec<PropSpawnData>) {
    let by = site.base_y;

    // Quảng trường lát đá + giếng
    for lx in 0..CHUNK_SIZE_X as i32 {
        for lz in 0..CHUNK_SIZE_Z as i32 {
            let (wx, wz) = (ox + lx, oz + lz);
            if site.dist(wx, wz) <= 6.0 {
                set_world(chunk, ox, oz, wx, by, wz, BlockType::Cobble);
            }
        }
    }
    for dx in -1..=1 {
        for dz in -1..=1 {
            let (wx, wz) = (site.x + dx, site.z + dz);
            if dx == 0 && dz == 0 {
                set_world(chunk, ox, oz, wx, by, wz, BlockType::Water);
            } else {
                set_world(chunk, ox, oz, wx, by + 1, wz, BlockType::Cobble);
            }
            // mái giếng
            set_world(chunk, ox, oz, wx, by + 5, wz, BlockType::Planks);
        }
    }
    for (dx, dz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        for y in 2..=4 {
            set_world(chunk, ox, oz, site.x + dx, by + y, site.z + dz, BlockType::Wood);
        }
    }

    // Trạm chế tạo, đèn đường, biển chỉ dẫn và thùng gỗ quanh quảng trường
    let y = by as f32 + 1.0;
    for (name, dx, dz, yaw) in [
        ("crafting_station/anvil", -4, -4, 0.8),
        ("crafting_station/forge", 4, -4, -0.8),
        ("crafting_station/crafting_bench", -4, 4, 2.4),
        ("crafting_station/cooking_pot", 4, 4, -2.4),
        ("misc/street_lamp", 6, 0, 0.0),
        ("misc/street_lamp", -6, 0, 0.0),
        ("sign/basic", 0, -7, 0.0),
        ("furniture/barrel", 0, 5, 0.0),
    ] {
        add_sprite(props, ox, oz, sprite(name), (site.x + dx) as f32 + 0.5, y, (site.z + dz) as f32 + 0.5, yaw);
    }

    for house in village_houses(site) {
        build_house(chunk, ox, oz, by, &house, props);
    }

    // Ruộng bao quanh làng, nằm giữa các hướng nhà
    for (px, pz, kind) in village_fields(site) {
        build_field(chunk, ox, oz, by, px, pz, kind, props);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Wheat,
    Rice,
    /// Vườn rau: đất cày + sprite rau củ (cà rốt, bí, ngô, cà chua...)
    Garden,
}

/// Tâm các thửa ruộng (x, z, loại).
fn village_fields(site: &Site) -> Vec<(i32, i32, FieldKind)> {
    // Phải khớp số nhà của `village_houses` (cùng lần rút đầu tiên của RNG).
    let mut rng = SRng(site.seed);
    let n = 4 + rng.below(3) as i32;
    let step = std::f32::consts::TAU / n as f32;
    let mut rng = SRng(site.seed ^ 0x9e37_79b9);
    (0..n)
        .map(|i| {
            let a = (i as f32 + 0.5) * step;
            let px = site.x + (a.cos() * 35.0).round() as i32;
            let pz = site.z + (a.sin() * 35.0).round() as i32;
            let kind = match rng.below(3) {
                0 => FieldKind::Wheat,
                1 => FieldKind::Rice,
                _ => FieldKind::Garden,
            };
            (px, pz, kind)
        })
        .collect()
}

/// Một thửa ruộng 11x9: kênh nước ở giữa, hai bên là đất cày, cột rào ở bốn góc, bù nhìn và rơm.
#[allow(clippy::too_many_arguments)]
fn build_field(
    chunk: &mut ChunkData,
    ox: i32,
    oz: i32,
    by: i32,
    px: i32,
    pz: i32,
    kind: FieldKind,
    props: &mut Vec<PropSpawnData>,
) {
    let crop = match kind {
        FieldKind::Wheat => Some(BlockType::Wheat),
        FieldKind::Rice => Some(BlockType::Rice),
        FieldKind::Garden => None,
    };
    for x in (px - 5)..=(px + 5) {
        for z in (pz - 4)..=(pz + 4) {
            let edge = x == px - 5 || x == px + 5 || z == pz - 4 || z == pz + 4;
            for y in 1..=3 {
                set_world(chunk, ox, oz, x, by + y, z, BlockType::Air);
            }
            if z == pz {
                set_world(chunk, ox, oz, x, by, z, BlockType::Water);
            } else {
                set_world(chunk, ox, oz, x, by, z, BlockType::Farmland);
                if let (false, Some(block)) = (edge, crop) {
                    set_world(chunk, ox, oz, x, by + 1, z, block);
                }
            }
        }
    }
    for (cx, cz) in [(px - 5, pz - 4), (px + 5, pz - 4), (px - 5, pz + 4), (px + 5, pz + 4)] {
        for y in 1..=2 {
            set_world(chunk, ox, oz, cx, by + y, cz, BlockType::Wood);
        }
    }

    let seed = hash2(px, pz, 17);
    let y = by as f32 + 1.0;
    if kind == FieldKind::Garden {
        let cat = ["carrot", "turnip", "radish", "tomato", "pumpkin", "corn", "lettuce", "flax"][(seed % 8) as usize];
        for dx in (-4..=4).step_by(2) {
            for dz in [-3, -1, 1, 3] {
                let h = hash2(px + dx, pz + dz, seed);
                add_sprite(
                    props, ox, oz, pick(cat, h),
                    (px + dx) as f32 + 0.5, y, (pz + dz) as f32 + 0.5,
                    (h % 360) as f32 * std::f32::consts::PI / 180.0,
                );
            }
        }
    }
    // Bù nhìn ở rìa ruộng và bó rơm ngoài hàng rào
    add_sprite(props, ox, oz, sprite("misc/scarecrow"), px as f32 + 0.5, y, (pz - 4) as f32 + 0.5, 0.0);
    add_sprite(props, ox, oz, sprite("hay/hay"), (px + 7) as f32 + 0.5, y, pz as f32 + 0.5, 0.0);
}

fn build_house(chunk: &mut ChunkData, ox: i32, oz: i32, by: i32, h: &House, props: &mut Vec<PropSpawnData>) {
    const WALL_H: i32 = 4;
    let hx = (h.x0 + h.x1) / 2;
    let hz = (h.z0 + h.z1) / 2;
    let door_cell = match h.door {
        0 => (h.x1, hz),
        1 => (h.x0, hz),
        2 => (hx, h.z1),
        _ => (hx, h.z0),
    };

    for x in h.x0..=h.x1 {
        for z in h.z0..=h.z1 {
            let edge_x = x == h.x0 || x == h.x1;
            let edge_z = z == h.z0 || z == h.z1;
            // Sàn + dọn khoảng trống bên trong
            set_world(chunk, ox, oz, x, by, z, BlockType::Planks);
            for y in 1..=WALL_H {
                set_world(chunk, ox, oz, x, by + y, z, BlockType::Air);
            }
            if !(edge_x || edge_z) {
                continue;
            }
            let corner = edge_x && edge_z;
            let along = if edge_z { x - h.x0 } else { z - h.z0 };
            let near_door = (x - door_cell.0).abs() + (z - door_cell.1).abs() <= 1;
            for y in 1..=WALL_H {
                let mut b = if corner { BlockType::Wood } else { BlockType::Planks };
                if (x, z) == door_cell && y <= 3 {
                    b = BlockType::Air; // cửa ra vào cao 3 khối (bộ va chạm cần 3 ô trống)
                } else if !corner && !near_door && y == 2 && along % 3 == 2 {
                    b = BlockType::Air; // cửa sổ
                }
                set_world(chunk, ox, oz, x, by + y, z, b);
            }
        }
    }

    // Mái chữ A, đỉnh nóc chạy dọc trục X
    let half = (h.z1 - h.z0) / 2 + 1;
    for k in 0..=half {
        let y = by + WALL_H + 1 + k;
        for x in (h.x0 - 1)..=(h.x1 + 1) {
            for dz in -(half - k)..=(half - k) {
                set_world(chunk, ox, oz, x, y, hz + dz, BlockType::RoofTile);
            }
        }
    }

    // Nội thất: giường, bàn, rương
    set_world(chunk, ox, oz, h.x0 + 1, by + 1, h.z0 + 1, BlockType::Bed);
    set_world(chunk, ox, oz, h.x0 + 2, by + 1, h.z0 + 1, BlockType::Bed);
    set_world(chunk, ox, oz, h.x1 - 1, by + 1, h.z1 - 1, BlockType::Planks);
    let (cx, cz) = (h.x1 - 1, h.z0 + 1);
    if in_chunk(ox, oz, cx, cz) {
        props.push(PropSpawnData {
            kind: WorldPropKind::Chest,
            world_pos: Vec3::new(cx as f32 + 0.5, by as f32 + 1.45, cz as f32 + 0.5),
            rotation: Quat::from_rotation_y(match h.door {
                0 => std::f32::consts::FRAC_PI_2,
                1 => -std::f32::consts::FRAC_PI_2,
                2 => 0.0,
                _ => std::f32::consts::PI,
            }),
        });
    }

    // Đèn treo cạnh cửa, bàn ghế và đồ đạc bên trong
    let hh = hash2(h.x0, h.z0, 31);
    let (ox_dir, oz_dir) = match h.door {
        0 => (1, 0),
        1 => (-1, 0),
        2 => (0, 1),
        _ => (0, -1),
    };
    let (lat_x, lat_z) = (oz_dir, ox_dir);
    let y = by as f32 + 1.0;
    add_sprite(
        props, ox, oz, pick_where("lantern", "lamp_terracotta_on", hh),
        (door_cell.0 + ox_dir * 2 + lat_x * 2) as f32 + 0.5, y,
        (door_cell.1 + oz_dir * 2 + lat_z * 2) as f32 + 0.5, 0.0,
    );
    add_sprite(
        props, ox, oz, sprite("furniture/chair_wood_woodland"),
        (h.x1 - 2) as f32 + 0.5, y, (h.z1 - 1) as f32 + 0.5, std::f32::consts::FRAC_PI_2,
    );
    let decor = ["furniture/bookshelf_middle-0", "furniture/barrel", "furniture/basket_woven_M", "furniture/planter-0", "furniture/crate-0"];
    add_sprite(
        props, ox, oz, sprite(decor[(hh % decor.len() as u32) as usize]),
        (h.x0 + 1) as f32 + 0.5, y, (h.z1 - 1) as f32 + 0.5, 0.0,
    );
}

// ---------------------------------------------------------------------------------------------
// Đấu trường boss
// ---------------------------------------------------------------------------------------------

fn build_boss_arena(chunk: &mut ChunkData, ox: i32, oz: i32, site: &Site, props: &mut Vec<PropSpawnData>) {
    let by = site.base_y;
    let r_max = site.radius as f32;

    // Cột trụ obsidian quanh sân
    let pillars: Vec<(i32, i32)> = (0..8)
        .map(|k| {
            let a = (k as f32 * 45.0 + 22.5).to_radians();
            (
                site.x + (a.cos() * 10.5).round() as i32,
                site.z + (a.sin() * 10.5).round() as i32,
            )
        })
        .collect();

    for lx in 0..CHUNK_SIZE_X as i32 {
        for lz in 0..CHUNK_SIZE_Z as i32 {
            let (wx, wz) = (ox + lx, oz + lz);
            let dx = wx - site.x;
            let dz = wz - site.z;
            let r = site.dist(wx, wz);
            if r > r_max {
                continue;
            }
            let on_axis = dx.abs() <= 2 || dz.abs() <= 2;
            let tile = if (wx + wz).rem_euclid(2) == 0 { BlockType::Cobble } else { BlockType::CliffStone };

            if r <= 15.5 {
                // Sân đấu lát gạch ô cờ
                set_world(chunk, ox, oz, wx, by, wz, tile);
                for y in 1..=8 {
                    set_world(chunk, ox, oz, wx, by + y, wz, BlockType::Air);
                }
                if r <= 3.5 {
                    set_world(chunk, ox, oz, wx, by, wz, BlockType::Obsidian);
                    set_world(chunk, ox, oz, wx, by + 1, wz, BlockType::Obsidian);
                }
            } else if r <= 17.5 {
                if on_axis {
                    // Cổng vào theo 4 hướng
                    set_world(chunk, ox, oz, wx, by, wz, BlockType::Cobble);
                    for y in 1..=8 {
                        set_world(chunk, ox, oz, wx, by + y, wz, BlockType::Air);
                    }
                } else {
                    for y in 0..=7 {
                        // Lỗ châu mai trên đỉnh tường
                        if y == 7 && (wx + wz).rem_euclid(2) == 1 {
                            set_world(chunk, ox, oz, wx, by + y, wz, BlockType::Air);
                        } else {
                            set_world(chunk, ox, oz, wx, by + y, wz, BlockType::CliffStone);
                        }
                    }
                }
            } else if on_axis && (dx.abs() <= 1 || dz.abs() <= 1) {
                // Lối đi lát đá dẫn ra ngoài
                set_world(chunk, ox, oz, wx, by, wz, BlockType::Cobble);
            }
        }
    }

    for (px, pz) in pillars {
        for y in 1..=6 {
            set_world(chunk, ox, oz, px, by + y, pz, BlockType::Obsidian);
        }
        set_world(chunk, ox, oz, px, by + 7, pz, BlockType::Cobble);
    }

    if in_chunk(ox, oz, site.x, site.z) {
        props.push(PropSpawnData {
            kind: WorldPropKind::BossSpawn,
            world_pos: Vec3::new(site.x as f32 + 0.5, by as f32 + 2.0, site.z as f32 + 0.5),
            rotation: Quat::IDENTITY,
        });
    }

    // Trang trí: ấn pháp trên bục, nến ở chân cột, xương trong sân, mộ bên ngoài
    let y_floor = by as f32 + 1.0;
    add_sprite(props, ox, oz, sprite("wizard/magical_seal"), site.x as f32 + 0.5, by as f32 + 2.0, site.z as f32 + 0.5, 0.0);
    for k in 0..8 {
        let a = (k as f32 * 45.0 + 22.5).to_radians();
        let (cx, cz) = (site.x + (a.cos() * 10.5).round() as i32 + 1, site.z + (a.sin() * 10.5).round() as i32);
        add_sprite(props, ox, oz, sprite("wizard/candle"), cx as f32 + 0.5, y_floor, cz as f32 + 0.5, 0.0);
    }
    let mut rng = SRng(site.seed ^ 0x51ed_270b);
    for _ in 0..5 {
        let a = rng.f() * std::f32::consts::TAU;
        let r = 6.0 + rng.f() * 7.0;
        let h = rng.next();
        add_sprite(
            props, ox, oz, pick("bone", h),
            site.x as f32 + 0.5 + a.cos() * r, y_floor, site.z as f32 + 0.5 + a.sin() * r, a,
        );
    }
    for _ in 0..8 {
        let a = rng.f() * std::f32::consts::TAU;
        let r = 19.0 + rng.f() * 4.0;
        let (gx, gz) = (site.x as f32 + 0.5 + a.cos() * r, site.z as f32 + 0.5 + a.sin() * r);
        // tránh lối đi 4 hướng
        if (gx - site.x as f32 - 0.5).abs() < 3.0 || (gz - site.z as f32 - 0.5).abs() < 3.0 {
            continue;
        }
        add_sprite(props, ox, oz, pick("grave", rng.next()), gx, by as f32 + 1.0, gz, a + std::f32::consts::PI);
    }
}
