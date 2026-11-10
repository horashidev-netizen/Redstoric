//! Mô tả khung xương động vật: mỗi khớp (bone) có vị trí so với khớp cha và một mảnh model `.glb`
//! (tâm mảnh lệch so với khớp một đoạn `center`). Dữ liệu trong `layout.rs` được sinh tự động từ
//! skeleton + manifest voxel của Veloren.
use redstoric_common::fauna::{FishKind, Species};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartRole {
    Body,
    Rear,
    Neck,
    Head,
    Jaw,
    Ears,
    Tail,
    LegFL,
    LegFR,
    LegBL,
    LegBR,
    FootFL,
    FootFR,
    FootBL,
    FootBR,
    FinL,
    FinR,
}

pub struct PartDef {
    /// Đường dẫn trong `assets/placeholders/glb/npc/`.
    pub glb: &'static str,
    pub role: PartRole,
    /// Chỉ số khớp cha trong cùng bảng (`None` = gắn vào gốc model).
    pub parent: Option<usize>,
    /// Vị trí khớp so với cha (đơn vị voxel, trục Bevy).
    pub bone: [f32; 3],
    /// Tâm mảnh model so với khớp.
    pub center: [f32; 3],
}

pub fn layout_of(species: Species) -> &'static [PartDef] {
    match species {
        Species::Pig => super::layout::PIG,
        Species::Rabbit => super::layout::RABBIT,
        Species::Wolf => super::layout::WOLF,
        Species::Deer => super::layout::DEER,
    }
}

pub fn fish_layout_of(kind: FishKind) -> &'static [PartDef] {
    match kind {
        FishKind::Clownfish => super::layout::CLOWNFISH,
        FishKind::Piranha => super::layout::PIRANHA,
    }
}
