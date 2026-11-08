//! Bản đồ nhỏ (minimap) ở góc trên bên phải.
//!
//! Bản đồ luôn xoay sao cho "phía trên màn hình" = hướng camera đang nhìn, nên bấm W là đi
//! lên trên bản đồ. Chữ N chạy quanh viền bản đồ để chỉ hướng Bắc (trục -Z của thế giới).
//! Ảnh bản đồ được vẽ lại bằng CPU vài lần mỗi giây từ dữ liệu khối đã tải trong `VoxelWorldMap`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use redstoric_voxel::structures::SiteKind;
use redstoric_voxel::{BlockType, VoxelWorldMap, WorldGenerator, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

use crate::{Character, OrbitCamera};

/// Số pixel của ảnh bản đồ (vuông).
const MAP_PX: usize = 128;
/// Bản đồ bao phủ bao nhiêu khối thế giới theo mỗi cạnh.
const MAP_RANGE_BLOCKS: f32 = 96.0;
/// Kích thước hiển thị trên màn hình (px UI).
const MAP_UI_SIZE: f32 = 168.0;
const UPDATE_INTERVAL: f32 = 0.12;

#[derive(Resource)]
struct MinimapImage {
    handle: Handle<Image>,
    timer: Timer,
}

#[derive(Component)]
struct NorthLabel;

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_minimap)
            .add_systems(Update, (update_minimap_image, update_north_label));
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

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
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
            // Chữ N chạy quanh viền, vị trí cập nhật mỗi frame.
            root.spawn((
                Text::new("N"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.35, 0.3)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(MAP_UI_SIZE * 0.5),
                    top: Val::Px(4.0),
                    ..default()
                },
                NorthLabel,
            ));
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

fn update_north_label(
    camera: Query<&Transform, With<OrbitCamera>>,
    mut label: Query<&mut Node, With<NorthLabel>>,
) {
    let Ok(cam) = camera.single() else { return };
    let Ok(mut node) = label.single_mut() else { return };
    let (forward, right) = camera_axes(cam);
    let north = Vec3::new(0.0, 0.0, -1.0);
    let u = north.dot(right); // sang phải trên màn hình
    let v = north.dot(forward); // lên trên màn hình
    let c = 4.0 + MAP_UI_SIZE * 0.5;
    let radius = MAP_UI_SIZE * 0.5 - 10.0;
    node.left = Val::Px(c + u * radius - 5.0);
    node.top = Val::Px(c - v * radius - 9.0);
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
        BlockType::Air => [0.0, 0.0, 0.0],
    }
}

fn update_minimap_image(
    time: Res<Time>,
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
    let step = MAP_RANGE_BLOCKS / MAP_PX as f32;
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

    // Biểu tượng các khu công trình: làng (vàng), đấu trường boss (đỏ). Ở xa thì bám theo rìa bản đồ.
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
