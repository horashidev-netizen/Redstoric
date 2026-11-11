#!/usr/bin/env python3
"""Xuat danh sach cau thoai cua NPC de lam TTS.

Chay tu thu muc goc cua repo:  python tools/export_voice_lines.py
Tao `assets/audio/voice/voice_lines.csv` gom: id, nguoi noi, loai, cau can doc (text).
Dat file am thanh vao  assets/audio/voice/m/<id>.ogg  (giong nam)  va/hoac  .../f/<id>.ogg  (giong nu).
Dinh dang: .ogg (Vorbis), .wav hoac .mp3. Thieu mot giong thi game dung giong con lai; thieu ca hai thi chi hien chu.
`id` = FNV-1a 32 bit cua cau goc trong code (phai khop `redstoric_core::voice::voice_id`).
"""
import csv, os, re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
def read(rel):
    return open(os.path.join(ROOT, rel), encoding="utf-8").read()

def voice_id(text):
    h = 2166136261
    for b in text.encode("utf-8"):
        h = ((h ^ b) * 16777619) & 0xFFFFFFFF
    return f"{h:08x}"

STR = r'"((?:[^"\\]|\\.)*)"'
def unescape(s):
    return s.replace('\\"', '"').replace("\\'", "'").replace("\\\\", "\\")

rows = []  # (key, speaker, kind, spoken)

# 1. Cau tro chuyen theo nghe (phim E)
npc = read("crates/redstoric_common/src/npc.rs")
body = npc[npc.index("pub fn lines"):npc.index("pub fn goods")]
for m in re.finditer(r"Profession::(\w+) => &\[(.*?)\]", body, re.S):
    for s in re.findall(STR, m.group(2)):
        s = unescape(s)
        rows.append((s, m.group(1), "talk", s))

# 2. Loi giao nhiem vu (phim Q). Cho thay doi duoc doc bang cum tu chung.
quest = read("crates/redstoric_combat_ai/src/quest.rs")
owner = "Any"
for line in quest.splitlines():
    m = re.match(r"\s*(?:const|static) (\w+): .*Template", line)
    if m:
        owner = m.group(1).capitalize()
    m = re.search(r"\bt\((.+?), " + STR + ", " + STR + ", " + STR + r"\)", line)
    if not m:
        continue
    kind, intro = m.group(1), unescape(m.group(4))
    n = "three" if kind in ("Fetch", "Patrol") else "a few"
    place = "the next village" if kind == "Courier" else "the dungeon nearby"
    spoken = intro.replace("{n}", n).replace("{place}", place).replace("{boss}", "beast").replace("{item}", m.group(3))
    rows.append((intro, owner, "quest", spoken))
rows.append(("Thank you!", "Any", "quest", "Thank you!"))

# 3. Cau ho khi danh nhau / bao ve lang (bong bong thoai)
fight = read("crates/redstoric_combat_ai/src/villager_fight.rs")
for m in re.finditer(r"const (\w+_LINES): &\[&str\] = &\[(.*?)\];", fight, re.S):
    for s in re.findall(STR, m.group(2)):
        rows.append((unescape(s), "Any", m.group(1).replace("_LINES", "").lower(), unescape(s)))
shout = fight[fight.index("fn job_shout"):fight.index("fn pick_line")]
for m in re.finditer(r"P::(\w+) => " + STR, shout):
    rows.append((unescape(m.group(2)), m.group(1), "alarm", unescape(m.group(2))))

out_dir = os.path.join(ROOT, "assets", "audio", "voice")
os.makedirs(os.path.join(out_dir, "m"), exist_ok=True)
os.makedirs(os.path.join(out_dir, "f"), exist_ok=True)
seen, n = set(), 0
with open(os.path.join(out_dir, "voice_lines.csv"), "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["id", "speaker", "kind", "text", "have_m", "have_f"])
    for key, speaker, kind, spoken in rows:
        vid = voice_id(key)
        if vid in seen:
            continue
        seen.add(vid)
        have = lambda sex: "yes" if any(os.path.exists(os.path.join(out_dir, sex, f"{vid}.{e}")) for e in ("ogg", "wav", "mp3")) else ""
        w.writerow([vid, speaker, kind, spoken, have("m"), have("f")])
        n += 1
print(f"{n} cau -> assets/audio/voice/voice_lines.csv")
