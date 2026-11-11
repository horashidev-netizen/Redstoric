#!/usr/bin/env python3
"""Tao giong noi cho NPC bang Kokoro TTS (chay offline tren may ban, CPU cung duoc).

Cai dat (Python 3.10 - 3.12):
    pip install kokoro soundfile
    Windows: cai them espeak-ng (https://github.com/espeak-ng/espeak-ng/releases, file .msi)

Chay tu thu muc goc cua repo:
    python tools/export_voice_lines.py        # cap nhat danh sach cau
    python tools/make_npc_voices.py           # tao tat ca (bo qua file da co)
    python tools/make_npc_voices.py --only m_old --limit 5    # thu nhanh vai cau
    python tools/make_npc_voices.py --force   # tao lai tat ca

Ket qua: assets/audio/voice/<m|f>_<young|adult|old>/<id>.ogg
Game tu chon thu muc theo gioi tinh + do tuoi cua NPC.
"""
import argparse, csv, os, sys

# Giong Kokoro cho tung nhom (ten giong, toc do doc). Doi tuy y: danh sach giong o
# https://huggingface.co/hexgrad/Kokoro-82M/blob/main/VOICES.md
# Tien to a* = giong My, b* = giong Anh; *f_ = nu, *m_ = nam.
VOICES = {
    "m_young": ("am_puck", 1.06),
    "m_adult": ("am_michael", 1.00),
    "m_old":   ("bm_george", 0.88),
    "f_young": ("af_sky", 1.06),
    "f_adult": ("af_heart", 1.00),
    "f_old":   ("bf_emma", 0.90),
}
# Nghe chi co mot so do tuoi (khop `npc::age_of` trong game): khong can tao giong thua.
AGES_BY_SPEAKER = {
    "Noble": ("old",),
    "Scholar": ("adult", "old"),
    "Priest": ("adult", "old"),
}

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOICE_DIR = os.path.join(ROOT, "assets", "audio", "voice")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", help="chi tao mot nhom, vd m_old")
    ap.add_argument("--limit", type=int, default=0, help="chi tao N cau dau moi nhom (de nghe thu)")
    ap.add_argument("--force", action="store_true", help="tao lai ca file da co")
    ap.add_argument("--wav", action="store_true", help="xuat .wav thay vi .ogg")
    args = ap.parse_args()

    csv_path = os.path.join(VOICE_DIR, "voice_lines.csv")
    if not os.path.exists(csv_path):
        sys.exit("Chua co voice_lines.csv: chay  python tools/export_voice_lines.py  truoc.")
    lines = list(csv.DictReader(open(csv_path, encoding="utf-8")))

    import numpy as np
    import soundfile as sf
    from kokoro import KPipeline

    pipelines = {}
    def pipeline_for(voice):
        code = voice[0]  # 'a' = American English, 'b' = British English
        if code not in pipelines:
            pipelines[code] = KPipeline(lang_code=code)
        return pipelines[code]

    ext = "wav" if args.wav else "ogg"
    made = skipped = 0
    for group, (voice, speed) in VOICES.items():
        if args.only and group != args.only:
            continue
        age = group.split("_")[1]
        out_dir = os.path.join(VOICE_DIR, group)
        os.makedirs(out_dir, exist_ok=True)
        pipe = pipeline_for(voice)
        count = 0
        for row in lines:
            if age not in AGES_BY_SPEAKER.get(row["speaker"], ("young", "adult", "old")):
                continue
            if args.limit and count >= args.limit:
                break
            count += 1
            path = os.path.join(out_dir, f"{row['id']}.{ext}")
            if os.path.exists(path) and not args.force:
                skipped += 1
                continue
            chunks = [np.asarray(audio, dtype=np.float32) for _, _, audio in pipe(row["text"], voice=voice, speed=speed)]
            if not chunks:
                print("  (bo qua, khong co am thanh):", row["text"])
                continue
            audio = np.concatenate(chunks)
            # Dem 80 ms im lang o cuoi de cau khong bi cut dot ngot
            audio = np.concatenate([audio, np.zeros(int(0.08 * 24000), dtype=np.float32)])
            if ext == "ogg":
                sf.write(path, audio, 24000, format="OGG", subtype="VORBIS")
            else:
                sf.write(path, audio, 24000, subtype="PCM_16")
            made += 1
            print(f"[{group}] {row['id']}  {row['text'][:60]}")
    print(f"Xong: tao {made} file, bo qua {skipped} file da co.")

if __name__ == "__main__":
    main()
