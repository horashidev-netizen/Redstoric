//! Mạng nhiều người chơi cho Redstoric, chạy trên QUIC (qua `bevy_quinnet`).
//!
//! Mô hình giống game online kiểu Genshin: **server chuyên dụng giữ quyền quyết định**
//! (máu người chơi, AI và máu boss, kiểm tra tốc độ di chuyển, kiểm tra tầm đánh), client chỉ
//! gửi vị trí và ý định đánh rồi hiển thị trạng thái server trả về.
//!
//! * `protocol`: (crate `redstoric_protocol`) các thông điệp client <-> server.
//! * `server`: đã tách sang crate `redstoric_server` (không cửa sổ, chạy được trên điện thoại).
//! * `progress`: lưu/khôi phục ví, túi đồ, uy tín qua server (dữ liệu nằm ở crate `redstoric_store`).
//! * `client`: `NetClientPlugin`, thêm vào game chính; chỉ kết nối khi có `--connect`.

pub mod chat;
pub mod client;
pub mod progress;
pub use redstoric_protocol as protocol;

pub use client::{NetClientPlugin, NetConfig, NetStatus};
pub use protocol::{DEFAULT_PORT, PROTOCOL_VERSION};
