//! Bản đồ nhỏ (minimap) ở góc trên bên phải, kiểu Veloren.
//!
//! * Thanh tiêu đề: nút −/+ để thu phóng (hoặc phím `-` / `=`), tên địa danh hiện tại
//!   (tên làng, "Boss Arena" hoặc vùng sinh thái như Forest, Desert, Snow Mountains).
//! * Bốn nhãn N / E / S / W chạy quanh viền theo hướng camera; N màu đỏ.
//!
//! Bản đồ luôn xoay sao cho "phía trên màn hình" = hướng camera đang nhìn, nên bấm W là đi
//! lên trên bản đồ. Chữ N chạy quanh viền bản đồ để chỉ hướng Bắc (trục -Z của thế giới).
//! Ảnh bản đồ được vẽ lại bằng CPU vài lần mỗi giây từ dữ liệu khối đã tải trong `VoxelWorldMap`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::FocusPolicy;
use redstoric_voxel::structures::SiteKind;
use redstoric_voxel::{BlockType, VoxelWorldMap, WorldGenerator, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

use crate::{Character, OrbitCamera};

/// Số pixel của ảnh bản đồ (vuông).
const MAP_PX: usize = 128;
/// Các mức thu phóng: bản đồ bao phủ bao nhiêu khối thế giới theo mỗi cạnh.
const ZOOM_LEVELS: [f32; 4] = [48.0, 96.0, 192.0, 384.0];
/// Chiều cao thanh tiêu đề (px UI).
const HEADER_H: f32 = 26.0;
/// Kích thước hiển thị trên màn hình (px UI).
const MAP_UI_SIZE: f32 = 168.0;
const UPDATE_INTERVAL: f32 = 0.12;

#[derive(Resource)]
struct MinimapImage {
    handle: Handle<Image>,
    timer: Timer,
}

/// Mức thu phóng hiện tại (chỉ số trong `ZOOM_LEVELS`).
#[derive(Resource)]
struct MinimapZoom {
    level: usize,
}

/// Nhãn la bàn; `dir` là hướng thế giới mà nhãn chỉ tới.
#[derive(Component)]
struct CompassLabel {
    dir: Vec3,
}

/// Nút thu phóng: `-1` thu nhỏ (xem xa hơn), `+1` phóng to.
#[derive(Component)]
struct ZoomButton(i32);

#[derive(Component)]
struct LocationLabel;

const BTN_NORMAL: Color = Color::srgb(0.16, 0.18, 0.22);
const BTN_HOVER: Color = Color::srgb(0.28, 0.32, 0.40);
const BTN_PRESSED: Color = Color::srgb(0.10, 0.12, 0.16);

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MinimapZoom { level: 1 })
            .add_systems(Startup, setup_minimap)
            .add_systems(
                Update,
                (
                    handle_zoom,
                    update_minimap_image,
                    update_compass,
                    update_location_label,
                ),
            );
    }
}

fn setup_minimap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new(
        Extent3d {
            width: MAP_PX as u32,
            height: MAP_PX as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![20u8; MAP_PX * MAP_PX * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    commands.insert_resource(MinimapImage {
        handle: handle.clone(),
        timer: Timer::from_seconds(UPDATE_INTERVAL, TimerMode::Repeating),
    });

    // Thanh tiêu đề: [−][+]  Tên địa danh
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(12.0),
                width: Val::Px(MAP_UI_SIZE + 8.0),
                height: Val::Px(HEADER_H),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.92)),
        ))
        .with_children(|header| {
            for (label, delta) in [("-", -1), ("+", 1)] {
                header
                    .spawn((
                        Interaction::None,
                        FocusPolicy::Block,
                        Node {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BTN_NORMAL),
                        ZoomButton(delta),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(label),
                            TextFont { font_size: FontSize::Px(16.0), ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
            header
                .spawn(Node {
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn((
                        Text::new(""),
                        TextFont { font_size: FontSize::Px(17.0), ..default() },
                        TextColor(Color::srgb(1.0, 0.95, 0.80)),
                        LocationLabel,
                    ));
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0 + HEADER_H),
                right: Val::Px(12.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.9)),
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(handle),
                Node {
                    width: Val::Px(MAP_UI_SIZE),
                    height: Val::Px(MAP_UI_SIZE),
                    ..default()
                },
            ));
            // Bốn nhãn la bàn chạy quanh viền, vị trí cập nhật mỗi frame.
            // Bắc = trục -Z; Đông = +X; Nam = +Z; Tây = -X.
            for (text, dir, color) in [
                ("N", Vec3::new(0.0, 0.0, -1.0), Color::srgb(1.0, 0.35, 0.3)),
                ("E", Vec3::new(1.0, 0.0, 0.0), Color::WHITE),
                ("S", Vec3::new(0.0, 0.0, 1.0), Color::WHITE),
                ("W", Vec3::new(-1.0, 0.0, 0.0), Color::WHITE),
            ] {
                root.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(color),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(MAP_UI_SIZE * 0.5),
                        top: Val::Px(4.0),
                        ..default()
                    },
                    CompassLabel { dir },
                ));
            }
        });
}

/// Hướng nhìn phẳng của camera: (forward, right) trên mặt phẳng XZ.
fn camera_axes(cam: &Transform) -> (Vec3, Vec3) {
    let f = cam.forward();
    let r = cam.right();
    (
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero(),
        Vec3::new(r.x, 0.0, r.z).normalize_or_zero(),
    )
}

fn update_compass(
    camera: Query<&Transform, With<OrbitCamera>>,
    mut labels: Query<(&CompassLabel, &mut Node)>,
) {
    let Ok(cam) = camera.single() else { return };
    let (forward, right) = camera_axes(cam);
    let c = 4.0 + MAP_UI_SIZE * 0.5;
    let radius = MAP_UI_SIZE * 0.5 - 10.0;
    for (label, mut node) in &mut labels {
        let u = label.dir.dot(right); // sang phải trên màn hình
        let v = label.dir.dot(forward); // lên trên màn hình
        node.left = Val::Px(c + u * radius - 5.0);
        node.top = Val::Px(c - v * radius - 9.0);
    }
}

/// Nút −/+ và phím `-` / `=` đổi mức thu phóng.
fn handle_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom: ResMut<MinimapZoom>,
    mut buttons: Query<(&Interaction, &ZoomButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    let mut delta = 0i32;
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        delta += 1;
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        delta -= 1;
    }
    for (interaction, button, mut bg) in &mut buttons {
        *bg = BackgroundColor(match interaction {
            Interaction::Pressed => BTN_PRESSED,
            Interaction::Hovered => BTN_HOVER,
            Interaction::None => BTN_NORMAL,
        });
        if *interaction == Interaction::Pressed {
            delta += button.0;
        }
    }
    if delta != 0 {
        let step = delta.signum();
        zoom.level = (zoom.level as i32 - step).clamp(0, ZOOM_LEVELS.len() as i32 - 1) as usize;
    }
}

/// Tên địa danh: tên làng / "Boss Arena" nếu đang ở trong khu, ngược lại là vùng sinh thái.
fn update_location_label(
    time: Res<Time>,
    mut since: Local<f32>,
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Transform, With<Character>>,
    mut label: Query<&mut Text, With<LocationLabel>>,
) {
    *since += time.delta_secs();
    if *since < 0.3 {
        return;
    }
    *since = 0.0;
    let (Some(generator), Ok(player), Ok(mut label)) = (generator, player.single(), label.single_mut()) else {
        return;
    };
    let (x, z) = (player.translation.x.floor() as i32, player.translation.z.floor() as i32);
    let in_site = generator
        .sites_in_area(x - 1, z - 1, x + 1, z + 1)
        .into_iter()
        .find(|s| s.dist(x, z) <= s.radius as f32);
    let name = match in_site {
        Some(site) => site.name(),
        None => generator.biome_label(x as f64, z as f64).to_string(),
    };
    if label.0 != name {
        label.0 = name;
    }
}

fn block_rgb(b: BlockType) -> [f32; 3] {
    match b {
        BlockType::Grass => [0.28, 0.62, 0.26],
        BlockType::Dirt => [0.45, 0.30, 0.18],
        BlockType::Stone => [0.50, 0.50, 0.53],
        BlockType::CliffStone => [0.38, 0.38, 0.41],
        BlockType::Sand => [0.84, 0.78, 0.56],
        BlockType::Snow => [0.94, 0.96, 0.99],
        BlockType::Water => [0.18, 0.42, 0.80],
        BlockType::Wood => [0.36, 0.25, 0.14],
        BlockType::BirchWood => [0.80, 0.80, 0.76],
        BlockType::Cactus => [0.20, 0.52, 0.24],
        BlockType::Leaves => [0.14, 0.45, 0.14],
        BlockType::LeavesLight => [0.42, 0.60, 0.20],
        BlockType::PineLeaves => [0.08, 0.30, 0.20],
        BlockType::Planks => [0.62, 0.46, 0.27],
        BlockType::Cobble => [0.55, 0.55, 0.56],
        BlockType::RoofTile => [0.65, 0.22, 0.18],
        BlockType::Bed => [0.75, 0.20, 0.25],
        BlockType::Obsidian => [0.12, 0.10, 0.18],
        BlockType::Farmland => [0.30, 0.19, 0.11],
        BlockType::Wheat => [0.85, 0.72, 0.22],
        BlockType::Rice => [0.55, 0.80, 0.30],
        BlockType::Plaster => [0.86, 0.80, 0.68],
        BlockType::DarkWood => [0.24, 0.15, 0.08],
        BlockType::RoofTileBlue => [0.22, 0.32, 0.50],
        BlockType::RoofTileGreen => [0.26, 0.42, 0.24],
        BlockType::Thatch => [0.72, 0.58, 0.30],
        BlockType::Air => [0.0, 0.0, 0.0],
    }
}

fn update_minimap_image(
    time: Res<Time>,
    zoom: Res<MinimapZoom>,
    mut state: ResMut<MinimapImage>,
    mut images: ResMut<Assets<Image>>,
    map: Option<Res<VoxelWorldMap>>,
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Transform, With<Character>>,
    camera: Query<&Transform, With<OrbitCamera>>,
) {
    state.timer.tick(time.delta());
    if !state.timer.just_finished() {
        return;
    }
    let (Some(map), Ok(player), Ok(cam)) = (map, player.single(), camera.single()) else {
        return;
    };
    let Some(mut image) = images.get_mut(&state.handle) else { return };

    let (forward, right) = camera_axes(cam);
    let step = ZOOM_LEVELS[zoom.level] / MAP_PX as f32;
    let half = MAP_PX as f32 * 0.5;
    let origin = player.translation;

    let mut buf = vec![0u8; MAP_PX * MAP_PX * 4];
    for j in 0..MAP_PX {
        for i in 0..MAP_PX {
            let u = (i as f32 + 0.5 - half) * step;
            let v = (half - j as f32 - 0.5) * step;
            let wx = origin.x + right.x * u + forward.x * v;
            let wz = origin.z + right.z * u + forward.z * v;

            let bx = wx.floor() as i32;
            let bz = wz.floor() as i32;
            let cx = bx.div_euclid(CHUNK_SIZE_X as i32);
            let cz = bz.div_euclid(CHUNK_SIZE_Z as i32);
            let lx = bx.rem_euclid(CHUNK_SIZE_X as i32) as usize;
            let lz = bz.rem_euclid(CHUNK_SIZE_Z as i32) as usize;

            let mut rgb = [0.10, 0.10, 0.12]; // vùng chưa tải
            if let Some(chunk) = map.chunks.get(&(cx, cz)) {
                for y in (0..CHUNK_SIZE_Y).rev() {
                    let b = chunk.get_block(lx, y, lz);
                    if !b.is_air() {
                        let base = block_rgb(b);
                        // Càng cao càng sáng một chút để thấy địa hình nổi lên.
                        let shade = 0.72 + 0.4 * (y as f32 / CHUNK_SIZE_Y as f32);
                        rgb = [base[0] * shade, base[1] * shade, base[2] * shade];
                        break;
                    }
                }
            }
            let k = (j * MAP_PX + i) * 4;
            buf[k] = (rgb[0].clamp(0.0, 1.0) * 255.0) as u8;
            buf[k + 1] = (rgb[1].clamp(0.0, 1.0) * 255.0) as u8;
            buf[k + 2] = (rgb[2].clamp(0.0, 1.0) * 255.0) as u8;
            buf[k + 3] = 255;
        }
    }

    // Biểu tượng các khu công trình: thị trấn (xanh lam), làng (vàng), đấu trường boss (đỏ). Ở xa thì bám theo rìa bản đồ.
    if let Some(generator) = generator {
        let reach = 400;
        let (ox, oz) = (origin.x as i32, origin.z as i32);
        for site in generator.sites_in_area(ox - reach, oz - reach, ox + reach, oz + reach) {
            let d = Vec3::new(site.x as f32 - origin.x, 0.0, site.z as f32 - origin.z);
            let margin = 6.0;
            let limit = MAP_PX as f32 - margin;
            let px = (half + d.dot(right) / step).clamp(margin, limit) as i32;
            let py = (half - d.dot(forward) / step).clamp(margin, limit) as i32;
            let color = match site.kind {
                SiteKind::Village if site.tier == 0 => [90u8, 200, 255, 255],
                SiteKind::Village => [255u8, 205, 40, 255],
                SiteKind::BossArena => [235u8, 40, 40, 255],
            };
            for dy in -3..=3i32 {
                for dx in -3..=3i32 {
                    let (x, y) = (px + dx, py + dy);
                    if x < 0 || y < 0 || x as usize >= MAP_PX || y as usize >= MAP_PX {
                        continue;
                    }
                    let k = (y as usize * MAP_PX + x as usize) * 4;
                    let edge = dx.abs() == 3 || dy.abs() == 3;
                    let c = if edge { [20u8, 20, 20, 255] } else { color };
                    buf[k..k + 4].copy_from_slice(&c);
                }
            }
        }
    }

    // Mũi tên nhân vật ở giữa, luôn chỉ lên trên (= hướng camera / hướng đi khi bấm W).
    let c = MAP_PX as i32 / 2;
    for row in 0..9i32 {
        let width = row / 2; // thân mũi tên rộng dần xuống dưới
        for dx in -width..=width {
            let x = c + dx;
            let y = c - 5 + row;
            if x >= 0 && y >= 0 && (x as usize) < MAP_PX && (y as usize) < MAP_PX {
                let k = (y as usize * MAP_PX + x as usize) * 4;
                buf[k..k + 4].copy_from_slice(&[255, 230, 60, 255]);
            }
        }
    }

    image.data = Some(buf);
}
