#!/usr/bin/env python3
"""Sinh crates/redstoric_combat_ai/src/boss_models.rs: bố cục tư thế đứng của các boss người khổng lồ (biped_large Veloren).

Mỗi loài có 10 mảnh glb (head, torso_upper, torso_lower, shoulder_l/r, hand_l/r, leg_l/r, foot_l/r) trong
assets/placeholders/glb/npc/<loài>/male/. Ta đọc hộp bao (accessor POSITION min/max) của từng mảnh rồi xếp chồng:
chân -> đùi -> thân dưới -> thân trên -> đầu; vai và tay treo hai bên thân trên. Chạy lại khi thêm loài mới:

    python tools/gen_boss_models.py
"""
import json
import os
import struct

ROOT = "assets/placeholders/glb/npc"
OUT = "crates/redstoric_combat_ai/src/boss_models.rs"
SPECIES = [("ogre", "OGRE"), ("minotaur", "MINOTAUR"), ("yeti", "YETI"), ("troll_mountain", "TROLL_MOUNTAIN"), ("cyclops", "CYCLOPS")]
PARTS = ["head", "torso_upper", "torso_lower", "shoulder_l", "shoulder_r", "hand_l", "hand_r", "leg_l", "leg_r", "foot_l", "foot_r"]


def bounds(path):
    d = open(path, "rb").read()
    n = struct.unpack("<I", d[12:16])[0]
    j = json.loads(d[20:20 + n])
    mn = [1e9] * 3
    mx = [-1e9] * 3
    for m in j["meshes"]:
        for pr in m["primitives"]:
            a = j["accessors"][pr["attributes"]["POSITION"]]
            for i in range(3):
                mn[i] = min(mn[i], a["min"][i])
                mx[i] = max(mx[i], a["max"][i])
    c = [(mn[i] + mx[i]) / 2 for i in range(3)]
    s = [mx[i] - mn[i] for i in range(3)]
    return c, s


def f(v):
    return f"{v:.2f}"


def build(sp):
    b = {}
    for p in PARTS:
        path = f"{ROOT}/{sp}/male/{p}.glb"
        if not os.path.exists(path):
            return None
        b[p] = bounds(path)
    sz = {p: b[p][1] for p in PARTS}
    cen = {p: b[p][0] for p in PARTS}
    fy, ly = sz["foot_l"][1], sz["leg_l"][1]
    tly, tuy, hy = sz["torso_lower"][1], sz["torso_upper"][1], sz["head"][1]
    leg_x = sz["torso_lower"][0] / 4.0
    ov = 1.0
    foot_c = fy / 2
    leg_top = fy + ly - ov
    leg_c = fy + ly / 2 - ov
    tl_c = fy + ly + tly / 2 - 2 * ov
    tl_top = fy + ly + tly - 2 * ov
    tu_c = tl_top + tuy / 2 - 2 * ov
    tu_top = tl_top + tuy - 2 * ov
    head_c = tu_top + hy / 2 - 3 * ov
    arm_x = sz["torso_upper"][0] / 2 + sz["shoulder_l"][0] / 2 - 1.5
    sh_c = tu_top - sz["shoulder_l"][1] / 2 - ov
    hand_c = sh_c - sz["shoulder_l"][1] / 2 - sz["hand_l"][1] / 2 + 2.5 * ov
    pivots = {
        "ARM_L": (-arm_x, tu_top - 2 * ov, 0.0),
        "ARM_R": (arm_x, tu_top - 2 * ov, 0.0),
        "LEG_L": (-leg_x, leg_top, 0.0),
        "LEG_R": (leg_x, leg_top, 0.0),
    }
    # (glb, limb, desired centre in model space)
    placed = [
        ("torso_lower", "Body", (0.0, tl_c, 0.0)),
        ("torso_upper", "Body", (0.0, tu_c, 0.0)),
        ("head", "Body", (0.0, head_c, -1.0)),
        ("shoulder_l", "ArmL", (-arm_x, sh_c, 0.0)),
        ("hand_l", "ArmL", (-arm_x, hand_c, -2.0)),
        ("shoulder_r", "ArmR", (arm_x, sh_c, 0.0)),
        ("hand_r", "ArmR", (arm_x, hand_c, -2.0)),
        ("leg_l", "LegL", (-leg_x, leg_c, 0.0)),
        ("foot_l", "LegL", (-leg_x, foot_c, -1.0)),
        ("leg_r", "LegR", (leg_x, leg_c, 0.0)),
        ("foot_r", "LegR", (leg_x, foot_c, -1.0)),
    ]
    height = tu_top + hy - 3 * ov
    rows = []
    for name, limb, want in placed:
        piv = (0.0, 0.0, 0.0)
        if limb != "Body":
            piv = pivots[{"ArmL": "ARM_L", "ArmR": "ARM_R", "LegL": "LEG_L", "LegR": "LEG_R"}[limb]]
        c = cen[name]
        pos = [want[i] - piv[i] - c[i] for i in range(3)]
        rows.append(f'        BossPart {{ glb: "{sp}/male/{name}.glb", limb: Limb::{limb}, pos: [{f(pos[0])}, {f(pos[1])}, {f(pos[2])}] }},')
    pv = ", ".join("[%s, %s, %s]" % tuple(f(x) for x in pivots[k]) for k in ["ARM_L", "ARM_R", "LEG_L", "LEG_R"])
    return rows, pv, height


def main():
    out = [
        "//! Tự sinh bởi tools/gen_boss_models.py từ hộp bao các mảnh glb của Veloren. KHÔNG sửa tay.",
        "//! Đơn vị voxel của glb; nhân với `MODEL_SCALE * BossKind::scale()` khi dựng.",
        "",
        "#[derive(Clone, Copy, PartialEq, Eq, Debug)]",
        "pub enum Limb {",
        "    Body,",
        "    ArmL,",
        "    ArmR,",
        "    LegL,",
        "    LegR,",
        "}",
        "",
        "pub struct BossPart {",
        "    /// Đường dẫn trong `assets/placeholders/glb/npc/`.",
        "    pub glb: &'static str,",
        "    pub limb: Limb,",
        "    /// Vị trí scene so với khớp của chi (hoặc gốc model với `Limb::Body`).",
        "    pub pos: [f32; 3],",
        "}",
        "",
        "pub struct BossModel {",
        "    pub parts: &'static [BossPart],",
        "    /// Vị trí khớp: tay trái, tay phải, chân trái, chân phải.",
        "    pub pivots: [[f32; 3]; 4],",
        "    /// Chiều cao model (voxel).",
        "    pub height: f32,",
        "}",
        "",
    ]
    for sp, const in SPECIES:
        r = build(sp)
        if r is None:
            print("thiếu mảnh cho", sp)
            continue
        rows, pv, height = r
        out.append(f"pub const {const}: BossModel = BossModel {{")
        out.append("    parts: &[")
        out.extend(rows)
        out.append("    ],")
        out.append(f"    pivots: [{pv}],")
        out.append(f"    height: {height:.2f},")
        out.append("};")
        out.append("")
    open(OUT, "w", encoding="utf-8").write("\n".join(out))
    print("ghi", OUT)


if __name__ == "__main__":
    main()
