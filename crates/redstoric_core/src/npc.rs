//! Dân làng (NPC) tương tác kiểu Veloren.
//!
//! * Voxel đặt `VillagerSpawn` ở các làng; ở đây biến nó thành NPC: dựng model người từ các mảnh
//!   `figure` (ngoại hình theo hạt giống), gắn tên + nghề.
//! * Bảng tên bay trên đầu NPC (đến 28 khối). Lại gần (<= 3.5 khối) thì hiện gợi ý **E Talk / T Trade**.
//! * **E**: nói chuyện (bấm tiếp để đọc câu kế). **T**: mở cửa hàng, phím **1-4** để mua bằng xu.
//!   **Esc** hoặc bước ra xa để đóng.
//! * Giao dịch hiện chỉ là bộ đếm xu + túi đồ tối giản (chưa có kho đồ thật).

use bevy::prelude::*;
use redstoric_common::npc::{person_name, Profession};
use redstoric_voxel::VillagerSpawn;

use crate::character_creator::{spawn_humanoid, AppearanceCatalog};
use crate::{Character, OrbitCamera};

/// Tầm hiện bảng tên.
const NAME_RANGE: f32 = 28.0;
/// Tầm bắt đầu tương tác.
const INTERACT_RANGE: f32 = 3.5;
/// Tầm NPC quay mặt về phía người chơi.
const FACE_RANGE: f32 = 8.0;
const START_COINS: u32 = 100;

/// Dân làng đã dựng xong.
#[derive(Component)]
pub struct Villager {
    pub name: String,
    pub job: Profession,
}

/// Bảng tên (UI) bám theo một dân làng.
#[derive(Component)]
struct Plate {
    target: Entity,
}

/// Phần gợi ý "E Talk / T Trade" của bảng tên.
#[derive(Component)]
struct PlatePrompt;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DialogMode {
    #[default]
    Closed,
    Talk,
    Trade,
}

/// Trạng thái hội thoại / giao dịch hiện tại. Hệ thống chiến đấu đọc `is_open` để không chém khi đang mở.
#[derive(Resource, Default)]
pub struct NpcDialog {
    pub target: Option<Entity>,
    pub mode: DialogMode,
    pub line: usize,
}

impl NpcDialog {
    pub fn is_open(&self) -> bool {
        self.mode != DialogMode::Closed
    }
}

/// Dân làng gần người chơi nhất trong tầm tương tác.
#[derive(Resource, Default)]
struct Nearby(Option<Entity>);

#[derive(Resource)]
pub struct Wallet(pub u32);

/// Túi đồ tối giản: (tên, số lượng).
#[derive(Resource, Default)]
pub struct Inventory(pub Vec<(String, u32)>);

#[derive(Component)]
struct DialogPanel;
#[derive(Component)]
struct DialogTitle;
#[derive(Component)]
struct DialogBody;
#[derive(Component)]
struct DialogHint;

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NpcDialog>()
            .init_resource::<Nearby>()
            .init_resource::<Inventory>()
            .insert_resource(Wallet(START_COINS))
            .add_systems(Startup, spawn_dialog_panel)
            .add_systems(
                Update,
                (
                    init_villagers.run_if(resource_exists::<AppearanceCatalog>),
                    find_nearby,
                    dialog_input,
                    villager_face_player,
                    update_plates,
                    refresh_dialog_panel,
                )
                    .chain(),
            );
    }
}

fn ui_text(text: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn init_villagers(
    mut commands: Commands,
    new: Query<(Entity, &VillagerSpawn), Added<VillagerSpawn>>,
    catalog: Res<AppearanceCatalog>,
    asset_server: Res<AssetServer>,
) {
    for (entity, spawn) in &new {
        let job = Profession::from_index(spawn.job);
        let name = person_name(spawn.seed);
        let appearance = catalog.villager_appearance(spawn.seed);
        spawn_humanoid(&mut commands, entity, &appearance, &catalog, &asset_server, false);

        let title = format!("{name} ({})", job.label());
        commands.entity(entity).insert(Villager { name, job });

        // Bảng tên: đặt ngoài màn hình cho tới khi `update_plates` tính được vị trí.
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(220.0),
                    left: Val::Px(-1000.0),
                    top: Val::Px(-1000.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Visibility::Hidden,
                Plate { target: entity },
            ))
            .with_children(|plate| {
                plate
                    .spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                    ))
                    .with_children(|n| {
                        n.spawn(ui_text(&title, 18.0, Color::WHITE));
                    });
                plate
                    .spawn((
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                        PlatePrompt,
                    ))
                    .with_children(|n| {
                        n.spawn(ui_text("E  Talk", 16.0, Color::srgb(1.0, 0.92, 0.55)));
                        n.spawn(ui_text("T  Trade", 16.0, Color::srgb(1.0, 0.92, 0.55)));
                    });
            });
    }
}

fn find_nearby(
    player: Query<&GlobalTransform, With<Character>>,
    villagers: Query<(Entity, &GlobalTransform), With<Villager>>,
    mut nearby: ResMut<Nearby>,
) {
    let Ok(p) = player.single() else {
        nearby.0 = None;
        return;
    };
    let pos = p.translation();
    let mut best: Option<(f32, Entity)> = None;
    for (entity, g) in &villagers {
        let d = g.translation().distance(pos);
        if d <= INTERACT_RANGE && best.map_or(true, |(bd, _)| d < bd) {
            best = Some((d, entity));
        }
    }
    nearby.0 = best.map(|(_, e)| e);
}

fn dialog_input(
    keys: Res<ButtonInput<KeyCode>>,
    nearby: Res<Nearby>,
    mut dialog: ResMut<NpcDialog>,
    villagers: Query<&Villager>,
    mut wallet: ResMut<Wallet>,
    mut inventory: ResMut<Inventory>,
) {
    if dialog.is_open() {
        let target_ok = dialog.target.is_some() && dialog.target == nearby.0;
        if keys.just_pressed(KeyCode::Escape) || !target_ok {
            *dialog = NpcDialog::default();
            return;
        }
        let Some(villager) = dialog.target.and_then(|e| villagers.get(e).ok()) else {
            *dialog = NpcDialog::default();
            return;
        };
        match dialog.mode {
            DialogMode::Talk => {
                if keys.just_pressed(KeyCode::KeyE) {
                    dialog.line += 1;
                    if dialog.line >= villager.job.lines().len() {
                        *dialog = NpcDialog::default();
                    }
                }
            }
            DialogMode::Trade => {
                let goods = villager.job.goods();
                for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4]
                    .into_iter()
                    .enumerate()
                {
                    if !keys.just_pressed(key) {
                        continue;
                    }
                    let Some(good) = goods.get(i) else { continue };
                    if wallet.0 < good.price {
                        continue;
                    }
                    wallet.0 -= good.price;
                    match inventory.0.iter_mut().find(|(n, _)| n == good.name) {
                        Some((_, count)) => *count += 1,
                        None => inventory.0.push((good.name.to_string(), 1)),
                    }
                }
            }
            DialogMode::Closed => {}
        }
        return;
    }

    let Some(target) = nearby.0 else { return };
    if keys.just_pressed(KeyCode::KeyE) {
        *dialog = NpcDialog { target: Some(target), mode: DialogMode::Talk, line: 0 };
    } else if keys.just_pressed(KeyCode::KeyT) {
        *dialog = NpcDialog { target: Some(target), mode: DialogMode::Trade, line: 0 };
    }
}

/// Dân làng quay mặt về phía người chơi khi lại gần (model nhìn về -Z).
fn villager_face_player(
    player: Query<&GlobalTransform, With<Character>>,
    mut villagers: Query<(&GlobalTransform, &mut Transform), With<Villager>>,
) {
    let Ok(p) = player.single() else { return };
    let pos = p.translation();
    for (g, mut t) in &mut villagers {
        let d = pos - g.translation();
        let flat = d.x * d.x + d.z * d.z;
        if flat < 0.01 || flat > FACE_RANGE * FACE_RANGE {
            continue;
        }
        let target = Quat::from_rotation_y((-d.x).atan2(-d.z));
        t.rotation = t.rotation.slerp(target, 0.15);
    }
}

fn update_plates(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    player: Query<&GlobalTransform, With<Character>>,
    villagers: Query<&GlobalTransform, With<Villager>>,
    nearby: Res<Nearby>,
    dialog: Res<NpcDialog>,
    mut plates: Query<(Entity, &Plate, &mut Node, &mut Visibility, &Children)>,
    mut prompts: Query<&mut Node, (With<PlatePrompt>, Without<Plate>)>,
) {
    let (Ok((cam, cam_tf)), Ok(player_tf)) = (camera.single(), player.single()) else {
        return;
    };
    let player_pos = player_tf.translation();
    for (plate_entity, plate, mut node, mut vis, children) in &mut plates {
        let Ok(g) = villagers.get(plate.target) else {
            commands.entity(plate_entity).despawn();
            continue;
        };
        let world = g.translation();
        let show_prompt = nearby.0 == Some(plate.target) && !dialog.is_open();

        for child in children.iter() {
            if let Ok(mut prompt) = prompts.get_mut(child) {
                prompt.display = if show_prompt { Display::Flex } else { Display::None };
            }
        }

        let in_range = world.distance(player_pos) <= NAME_RANGE;
        match cam.world_to_viewport(cam_tf, world + Vec3::Y * 2.4) {
            Ok(screen) if in_range => {
                let height = if show_prompt { 70.0 } else { 28.0 };
                node.left = Val::Px(screen.x - 110.0);
                node.top = Val::Px(screen.y - height);
                *vis = Visibility::Visible;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

fn spawn_dialog_panel(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(120.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-300.0)),
                width: Val::Px(600.0),
                padding: UiRect::all(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.92)),
            DialogPanel,
        ))
        .with_children(|panel| {
            panel.spawn((ui_text("", 22.0, Color::srgb(1.0, 0.92, 0.55)), DialogTitle));
            panel.spawn((ui_text("", 18.0, Color::WHITE), DialogBody));
            panel.spawn((ui_text("", 15.0, Color::srgb(0.65, 0.70, 0.80)), DialogHint));
        });
}

fn refresh_dialog_panel(
    dialog: Res<NpcDialog>,
    wallet: Res<Wallet>,
    inventory: Res<Inventory>,
    villagers: Query<&Villager>,
    mut panel: Query<&mut Node, With<DialogPanel>>,
    mut title: Query<&mut Text, (With<DialogTitle>, Without<DialogBody>, Without<DialogHint>)>,
    mut body: Query<&mut Text, (With<DialogBody>, Without<DialogTitle>, Without<DialogHint>)>,
    mut hint: Query<&mut Text, (With<DialogHint>, Without<DialogTitle>, Without<DialogBody>)>,
) {
    let (Ok(mut panel), Ok(mut title), Ok(mut body), Ok(mut hint)) =
        (panel.single_mut(), title.single_mut(), body.single_mut(), hint.single_mut())
    else {
        return;
    };
    let villager = dialog.target.and_then(|e| villagers.get(e).ok());
    let (true, Some(villager)) = (dialog.is_open(), villager) else {
        panel.display = Display::None;
        return;
    };
    panel.display = Display::Flex;

    match dialog.mode {
        DialogMode::Talk => {
            let lines = villager.job.lines();
            title.0 = format!("{} ({})", villager.name, villager.job.label());
            body.0 = lines[dialog.line.min(lines.len() - 1)].to_string();
            hint.0 = format!(
                "[E] {}    [Esc] Close",
                if dialog.line + 1 < lines.len() { "Continue" } else { "Goodbye" }
            );
        }
        DialogMode::Trade => {
            title.0 = format!("{}'s wares      Coins: {}", villager.name, wallet.0);
            let mut text = String::new();
            for (i, good) in villager.job.goods().iter().enumerate() {
                text.push_str(&format!("[{}] {:<20} {} coins\n", i + 1, good.name, good.price));
            }
            if inventory.0.is_empty() {
                text.push_str("\nBag: empty");
            } else {
                let items: Vec<String> = inventory.0.iter().map(|(n, c)| format!("{n} x{c}")).collect();
                text.push_str(&format!("\nBag: {}", items.join(", ")));
            }
            body.0 = text;
            hint.0 = "[1-4] Buy    [Esc] Close".to_string();
        }
        DialogMode::Closed => {}
    }
}
