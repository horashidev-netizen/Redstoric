//! Tải trước toàn bộ thế giới (vì thế giới hữu hạn).
//!
//! Lúc vào game, mọi chunk của hòn đảo được sinh một lần trên các luồng worker kèm màn hình
//! tải có thanh tiến độ. Dữ liệu khối và danh sách prop được giữ trong bộ nhớ, nên khi chơi:
//!
//! * không còn sinh địa hình / cây / làng giữa chừng (hết giật khi chạy sang vùng mới);
//! * chunk đi xa chỉ bị dỡ mesh chứ không mất dữ liệu, va chạm và minimap luôn đúng trên cả đảo;
//! * quay lại vùng cũ chỉ tốn công dựng mesh.
//!
//! Chỉ phần *đang nhìn thấy* được dựng mesh và spawn (xem `systems::chunk_streaming_system`),
//! vì vẽ cả đảo cùng lúc sẽ làm FPS tụt chứ không tăng.
//!
//! Đặt biến môi trường `REDSTORIC_NO_PRELOAD=1` để tắt (quay lại sinh theo yêu cầu, tiện khi dev).

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use redstoric_common::{chunk_in_world, WORLD_CHUNKS_HALF};

use crate::chunk::ChunkData;
use crate::map::VoxelWorldMap;
use crate::prop::PropSpawnData;
use crate::WorldGenerator;

/// Số chunk mỗi tác vụ worker (đủ nhỏ để thanh tiến độ mượt, đủ lớn để ít tốn điều phối).
const BATCH: usize = 32;

type BatchResult = Vec<((i32, i32), ChunkData, Vec<PropSpawnData>)>;

#[derive(Resource)]
pub struct WorldPreload {
    pub total: usize,
    done: Arc<AtomicUsize>,
    tasks: Vec<Task<BatchResult>>,
    pub finished: bool,
    /// Giữ dữ liệu chunk trong bộ nhớ khi dỡ mesh (false khi tắt tải trước).
    pub keep_data: bool,
    started: Instant,
}

impl WorldPreload {
    fn disabled() -> Self {
        Self {
            total: 0,
            done: Arc::new(AtomicUsize::new(0)),
            tasks: Vec::new(),
            finished: true,
            keep_data: false,
            started: Instant::now(),
        }
    }

    pub fn done(&self) -> usize {
        self.done.load(Ordering::Relaxed)
    }

    /// Tiến độ 0..1.
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            (self.done() as f32 / self.total as f32).min(1.0)
        }
    }
}

/// Props đã sinh sẵn của từng chunk; chờ chunk được dựng mesh thì mới spawn.
#[derive(Resource, Default)]
pub struct PreloadedProps(pub HashMap<(i32, i32), Vec<PropSpawnData>>);

/// Điều kiện chạy: thế giới đã tải xong (hoặc tính năng tải trước bị tắt).
pub fn world_ready(preload: Option<Res<WorldPreload>>) -> bool {
    preload.map_or(true, |p| p.finished)
}

#[derive(Component)]
pub(crate) struct LoadingScreen;
#[derive(Component)]
pub(crate) struct LoadingBar;
#[derive(Component)]
pub(crate) struct LoadingText;

pub(crate) fn start_preload(
    mut commands: Commands,
    generator: Res<WorldGenerator>,
    mut map: ResMut<VoxelWorldMap>,
) {
    if std::env::var_os("REDSTORIC_NO_PRELOAD").is_some() {
        commands.insert_resource(WorldPreload::disabled());
        return;
    }

    // Sinh từ tâm ra ngoài để vùng xuất phát xong trước.
    let n = WORLD_CHUNKS_HALF;
    let mut coords: Vec<(i32, i32)> = Vec::with_capacity((4 * n * n) as usize);
    for cx in -n..n {
        for cz in -n..n {
            coords.push((cx, cz));
        }
    }
    coords.sort_by_key(|&(x, z)| x * x + z * z);
    let total = coords.len();
    map.chunks.reserve(total);

    let done = Arc::new(AtomicUsize::new(0));
    let pool = AsyncComputeTaskPool::get();
    let mut tasks = Vec::with_capacity(total / BATCH + 1);
    for batch in coords.chunks(BATCH) {
        let batch = batch.to_vec();
        let world_gen = generator.clone();
        let done = done.clone();
        tasks.push(pool.spawn(async move {
            batch
                .into_iter()
                .map(|(cx, cz)| {
                    let (chunk, props) = world_gen.generate_chunk_with_props(cx, cz);
                    done.fetch_add(1, Ordering::Relaxed);
                    ((cx, cz), chunk, props)
                })
                .collect::<BatchResult>()
        }));
    }

    commands.insert_resource(WorldPreload {
        total,
        done,
        tasks,
        finished: false,
        keep_data: true,
        started: Instant::now(),
    });
    spawn_loading_screen(&mut commands);
}

fn spawn_loading_screen(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.08)),
            GlobalZIndex(1000),
            LoadingScreen,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("REDSTORIC"),
                TextFont { font_size: FontSize::Px(48.0), ..default() },
                TextColor(Color::srgb(1.0, 0.92, 0.55)),
            ));
            root.spawn((
                Node {
                    width: Val::Px(460.0),
                    height: Val::Px(14.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.16, 0.18, 0.24)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.75, 0.25)),
                    LoadingBar,
                ));
            });
            root.spawn((
                Text::new("Generating world..."),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::srgb(0.75, 0.80, 0.90)),
                LoadingText,
            ));
        });
}

/// Nhận các batch chunk đã sinh xong và cất vào bản đồ.
pub(crate) fn poll_preload(
    mut preload: ResMut<WorldPreload>,
    mut map: ResMut<VoxelWorldMap>,
    mut props: ResMut<PreloadedProps>,
) {
    if preload.finished {
        return;
    }
    preload.tasks.retain_mut(|task| match block_on(future::poll_once(task)) {
        Some(batch) => {
            for (coord, chunk, chunk_props) in batch {
                debug_assert!(chunk_in_world(coord.0, coord.1));
                map.chunks.insert(coord, chunk);
                if !chunk_props.is_empty() {
                    props.0.insert(coord, chunk_props);
                }
            }
            false
        }
        None => true,
    });
    if preload.tasks.is_empty() {
        preload.finished = true;
        info!(
            "Đã tải trước {} chunk trong {:.1}s",
            preload.total,
            preload.started.elapsed().as_secs_f32()
        );
    }
}

pub(crate) fn update_loading_screen(
    mut commands: Commands,
    preload: Res<WorldPreload>,
    screen: Query<Entity, With<LoadingScreen>>,
    mut bar: Query<&mut Node, With<LoadingBar>>,
    mut text: Query<&mut Text, With<LoadingText>>,
) {
    if preload.finished {
        for entity in &screen {
            commands.entity(entity).despawn();
        }
        return;
    }
    let p = preload.progress();
    if let Ok(mut node) = bar.single_mut() {
        node.width = Val::Percent(p * 100.0);
    }
    if let Ok(mut text) = text.single_mut() {
        let elapsed = preload.started.elapsed().as_secs_f32();
        let eta = if p > 0.02 { elapsed * (1.0 - p) / p } else { 0.0 };
        text.0 = format!(
            "Generating world...  {:.0}%   ({}/{})   ~{:.0}s left",
            p * 100.0,
            preload.done(),
            preload.total,
            eta
        );
    }
}
