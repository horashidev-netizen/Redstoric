//! Cài đặt đồ họa: preset chất lượng, tự nhận diện GPU và tự điều chỉnh theo FPS.
//!
//! Cách hoạt động (tham khảo hướng preset Minimal/Low/Medium/High/Ultra của Veloren):
//! 1. Lúc khởi động đọc `RenderAdapterInfo` để đoán mức chất lượng hợp với GPU.
//! 2. Chế độ `Auto` đo FPS trung bình: tụt dưới mục tiêu thì hạ một mức,
//!    dư dả nhiều thì nâng một mức (không nâng lại mức vừa bị hạ).
//! 3. Phím F4 đổi vòng: Auto -> Low -> Medium -> High -> Ultra -> Auto.
//! 4. Lựa chọn của người chơi được lưu vào `graphics_settings.json`.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use redstoric_voxel::{VoxelStreamSettings, CHUNK_SIZE_X};
use redstoric_common::paths::data_path;
use serde::{Deserialize, Serialize};

const SETTINGS_PATH: &str = "graphics_settings.json";
/// Chờ bao lâu sau khi khởi động / đổi mức rồi mới đo FPS (chunk đang tải làm FPS tụt tạm thời)
const WARMUP_SECS: f32 = 8.0;
/// Khoảng cách giữa hai lần đo khi không có thay đổi
const CHECK_INTERVAL_SECS: f32 = 3.0;

// ─────────────────────────────────────────────────────────────
// Mức chất lượng và preset
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

/// Bộ thông số cụ thể của một mức chất lượng
#[derive(Clone, Copy, Debug)]
pub struct QualityPreset {
    /// Bán kính tải chunk (đơn vị chunk)
    pub load_distance: i32,
    pub unload_distance: i32,
    /// Chỉ spawn props (hoa, bụi, rương...) trong bán kính này (đơn vị chunk)
    pub prop_distance: i32,
    pub shadows: bool,
    /// Kích thước shadow map, phải là lũy thừa của 2
    pub shadow_map_size: usize,
    pub cascades: usize,
    /// Khoảng cách đổ bóng tối đa (mét)
    pub shadow_distance: f32,
    /// Số mẫu MSAA: 1 (tắt), 2, 4
    pub msaa_samples: u32,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Quality::Low => "Low",
            Quality::Medium => "Medium",
            Quality::High => "High",
            Quality::Ultra => "Ultra",
        }
    }

    fn lower(self) -> Self {
        match self {
            Quality::Ultra => Quality::High,
            Quality::High => Quality::Medium,
            _ => Quality::Low,
        }
    }

    fn higher(self) -> Self {
        match self {
            Quality::Low => Quality::Medium,
            Quality::Medium => Quality::High,
            _ => Quality::Ultra,
        }
    }

    pub fn preset(self) -> QualityPreset {
        match self {
            Quality::Low => QualityPreset {
                load_distance: 4,
                unload_distance: 6,
                prop_distance: 2,
                shadows: false,
                shadow_map_size: 1024,
                cascades: 1,
                shadow_distance: 40.0,
                msaa_samples: 1,
            },
            Quality::Medium => QualityPreset {
                load_distance: 6,
                unload_distance: 8,
                prop_distance: 3,
                shadows: true,
                shadow_map_size: 1024,
                cascades: 2,
                shadow_distance: 60.0,
                msaa_samples: 1,
            },
            Quality::High => QualityPreset {
                load_distance: 8,
                unload_distance: 10,
                prop_distance: 5,
                shadows: true,
                shadow_map_size: 2048,
                cascades: 3,
                shadow_distance: 100.0,
                msaa_samples: 2,
            },
            Quality::Ultra => QualityPreset {
                load_distance: 10,
                unload_distance: 12,
                prop_distance: 7,
                shadows: true,
                shadow_map_size: 4096,
                cascades: 4,
                shadow_distance: 150.0,
                msaa_samples: 4,
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Cài đặt người chơi (lưu ra file) và trạng thái chạy
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityMode {
    /// Tự nhận diện GPU rồi tự điều chỉnh theo FPS
    Auto,
    /// Người chơi chọn cố định
    Fixed(Quality),
}

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub mode: QualityMode,
    /// FPS mục tiêu của chế độ Auto
    pub target_fps: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        // Điện thoại: mặc định mức Low cố định (GPU di động yếu, Auto có thể nâng lên quá tay)
        let mode = if cfg!(target_os = "android") { QualityMode::Fixed(Quality::Low) } else { QualityMode::Auto };
        Self { mode, target_fps: 60.0 }
    }
}

/// Trạng thái hiện tại (không lưu ra file)
#[derive(Resource)]
pub struct GraphicsState {
    /// Mức đang áp dụng
    pub active: Quality,
    /// Mức cao nhất Auto còn được phép nâng tới (chặn việc nâng lên mức vừa làm tụt FPS)
    pub ceiling: Quality,
    /// Mức đoán được từ GPU
    pub detected: Quality,
    pub gpu_name: String,
    pub(crate) dirty: bool,
}

/// Bộ đếm thời gian cho chế độ Auto
#[derive(Resource)]
pub(crate) struct AutoTuner {
    pub(crate) cooldown: f32,
}

#[derive(Component)]
struct QualityLabel;

fn load_settings() -> GraphicsSettings {
    std::fs::read_to_string(data_path(SETTINGS_PATH))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub(crate) fn save_settings(settings: &GraphicsSettings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(data_path(SETTINGS_PATH), json);
    }
}

// ─────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.insert_resource(load_settings())
            .insert_resource(GraphicsState {
                active: Quality::Medium,
                ceiling: Quality::Ultra,
                detected: Quality::Medium,
                gpu_name: String::new(),
                dirty: true,
            })
            .insert_resource(AutoTuner { cooldown: WARMUP_SECS })
            .add_systems(Startup, (detect_hardware, setup_quality_label))
            .add_systems(
                Update,
                (
                    handle_quality_hotkey,
                    auto_adjust_quality,
                    apply_quality,
                    update_quality_label,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────
// Nhận diện phần cứng
// ─────────────────────────────────────────────────────────────

/// Đoán mức chất lượng từ loại và tên GPU. Đây chỉ là ước lượng ban đầu,
/// chế độ Auto sẽ chỉnh lại theo FPS đo được.
fn tier_for_gpu(name: &str, kind: &str) -> Quality {
    let n = name.to_lowercase();

    match kind {
        "Cpu" | "Other" | "VirtualGpu" => return Quality::Low,
        "IntegratedGpu" => {
            // iGPU đời mới chạy được mức Medium
            let modern = ["iris xe", "780m", "760m", "740m", "880m", "890m", "arc graphics"];
            return if modern.iter().any(|k| n.contains(k)) { Quality::Medium } else { Quality::Low };
        }
        _ => {}
    }

    let has = |list: &[&str]| list.iter().any(|k| n.contains(k));
    if has(&["rtx 4080", "rtx 4090", "rtx 5070", "rtx 5080", "rtx 5090", "rtx 3090", "rx 7900", "rx 9070"]) {
        Quality::Ultra
    } else if has(&[
        "rtx 40", "rtx 50", "rtx 3060", "rtx 3070", "rtx 3080", "rx 6700", "rx 6800", "rx 6900",
        "rx 7", "rx 9", "arc a7", "arc b5", "arc b7",
    ]) {
        Quality::High
    } else if has(&["gt 7", "gt 6", "gt 10", "gtx 7", "gtx 6", "gtx 9"]) {
        Quality::Low
    } else {
        Quality::Medium
    }
}

fn detect_hardware(
    adapter: Option<Res<RenderAdapterInfo>>,
    settings: Res<GraphicsSettings>,
    mut state: ResMut<GraphicsState>,
) {
    if let Some(adapter) = adapter {
        let name = adapter.name.clone();
        let kind = format!("{:?}", adapter.device_type);
        state.detected = tier_for_gpu(&name, &kind);
        bevy::log::info!("GPU: {} ({}) -> mức gợi ý: {}", name, kind, state.detected.label());
        state.gpu_name = name;
    }

    state.active = match settings.mode {
        QualityMode::Auto => state.detected,
        QualityMode::Fixed(quality) => quality,
    };
    state.dirty = true;
}

// ─────────────────────────────────────────────────────────────
// Điều khiển: phím F4 và tự điều chỉnh
// ─────────────────────────────────────────────────────────────

/// Đổi chế độ chất lượng (dùng chung cho phím F4 và menu F10)
pub(crate) fn set_quality_mode(
    settings: &mut GraphicsSettings,
    state: &mut GraphicsState,
    tuner: &mut AutoTuner,
    mode: QualityMode,
) {
    settings.mode = mode;
    state.active = match mode {
        QualityMode::Auto => state.detected,
        QualityMode::Fixed(quality) => quality,
    };
    state.ceiling = Quality::Ultra;
    state.dirty = true;
    tuner.cooldown = WARMUP_SECS;
    save_settings(settings);
}

fn handle_quality_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GraphicsSettings>,
    mut state: ResMut<GraphicsState>,
    mut tuner: ResMut<AutoTuner>,
) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }

    let next_mode = match settings.mode {
        QualityMode::Auto => QualityMode::Fixed(Quality::Low),
        QualityMode::Fixed(Quality::Low) => QualityMode::Fixed(Quality::Medium),
        QualityMode::Fixed(Quality::Medium) => QualityMode::Fixed(Quality::High),
        QualityMode::Fixed(Quality::High) => QualityMode::Fixed(Quality::Ultra),
        QualityMode::Fixed(Quality::Ultra) => QualityMode::Auto,
    };

    set_quality_mode(&mut settings, &mut state, &mut tuner, next_mode);
}

fn auto_adjust_quality(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    settings: Res<GraphicsSettings>,
    mut state: ResMut<GraphicsState>,
    mut tuner: ResMut<AutoTuner>,
) {
    if settings.mode != QualityMode::Auto {
        return;
    }

    tuner.cooldown -= time.delta_secs();
    if tuner.cooldown > 0.0 {
        return;
    }

    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.average())
    else {
        return;
    };

    let target = settings.target_fps as f64;

    if fps < target * 0.9 && state.active > Quality::Low {
        // FPS tụt dưới mục tiêu: hạ một mức và không cho nâng lại lên mức vừa thất bại
        state.active = state.active.lower();
        state.ceiling = state.active;
        state.dirty = true;
        tuner.cooldown = WARMUP_SECS;
    } else if fps > target * 1.6 && state.active < state.ceiling {
        // FPS dư dả: thử nâng một mức
        state.active = state.active.higher();
        state.dirty = true;
        tuner.cooldown = WARMUP_SECS;
    } else {
        tuner.cooldown = CHECK_INTERVAL_SECS;
    }
}

// ─────────────────────────────────────────────────────────────
// Áp dụng preset lên game
// ─────────────────────────────────────────────────────────────

fn apply_quality(
    mut commands: Commands,
    mut state: ResMut<GraphicsState>,
    mut stream: ResMut<VoxelStreamSettings>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut lights: Query<(Entity, &mut DirectionalLight)>,
    cameras: Query<Entity, With<Camera3d>>,
    mut fogs: Query<&mut DistanceFog>,
) {
    if !state.dirty {
        return;
    }
    // Đợi đến khi đèn và camera được spawn xong rồi mới áp dụng
    if lights.is_empty() || cameras.is_empty() {
        return;
    }

    let p = state.active.preset();

    // 1. Khoảng cách tải địa hình
    stream.load_distance = p.load_distance;
    stream.unload_distance = p.unload_distance;
    stream.prop_distance = p.prop_distance;

    // 2. Bóng đổ
    shadow_map.size = p.shadow_map_size;
    for (entity, mut light) in &mut lights {
        light.shadow_maps_enabled = p.shadows;
        commands.entity(entity).insert(
            CascadeShadowConfigBuilder {
                num_cascades: p.cascades,
                maximum_distance: p.shadow_distance,
                first_cascade_far_bound: p.shadow_distance / 4.0,
                ..default()
            }
            .build(),
        );
    }

    // 3. MSAA trên camera
    for camera in &cameras {
        commands.entity(camera).insert(Msaa::from_samples(p.msaa_samples));
    }

    // 4. Sương mù khớp với tầm nhìn để che chỗ chunk mới hiện ra
    let far = p.load_distance as f32 * CHUNK_SIZE_X as f32;
    for mut fog in &mut fogs {
        fog.falloff = FogFalloff::Linear { start: far * 0.5, end: far * 0.95 };
    }

    bevy::log::info!(
        "Graphics: {} (chunk {}, bóng {}, MSAA x{})",
        state.active.label(),
        p.load_distance,
        if p.shadows { "bật" } else { "tắt" },
        p.msaa_samples
    );
    state.dirty = false;
}

// ─────────────────────────────────────────────────────────────
// Hiển thị mức chất lượng dưới bộ đếm FPS
// ─────────────────────────────────────────────────────────────

fn setup_quality_label(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(46.0),
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Text::new("Quality: "),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ))
                .with_children(|text| {
                    text.spawn((
                        TextSpan::new("..."),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.6, 0.9, 1.0)),
                        QualityLabel,
                    ));
                });
        });
}

fn update_quality_label(
    settings: Res<GraphicsSettings>,
    state: Res<GraphicsState>,
    mut labels: Query<&mut TextSpan, With<QualityLabel>>,
) {
    if !(settings.is_changed() || state.is_changed()) {
        return;
    }

    let text = match settings.mode {
        QualityMode::Auto => format!("Auto ({}) [F4 cycle, F10 menu]", state.active.label()),
        QualityMode::Fixed(quality) => format!("{} [F4 cycle, F10 menu]", quality.label()),
    };
    for mut span in &mut labels {
        **span = text.clone();
    }
}
