#![allow(unused_imports)]
use super::*;

/// Bong bóng thoại hiện bao lâu (giây) và thấy được trong tầm bao xa.
pub(super) const SPEECH_TIME: f32 = 3.5;

pub(super) const SPEECH_RANGE: f32 = 32.0;

pub(super) const HELP_LINES: &[&str] = &[
    "Hang on, let me give you a hand!",
    "I've got your back, traveller!",
    "Stand with me, friend!",
    "Don't worry, I'll help you out!",
];

pub(super) const ALARM_LINES: &[&str] = &[
    "Monster in the village! To arms!",
    "Stay back, beast!",
    "Not in my village!",
    "Everyone inside, I'll handle this!",
];

pub(super) const KILL_LINES: &[&str] = &["That's one less beast.", "Phew... everyone alright?", "And stay out!"];

pub(super) const THANKS_LINES: &[&str] = &["Thanks for the help, friend!", "We make a fine team!", "Couldn't have done it without you."];

pub(super) const ANGRY_LINES: &[&str] = &["How dare you!", "Guards! Seize them!", "You'll regret that!", "Get out of our village!"];

pub(super) const CALM_LINES: &[&str] = &["Hmph. Don't try that again.", "And don't come back...", "Good riddance."];

/// Câu riêng theo nghề khi lao vào đánh thú dữ.
pub(super) fn job_shout(job: redstoric_common::npc::Profession) -> Option<&'static str> {
    use redstoric_common::npc::Profession as P;
    Some(match job {
        P::Guard => "For the village!",
        P::Farmer => "Off my fields, you mongrel!",
        P::Priest => "The light shields us!",
        P::Hunter => "Easy prey.",
        P::Blacksmith => "Taste my hammer!",
        P::Cook => "You'll make a fine stew!",
        P::Miner => "I've cracked harder rocks than you!",
        _ => return None,
    })
}

pub(super) fn pick_line(lines: &[&'static str], e: Entity, salt: u32) -> &'static str {
    let h = (e.to_bits() as u32).wrapping_mul(2654435761) ^ salt.wrapping_mul(40503);
    lines[(h >> 7) as usize % lines.len()]
}

/// Bong bóng thoại trên đầu dân làng.
#[derive(Component)]
pub struct Speech {
    pub(super) text: String,
    pub(super) t: f32,
    pub(super) voiced: bool,
}

impl Speech {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), t: SPEECH_TIME, voiced: false }
    }
}

/// Node UI của bong bóng (gắn trên dân làng đang nói).
#[derive(Component)]
pub struct SpeechBubble(Entity);

#[derive(Component)]
pub struct BubbleUi {
    pub(super) owner: Entity,
}

/// Vẽ bong bóng thoại: bám theo đầu người nói, ẩn khi xa / ngoài màn hình, hết giờ thì xoá.
pub fn villager_speech_system(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    player: Query<&GlobalTransform, With<Character>>,
    mut speakers: Query<(Entity, &GlobalTransform, &mut Speech, Option<&SpeechBubble>, Option<&Villager>)>,
    mut voice: ResMut<redstoric_core::voice::VoiceQueue>,
    mut bubbles: Query<(Entity, &BubbleUi, &mut Node, &mut Text, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    let cam = camera.single().ok();
    let ppos = player.single().ok().map(|p| p.translation());

    for (e, gt, mut sp, bubble, villager) in &mut speakers {
        if !sp.voiced {
            sp.voiced = true;
            if let Some(v) = villager {
                voice.say(&v.name, v.female, v.age, gt.translation(), &sp.text);
            }
        }
        sp.t -= dt;
        if sp.t <= 0.0 {
            if let Some(b) = bubble {
                commands.entity(b.0).try_despawn();
            }
            commands.entity(e).remove::<(Speech, SpeechBubble)>();
            continue;
        }
        let Some(b) = bubble else {
            let node = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        max_width: Val::Px(260.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 0.98, 0.9, 0.88)),
                    Text::new(sp.text.clone()),
                    TextFont { font_size: FontSize::Px(15.0), ..default() },
                    TextColor(Color::srgb(0.12, 0.10, 0.08)),
                    Visibility::Hidden,
                    BubbleUi { owner: e },
                ))
                .id();
            commands.entity(e).insert(SpeechBubble(node));
            continue;
        };
        let Ok((_, _, mut node, mut text, mut vis)) = bubbles.get_mut(b.0) else {
            continue;
        };
        if text.0 != sp.text {
            text.0 = sp.text.clone();
        }
        let world = gt.translation() + Vec3::Y * 3.0;
        let near = ppos.map_or(false, |p| p.distance(world) <= SPEECH_RANGE);
        match cam.map(|(c, ct)| c.world_to_viewport(ct, world)) {
            Some(Ok(screen)) if near => {
                node.left = Val::Px(screen.x - 90.0);
                node.top = Val::Px(screen.y - 34.0);
                *vis = Visibility::Visible;
            }
            _ => *vis = Visibility::Hidden,
        }
    }

    // Người nói đã biến mất (bị hạ, chunk bị xoá) hoặc đã thôi nói
    for (b, ui, _, _, _) in &bubbles {
        if !speakers.contains(ui.owner) {
            commands.entity(b).try_despawn();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bảo vệ làng khỏi thú dữ
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use redstoric_common::npc::Profession;
    use redstoric_core::voice::voice_id;

    const JOBS: [Profession; 14] = [
        Profession::Farmer,
        Profession::Blacksmith,
        Profession::Alchemist,
        Profession::Merchant,
        Profession::Guard,
        Profession::Innkeeper,
        Profession::Hunter,
        Profession::Cook,
        Profession::Miner,
        Profession::Fisherman,
        Profession::Scholar,
        Profession::Priest,
        Profession::Bard,
        Profession::Noble,
    ];

    fn tables() -> [(&'static str, &'static [&'static str]); 6] {
        [
            ("HELP", HELP_LINES),
            ("ALARM", ALARM_LINES),
            ("KILL", KILL_LINES),
            ("THANKS", THANKS_LINES),
            ("ANGRY", ANGRY_LINES),
            ("CALM", CALM_LINES),
        ]
    }

    fn all_lines() -> Vec<&'static str> {
        let mut v: Vec<&'static str> = Vec::new();
        for (_, lines) in tables() {
            v.extend(lines.iter().copied());
        }
        for job in JOBS {
            if let Some(s) = job_shout(job) {
                v.push(s);
            }
        }
        v
    }

    #[test]
    fn line_tables_are_not_empty_and_fit_a_speech_bubble() {
        for (name, lines) in tables() {
            assert!(!lines.is_empty(), "bảng {name} rỗng");
        }
        for line in all_lines() {
            assert!(!line.trim().is_empty(), "có câu thoại rỗng");
            assert_eq!(line, line.trim(), "câu {line:?} có khoảng trắng thừa");
            assert!(line.is_ascii(), "câu {line:?} có ký tự ngoài ASCII");
            assert!(line.len() <= 120, "câu {line:?} quá dài cho bong bóng thoại");
        }
    }

    #[test]
    fn village_lines_are_in_voice_csv() {
        // `assets/` bị .gitignore: không có file thì bỏ qua.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/audio/voice/voice_lines.csv");
        let Ok(csv) = std::fs::read_to_string(path) else { return };
        for line in all_lines() {
            let id = voice_id(line);
            let prefix = format!("{id},");
            assert!(
                csv.lines().any(|l| l.starts_with(&prefix)),
                "câu {line:?} (id {id}) chưa có trong voice_lines.csv; chạy: python tools/export_voice_lines.py"
            );
        }
    }
}
