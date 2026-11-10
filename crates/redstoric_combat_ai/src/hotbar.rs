//! Ô vũ khí ở giữa dưới màn hình: hiện chức nghiệp và vũ khí của người chơi.
//! (Chữ không dấu vì font mặc định không có ký tự tiếng Việt.)
use bevy::prelude::*;
use redstoric_core::weapons::{Loadout, CLASS_LABELS};

#[derive(Component)]
pub struct HotbarText;

pub fn setup_hotbar(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.75)),
            ))
            .with_children(|slot| {
                slot.spawn((
                    Text::new("..."),
                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                    TextColor(Color::WHITE),
                    HotbarText,
                ));
                slot.spawn((
                    Text::new("Chuot trai: danh   |   F7: ngoai hinh"),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(Color::srgb(0.7, 0.7, 0.75)),
                ));
            });
        });
}

pub fn update_hotbar(loadout: Res<Loadout>, mut texts: Query<(&mut Text, &mut TextColor), With<HotbarText>>) {
    if !loadout.is_changed() {
        return;
    }
    let def = loadout.current();
    for (mut t, mut c) in &mut texts {
        **t = format!("{}", CLASS_LABELS[loadout.selected.min(CLASS_LABELS.len() - 1)]);
        let (r, g, b) = def.color;
        c.0 = Color::srgb(r, g, b);
    }
}
