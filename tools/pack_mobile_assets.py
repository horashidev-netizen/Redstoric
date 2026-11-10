#!/usr/bin/env python3
"""Tạo thư mục asset nhẹ cho APK Android: target/mobile_assets (từ assets/ đầy đủ ~950 MB).

Giữ: nhân vật (figure), NPC, sprite, object, item, 6 vũ khí khởi đầu, âm thanh môi trường + sfx
và vài bản nhạc nền. Bỏ: áo giáp, tàu lượn, hơn 700 vũ khí thừa, font/ngôn ngữ/ảnh nền của Veloren
và gần 250 MB nhạc nền.

Chạy từ thư mục gốc dự án:  python3 tools/pack_mobile_assets.py
"""
import os
import shutil
import sys

SRC = "assets"
DST = os.path.join("target", "mobile_assets")
GLB = "placeholders/glb"
AUDIO = "placeholders/voxygen/audio"

# Thư mục/file chép nguyên vẹn
COPY_DIRS = [
    f"{GLB}/figure", f"{GLB}/npc", f"{GLB}/sprite", f"{GLB}/object", f"{GLB}/item",
    f"{AUDIO}/ambience", f"{AUDIO}/sfx",
]
COPY_FILES = [
    f"{GLB}/not_found.glb", f"{GLB}/char_template.glb", f"{GLB}/particle.glb",
    f"{AUDIO}/null.ogg",
    # 6 vũ khí khởi đầu (khớp bảng WEAPONS trong crates/redstoric_core/src/weapons.rs)
    f"{GLB}/weapon/sword/starter.glb", f"{GLB}/weapon/axe/2haxe_rusty.glb",
    f"{GLB}/weapon/hammer/2hhammer_rusty.glb", f"{GLB}/weapon/bow/starter.glb",
    f"{GLB}/weapon/staff/firestaff_starter.glb", f"{GLB}/weapon/sceptre/wood-simple.glb",
]
# Nhạc nền (khớp MUSIC_* trong src/audio.rs)
MUSIC = [
    "calming_hills", "serene_meadows", "verdant_glades", "soaring_skies", "adventurous_soul",
    "moonbeams", "stargazing", "travel_by_night", "moonlit_canopy",
    "the_heavens_weep", "drifting_along",
]


def copy_file(rel):
    src = os.path.join(SRC, rel)
    if not os.path.isfile(src):
        print("  thiếu:", rel)
        return
    dst = os.path.join(DST, rel)
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    shutil.copy2(src, dst)


def main():
    if not os.path.isdir(SRC):
        sys.exit("Chạy từ thư mục gốc dự án (không thấy assets/)")
    if os.path.isdir(DST):
        shutil.rmtree(DST)
    for d in COPY_DIRS:
        src = os.path.join(SRC, d)
        if os.path.isdir(src):
            shutil.copytree(src, os.path.join(DST, d))
        else:
            print("  thiếu thư mục:", d)
    for f in COPY_FILES:
        copy_file(f)
    for m in MUSIC:
        copy_file(f"{AUDIO}/soundtrack/overworld/{m}.ogg")
    total = sum(os.path.getsize(os.path.join(r, f)) for r, _, fs in os.walk(DST) for f in fs)
    print(f"Xong: {DST} ({total / 1e6:.0f} MB)")


if __name__ == "__main__":
    main()
