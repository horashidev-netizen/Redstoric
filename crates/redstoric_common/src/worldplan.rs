//! # WorldPlan: bố cục thế giới có chủ đích (tham khảo `world/src/civ` của Veloren)
//!
//! Thay vì rải làng/boss ngẫu nhiên theo từng vùng, thế giới được lập kế hoạch một lần từ seed:
//!
//! 1. **Thị trấn trung tâm** (bậc 0, `Site::id == 0`): làng an toàn gần điểm xuất phát, nơi người chơi
//!    nhận nhiệm vụ đầu tiên.
//! 2. **Các làng** rải đều (khoảng cách tối thiểu giữa hai làng), bậc 1..=3 tăng theo khoảng cách tới trung tâm.
//! 3. **Đấu trường boss** (dungeon) đặt xa thị trấn, bậc 1..=5 tăng theo khoảng cách: càng ra rìa đảo càng khó.
//!
//! Mỗi khu có `id` (SiteId) ổn định theo seed để hệ thống nhiệm vụ tham chiếu. Kết quả chỉ phụ thuộc vào
//! `seed` và địa hình nên server và mọi client tự lập ra cùng một bố cục.

use crate::generation::WorldGenerator;
use crate::structures::{hash2, Site, SiteKind};
use crate::WORLD_HALF;

/// Số làng ngoài thị trấn trung tâm.
pub const EXTRA_VILLAGES: usize = 7;
/// Số đấu trường boss.
pub const ARENAS: usize = 9;

/// Khoảng cách tối thiểu giữa hai làng (khối).
const VILLAGE_SPACING: f32 = 260.0;
/// Khoảng cách tối thiểu giữa hai đấu trường.
const ARENA_SPACING: f32 = 190.0;
/// Khoảng cách tối thiểu giữa đấu trường và làng gần nhất.
const ARENA_VILLAGE_GAP: f32 = 140.0;
/// Đấu trường phải cách thị trấn trung tâm ít nhất từng này (vùng an toàn quanh thị trấn).
const HUB_SAFE_RADIUS: f32 = 300.0;
/// Số lần thử đặt tối đa cho mỗi loại khu.
const MAX_TRIES: usize = 6000;

struct PlanRng(u32);
impl PlanRng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Số thực trong [0, 1).
    fn f(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

fn dist(a: (i32, i32), b: (i32, i32)) -> f32 {
    let dx = (a.0 - b.0) as f32;
    let dz = (a.1 - b.1) as f32;
    (dx * dx + dz * dz).sqrt()
}

fn min_dist(sites: &[Site], x: i32, z: i32, kind: Option<SiteKind>) -> f32 {
    sites
        .iter()
        .filter(|s| kind.map_or(true, |k| s.kind == k))
        .map(|s| dist((s.x, s.z), (x, z)))
        .fold(f32::MAX, f32::min)
}

/// Bậc độ khó của một khu theo khoảng cách `d` từ thị trấn trung tâm.
pub fn tier_for_distance(d: f32, kind: SiteKind) -> u8 {
    match kind {
        SiteKind::Village => {
            if d < 450.0 {
                1
            } else if d < 800.0 {
                2
            } else {
                3
            }
        }
        SiteKind::BossArena => ((1.0 + d / 260.0) as u8).clamp(1, 5),
    }
}

/// Lập bố cục thế giới. Phần tử `i` của kết quả có `id == i`.
pub fn generate_plan(generator: &WorldGenerator, seed: u32) -> Vec<Site> {
    let mut rng = PlanRng(hash2(seed as i32, 4242, 99) | 1);
    let mut sites: Vec<Site> = Vec::new();
    let reach = (WORLD_HALF - 140) as f32;

    // 1. Thị trấn trung tâm: tìm quanh điểm xuất phát theo vòng xoắn, chừa chỗ để người chơi
    //    xuất hiện ngay ngoài rìa thị trấn rồi đi vào.
    'hub: for ring in 0..40 {
        let radius = 60.0 + ring as f32 * 24.0;
        for k in 0..24 {
            let a = k as f32 / 24.0 * std::f32::consts::TAU + ring as f32 * 0.37;
            let x = (a.cos() * radius).round() as i32;
            let z = (a.sin() * radius).round() as i32;
            if let Some(s) = generator.make_site(SiteKind::Village, x, z, 0, 0) {
                sites.push(s);
                break 'hub;
            }
        }
    }
    let hub = sites.first().map(|s| (s.x, s.z)).unwrap_or((0, 0));

    // 2. Các làng: lấy mẫu ngẫu nhiên, giữ khoảng cách đều nhau.
    let mut villages = 0;
    for _ in 0..MAX_TRIES {
        if villages >= EXTRA_VILLAGES {
            break;
        }
        let x = ((rng.f() * 2.0 - 1.0) * reach) as i32;
        let z = ((rng.f() * 2.0 - 1.0) * reach) as i32;
        if min_dist(&sites, x, z, None) < VILLAGE_SPACING {
            continue;
        }
        let tier = tier_for_distance(dist((x, z), hub), SiteKind::Village);
        if let Some(s) = generator.make_site(SiteKind::Village, x, z, sites.len() as u32, tier) {
            sites.push(s);
            villages += 1;
        }
    }

    // 3. Đấu trường boss: xa thị trấn, cách xa làng, độ khó tăng theo khoảng cách.
    let mut arenas = 0;
    for _ in 0..MAX_TRIES {
        if arenas >= ARENAS {
            break;
        }
        let x = ((rng.f() * 2.0 - 1.0) * reach) as i32;
        let z = ((rng.f() * 2.0 - 1.0) * reach) as i32;
        let d_hub = dist((x, z), hub);
        if d_hub < HUB_SAFE_RADIUS
            || min_dist(&sites, x, z, Some(SiteKind::Village)) < ARENA_VILLAGE_GAP
            || min_dist(&sites, x, z, Some(SiteKind::BossArena)) < ARENA_SPACING
        {
            continue;
        }
        let tier = tier_for_distance(d_hub, SiteKind::BossArena);
        if let Some(s) = generator.make_site(SiteKind::BossArena, x, z, sites.len() as u32, tier) {
            sites.push(s);
            arenas += 1;
        }
    }

    sites
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WORLD_SEED;

    fn plan(seed: u32) -> Vec<Site> {
        generate_plan(&WorldGenerator::new(seed), seed)
    }

    #[test]
    fn plan_has_hub_and_stable_ids() {
        let sites = plan(WORLD_SEED);
        assert!(!sites.is_empty(), "WorldPlan rỗng");
        for (i, s) in sites.iter().enumerate() {
            assert_eq!(s.id as usize, i, "Site::id phải bằng chỉ số trong plan");
        }
        assert!(sites[0].kind == SiteKind::Village, "khu đầu tiên phải là thị trấn trung tâm");
        assert_eq!(sites[0].tier, 0);
    }

    #[test]
    fn plan_respects_counts_tiers_and_spacing() {
        let sites = plan(WORLD_SEED);
        let hub = (sites[0].x, sites[0].z);
        let villages = sites.iter().filter(|s| s.kind == SiteKind::Village).count();
        let arenas = sites.iter().filter(|s| s.kind == SiteKind::BossArena).count();
        assert!(villages >= 2 && villages <= 1 + EXTRA_VILLAGES, "số làng bất thường: {villages}");
        assert!(arenas >= 1 && arenas <= ARENAS, "số đấu trường bất thường: {arenas}");

        for (i, a) in sites.iter().enumerate() {
            assert!(a.x.abs() < WORLD_HALF && a.z.abs() < WORLD_HALF, "khu {i} nằm ngoài thế giới");
            match a.kind {
                SiteKind::Village if i > 0 => {
                    assert!((1..=3).contains(&a.tier), "bậc làng {i} = {}", a.tier);
                }
                SiteKind::BossArena => {
                    assert!((1..=5).contains(&a.tier), "bậc đấu trường {i} = {}", a.tier);
                    assert!(
                        dist((a.x, a.z), hub) >= HUB_SAFE_RADIUS - 0.01,
                        "đấu trường {i} nằm trong vùng an toàn quanh thị trấn"
                    );
                }
                _ => {}
            }
            for b in sites.iter().skip(i + 1) {
                let d = dist((a.x, a.z), (b.x, b.z));
                match (a.kind, b.kind) {
                    (SiteKind::Village, SiteKind::Village) => {
                        assert!(d >= VILLAGE_SPACING - 0.01, "làng {} và {} quá gần ({d})", a.id, b.id)
                    }
                    (SiteKind::BossArena, SiteKind::BossArena) => {
                        assert!(d >= ARENA_SPACING - 0.01, "đấu trường {} và {} quá gần ({d})", a.id, b.id)
                    }
                    _ => assert!(d >= ARENA_VILLAGE_GAP - 0.01, "khu {} và {} quá gần ({d})", a.id, b.id),
                }
            }
        }
    }

    #[test]
    fn plan_is_deterministic_and_seed_dependent() {
        let a = plan(WORLD_SEED);
        let b = plan(WORLD_SEED);
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(b.iter()) {
            assert!(
                p.kind == q.kind
                    && p.x == q.x
                    && p.z == q.z
                    && p.base_y == q.base_y
                    && p.radius == q.radius
                    && p.seed == q.seed
                    && p.id == q.id
                    && p.tier == q.tier,
                "cùng seed phải ra cùng bố cục (khu {})",
                p.id
            );
        }
        let c = plan(WORLD_SEED.wrapping_add(1));
        let same = a.len() == c.len() && a.iter().zip(c.iter()).all(|(p, q)| p.x == q.x && p.z == q.z);
        assert!(!same, "hai seed khác nhau không được cho cùng bố cục");
    }

    #[test]
    fn tier_grows_with_distance() {
        assert_eq!(tier_for_distance(0.0, SiteKind::Village), 1);
        assert_eq!(tier_for_distance(449.0, SiteKind::Village), 1);
        assert_eq!(tier_for_distance(450.0, SiteKind::Village), 2);
        assert_eq!(tier_for_distance(799.0, SiteKind::Village), 2);
        assert_eq!(tier_for_distance(800.0, SiteKind::Village), 3);
        assert_eq!(tier_for_distance(0.0, SiteKind::BossArena), 1);
        assert_eq!(tier_for_distance(1.0e9, SiteKind::BossArena), 5);
        for kind in [SiteKind::Village, SiteKind::BossArena] {
            let mut prev = 0u8;
            for d in (0..3000).step_by(10) {
                let t = tier_for_distance(d as f32, kind);
                assert!(t >= prev, "bậc giảm khi khoảng cách tăng (d = {d})");
                prev = t;
            }
        }
    }
}
