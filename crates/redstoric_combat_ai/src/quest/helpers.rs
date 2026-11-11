#![allow(unused_imports)]
use super::*;

pub(super) fn mix(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(2654435761) ^ b.wrapping_mul(40503).wrapping_add(0x9e3779b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h
}

pub(super) fn name_hash(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

pub(super) fn site_dist(site: &Site, pos: Vec3) -> f32 {
    let dx = site.x as f32 - pos.x;
    let dz = site.z as f32 - pos.z;
    (dx * dx + dz * dz).sqrt()
}

pub(super) fn sites_dist(a: &Site, b: &Site) -> f32 {
    (((a.x - b.x).pow(2) + (a.z - b.z).pow(2)) as f32).sqrt()
}

pub(super) fn species_name(s: Option<Species>) -> &'static str {
    match s {
        None => "wild animals",
        Some(Species::Pig) => "pigs",
        Some(Species::Rabbit) => "rabbits",
        Some(Species::Deer) => "deer",
        Some(Species::Wolf) => "wolves",
    }
}

/// Chỗ đánh rơi đồ cho Fetch: cách làng 70-160 khối, trên cạn.
pub(super) fn fetch_spot(generator: &WorldGenerator, site: &Site, h: u32) -> (i32, i32) {
    for i in 0..12u32 {
        let a = (mix(h, i) & 0xffff) as f32 / 65536.0 * std::f32::consts::TAU;
        let r = 70.0 + (mix(h, i + 99) % 90) as f32;
        let x = site.x + (a.cos() * r) as i32;
        let z = site.z + (a.sin() * r) as i32;
        if generator.get_surface_height(x as f64, z as f64) > generator.sea_level + 1 {
            return (x, z);
        }
    }
    (site.x + 60, site.z)
}

/// Cột mốc Patrol: `n` điểm cách đều quanh làng, bán kính 40-55 khối.
pub(super) fn patrol_points(site: &Site, h: u32, n: u32) -> Vec<(i32, i32)> {
    let base = (h & 0xffff) as f32 / 65536.0 * std::f32::consts::TAU;
    (0..n)
        .map(|k| {
            let a = base + k as f32 * std::f32::consts::TAU / n as f32;
            let r = 40.0 + (mix(h, k) % 15) as f32;
            (site.x + (a.cos() * r) as i32, site.z + (a.sin() * r) as i32)
        })
        .collect()
}

pub(super) fn site_name(generator: &WorldGenerator, id: u32) -> String {
    generator.site_by_id(id).map(|s| s.name()).unwrap_or_else(|| "?".to_string())
}

pub(super) fn goal_site(q: &Quest) -> u32 {
    match q.goal {
        QuestGoal::Hunt { site, .. }
        | QuestGoal::SlayBoss { site }
        | QuestGoal::Scout { site }
        | QuestGoal::Patrol { site, .. } => site,
        QuestGoal::Courier { to } => to,
        QuestGoal::Fetch { .. } => q.giver,
    }
}

pub(super) fn add_item(inv: &mut Inventory, name: &str, n: u32) {
    match inv.0.iter_mut().find(|(i, _)| i == name) {
        Some((_, c)) => *c += n,
        None => inv.0.push((name.to_string(), n)),
    }
}

pub(super) fn remove_item(inv: &mut Inventory, name: &str, n: u32) {
    if let Some((_, c)) = inv.0.iter_mut().find(|(i, _)| i == name) {
        *c = c.saturating_sub(n);
    }
    inv.0.retain(|(_, c)| *c > 0);
}
