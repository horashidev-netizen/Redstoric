//! Lưu trữ phía server: tài khoản và dữ liệu người chơi / thế giới, ghi ra file JSON.
//!
//! * **Tài khoản** = một khoá bí mật ngẫu nhiên do client tự sinh và giữ (`account_key.txt`).
//!   Server chỉ lưu `sha256(khoá)`, nên lộ thư mục dữ liệu cũng không lộ được khoá.
//! * Mỗi người chơi một file `players/<account_id>.json`, thế giới một file `world.json`.
//! * Ghi nguyên tử (ghi file tạm rồi `rename`) nên mất điện giữa chừng không làm hỏng dữ liệu cũ.
//!
//! Crate thuần Rust, không phụ thuộc Bevy: test chạy trong vài giây.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Độ dài khoá tài khoản (ký tự hex).
pub const KEY_LEN: usize = 64;
/// Giới hạn kích thước blob tiến độ client gửi lên.
pub const MAX_PROGRESS_BYTES: usize = 4096;

/// Kiểm tra khoá hợp lệ: đúng `KEY_LEN` ký tự hex.
pub fn is_valid_key(key: &str) -> bool {
    key.len() == KEY_LEN && key.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Mã tài khoản = 32 ký tự hex đầu của sha256(khoá). `None` nếu khoá sai dạng.
pub fn account_id(key: &str) -> Option<String> {
    if !is_valid_key(key) {
        return None;
    }
    let digest = Sha256::digest(key.to_ascii_lowercase().as_bytes());
    Some(digest.iter().take(16).map(|b| format!("{b:02x}")).collect())
}

/// Sinh khoá mới. Dùng `RandomState` của std (được hệ điều hành gieo ngẫu nhiên) trộn thêm thời gian.
pub fn generate_key() -> String {
    let mut hasher = Sha256::new();
    for _ in 0..4 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(0x5245_4453);
        hasher.update(h.finish().to_le_bytes());
    }
    if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.update(t.as_nanos().to_le_bytes());
    }
    hasher.update(std::process::id().to_le_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Dữ liệu một nhân vật.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerRecord {
    pub name: String,
    /// JSON ngoại hình (`CharacterAppearance`), server không diễn giải.
    pub appearance: String,
    pub pos: [f32; 3],
    pub class: u8,
    pub hp: f32,
    /// Blob JSON tiến độ (ví, túi đồ, nhiệm vụ, uy tín) do client gửi, server chưa kiểm chứng.
    pub progress: String,
    /// Số lần đăng nhập và thời điểm (giây Unix) lần cuối.
    pub logins: u32,
    pub last_seen: u64,
}

/// Dữ liệu của cả thế giới.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WorldRecord {
    /// Khoá các boss đã bị hạ vĩnh viễn.
    pub defeated: Vec<[i32; 2]>,
    pub hour: f32,
}

impl Default for WorldRecord {
    fn default() -> Self {
        Self { defeated: Vec::new(), hour: 9.0 }
    }
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Mở (và tạo nếu chưa có) thư mục dữ liệu.
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(dir.join("players"))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn player_path(&self, account: &str) -> Option<PathBuf> {
        // Chỉ nhận mã do `account_id` sinh ra, chặn mọi ý đồ đi ra ngoài thư mục.
        (account.len() == 32 && account.bytes().all(|b| b.is_ascii_hexdigit())).then(|| self.dir.join("players").join(format!("{account}.json")))
    }

    pub fn load_player(&self, account: &str) -> Option<PlayerRecord> {
        read_json(&self.player_path(account)?)
    }

    pub fn save_player(&self, account: &str, record: &PlayerRecord) -> io::Result<()> {
        let path = self.player_path(account).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "mã tài khoản sai"))?;
        write_json_atomic(&path, record)
    }

    pub fn load_world(&self) -> WorldRecord {
        read_json(&self.dir.join("world.json")).unwrap_or_default()
    }

    pub fn save_world(&self, world: &WorldRecord) -> io::Result<()> {
        write_json_atomic(&self.dir.join("world.json"), world)
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(&tmp, path)
}

/// Thời điểm hiện tại (giây Unix).
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("redstoric_store_{tag}_{}_{}", std::process::id(), generate_key()[..8].to_string()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sample() -> PlayerRecord {
        PlayerRecord {
            name: "Tam".into(),
            appearance: "{\"x\":1}".into(),
            pos: [1.0, 25.0, -3.5],
            class: 2,
            hp: 80.0,
            progress: "{\"wallet\":12}".into(),
            logins: 3,
            last_seen: 1,
        }
    }

    #[test]
    fn generated_keys_are_valid_and_distinct() {
        let (a, b) = (generate_key(), generate_key());
        assert!(is_valid_key(&a) && is_valid_key(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn account_id_is_stable_and_hides_key() {
        let key = generate_key();
        let id = account_id(&key).unwrap();
        assert_eq!(id.len(), 32);
        assert_eq!(account_id(&key).unwrap(), id);
        assert_eq!(account_id(&key.to_ascii_uppercase()).unwrap(), id);
        assert!(!key.contains(&id));
        assert_ne!(account_id(&generate_key()).unwrap(), id);
    }

    #[test]
    fn bad_keys_are_rejected() {
        assert!(account_id("").is_none());
        assert!(account_id("abc").is_none());
        assert!(account_id(&"g".repeat(KEY_LEN)).is_none());
        assert!(account_id(&format!("../{}", "a".repeat(KEY_LEN - 3))).is_none());
    }

    #[test]
    fn player_round_trip_and_unknown_is_none() {
        let dir = temp_dir("player");
        let store = Store::open(&dir).unwrap();
        let id = account_id(&generate_key()).unwrap();
        assert!(store.load_player(&id).is_none());
        store.save_player(&id, &sample()).unwrap();
        assert_eq!(store.load_player(&id).unwrap(), sample());
        let mut updated = sample();
        updated.hp = 10.0;
        store.save_player(&id, &updated).unwrap();
        assert_eq!(store.load_player(&id).unwrap().hp, 10.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn path_traversal_ids_are_refused() {
        let dir = temp_dir("trav");
        let store = Store::open(&dir).unwrap();
        assert!(store.load_player("../world").is_none());
        assert!(store.save_player("../../evil", &sample()).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn world_defaults_then_round_trips() {
        let dir = temp_dir("world");
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.load_world(), WorldRecord::default());
        let w = WorldRecord { defeated: vec![[10, -20], [5, 5]], hour: 17.5 };
        store.save_world(&w).unwrap();
        assert_eq!(store.load_world(), w);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_file_reads_as_missing() {
        let dir = temp_dir("corrupt");
        let store = Store::open(&dir).unwrap();
        let id = account_id(&generate_key()).unwrap();
        std::fs::write(dir.join("players").join(format!("{id}.json")), b"{not json").unwrap();
        assert!(store.load_player(&id).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
