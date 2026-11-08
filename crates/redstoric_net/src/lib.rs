//! Mạng nhiều người chơi cho Redstoric, chạy trên QUIC (qua `bevy_quinnet`).
//!
//! Mô hình giống game online kiểu Genshin: **server chuyên dụng giữ quyền quyết định**
//! (máu người chơi, AI và máu boss, kiểm tra tốc độ di chuyển, kiểm tra tầm đánh), client chỉ
//! gửi vị trí và ý định đánh rồi hiển thị trạng thái server trả về.
//!
//! * `protocol`: các thông điệp client <-> server.
//! * `server`: `NetServerPlugin`, dùng trong binary `redstoric_server` (không cửa sổ).
//! * `client`: `NetClientPlugin`, thêm vào game chính; chỉ kết nối khi có `--connect`.

pub mod client;
pub mod protocol;
pub mod server;

pub use client::{NetClientPlugin, NetConfig, NetStatus};
pub use protocol::{DEFAULT_PORT, PROTOCOL_VERSION};
pub use server::NetServerPlugin;
