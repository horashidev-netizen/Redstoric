//! Điều khiển cảm ứng cho điện thoại: joystick ảo bên trái, kéo ngón bên phải để xoay camera,
//! nút Đánh / Nhảy / Nhân vật (F7). Cơ chế: biến chạm thành phím / chuột ảo, nên không cần sửa
//! code điều khiển hiện có. Chỉ bật trên Android, hoặc khi đặt `REDSTORIC_TOUCH=1` (thử trên
//! laptop cảm ứng).
use std::collections::{HashMap, HashSet};

use bevy::input::mouse::MouseMotion;
use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Bán kính (pixel logic) để đẩy joystick tới mức tối đa.
const STICK_RADIUS: f32 = 70.0;
const DEAD_ZONE: f32 = 0.25;
/// Phía trái màn hình (theo tỉ lệ bề rộng) dành cho joystick.
const LEFT_ZONE: f32 = 0.45;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Stick,
    Camera,
    Attack,
    Jump,
    Menu,
}

#[derive(Default)]
struct TouchState {
    roles: HashMap<u64, Role>,
    stick_origin: Vec2,
    /// Phím/chuột ảo đang giữ do chạm (để nhả đúng lúc, không đè lên bàn phím thật)
    keys: HashSet<KeyCode>,
    buttons: HashSet<MouseButton>,
}

pub struct TouchControlsPlugin;

impl Plugin for TouchControlsPlugin {
    fn build(&self, app: &mut App) {
        let enabled = cfg!(target_os = "android") || std::env::var("REDSTORIC_TOUCH").is_ok_and(|v| v == "1");
        if !enabled {
            return;
        }
        app.add_systems(Startup, spawn_touch_ui)
            .add_systems(PreUpdate, touch_input_system.after(InputSystems));
    }
}

fn attack_center(w: f32, h: f32) -> (Vec2, f32) {
    (Vec2::new(w - 85.0, h - 85.0), 55.0)
}
fn jump_center(w: f32, h: f32) -> (Vec2, f32) {
    (Vec2::new(w - 200.0, h - 60.0), 40.0)
}
fn menu_center(w: f32, _h: f32) -> (Vec2, f32) {
    (Vec2::new(w - 40.0, 40.0), 30.0)
}

fn spawn_touch_ui(mut commands: Commands) {
    let button = |right: f32, bottom: Option<f32>, top: Option<f32>, size: f32, label: &str| {
        (
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(right),
                bottom: bottom.map(Val::Px).unwrap_or(Val::Auto),
                top: top.map(Val::Px).unwrap_or(Val::Auto),
                width: Val::Px(size),
                height: Val::Px(size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.22)),
            children![(
                Text::new(label.to_string()),
                TextFont { font_size: FontSize::Px(20.0), ..default() },
                TextColor(Color::WHITE),
            )],
        )
    };
    commands.spawn(button(30.0, Some(30.0), None, 110.0, "Danh"));
    commands.spawn(button(160.0, Some(20.0), None, 80.0, "Nhay"));
    commands.spawn(button(10.0, None, Some(10.0), 60.0, "F7"));
    // Gợi ý vùng joystick
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(40.0),
            bottom: Val::Px(40.0),
            width: Val::Px(150.0),
            height: Val::Px(150.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)),
    ));
}

fn set_key(keys: &mut ButtonInput<KeyCode>, state: &mut HashSet<KeyCode>, key: KeyCode, want: bool) {
    if want && state.insert(key) {
        keys.press(key);
    } else if !want && state.remove(&key) {
        keys.release(key);
    }
}

fn set_button(input: &mut ButtonInput<MouseButton>, state: &mut HashSet<MouseButton>, button: MouseButton, want: bool) {
    if want && state.insert(button) {
        input.press(button);
    } else if !want && state.remove(&button) {
        input.release(button);
    }
}

fn touch_input_system(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: MessageWriter<MouseMotion>,
    mut st: Local<TouchState>,
) {
    let Ok(window) = windows.single() else { return };
    let (w, h) = (window.width(), window.height());

    // Quên các ngón đã nhấc
    st.roles.retain(|id, _| touches.get_pressed(*id).is_some());

    // Phân vai cho ngón mới chạm
    for t in touches.iter_just_pressed() {
        let p = t.position();
        let hit = |(c, r): (Vec2, f32)| p.distance(c) <= r;
        let role = if hit(attack_center(w, h)) {
            Role::Attack
        } else if hit(jump_center(w, h)) {
            Role::Jump
        } else if hit(menu_center(w, h)) {
            Role::Menu
        } else if p.x < w * LEFT_ZONE {
            Role::Stick
        } else {
            Role::Camera
        };
        if role == Role::Stick {
            st.stick_origin = p;
        }
        st.roles.insert(t.id(), role);
    }

    let mut stick = Vec2::ZERO;
    let (mut attack, mut jump, mut menu) = (false, false, false);
    let mut cam_delta = Vec2::ZERO;
    let mut cam_active = false;
    for t in touches.iter() {
        match st.roles.get(&t.id()) {
            Some(Role::Stick) => stick = (t.position() - st.stick_origin) / STICK_RADIUS,
            Some(Role::Camera) => {
                cam_active = true;
                cam_delta += t.delta();
            }
            Some(Role::Attack) => attack = true,
            Some(Role::Jump) => jump = true,
            Some(Role::Menu) => menu = true,
            None => {}
        }
    }
    let mag = stick.length().min(1.0);
    let dir = if mag > 0.0 { stick.normalize() * mag } else { Vec2::ZERO };
    let moving = mag > DEAD_ZONE;

    // Màn hình y hướng xuống: kéo lên = tiến
    let s = &mut *st;
    set_key(&mut keys, &mut s.keys, KeyCode::KeyW, moving && dir.y < -0.35);
    set_key(&mut keys, &mut s.keys, KeyCode::KeyS, moving && dir.y > 0.35);
    set_key(&mut keys, &mut s.keys, KeyCode::KeyA, moving && dir.x < -0.35);
    set_key(&mut keys, &mut s.keys, KeyCode::KeyD, moving && dir.x > 0.35);
    set_key(&mut keys, &mut s.keys, KeyCode::ShiftLeft, moving && mag > 0.92);
    set_key(&mut keys, &mut s.keys, KeyCode::Space, jump);
    set_key(&mut keys, &mut s.keys, KeyCode::F7, menu);

    set_button(&mut mouse, &mut s.buttons, MouseButton::Left, attack);
    // Kéo ngón bên phải = giữ chuột phải + di chuột để xoay camera
    set_button(&mut mouse, &mut s.buttons, MouseButton::Right, cam_active);
    if cam_active && cam_delta != Vec2::ZERO {
        motion.write(MouseMotion { delta: cam_delta });
    }
}
