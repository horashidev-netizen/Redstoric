//! Chat chữ giữa người chơi (tiếng Anh / ASCII).
//!
//! * **Enter** mở ô chat, gõ rồi **Enter** để gửi, **Esc** để huỷ. Trong lúc gõ, bàn phím không điều khiển nhân vật
//!   (`ButtonInput<KeyCode>` bị xoá ở `PreUpdate`, nên không system nào khác thấy phím bấm).
//! * Online: tin nhắn gửi lên server (`ClientMsg::Chat`), server lọc + giới hạn tần suất rồi phát cho mọi người
//!   (`ServerMsg::Chat`). Offline: chỉ hiện cục bộ.
//! * Chỉ nhận ký tự ASCII in được (chữ cái tiếng Anh, số, dấu câu) để mọi máy hiển thị giống nhau.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonState, InputSystems};
use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use redstoric_core::ChatFocus;

use crate::client::{NetClient, NetConfig, NetStatus};
use crate::protocol::{ClientMsg, CH_RELIABLE, MAX_CHAT_CHARS};

/// Số dòng giữ lại / hiển thị.
const MAX_LINES: usize = 40;
const SHOWN_OPEN: usize = 10;
const SHOWN_IDLE: usize = 5;
/// Dòng chat tự ẩn sau từng này giây (khi không mở ô chat).
const LINE_TTL: f32 = 12.0;

#[derive(Resource, Default)]
pub struct ChatLog {
    /// (nội dung, tuổi tính bằng giây)
    lines: Vec<(String, f32)>,
    input: String,
    pub open: bool,
    outbox: Vec<String>,
}

impl ChatLog {
    pub fn push(&mut self, line: String) {
        self.lines.push((line, 0.0));
        if self.lines.len() > MAX_LINES {
            let extra = self.lines.len() - MAX_LINES;
            self.lines.drain(0..extra);
        }
    }
}

pub use crate::protocol::sanitize_chat;

#[derive(Component)]
struct ChatRoot;
#[derive(Component)]
struct ChatLogText;
#[derive(Component)]
struct ChatInputText;

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatLog>()
            .add_systems(Startup, setup_chat_ui)
            .add_systems(PreUpdate, chat_input_system.after(InputSystems))
            .add_systems(Update, (chat_send_system, chat_ui_system).chain().in_set(redstoric_core::GameSet::Ui));
    }
}

fn setup_chat_ui(mut commands: Commands) {
    let font = |size: f32| TextFont { font_size: bevy::text::FontSize::Px(size), ..default() };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(48.0),
                left: Val::Px(10.0),
                width: Val::Px(440.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChatRoot,
        ))
        .with_children(|p| {
            p.spawn((Text::new(""), font(15.0), TextColor(Color::srgb(0.95, 0.95, 0.9)), ChatLogText));
            p.spawn((Text::new(""), font(15.0), TextColor(Color::srgb(1.0, 0.92, 0.55)), ChatInputText));
        });
}

/// Đọc phím gõ. Chạy ngay sau khi Bevy cập nhật bàn phím, rồi xoá trạng thái phím nếu đang chat.
fn chat_input_system(
    mut events: MessageReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut chat: ResMut<ChatLog>,
    mut focus: ResMut<ChatFocus>,
) {
    let mut swallow = chat.open;
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        if !chat.open {
            if ev.key_code == KeyCode::Enter && !ev.repeat {
                chat.open = true;
                chat.input.clear();
                swallow = true;
            }
            continue;
        }
        match &ev.logical_key {
            Key::Enter => {
                let text = sanitize_chat(&chat.input);
                if !text.is_empty() {
                    chat.outbox.push(text);
                }
                chat.input.clear();
                chat.open = false;
            }
            Key::Escape => {
                chat.input.clear();
                chat.open = false;
            }
            Key::Backspace => {
                chat.input.pop();
            }
            Key::Space => {
                if chat.input.len() < MAX_CHAT_CHARS {
                    chat.input.push(' ');
                }
            }
            Key::Character(s) => {
                for c in s.chars() {
                    if c.is_ascii_graphic() && chat.input.len() < MAX_CHAT_CHARS {
                        chat.input.push(c);
                    }
                }
            }
            _ => {}
        }
    }
    if swallow {
        // Không cho phím gõ chat lọt xuống điều khiển nhân vật / menu
        keys.reset_all();
    }
    if focus.0 != chat.open {
        focus.0 = chat.open;
    }
}

/// Gửi tin đang chờ lên server; offline thì hiện tại chỗ.
fn chat_send_system(
    mut chat: ResMut<ChatLog>,
    mut client: Option<ResMut<QuinnetClient>>,
    net: Res<NetClient>,
    config: Res<NetConfig>,
) {
    if chat.outbox.is_empty() {
        return;
    }
    let out = std::mem::take(&mut chat.outbox);
    for text in out {
        let mut sent = false;
        if matches!(net.status, NetStatus::Online) {
            if let Some(conn) = client.as_mut().and_then(|c| c.get_connection_mut()) {
                conn.try_send_message_on(CH_RELIABLE, ClientMsg::Chat(text.clone()));
                sent = true;
            }
        }
        if !sent {
            chat.push(format!("{} (offline): {}", config.name, text));
        }
    }
}

fn chat_ui_system(
    time: Res<Time>,
    mut chat: ResMut<ChatLog>,
    mut root: Query<&mut BackgroundColor, With<ChatRoot>>,
    mut log_text: Query<&mut Text, (With<ChatLogText>, Without<ChatInputText>)>,
    mut input_text: Query<&mut Text, (With<ChatInputText>, Without<ChatLogText>)>,
) {
    let dt = time.delta_secs();
    let open = chat.open;
    // Chỉ đụng vào resource khi còn dòng đang hiện (tránh đánh dấu thay đổi mỗi khung hình)
    if chat.lines.iter().any(|(_, age)| *age < LINE_TTL) {
        for (_, age) in chat.lines.iter_mut() {
            *age += dt;
        }
    }
    let shown: Vec<&str> = if open {
        let n = chat.lines.len().saturating_sub(SHOWN_OPEN);
        chat.lines[n..].iter().map(|(l, _)| l.as_str()).collect()
    } else {
        let fresh: Vec<&str> = chat.lines.iter().filter(|(_, age)| *age < LINE_TTL).map(|(l, _)| l.as_str()).collect();
        let n = fresh.len().saturating_sub(SHOWN_IDLE);
        fresh[n..].to_vec()
    };
    let log = shown.join("\n");
    let blink = (time.elapsed_secs() * 2.0) as i32 % 2 == 0;
    let input = if open { format!("Say: {}{}", chat.input, if blink { "_" } else { " " }) } else { String::new() };

    if let Ok(mut t) = log_text.single_mut() {
        if t.0 != log {
            t.0 = log.clone();
        }
    }
    if let Ok(mut t) = input_text.single_mut() {
        if t.0 != input {
            t.0 = input;
        }
    }
    if let Ok(mut bg) = root.single_mut() {
        let want = if open || !log.is_empty() { Color::srgba(0.0, 0.0, 0.0, if open { 0.6 } else { 0.35 }) } else { Color::NONE };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_plain_text_and_trims() {
        assert_eq!(sanitize_chat("  hi there  "), "hi there");
        assert_eq!(sanitize_chat("Hello, world! 123"), "Hello, world! 123");
    }

    #[test]
    fn sanitize_drops_non_ascii_and_control_characters() {
        assert_eq!(sanitize_chat("h\u{e9}llo"), "hllo");
        assert_eq!(sanitize_chat("a\nb\tc"), "abc");
        assert_eq!(sanitize_chat("ok \u{1F600}"), "ok");
    }

    #[test]
    fn sanitize_limits_length() {
        let long = "x".repeat(MAX_CHAT_CHARS * 4);
        assert_eq!(sanitize_chat(&long).len(), MAX_CHAT_CHARS);
    }

    #[test]
    fn sanitize_can_return_empty() {
        assert_eq!(sanitize_chat(""), "");
        assert_eq!(sanitize_chat("   \n\t  "), "");
        assert_eq!(sanitize_chat("\u{e9}\u{e9}"), "");
    }
}
