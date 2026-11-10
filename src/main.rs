//! Điểm vào desktop. Toàn bộ game nằm trong thư viện `redstoric_game` (src/lib.rs) để Android
//! dùng chung: trên Android hệ thống nạp thư viện và gọi `android_main` do `#[bevy_main]` sinh ra.
fn main() {
    redstoric_game::main();
}
