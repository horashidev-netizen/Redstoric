//! Menu cài đặt đồ họa trong game. Mở / đóng bằng F10.
//!
//! Dùng `Interaction` của bevy_ui để bắt click. Trong Bevy 0.20 kiểu này (và `Button`) bị đánh dấu
//! deprecated để chuyển sang `bevy_ui_widgets`, nhưng vẫn hoạt động đầy đủ; tắt cảnh báo ở đây.
//! `Button` hiện chỉ còn là type alias nên không spawn được như một giá trị: ta spawn trực tiếp
//! `Interaction::None` + `FocusPolicy::Block`, đúng những thành phần `Button` từng yêu cầu.
#![allow(deprecated)]

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::graphics::{
    save_settings, set_quality_mode, AutoTuner, GraphicsSettings, GraphicsState, Quality,
    QualityMode,
};

const BTN_NORMAL: Color = Color::srgb(0.20, 0.22, 0.28);
const BTN_HOVER: Color = Color::srgb(0.30, 0.34, 0.44);
const BTN_PRESSED: Color = Color::srgb(0.12, 0.14, 0.20);
const BTN_SELECTED: Color = Color::srgb(0.18, 0.45, 0.80);

const MIN_TARGET_FPS: f32 = 30.0;
const MAX_TARGET_FPS: f32 = 240.0;

/// Trạng thái menu. Các hệ thống khác (vd. chiến đấu) đọc `open` để bỏ qua click chuột
/// khi người chơi đang bấm trong menu.
#[derive(Resource, Default)]
pub struct GraphicsMenu {
    pub open: bool,
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component, Clone, Copy, PartialEq)]
enum MenuAction {
    Mode(QualityMode),
    TargetFps(f32),
    Close,
}

#[derive(Component)]
struct TargetFpsText;

#[derive(Component)]
struct GpuText;

pub struct GraphicsMenuPlugin;

impl Plugin for GraphicsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphicsMenu>()
            .add_systems(Startup, spawn_menu)
            .add_systems(
                Update,
                (toggle_menu, handle_menu_buttons, refresh_menu).chain(),
            );
    }
}

fn menu_text(text: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// Spawn một nút bấm làm con của `$parent`. Dùng macro để khỏi phải gọi tên kiểu spawner.
macro_rules! menu_button {
    ($parent:expr, $label:expr, $action:expr) => {
        $parent
            .spawn((
                Interaction::None,
                FocusPolicy::Block,
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                $action,
            ))
            .with_children(|button| {
                button.spawn(menu_text($label, 16.0, Color::WHITE));
            });
    };
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            MenuRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        min_width: Val::Px(460.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.94)),
                ))
                .with_children(|panel| {
                    // Tiêu đề
                    panel.spawn(menu_text("Graphics Settings", 22.0, Color::WHITE));

                    // Dòng GPU
                    panel
                        .spawn(menu_text("GPU: ", 14.0, Color::srgb(0.7, 0.7, 0.7)))
                        .with_children(|text| {
                            text.spawn((
                                TextSpan::new("..."),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.6, 0.9, 1.0)),
                                GpuText,
                            ));
                        });

                    // Hàng chọn mức chất lượng
                    panel.spawn(menu_text("Quality", 16.0, Color::WHITE));
                    panel
                        .spawn(Node {
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for (name, mode) in [
                                ("Auto", QualityMode::Auto),
                                ("Low", QualityMode::Fixed(Quality::Low)),
                                ("Medium", QualityMode::Fixed(Quality::Medium)),
                                ("High", QualityMode::Fixed(Quality::High)),
                                ("Ultra", QualityMode::Fixed(Quality::Ultra)),
                            ] {
                                menu_button!(row, name, MenuAction::Mode(mode));
                            }
                        });

                    // Hàng FPS mục tiêu (dùng cho chế độ Auto)
                    panel
                        .spawn(Node {
                            column_gap: Val::Px(10.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            menu_button!(row, "-10", MenuAction::TargetFps(-10.0));
                            row.spawn(menu_text("Auto target FPS: ", 16.0, Color::WHITE))
                                .with_children(|text| {
                                    text.spawn((
                                        TextSpan::new("60"),
                                        TextFont {
                                            font_size: FontSize::Px(16.0),
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.6, 0.9, 1.0)),
                                        TargetFpsText,
                                    ));
                                });
                            menu_button!(row, "+10", MenuAction::TargetFps(10.0));
                        });

                    // Nút đóng
                    panel
                        .spawn(Node {
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            menu_button!(row, "Close (F10)", MenuAction::Close);
                        });
                });
        });
}

fn toggle_menu(keys: Res<ButtonInput<KeyCode>>, mut menu: ResMut<GraphicsMenu>) {
    if keys.just_pressed(KeyCode::F10) {
        menu.open = !menu.open;
    }
}

fn handle_menu_buttons(
    interactions: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut menu: ResMut<GraphicsMenu>,
    mut settings: ResMut<GraphicsSettings>,
    mut state: ResMut<GraphicsState>,
    mut tuner: ResMut<AutoTuner>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            MenuAction::Mode(mode) => {
                set_quality_mode(&mut settings, &mut state, &mut tuner, mode);
            }
            MenuAction::TargetFps(delta) => {
                settings.target_fps =
                    (settings.target_fps + delta).clamp(MIN_TARGET_FPS, MAX_TARGET_FPS);
                save_settings(&settings);
            }
            MenuAction::Close => menu.open = false,
        }
    }
}

fn refresh_menu(
    menu: Res<GraphicsMenu>,
    settings: Res<GraphicsSettings>,
    state: Res<GraphicsState>,
    mut roots: Query<&mut Visibility, With<MenuRoot>>,
    mut buttons: Query<(&MenuAction, &Interaction, &mut BackgroundColor)>,
    mut fps_text: Query<&mut TextSpan, (With<TargetFpsText>, Without<GpuText>)>,
    mut gpu_text: Query<&mut TextSpan, (With<GpuText>, Without<TargetFpsText>)>,
) {
    // Đồng bộ hiển thị theo trạng thái mở / đóng
    let wanted = if menu.open { Visibility::Inherited } else { Visibility::Hidden };
    for mut visibility in &mut roots {
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    if !menu.open {
        return;
    }

    // Màu nút: nút đang chọn nổi bật, các nút khác đổi theo hover / nhấn
    for (action, interaction, mut background) in &mut buttons {
        let selected = matches!(action, MenuAction::Mode(mode) if *mode == settings.mode);
        let color = if selected {
            BTN_SELECTED
        } else {
            match *interaction {
                Interaction::Pressed => BTN_PRESSED,
                Interaction::Hovered => BTN_HOVER,
                Interaction::None => BTN_NORMAL,
            }
        };
        if background.0 != color {
            background.0 = color;
        }
    }

    if menu.is_changed() || settings.is_changed() || state.is_changed() {
        for mut span in &mut fps_text {
            **span = format!("{:.0}", settings.target_fps);
        }
        for mut span in &mut gpu_text {
            **span = if state.gpu_name.is_empty() {
                "unknown".to_string()
            } else {
                format!("{} (suggested: {})", state.gpu_name, state.detected.label())
            };
        }
    }
}
