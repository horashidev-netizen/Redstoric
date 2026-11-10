//! Thông điệp trao đổi giữa client và server (mã hoá bincode, truyền bằng QUIC).
//!
//! Hai kênh:
//! * `CH_RELIABLE` (0): có thứ tự, đảm bảo đến nơi. Dùng cho bắt tay, vào/ra, đòn đánh, dịch chuyển.
//! * `CH_UNRELIABLE` (1): datagram, có thể mất. Dùng cho vị trí / snapshot 20 lần mỗi giây,
//!   gói cũ bị mất cũng không sao vì gói mới sẽ thay thế.

use bevy_quinnet::shared::channels::{ChannelConfig, SendChannelsConfiguration};
use serde::{Deserialize, Serialize};

/// Tăng khi đổi định dạng thông điệp; client và server khác phiên bản sẽ bị từ chối.
pub const PROTOCOL_VERSION: u32 = 2;
pub const DEFAULT_PORT: u16 = 6000;
pub const MAX_PLAYERS: usize = 8;
/// Giới hạn kích thước chuỗi ngoại hình (JSON) client gửi lên.
pub const MAX_APPEARANCE_BYTES: usize = 1024;
pub const MAX_NAME_CHARS: usize = 20;

pub const CH_RELIABLE: u8 = 0;
pub const CH_UNRELIABLE: u8 = 1;

/// Điểm hồi sinh khi người chơi gục.
pub const RESPAWN_POINT: [f32; 3] = redstoric_common::rules::player::RESPAWN_POINT;
pub const PLAYER_MAX_HP: f32 = redstoric_common::rules::player::MAX_HP;

/// Cấu hình kênh gửi dùng chung cho client và server (thứ tự quyết định id kênh: 0 và 1).
pub fn channels() -> SendChannelsConfiguration {
    SendChannelsConfiguration::from_configs(vec![
        ChannelConfig::default_ordered_reliable(),
        ChannelConfig::default_unreliable(),
    ])
    .expect("cấu hình kênh hợp lệ")
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct PlayerState {
    pub pos: [f32; 3],
    /// Góc quay quanh trục Y (radian).
    pub yaw: f32,
    pub moving: bool,
    /// Chức nghiệp (chỉ số trong `redstoric_core::weapons::WEAPONS`)
    pub class: u8,
    /// Đang trong nhát đánh
    pub attacking: bool,
}

impl PlayerState {
    pub fn is_valid(&self) -> bool {
        self.pos.iter().all(|v| v.is_finite() && v.abs() < 1.0e6) && self.yaw.is_finite() && (self.class as usize) < redstoric_core::weapons::WEAPONS.len()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMsg {
    /// Gói đầu tiên sau khi kết nối.
    Hello { version: u32, name: String, appearance: String },
    /// Đổi ngoại hình (JSON của `CharacterAppearance`).
    Appearance(String),
    /// Vị trí hiện tại (kênh không tin cậy).
    State(PlayerState),
    /// Chém trúng boss có khoá `boss` (toạ độ nguyên điểm spawn). Server tự kiểm tra tầm đánh.
    Attack { boss: [i32; 2], class: u8 },
    /// Đạn / hiệu ứng vũ khí vừa tạo (0 tên, 1 cầu lửa, 2 tia xanh, 3 nện búa, 4 xoay rìu); server phát lại cho người ở gần.
    Fire { kind: u8, pos: [f32; 3], dir: [f32; 3] },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossPhase {
    Idle,
    Chase,
    Windup,
    Recover,
    Dying,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct BossNet {
    pub key: [i32; 2],
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub phase: BossPhase,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMsg {
    Welcome { your_id: u64, seed: u32 },
    Rejected { reason: String },
    PlayerJoined { id: u64, name: String, appearance: String },
    PlayerLeft { id: u64 },
    PlayerAppearance { id: u64, appearance: String },
    /// Server bắt client về vị trí này (di chuyển bất hợp lệ hoặc hồi sinh).
    Teleport { pos: [f32; 3] },
    /// Giờ trong ngày và cường độ mưa mục tiêu do server quyết định.
    Env { hour: f32, rain: f32 },
    /// Hiệu ứng vũ khí của người chơi khác.
    Fx { id: u64, kind: u8, pos: [f32; 3], dir: [f32; 3] },
    /// Boss đã bị hạ vĩnh viễn trong phiên server này.
    BossDown { boss: [i32; 2] },
    /// Trạng thái thế giới ~20 lần/giây (kênh không tin cậy).
    Snapshot {
        players: Vec<(u64, PlayerState)>,
        bosses: Vec<BossNet>,
        hp: f32,
        max_hp: f32,
    },
}
