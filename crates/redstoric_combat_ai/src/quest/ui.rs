#![allow(unused_imports)]
use super::*;

#[derive(Component)]
pub struct QuestText;

/// Dòng mục tiêu hiện tại.
pub(super) fn objective(generator: &WorldGenerator, q: &Quest) -> String {
    if q.done {
        return format!("Return to {} in {} and press [Q]", q.giver_name, site_name(generator, q.giver));
    }
    match q.goal {
        QuestGoal::Hunt { species, need, .. } => {
            format!("Hunt {} {}  [{}/{}]", need, species_name(species), q.progress.min(need), need)
        }
        QuestGoal::SlayBoss { site } => {
            let boss = generator
                .site_by_id(site)
                .map(|s| BossKind::for_tier(s.tier, s.seed).name())
                .unwrap_or("guardian");
            format!("Defeat the {} of {}", boss, site_name(generator, site))
        }
        QuestGoal::Courier { to } => format!("Deliver it to {}", site_name(generator, to)),
        QuestGoal::Fetch { need, .. } => format!("Collect {}  [{}/{}]", q.item, q.progress.min(need), need),
        QuestGoal::Scout { site } => format!("Scout {}", site_name(generator, site)),
        QuestGoal::Patrol { need, .. } => format!("Visit the glowing posts  [{}/{}]", q.progress.min(need), need),
    }
}

/// Hướng la bàn từ người chơi tới mục tiêu (-Z là Bắc).
pub(super) fn compass(dx: f32, dz: f32) -> &'static str {
    const DIRS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let a = dx.atan2(-dz).rem_euclid(std::f32::consts::TAU);
    DIRS[((a / (std::f32::consts::TAU / 8.0)).round() as usize) % 8]
}

pub fn setup_quest_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(44.0),
            left: Val::Px(10.0),
            width: Val::Px(360.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.04, 0.07, 0.65)),
        Text::new(""),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(Color::srgb(0.95, 0.92, 0.80)),
        QuestText,
    ));
}

/// Vẽ nhật ký nhiệm vụ.
pub fn quest_hud_system(
    time: Res<Time>,
    generator: Res<WorldGenerator>,
    player: Query<&GlobalTransform, With<Character>>,
    villagers: Query<(&GlobalTransform, &Villager), Without<Hostile>>,
    markers: Query<(Entity, &QuestMarker, &GlobalTransform)>,
    mut log: ResMut<QuestLog>,
    mut text: Query<&mut Text, With<QuestText>>,
    mut acc: Local<f32>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    log.message_t = (log.message_t - dt).max(0.0);
    log.abandon_t = (log.abandon_t - dt).max(0.0);
    // Dựng lại chữ 10 lần/giây là đủ (tìm làng + ghép chuỗi mỗi khung hình là lãng phí)
    *acc += dt;
    if *acc < 0.1 {
        return;
    }
    *acc = 0.0;
    let ppos = player.single().ok().map(|p| p.translation());

    let mut s = String::from("QUEST LOG\n");
    match &log.active {
        Some(q) => {
            s.push_str(&format!(
                "{}  -  {} ({}), {}\n",
                q.title,
                q.giver_name,
                q.job.label(),
                site_name(&generator, q.giver)
            ));
            s.push_str(&format!("> {}\n", objective(&generator, q)));
            if let Some(p) = ppos {
                // Mục tiêu: người giao việc (khi đã xong), cột mốc/vật phẩm gần nhất, hoặc tâm khu
                let target = if q.done {
                    villagers
                        .iter()
                        .find(|(_, v)| v.name == q.giver_name)
                        .map(|(g, _)| g.translation())
                        .or_else(|| generator.site_by_id(q.giver).map(|s| Vec3::new(s.x as f32, 0.0, s.z as f32)))
                } else {
                    markers
                        .iter()
                        .filter(|(_, m, _)| m.pickup || m.touch)
                        .map(|(_, _, g)| g.translation())
                        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
                        .or_else(|| match q.goal {
                            QuestGoal::Fetch { x, z, .. } => Some(Vec3::new(x as f32, 0.0, z as f32)),
                            _ => generator
                                .site_by_id(goal_site(q))
                                .map(|s| Vec3::new(s.x as f32, 0.0, s.z as f32)),
                        })
                };
                if let Some(tp) = target {
                    let (dx, dz) = (tp.x - p.x, tp.z - p.z);
                    s.push_str(&format!(
                        "Target ({}, {}) - {} m {}\n",
                        tp.x as i32,
                        tp.z as i32,
                        (dx * dx + dz * dz).sqrt() as i32,
                        compass(dx, dz)
                    ));
                }
            }
            s.push_str(&format!("Reward: {} coins", q.reward));
            if let Some(item) = q.reward_item {
                s.push_str(&format!(" + {item}"));
            }
            s.push_str("\n[X][X] abandon");
        }
        None => {
            s.push_str("No active quest.\nTalk to a villager and press [Q].");
        }
    }
    if let Some(p) = ppos {
        if let Some(g) = giver_in_reach(p, &villagers, &generator) {
            match &log.active {
                None => s.push_str(&format!(
                    "\n[Q] Ask {} the {} for work  (rep {})",
                    g.name,
                    g.job.label(),
                    log.rep(g.site.id)
                )),
                Some(q) if q.done && q.giver_name == g.name => s.push_str("\n[Q] Turn in quest"),
                _ => {}
            }
        }
    }
    s.push_str(&format!("\nCompleted: {}", log.completed));
    if log.message_t > 0.0 && !log.message.is_empty() {
        s.push_str(&format!("\n\n{}", log.message));
    }
    if text.0 != s {
        text.0 = s;
    }
}
