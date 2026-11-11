//! Tự sinh bởi tools/gen_boss_models.py từ hộp bao các mảnh glb của Veloren. KHÔNG sửa tay.
//! Đơn vị voxel của glb; nhân với `MODEL_SCALE * BossKind::scale()` khi dựng.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Limb {
    Body,
    ArmL,
    ArmR,
    LegL,
    LegR,
}

pub struct BossPart {
    /// Đường dẫn trong `assets/placeholders/glb/npc/`.
    pub glb: &'static str,
    pub limb: Limb,
    /// Vị trí scene so với khớp của chi (hoặc gốc model với `Limb::Body`).
    pub pos: [f32; 3],
}

pub struct BossModel {
    pub parts: &'static [BossPart],
    /// Vị trí khớp: tay trái, tay phải, chân trái, chân phải.
    pub pivots: [[f32; 3]; 4],
    /// Chiều cao model (voxel).
    pub height: f32,
}

pub const OGRE: BossModel = BossModel {
    parts: &[
        BossPart { glb: "ogre/male/torso_lower.glb", limb: Limb::Body, pos: [0.00, 19.00, 0.00] },
        BossPart { glb: "ogre/male/torso_upper.glb", limb: Limb::Body, pos: [0.00, 30.00, 0.00] },
        BossPart { glb: "ogre/male/head.glb", limb: Limb::Body, pos: [0.00, 40.50, -1.00] },
        BossPart { glb: "ogre/male/shoulder_l.glb", limb: Limb::ArmL, pos: [-1.50, 0.00, 0.00] },
        BossPart { glb: "ogre/male/hand_l.glb", limb: Limb::ArmL, pos: [-1.50, -12.00, -2.00] },
        BossPart { glb: "ogre/male/shoulder_r.glb", limb: Limb::ArmR, pos: [0.00, -3.00, 0.00] },
        BossPart { glb: "ogre/male/hand_r.glb", limb: Limb::ArmR, pos: [0.00, -12.00, -2.00] },
        BossPart { glb: "ogre/male/leg_l.glb", limb: Limb::LegL, pos: [0.00, -4.00, 0.00] },
        BossPart { glb: "ogre/male/foot_l.glb", limb: Limb::LegL, pos: [0.00, -11.00, -1.00] },
        BossPart { glb: "ogre/male/leg_r.glb", limb: Limb::LegR, pos: [0.00, -4.00, 0.00] },
        BossPart { glb: "ogre/male/foot_r.glb", limb: Limb::LegR, pos: [0.00, -11.00, -1.00] },
    ],
    pivots: [[-11.00, 35.00, 0.00], [11.00, 35.00, 0.00], [-3.00, 15.00, 0.00], [3.00, 15.00, 0.00]],
    height: 47.00,
};

pub const MINOTAUR: BossModel = BossModel {
    parts: &[
        BossPart { glb: "minotaur/male/torso_lower.glb", limb: Limb::Body, pos: [0.00, 22.00, 0.00] },
        BossPart { glb: "minotaur/male/torso_upper.glb", limb: Limb::Body, pos: [0.00, 37.00, 0.00] },
        BossPart { glb: "minotaur/male/head.glb", limb: Limb::Body, pos: [0.00, 50.50, -1.00] },
        BossPart { glb: "minotaur/male/shoulder_l.glb", limb: Limb::ArmL, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "minotaur/male/hand_l.glb", limb: Limb::ArmL, pos: [0.00, -12.50, -2.00] },
        BossPart { glb: "minotaur/male/shoulder_r.glb", limb: Limb::ArmR, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "minotaur/male/hand_r.glb", limb: Limb::ArmR, pos: [0.00, -12.50, -2.00] },
        BossPart { glb: "minotaur/male/leg_l.glb", limb: Limb::LegL, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "minotaur/male/foot_l.glb", limb: Limb::LegL, pos: [0.00, -11.00, -1.00] },
        BossPart { glb: "minotaur/male/leg_r.glb", limb: Limb::LegR, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "minotaur/male/foot_r.glb", limb: Limb::LegR, pos: [0.00, -11.00, -1.00] },
    ],
    pivots: [[-10.50, 45.00, 0.00], [10.50, 45.00, 0.00], [-2.75, 16.00, 0.00], [2.75, 16.00, 0.00]],
    height: 57.00,
};

pub const YETI: BossModel = BossModel {
    parts: &[
        BossPart { glb: "yeti/male/torso_lower.glb", limb: Limb::Body, pos: [0.00, 16.00, 0.00] },
        BossPart { glb: "yeti/male/torso_upper.glb", limb: Limb::Body, pos: [0.00, 25.50, 0.00] },
        BossPart { glb: "yeti/male/head.glb", limb: Limb::Body, pos: [0.00, 35.00, -1.00] },
        BossPart { glb: "yeti/male/shoulder_l.glb", limb: Limb::ArmL, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "yeti/male/hand_l.glb", limb: Limb::ArmL, pos: [0.00, -11.50, -2.00] },
        BossPart { glb: "yeti/male/shoulder_r.glb", limb: Limb::ArmR, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "yeti/male/hand_r.glb", limb: Limb::ArmR, pos: [0.00, -11.50, -2.00] },
        BossPart { glb: "yeti/male/leg_l.glb", limb: Limb::LegL, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "yeti/male/foot_l.glb", limb: Limb::LegL, pos: [0.00, -10.00, -1.00] },
        BossPart { glb: "yeti/male/leg_r.glb", limb: Limb::LegR, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "yeti/male/foot_r.glb", limb: Limb::LegR, pos: [0.00, -10.00, -1.00] },
    ],
    pivots: [[-10.50, 32.00, 0.00], [10.50, 32.00, 0.00], [-3.25, 14.00, 0.00], [3.25, 14.00, 0.00]],
    height: 39.00,
};

pub const TROLL_MOUNTAIN: BossModel = BossModel {
    parts: &[
        BossPart { glb: "troll_mountain/male/torso_lower.glb", limb: Limb::Body, pos: [0.00, 14.50, 0.00] },
        BossPart { glb: "troll_mountain/male/torso_upper.glb", limb: Limb::Body, pos: [0.00, 30.50, 0.00] },
        BossPart { glb: "troll_mountain/male/head.glb", limb: Limb::Body, pos: [0.00, 52.00, -1.00] },
        BossPart { glb: "troll_mountain/male/shoulder_l.glb", limb: Limb::ArmL, pos: [0.00, -7.50, 0.00] },
        BossPart { glb: "troll_mountain/male/hand_l.glb", limb: Limb::ArmL, pos: [0.00, -20.50, -2.00] },
        BossPart { glb: "troll_mountain/male/shoulder_r.glb", limb: Limb::ArmR, pos: [0.00, -7.50, 0.00] },
        BossPart { glb: "troll_mountain/male/hand_r.glb", limb: Limb::ArmR, pos: [0.00, -20.50, -2.00] },
        BossPart { glb: "troll_mountain/male/leg_l.glb", limb: Limb::LegL, pos: [0.00, -4.00, 0.00] },
        BossPart { glb: "troll_mountain/male/foot_l.glb", limb: Limb::LegL, pos: [0.00, -9.50, -1.00] },
        BossPart { glb: "troll_mountain/male/leg_r.glb", limb: Limb::LegR, pos: [0.00, -4.00, 0.00] },
        BossPart { glb: "troll_mountain/male/foot_r.glb", limb: Limb::LegR, pos: [0.00, -9.50, -1.00] },
    ],
    pivots: [[-14.00, 43.00, 0.00], [14.00, 43.00, 0.00], [-3.50, 12.00, 0.00], [3.50, 12.00, 0.00]],
    height: 62.00,
};

pub const CYCLOPS: BossModel = BossModel {
    parts: &[
        BossPart { glb: "cyclops/male/torso_lower.glb", limb: Limb::Body, pos: [0.00, 22.00, 0.00] },
        BossPart { glb: "cyclops/male/torso_upper.glb", limb: Limb::Body, pos: [0.00, 35.50, 0.00] },
        BossPart { glb: "cyclops/male/head.glb", limb: Limb::Body, pos: [0.00, 50.00, -1.00] },
        BossPart { glb: "cyclops/male/shoulder_l.glb", limb: Limb::ArmL, pos: [0.00, -6.00, 0.00] },
        BossPart { glb: "cyclops/male/hand_l.glb", limb: Limb::ArmL, pos: [0.00, -18.00, -2.00] },
        BossPart { glb: "cyclops/male/shoulder_r.glb", limb: Limb::ArmR, pos: [0.00, -6.00, 0.00] },
        BossPart { glb: "cyclops/male/hand_r.glb", limb: Limb::ArmR, pos: [0.00, -18.00, -2.00] },
        BossPart { glb: "cyclops/male/leg_l.glb", limb: Limb::LegL, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "cyclops/male/foot_l.glb", limb: Limb::LegL, pos: [0.00, -12.00, -1.00] },
        BossPart { glb: "cyclops/male/leg_r.glb", limb: Limb::LegR, pos: [0.00, -3.50, 0.00] },
        BossPart { glb: "cyclops/male/foot_r.glb", limb: Limb::LegR, pos: [0.00, -12.00, -1.00] },
    ],
    pivots: [[-14.00, 44.00, 0.00], [14.00, 44.00, 0.00], [-4.00, 18.00, 0.00], [4.00, 18.00, 0.00]],
    height: 57.00,
};
