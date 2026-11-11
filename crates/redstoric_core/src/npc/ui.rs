#![allow(unused_imports)]
use super::*;

/// Tầm hiện bảng tên.
pub(super) const NAME_RANGE: f32 = 28.0;

/// Bảng tên (UI) bám theo một dân làng.
#[derive(Component)]
pub(super) struct Plate {
    pub(super) target: Entity,
}

/// Phần gợi ý "E Talk / T Trade" của bảng tên.
#[derive(Component)]
pub(super) struct PlatePrompt;

#[derive(Component)]
pub(super) struct DialogPanel;

#[derive(Component)]
pub(super) struct DialogTitle;

#[derive(Component)]
pub(super) struct DialogBody;

#[derive(Component)]
pub(super) struct DialogHint;

pub(super) fn ui_text(text: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// Đọc to câu thoại mỗi khi hộp thoại mở hoặc sang câu mới.
pub(super) fn dialog_voice(
    dialog: Res<NpcDialog>,
    villagers: Query<(&Villager, &GlobalTransform)>,
    mut voice: ResMut<crate::voice::VoiceQueue>,
    mut last: Local<(Option<Entity>, usize)>,
) {
    if dialog.mode != DialogMode::Talk {
        last.0 = None;
        return;
    }
    let now = (dialog.target, dialog.line);
    if *last == now {
        return;
    }
    *last = now;
    let Some((v, gt)) = dialog.target.and_then(|e| villagers.get(e).ok()) else {
        return;
    };
    let lines = v.job.lines();
    if let Some(line) = lines.get(dialog.line) {
        voice.say(&v.name, v.female, v.age, gt.translation(), line);
    }
}

pub(super) fn update_plates(
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

pub(super) fn spawn_dialog_panel(mut commands: Commands) {
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

pub(super) fn refresh_dialog_panel(
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
