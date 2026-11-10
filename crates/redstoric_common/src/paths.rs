//! Đường dẫn file dữ liệu người dùng (cài đặt đồ hoạ, ngoại hình...).
//! Desktop: thư mục hiện tại (hoặc `REDSTORIC_DATA_DIR`). Android: thư mục riêng của ứng dụng,
//! vì thư mục hiện tại trên điện thoại không ghi được.
use std::path::PathBuf;

/// Gói ứng dụng Android, khớp `package.metadata.android.package` trong `Cargo.toml` gốc.
pub const ANDROID_PACKAGE: &str = "org.redstoric.game";

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("REDSTORIC_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(target_os = "android") {
        return PathBuf::from(format!("/data/data/{ANDROID_PACKAGE}/files"));
    }
    PathBuf::from(".")
}

pub fn data_path(name: &str) -> PathBuf {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}
