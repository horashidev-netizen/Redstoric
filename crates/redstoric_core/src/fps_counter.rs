use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

pub struct FpsOverlayPlugin;

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        // Đăng ký FrameTimeDiagnosticsPlugin để Bevy thu thập dữ liệu frame
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.add_systems(Startup, setup_fps_ui)
            .add_systems(Update, (update_fps_text, toggle_fps_visibility));
    }
}

/// Đánh dấu span chứa giá trị FPS
#[derive(Component)]
pub struct FpsValueSpan;

/// Đánh dấu span chứa frame time (ms)
#[derive(Component)]
pub struct FrameTimeSpan;

/// Root container của FPS UI để tiện ẩn/hiện
#[derive(Component)]
pub struct FpsRoot;

fn setup_fps_ui(mut commands: Commands) {
    // Lưu ý: Đảm bảo trong game đã có Camera2d hoặc Camera3d
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)), // Nền đen mờ dễ nhìn
            FpsRoot,
        ))
        .with_children(|parent| {
            // Span đầu tiên là chính component Text, các span sau là con (TextSpan)
            parent
                .spawn((
                    Text::new("FPS: "),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ))
                .with_children(|text| {
                    text.spawn((
                        TextSpan::new("0.0"),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.0, 1.0, 0.0)),
                        FpsValueSpan,
                    ));
                    text.spawn((
                        TextSpan::new(" (0.0 ms)"),
                        TextFont {
                            font_size: bevy::text::FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        FrameTimeSpan,
                    ));
                });
        });
}

// Cập nhật giá trị FPS và Frame time mỗi frame
fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut fps_q: Query<(&mut TextSpan, &mut TextColor), (With<FpsValueSpan>, Without<FrameTimeSpan>)>,
    mut ft_q: Query<&mut TextSpan, (With<FrameTimeSpan>, Without<FpsValueSpan>)>,
) {
    let mut current_fps = 0.0;
    let mut frame_time = 0.0;

    // Lấy smoothed FPS
    if let Some(fps_val) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        current_fps = fps_val;
    }

    // Lấy smoothed frame time (ms)
    if let Some(ft_val) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
    {
        frame_time = ft_val;
    }

    // Đổi màu hiển thị theo mức FPS
    let color = if current_fps >= 59.5 {
        Color::srgb(0.2, 0.9, 0.2) // Xanh lá mượt mà
    } else if current_fps >= 30.0 {
        Color::srgb(0.9, 0.8, 0.2) // Vàng tụt nhẹ
    } else {
        Color::srgb(0.9, 0.2, 0.2) // Đỏ cảnh báo giật lag
    };

    for (mut span, mut text_color) in &mut fps_q {
        **span = format!("{:.1}", current_fps);
        text_color.0 = color;
    }
    for mut span in &mut ft_q {
        **span = format!(" ({:.2} ms)", frame_time);
    }
}

// Bật / tắt hiển thị FPS bằng phím F3
fn toggle_fps_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<FpsRoot>>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        for mut visibility in &mut query {
            *visibility = match *visibility {
                Visibility::Hidden => Visibility::Inherited,
                _ => Visibility::Hidden,
            };
        }
    }
}