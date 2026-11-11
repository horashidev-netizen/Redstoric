//! Thanh máu: máu người chơi (góc dưới trái) và máu boss (giữa phía trên, chỉ hiện khi giao chiến).

use bevy::prelude::*;
use redstoric_core::Character;

use crate::boss::{Boss, BossState};
use crate::types::Health;

#[derive(Component)]
pub struct PlayerBarFill;
#[derive(Component)]
pub struct PlayerHpText;
#[derive(Component)]
pub struct BossBarRoot;
#[derive(Component)]
pub struct BossBarFill;
#[derive(Component)]
pub struct BossNameText;

fn label(text: &str, size: f32) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(Color::WHITE),
    )
}

pub fn setup_hud(mut commands: Commands) {
    // Máu người chơi
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                width: Val::Px(260.0),
                height: Val::Px(24.0),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.85)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.80, 0.15, 0.18)),
                PlayerBarFill,
            ));
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                label("HP ", 15.0),
            ))
            .with_children(|text| {
                text.spawn((
                    TextSpan::new("100/100"),
                    TextFont { font_size: FontSize::Px(15.0), ..default() },
                    TextColor(Color::WHITE),
                    PlayerHpText,
                ));
            });
        });

    // Máu boss
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
            BossBarRoot,
        ))
        .with_children(|root| {
            root.spawn((label("Obsidian Guardian", 18.0), BossNameText));
            root.spawn((
                Node {
                    width: Val::Px(460.0),
                    height: Val::Px(18.0),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.85)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.65, 0.10, 0.45)),
                    BossBarFill,
                ));
            });
        });
}

pub fn update_hud(
    player: Query<&Health, With<Character>>,
    bosses: Query<(&Boss, &Health), Without<Character>>,
    mut player_fill: Query<&mut Node, (With<PlayerBarFill>, Without<BossBarFill>)>,
    mut boss_fill: Query<&mut Node, (With<BossBarFill>, Without<PlayerBarFill>)>,
    mut hp_text: Query<&mut TextSpan, With<PlayerHpText>>,
    mut boss_root: Query<&mut Visibility, With<BossBarRoot>>,
    mut boss_name: Query<&mut Text, With<BossNameText>>,
) {
    if let Ok(health) = player.single() {
        let pct = (health.current / health.max * 100.0).clamp(0.0, 100.0);
        for mut node in &mut player_fill {
            node.width = Val::Percent(pct);
        }
        for mut span in &mut hp_text {
            **span = format!("{:.0}/{:.0}", health.current, health.max);
        }
    }

    // Hiện thanh máu của boss đang giao chiến
    let engaged = bosses
        .iter()
        .find(|(boss, _)| !matches!(boss.state, BossState::Idle | BossState::Dying(_)));
    for mut visibility in &mut boss_root {
        *visibility = if engaged.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some((boss, health)) = engaged {
        for mut name in &mut boss_name {
            if name.0 != boss.kind.name() {
                name.0 = boss.kind.name().to_string();
            }
        }
        let pct = (health.current / health.max * 100.0).clamp(0.0, 100.0);
        for mut node in &mut boss_fill {
            node.width = Val::Percent(pct);
        }
    }
}
