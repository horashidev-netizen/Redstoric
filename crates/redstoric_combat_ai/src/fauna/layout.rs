//! Sinh tự động từ manifest + skeleton của Veloren (quadruped_small / quadruped_medium).
//! Toạ độ theo trục Bevy (x ngang, y lên, -z là phía trước), đơn vị voxel.
use super::model::{PartDef, PartRole::*};

pub const PIG: &[PartDef] = &[
    PartDef { glb: "pig/male/chest.glb", role: Body, parent: None, bone: [0.00, 6.00, -0.00], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "pig/male/head.glb", role: Head, parent: Some(0), bone: [0.00, 2.00, -5.00], center: [0.00, 0.00, -4.00] },
    PartDef { glb: "pig/male/tail.glb", role: Tail, parent: Some(0), bone: [0.00, 2.50, 4.50], center: [0.00, 2.50, 1.00] },
    PartDef { glb: "pig/male/foot_fr.glb", role: LegFL, parent: Some(0), bone: [-4.50, -1.00, -3.50], center: [0.00, -2.00, -0.00] },
    PartDef { glb: "pig/male/foot_fr.glb", role: LegFR, parent: Some(0), bone: [4.50, -1.00, -3.50], center: [0.00, -2.00, -0.00] },
    PartDef { glb: "pig/male/foot_br.glb", role: LegBL, parent: Some(0), bone: [-3.50, 0.00, 2.00], center: [0.00, -2.50, -0.00] },
    PartDef { glb: "pig/male/foot_br.glb", role: LegBR, parent: Some(0), bone: [3.50, 0.00, 2.00], center: [0.00, -2.50, -0.00] },
];

pub const RABBIT: &[PartDef] = &[
    PartDef { glb: "rabbit/male/chest.glb", role: Body, parent: None, bone: [0.00, 6.00, 2.00], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "rabbit/male/head.glb", role: Head, parent: Some(0), bone: [0.00, 3.00, -4.00], center: [0.00, 2.00, -3.00] },
    PartDef { glb: "rabbit/male/tail.glb", role: Tail, parent: Some(0), bone: [0.00, 0.00, 4.00], center: [0.00, 1.00, 1.00] },
    PartDef { glb: "rabbit/male/foot_fr.glb", role: LegFL, parent: Some(0), bone: [-3.00, -3.00, -3.00], center: [0.00, -0.50, -2.00] },
    PartDef { glb: "rabbit/male/foot_fr.glb", role: LegFR, parent: Some(0), bone: [3.00, -3.00, -3.00], center: [0.00, -0.50, -2.00] },
    PartDef { glb: "rabbit/male/foot_br.glb", role: LegBL, parent: Some(0), bone: [-3.50, -1.00, 2.00], center: [0.00, -1.00, -0.00] },
    PartDef { glb: "rabbit/male/foot_br.glb", role: LegBR, parent: Some(0), bone: [3.50, -1.00, 2.00], center: [0.00, -1.00, -0.00] },
];

pub const WOLF: &[PartDef] = &[
    PartDef { glb: "wolf/male/torso_front.glb", role: Body, parent: None, bone: [0.00, 13.00, -12.00], center: [0.00, 0.00, 6.50] },
    PartDef { glb: "wolf/male/torso_back.glb", role: Rear, parent: Some(0), bone: [0.00, 1.00, 12.50], center: [0.00, -1.00, 5.50] },
    PartDef { glb: "wolf/male/neck.glb", role: Neck, parent: Some(0), bone: [0.00, 2.00, 4.50], center: [0.00, 0.00, -2.50] },
    PartDef { glb: "wolf/male/head.glb", role: Head, parent: Some(2), bone: [0.00, 3.00, -1.50], center: [0.00, 0.00, -6.00] },
    PartDef { glb: "wolf/male/jaw.glb", role: Jaw, parent: Some(3), bone: [0.00, -2.50, -5.00], center: [0.00, 0.00, -3.00] },
    PartDef { glb: "wolf/male/ears.glb", role: Ears, parent: Some(3), bone: [0.00, 2.50, -3.00], center: [0.00, 2.50, -0.00] },
    PartDef { glb: "wolf/male/tail.glb", role: Tail, parent: Some(1), bone: [0.00, 0.00, 11.00], center: [0.00, 0.00, 6.50] },
    PartDef { glb: "wolf/male/leg_fr.glb", role: LegFL, parent: Some(0), bone: [-4.50, -1.50, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "wolf/male/leg_fr.glb", role: LegFR, parent: Some(0), bone: [4.50, -1.50, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "wolf/male/leg_br.glb", role: LegBL, parent: Some(1), bone: [-5.00, -3.00, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "wolf/male/leg_br.glb", role: LegBR, parent: Some(1), bone: [5.00, -3.00, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "wolf/male/foot_fr.glb", role: FootFL, parent: Some(7), bone: [-0.50, -2.00, -0.00], center: [0.00, -5.00, -0.50] },
    PartDef { glb: "wolf/male/foot_fr.glb", role: FootFR, parent: Some(8), bone: [0.50, -2.00, -0.00], center: [0.00, -5.00, -0.50] },
    PartDef { glb: "wolf/male/foot_br.glb", role: FootBL, parent: Some(9), bone: [-0.00, -1.50, 1.00], center: [0.00, -5.00, -0.50] },
    PartDef { glb: "wolf/male/foot_br.glb", role: FootBR, parent: Some(10), bone: [0.00, -1.50, 1.00], center: [0.00, -5.00, -0.50] },
];

pub const DEER: &[PartDef] = &[
    PartDef { glb: "deer/male/torso_front.glb", role: Body, parent: None, bone: [0.00, 13.50, -11.00], center: [0.00, 0.00, 4.50] },
    PartDef { glb: "deer/male/torso_back.glb", role: Rear, parent: Some(0), bone: [0.00, 0.50, 9.00], center: [0.00, 0.00, 5.00] },
    PartDef { glb: "deer/male/neck.glb", role: Neck, parent: Some(0), bone: [0.00, 1.00, 2.50], center: [0.00, 0.00, -1.50] },
    PartDef { glb: "deer/male/head.glb", role: Head, parent: Some(2), bone: [0.00, 3.50, -1.50], center: [0.00, 8.00, -1.00] },
    PartDef { glb: "deer/male/jaw.glb", role: Jaw, parent: Some(3), bone: [0.00, 2.50, -3.50], center: [0.00, 0.00, -1.50] },
    PartDef { glb: "deer/male/tail.glb", role: Tail, parent: Some(1), bone: [0.00, 0.50, 8.50], center: [0.00, 3.00, 0.50] },
    PartDef { glb: "deer/male/leg_fr.glb", role: LegFL, parent: Some(0), bone: [-3.50, -3.50, 4.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "deer/male/leg_fr.glb", role: LegFR, parent: Some(0), bone: [3.50, -3.50, 4.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "deer/male/leg_br.glb", role: LegBL, parent: Some(1), bone: [-3.00, -3.50, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "deer/male/leg_br.glb", role: LegBR, parent: Some(1), bone: [3.00, -3.50, 6.50], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "deer/male/foot_fr.glb", role: FootFL, parent: Some(6), bone: [0.50, -2.50, 0.50], center: [0.00, -4.00, -0.50] },
    PartDef { glb: "deer/male/foot_fr.glb", role: FootFR, parent: Some(7), bone: [-0.50, -2.50, 0.50], center: [0.00, -4.00, -0.50] },
    PartDef { glb: "deer/male/foot_br.glb", role: FootBL, parent: Some(8), bone: [1.00, -2.00, 0.50], center: [0.00, -3.50, -0.00] },
    PartDef { glb: "deer/male/foot_br.glb", role: FootBR, parent: Some(9), bone: [-1.00, -2.00, 0.50], center: [0.00, -3.50, -0.00] },
];

pub const CLOWNFISH: &[PartDef] = &[
    PartDef { glb: "clownfish/male/chest.glb", role: Body, parent: None, bone: [0.00, 5.00, -0.00], center: [0.00, 0.00, 1.00] },
    PartDef { glb: "clownfish/male/tail.glb", role: Tail, parent: Some(0), bone: [0.00, -0.50, 7.50], center: [0.00, 0.00, 2.50] },
    PartDef { glb: "clownfish/male/fin.glb", role: FinL, parent: Some(0), bone: [-2.00, 1.00, -0.50], center: [0.00, -2.00, 1.50] },
    PartDef { glb: "clownfish/male/fin.glb", role: FinR, parent: Some(0), bone: [2.00, 1.00, -0.50], center: [0.00, -2.00, 1.50] },
];

pub const PIRANHA: &[PartDef] = &[
    PartDef { glb: "piranha/male/chest.glb", role: Body, parent: None, bone: [0.00, 5.00, -0.00], center: [0.00, 0.00, -0.00] },
    PartDef { glb: "piranha/male/tail.glb", role: Tail, parent: Some(0), bone: [0.00, -0.50, 5.50], center: [0.00, 0.00, 2.50] },
    PartDef { glb: "piranha/male/fin.glb", role: FinL, parent: Some(0), bone: [-2.00, -0.50, -0.50], center: [0.00, -1.50, 2.00] },
    PartDef { glb: "piranha/male/fin.glb", role: FinR, parent: Some(0), bone: [2.00, -0.50, -0.50], center: [0.00, -1.50, 2.00] },
];
