#![allow(unused_imports)]
use super::*;
use super::templates::Kind::*;

/// Dân làng đang đứng gần người chơi.
pub(super) struct Giver {
    pub(super) name: String,
    pub(super) female: bool,
    pub(super) age: u8,
    pub(super) job: Profession,
    pub(super) pos: Vec3,
    pub(super) site: Site,
}

/// Tạo nhiệm vụ do dân làng `g` giao, theo nghề của họ.
pub(super) fn make_quest(generator: &WorldGenerator, g: &Giver, log: &QuestLog) -> Quest {
    let site = &g.site;
    let rep = log.rep(site.id);
    let sites = generator.plan_sites();
    let h = mix(name_hash(&g.name), log.issued.wrapping_add(rep.wrapping_mul(7)));

    let villages: Vec<&Site> = sites.iter().filter(|s| s.kind == SiteKind::Village && s.id != site.id).collect();
    let mut dungeons: Vec<&Site> = sites.iter().filter(|s| s.kind == SiteKind::BossArena).collect();
    dungeons.sort_by_key(|s| (s.x - site.x).abs() + (s.z - site.z).abs());
    let arenas: Vec<&Site> = dungeons.iter().copied().filter(|s| !log.slain_sites.contains(&s.id)).collect();

    let can_slay = !arenas.is_empty()
        && log.issued > 0
        && (rep >= SLAY_REP || matches!(g.job, Profession::Noble | Profession::Guard));
    let usable = |k: Kind| match k {
        Kind::Slay => can_slay,
        Kind::Courier => !villages.is_empty(),
        Kind::Scout => !dungeons.is_empty(),
        _ => true,
    };
    let list = templates(g.job);
    let start = h as usize % list.len();
    let tpl = (0..list.len())
        .map(|i| &list[(start + i) % list.len()])
        .find(|t| usable(t.kind))
        .unwrap_or(&GENERIC);

    let tier = site.tier.max(1) as u32;
    let mut boss = "";
    let (goal, base, place, n) = match tpl.kind {
        Kind::Hunt(sp) => {
            let (need, pay) = if sp == WOLF { (1 + tier, 25) } else { (2 + tier, 12) };
            (QuestGoal::Hunt { site: site.id, species: sp, need }, 15 + pay * need, site.name(), need)
        }
        Kind::Slay => {
            let a = arenas[0];
            boss = BossKind::for_tier(a.tier, a.seed).name();
            (QuestGoal::SlayBoss { site: a.id }, 150 + 100 * a.tier as u32, a.name(), 1)
        }
        Kind::Courier => {
            let to = villages[(h >> 8) as usize % villages.len()];
            (QuestGoal::Courier { to: to.id }, 40 + (sites_dist(site, to) / 8.0) as u32, to.name(), 1)
        }
        Kind::Scout => {
            let a = dungeons[(h >> 8) as usize % dungeons.len().min(3)];
            boss = BossKind::for_tier(a.tier, a.seed).name();
            (QuestGoal::Scout { site: a.id }, 50 + (sites_dist(site, a) / 6.0) as u32, a.name(), 1)
        }
        Kind::Fetch => {
            let (x, z) = fetch_spot(generator, site, h);
            let d = (((x - site.x).pow(2) + (z - site.z).pow(2)) as f32).sqrt();
            (QuestGoal::Fetch { x, z, need: 3 }, 35 + (d / 5.0) as u32, format!("({x}, {z})"), 3)
        }
        Kind::Patrol => (QuestGoal::Patrol { site: site.id, need: 3 }, 45 + 10 * tier, site.name(), 3),
    };

    let reward = base * (100 + 10 * rep.min(10)) / 100;
    let goods = g.job.goods();
    let reward_item = if goods.is_empty() { None } else { Some(goods[(h >> 16) as usize % goods.len()].name) };
    let intro = tpl
        .intro
        .replace("{n}", &n.to_string())
        .replace("{place}", &place)
        .replace("{boss}", boss)
        .replace("{item}", tpl.item);

    Quest {
        giver: site.id,
        giver_name: g.name.clone(),
        job: g.job,
        title: tpl.title,
        intro,
        voice: tpl.intro,
        item: tpl.item,
        goal,
        progress: 0,
        done: false,
        reward,
        reward_item,
    }
}

/// Dân làng gần người chơi nhất trong tầm nhận việc.
pub(super) fn giver_in_reach(
    player: Vec3,
    villagers: &Query<(&GlobalTransform, &Villager), Without<Hostile>>,
    generator: &WorldGenerator,
) -> Option<Giver> {
    let (gt, v) = villagers
        .iter()
        .filter(|(g, _)| g.translation().distance(player) <= OFFER_RANGE)
        .min_by(|a, b| {
            let da = a.0.translation().distance(player);
            let db = b.0.translation().distance(player);
            da.total_cmp(&db)
        })?;
    let p = gt.translation();
    let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
    let site = generator
        .sites_in_area(x - 1, z - 1, x + 1, z + 1)
        .into_iter()
        .find(|s| s.kind == SiteKind::Village && s.contains(x, z, 8))?;
    Some(Giver { name: v.name.clone(), female: v.female, age: v.age, job: v.job, pos: p, site })
}

/// Trả thưởng, cộng uy tín và đóng nhiệm vụ.
pub(super) fn finish(generator: &WorldGenerator, log: &mut QuestLog, wallet: &mut Wallet, inv: &mut Inventory) {
    let Some(q) = log.active.take() else {
        return;
    };
    wallet.0 += q.reward;
    if !q.item.is_empty() {
        remove_item(inv, q.item, q.progress);
    }
    let mut text = format!("{}: \"Thank you!\"  +{} coins", q.giver_name, q.reward);
    if let Some(item) = q.reward_item {
        add_item(inv, item, 1);
        text.push_str(&format!(", +1 {item}"));
    }
    let rep = log.reputation.entry(q.giver).or_insert(0);
    *rep += 1;
    let rep = *rep;
    text.push_str(&format!("\nReputation with {}: {}", site_name(generator, q.giver), rep));
    if rep == SLAY_REP {
        text.push_str(" - they now trust you with dungeon bounties!");
    }
    log.completed += 1;
    log.say(text);
}

/// **Q** gần dân làng: nhận / trả việc. **X** hai lần: bỏ việc.
pub fn quest_accept_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<NpcDialog>,
    generator: Res<WorldGenerator>,
    player: Query<&GlobalTransform, With<Character>>,
    villagers: Query<(&GlobalTransform, &Villager), Without<Hostile>>,
    markers: Query<(Entity, &QuestMarker, &GlobalTransform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut wallet: ResMut<Wallet>,
    mut inventory: ResMut<Inventory>,
    mut log: ResMut<QuestLog>,
    mut voice: ResMut<VoiceQueue>,
) {
    if dialog.is_open() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyX) && log.active.is_some() {
        if log.abandon_t > 0.0 {
            log.abandon_t = 0.0;
            if let Some(q) = log.active.take() {
                if !q.item.is_empty() {
                    remove_item(&mut inventory, q.item, q.progress);
                }
                log.say(format!("Quest abandoned: {}", q.title));
            }
            clear_markers(&mut commands, &markers);
        } else {
            log.abandon_t = ABANDON_WINDOW;
            log.say("Press [X] again to abandon the quest.".to_string());
        }
        return;
    }
    if !keys.just_pressed(KeyCode::KeyQ) {
        return;
    }
    let Ok(p) = player.single() else {
        return;
    };
    let Some(giver) = giver_in_reach(p.translation(), &villagers, &generator) else {
        return;
    };

    if let Some(q) = log.active.clone() {
        let is_giver = q.giver_name == giver.name && q.giver == giver.site.id;
        if is_giver && q.done {
            finish(&generator, &mut log, &mut wallet, &mut inventory);
            clear_markers(&mut commands, &markers);
            voice.say(&giver.name, giver.female, giver.age, giver.pos, "Thank you!");
        } else if is_giver {
            voice.say(&giver.name, giver.female, giver.age, giver.pos, q.voice);
            log.say(format!("{}: \"{}\"", q.giver_name, q.intro));
        } else {
            log.say(format!("{}: \"You're busy helping {} already.\"", giver.name, q.giver_name));
        }
        return;
    }

    let quest = make_quest(&generator, &giver, &log);
    spawn_quest_markers(&mut commands, &mut meshes, &mut mats, &generator, &quest);
    log.say(format!("{} the {}: \"{}\"", giver.name, giver.job.label(), quest.intro));
    voice.say(&giver.name, giver.female, giver.age, giver.pos, quest.voice);
    log.issued += 1;
    log.active = Some(quest);
}

/// Đếm thú/boss bị hạ, nhặt đồ, chạm cột mốc, tới nơi.
pub fn quest_progress_system(
    mut commands: Commands,
    generator: Res<WorldGenerator>,
    player: Query<&GlobalTransform, With<Character>>,
    animals: Query<(Entity, &Agent, &Animal, &GlobalTransform)>,
    bosses: Query<(Entity, &Boss, &GlobalTransform)>,
    markers: Query<(Entity, &QuestMarker, &GlobalTransform)>,
    mut wallet: ResMut<Wallet>,
    mut inventory: ResMut<Inventory>,
    mut log: ResMut<QuestLog>,
    mut counted: Local<HashSet<Entity>>,
) {
    counted.retain(|e| animals.get(*e).is_ok() || bosses.get(*e).is_ok());
    let ppos = player.single().ok().map(|p| p.translation());
    let mut msg: Option<String> = None;

    // Thú chết
    for (entity, agent, animal, tf) in &animals {
        if agent.mood != Mood::Dead || !counted.insert(entity) {
            continue;
        }
        let pos = tf.translation();
        let Some(q) = log.active.as_mut() else {
            continue;
        };
        if let QuestGoal::Hunt { site, species, need } = q.goal {
            let right = species.map_or(true, |s| s == animal.species);
            let near = generator.site_by_id(site).map_or(false, |s| site_dist(s, pos) <= HUNT_RADIUS);
            if right && near && !q.done {
                q.progress += 1;
                if q.progress >= need {
                    q.done = true;
                    msg = Some(format!("Objective complete! Return to {}.", q.giver_name));
                }
            }
        }
    }

    // Boss chết
    for (entity, boss, tf) in &bosses {
        if !matches!(boss.state, BossState::Dying(_)) || !counted.insert(entity) {
            continue;
        }
        let pos = tf.translation();
        let arena = generator
            .plan_sites()
            .iter()
            .find(|s| s.kind == SiteKind::BossArena && site_dist(s, pos) <= BOSS_SITE_RADIUS)
            .map(|s| s.id);
        let Some(arena) = arena else {
            continue;
        };
        log.slain_sites.insert(arena);
        if let Some(q) = log.active.as_mut() {
            if q.goal == (QuestGoal::SlayBoss { site: arena }) && !q.done {
                q.done = true;
                msg = Some(format!("The {} has fallen! Return to {}.", boss.kind.name(), q.giver_name));
                clear_markers(&mut commands, &markers);
            }
        }
    }

    let Some(p) = ppos else {
        if let Some(m) = msg {
            log.say(m);
        }
        return;
    };

    // Nhặt đồ / chạm cột mốc
    if let Some(q) = log.active.as_mut() {
        if !q.done {
            for (e, m, gt) in &markers {
                if !(m.pickup || m.touch) {
                    continue;
                }
                let mp = gt.translation();
                let flat = Vec2::new(mp.x - p.x, mp.z - p.z).length();
                let range = if m.pickup { PICKUP_RANGE } else { PATROL_RANGE };
                if flat > range || (mp.y - p.y).abs() > 5.0 {
                    continue;
                }
                commands.entity(e).despawn();
                q.progress += 1;
                if m.pickup && !q.item.is_empty() {
                    add_item(&mut inventory, q.item, 1);
                }
                let need = match q.goal {
                    QuestGoal::Fetch { need, .. } | QuestGoal::Patrol { need, .. } => need,
                    _ => 1,
                };
                msg = Some(if q.progress >= need {
                    q.done = true;
                    format!("Objective complete! Return to {}.", q.giver_name)
                } else if m.pickup {
                    format!("Picked up {}  ({}/{})", q.item, q.progress, need)
                } else {
                    format!("Post reached  ({}/{})", q.progress, need)
                });
            }
        }
    }

    // Courier: tới nơi là xong; Scout: tới gần là xong mục tiêu
    let goal = log.active.as_ref().map(|q| (q.goal, q.done));
    match goal {
        Some((QuestGoal::Courier { to }, _)) => {
            if generator.site_by_id(to).map_or(false, |s| site_dist(s, p) <= ARRIVE_RANGE) {
                clear_markers(&mut commands, &markers);
                finish(&generator, &mut log, &mut wallet, &mut inventory);
                return;
            }
        }
        Some((QuestGoal::Scout { site }, false)) => {
            if generator.site_by_id(site).map_or(false, |s| site_dist(s, p) <= SCOUT_RANGE) {
                if let Some(q) = log.active.as_mut() {
                    q.done = true;
                    msg = Some(format!("You've seen enough. Report back to {}.", q.giver_name));
                }
                clear_markers(&mut commands, &markers);
            }
        }
        _ => {}
    }
    if let Some(m) = msg {
        log.say(m);
    }
}
