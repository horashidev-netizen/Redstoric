#!/usr/bin/env python3
"""Tao giong noi cho NPC bang ElevenLabs (tra phi, giong tu nhien hon Kokoro). Chi can Python, khong cai them gi.

1. Tao tai khoan + API key tai https://elevenlabs.io  (Settings -> API Keys).
2. Dat key vao bien moi truong (PowerShell):   $env:ELEVENLABS_API_KEY = "sk_..."
3. Chay tu thu muc goc cua repo:
     python tools/make_npc_voices_elevenlabs.py --list-voices      # xem giong trong tai khoan (ten, gioi, tuoi, id)
     python tools/make_npc_voices_elevenlabs.py --dry-run          # dem so ky tu se bi tinh tien, khong goi API
     python tools/make_npc_voices_elevenlabs.py --only m_old --limit 3   # nghe thu 3 cau
     python tools/make_npc_voices_elevenlabs.py                    # tao tat ca (bo qua file da co)

Ket qua: assets/audio/voice/<m|f>_<young|adult|old>/<id>.mp3  (game doc duoc .mp3).
File .ogg cu cua Kokoro trong cung thu muc nen xoa di, neu khong game co the phat ban cu.
"""
import argparse, csv, json, os, sys, time, urllib.request, urllib.error

# Dan voice_id vao day (lay tu --list-voices hoac Voice Library tren web). De trong ("") thi script
# tu chon giong dau tien trong tai khoan co nhan gioi tinh + do tuoi phu hop.
VOICE_IDS = {
    "m_young": "",
    "m_adult": "",
    "m_old":   "",
    "f_young": "",
    "f_adult": "",
    "f_old":   "",
}
# Model: "eleven_v3" dien cam nhat; "eleven_multilingual_v2" on dinh hon. Doi neu tai khoan co model moi hon.
MODEL_ID = "eleven_v3"
OUTPUT_FORMAT = "mp3_44100_128"
AGES_BY_SPEAKER = {"Noble": ("old",), "Scholar": ("adult", "old"), "Priest": ("adult", "old")}
AGE_LABELS = {"young": ("young",), "adult": ("middle_aged", "middle aged", "middle-aged"), "old": ("old",)}

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VOICE_DIR = os.path.join(ROOT, "assets", "audio", "voice")
API = "https://api.elevenlabs.io/v1"

def request(key, path, body=None):
    headers = {"xi-api-key": key}
    data = None
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(API + path, data=data, headers=headers)
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()

def list_voices(key):
    return json.loads(request(key, "/voices")).get("voices", [])

def auto_pick(voices, group):
    sex, age = group.split("_")
    gender = "male" if sex == "m" else "female"
    for v in voices:
        labels = {k: str(val).lower() for k, val in (v.get("labels") or {}).items()}
        if labels.get("gender") == gender and labels.get("age") in AGE_LABELS[age]:
            return v["voice_id"], v.get("name", "?")
    return None, None

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--list-voices", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--only")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()

    key = os.environ.get("ELEVENLABS_API_KEY", "")
    if not key and not args.dry_run:
        sys.exit("Thieu ELEVENLABS_API_KEY (xem huong dan o dau file).")

    if args.list_voices:
        for v in list_voices(key):
            l = v.get("labels") or {}
            print(f"{v['voice_id']}  {v.get('name','?'):<24} {l.get('gender','?'):<7} {l.get('age','?'):<12} {l.get('accent','')}")
        return

    csv_path = os.path.join(VOICE_DIR, "voice_lines.csv")
    if not os.path.exists(csv_path):
        sys.exit("Chua co voice_lines.csv: chay  python tools/export_voice_lines.py  truoc.")
    lines = list(csv.DictReader(open(csv_path, encoding="utf-8")))

    voices = [] if args.dry_run else list_voices(key)
    made = skipped = chars = 0
    for group in VOICE_IDS:
        if args.only and group != args.only:
            continue
        age = group.split("_")[1]
        voice_id, name = VOICE_IDS[group], "(cau hinh tay)"
        if not voice_id and not args.dry_run:
            voice_id, name = auto_pick(voices, group)
            if not voice_id:
                print(f"[{group}] khong tim thay giong phu hop trong tai khoan: them giong vao My Voices hoac dien VOICE_IDS. Bo qua.")
                continue
        if not args.dry_run:
            print(f"[{group}] dung giong {name} ({voice_id})")
        out_dir = os.path.join(VOICE_DIR, group)
        os.makedirs(out_dir, exist_ok=True)
        count = 0
        for row in lines:
            if age not in AGES_BY_SPEAKER.get(row["speaker"], ("young", "adult", "old")):
                continue
            if args.limit and count >= args.limit:
                break
            count += 1
            path = os.path.join(out_dir, f"{row['id']}.mp3")
            if os.path.exists(path) and not args.force:
                skipped += 1
                continue
            chars += len(row["text"])
            if args.dry_run:
                made += 1
                continue
            body = {"text": row["text"], "model_id": MODEL_ID}
            for attempt in range(4):
                try:
                    audio = request(key, f"/text-to-speech/{voice_id}?output_format={OUTPUT_FORMAT}", body)
                    break
                except urllib.error.HTTPError as e:
                    msg = e.read().decode("utf-8", "replace")[:300]
                    if e.code == 429 and attempt < 3:  # qua nhanh: cho roi thu lai
                        time.sleep(3 * (attempt + 1))
                        continue
                    sys.exit(f"Loi API {e.code} o cau {row['id']}: {msg}\nDa tao {made} file; chay lai lenh de tiep tuc.")
            with open(path, "wb") as f:
                f.write(audio)
            made += 1
            print(f"  {row['id']}  {row['text'][:60]}")
    verb = "se tao" if args.dry_run else "da tao"
    print(f"{verb} {made} file ({chars} ky tu bi tinh tien), bo qua {skipped} file da co.")

if __name__ == "__main__":
    main()
