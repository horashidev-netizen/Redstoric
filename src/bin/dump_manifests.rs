//! Sinh danh mục mảnh nhân vật và sprite thành JSON để nhúng vào bản Android (APK không quét
//! được thư mục asset như desktop).
//!
//! Chạy từ thư mục gốc dự án, trên máy có đủ `assets/`:
//!   cargo run --bin dump_manifests
//! rồi build lại bản Android (xem README, mục "Chạy trên điện thoại").
fn main() {
    let dir = std::path::Path::new("assets_manifest");
    std::fs::create_dir_all(dir).expect("không tạo được assets_manifest/");
    let figures = redstoric_core::character_creator::dump_catalog_json();
    let sprites = redstoric_common::sprites::dump_manifest_json();
    std::fs::write(dir.join("figure_catalog.json"), &figures).expect("ghi figure_catalog.json");
    std::fs::write(dir.join("sprite_catalog.json"), &sprites).expect("ghi sprite_catalog.json");
    println!("Đã ghi assets_manifest/figure_catalog.json ({} byte) và sprite_catalog.json ({} byte)", figures.len(), sprites.len());
}
