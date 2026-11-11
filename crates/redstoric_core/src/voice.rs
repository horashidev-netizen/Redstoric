//! Giọng nói của NPC. Các system khác chỉ cần `VoiceQueue::say(...)` với đúng **câu chữ** NPC nói;
//! phần âm thanh (`src/audio.rs`) tìm file `assets/audio/voice/<m|f>_<young|adult|old>/<id>.ogg|wav|mp3` với
//! `id = voice_id(câu chữ)` và phát nếu có. Không có file thì NPC chỉ hiện chữ như cũ.
//!
//! Danh sách câu + id nằm trong `assets/audio/voice/voice_lines.csv` (tạo bằng `tools/export_voice_lines.py`).

use bevy::prelude::*;

pub struct VoiceLine {
    /// Người nói (băm từ tên): câu mới của cùng một người sẽ cắt câu cũ.
    pub owner: u32,
    pub female: bool,
    /// Độ tuổi: 0 trẻ, 1 trung niên, 2 già
    pub age: u8,
    pub pos: Vec3,
    /// Câu chữ gốc (với nhiệm vụ: câu mẫu còn nguyên `{n}`, `{place}`...).
    pub key: String,
}

#[derive(Resource, Default)]
pub struct VoiceQueue(pub Vec<VoiceLine>);

fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// Tên file (không đuôi) của một câu thoại: FNV-1a 32 bit của câu chữ, viết dạng hex 8 ký tự.
pub fn voice_id(text: &str) -> String {
    format!("{:08x}", fnv1a(text))
}

impl VoiceQueue {
    pub fn say(&mut self, name: &str, female: bool, age: u8, pos: Vec3, text: &str) {
        self.0.push(VoiceLine { owner: fnv1a(name), female, age, pos, key: text.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redstoric_common::npc::Profession;

    const JOBS: [Profession; 14] = [
        Profession::Farmer,
        Profession::Blacksmith,
        Profession::Alchemist,
        Profession::Merchant,
        Profession::Guard,
        Profession::Innkeeper,
        Profession::Hunter,
        Profession::Cook,
        Profession::Miner,
        Profession::Fisherman,
        Profession::Scholar,
        Profession::Priest,
        Profession::Bard,
        Profession::Noble,
    ];

    /// `assets/` nằm trong .gitignore nên bản clone mới không có file này: khi đó bỏ qua các test dùng nó.
    fn voice_csv() -> Option<String> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/audio/voice/voice_lines.csv");
        std::fs::read_to_string(path).ok()
    }

    fn csv_has_id(csv: &str, id: &str) -> bool {
        let prefix = format!("{id},");
        csv.lines().any(|l| l.starts_with(&prefix))
    }

    #[test]
    fn fnv1a_known_vectors() {
        // Vector chuẩn của FNV-1a 32 bit; `tools/export_voice_lines.py` phải ra cùng kết quả.
        assert_eq!(voice_id(""), "811c9dc5");
        assert_eq!(voice_id("a"), "e40c292c");
        assert_eq!(voice_id("foobar"), "bf9cf968");
    }

    #[test]
    fn voice_id_is_eight_lowercase_hex_digits() {
        for text in ["", "Hello", "Good day, traveller!", "Thank you!"] {
            let id = voice_id(text);
            assert_eq!(id.len(), 8, "id {id:?} của {text:?} không đủ 8 ký tự");
            assert!(id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)), "id {id:?} không phải hex thường");
        }
    }

    #[test]
    fn voice_id_depends_on_exact_text() {
        assert_ne!(voice_id("Thank you!"), voice_id("Thank you"));
        assert_ne!(voice_id("Thank you!"), voice_id("thank you!"));
        assert_eq!(voice_id("Thank you!"), voice_id("Thank you!"));
    }

    #[test]
    fn say_queues_original_text_with_stable_owner() {
        let mut q = VoiceQueue::default();
        q.say("Bob", false, 2, Vec3::ZERO, "Hello there");
        q.say("Bob", false, 2, Vec3::ZERO, "Another line");
        q.say("Alice", true, 0, Vec3::ONE, "Hi");
        assert_eq!(q.0.len(), 3);
        assert_eq!(q.0[0].key, "Hello there");
        assert_eq!(q.0[0].owner, q.0[1].owner, "cùng tên phải cùng owner để câu mới cắt câu cũ");
        assert_ne!(q.0[0].owner, q.0[2].owner);
        assert!(q.0[2].female);
        assert_eq!(q.0[2].age, 0);
    }

    #[test]
    fn profession_talk_lines_are_in_voice_csv() {
        let Some(csv) = voice_csv() else { return };
        for job in JOBS {
            for line in job.lines() {
                let id = voice_id(line);
                assert!(
                    csv_has_id(&csv, &id),
                    "câu {line:?} của {job:?} (id {id}) chưa có trong voice_lines.csv; chạy: python tools/export_voice_lines.py"
                );
            }
        }
    }
}
