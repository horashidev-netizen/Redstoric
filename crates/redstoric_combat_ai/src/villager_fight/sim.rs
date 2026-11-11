#![allow(unused_imports)]
use super::*;

/// Dân làng trong bán kính này quanh người bị đánh sẽ cùng nổi giận.
pub(super) const ALARM_RADIUS: f32 = 70.0;

/// Bị kéo xa chỗ đứng ban đầu hơn khoảng này thì bỏ cuộc nhanh hơn.
pub(super) const LEASH: f32 = 60.0;

/// Người chơi xa hơn khoảng này thì bắt đầu nguôi giận.
pub(super) const CALM_DIST: f32 = 45.0;

/// Số giây không thấy người chơi gần để nguôi giận.
pub(super) const CALM_TIME: f32 = 10.0;

/// Thời gian vung tay trước khi đòn trúng.
pub(super) const WINDUP: f32 = 0.45;

pub(super) const RECOVER: f32 = 0.35;

pub(super) const DEATH_TIME: f32 = 2.5;

/// Cung thủ/pháp sư lùi lại khi người chơi gần hơn khoảng này.
pub(super) const KITE_DIST: f32 = 5.0;

/// Dân làng phát hiện thú dữ trong tầm này...
pub(super) const DEFEND_SIGHT: f32 = 22.0;

/// ...nếu nó ở trong vùng này quanh chỗ đứng của dân làng.
pub(super) const DEFEND_AREA: f32 = 55.0;

pub fn setup_villager_fx(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let glow = |m: &mut Assets<StandardMaterial>, base: Color, e: LinearRgba| {
        m.add(StandardMaterial { base_color: base, emissive: e, unlit: true, ..default() })
    };
    commands.insert_resource(VillagerFx {
        arrow: meshes.add(Cuboid::new(0.06, 0.06, 0.8)),
        orb: meshes.add(Sphere::new(0.28)),
        arrow_mat: mats.add(StandardMaterial { base_color: Color::srgb(0.45, 0.32, 0.2), ..default() }),
        fire_mat: glow(&mut mats, Color::srgb(1.0, 0.5, 0.15), LinearRgba::rgb(6.0, 2.0, 0.4)),
        holy_mat: glow(&mut mats, Color::srgb(1.0, 0.95, 0.6), LinearRgba::rgb(4.0, 3.8, 1.5)),
    });
}

/// Gắn chỉ số chiến đấu cho dân làng mới dựng.
pub fn villager_fighter_init(mut commands: Commands, new: Query<(Entity, &Villager), Added<Villager>>) {
    for (e, v) in &new {
        let profile = combat_profile(v.job);
        commands.entity(e).insert(Fighter {
            profile,
            hp: profile.hp,
            state: FightState::Idle,
            cooldown: 0.0,
            calm_t: 0.0,
            yaw: 0.0,
            home: None,
            arm: None,
            weapons: Vec::new(),
            linked: false,
            knock: Vec3::ZERO,
            target: None,
            scan_t: (e.to_bits() % 10) as f32 * 0.05,
        });
    }
}

/// Tìm vai phải + vũ khí trong cây model của dân làng (một lần) và ghi lại chỗ đứng ban đầu.
pub fn villager_link_parts(
    children: Query<&Children>,
    arms: Query<(), With<NpcArmRight>>,
    weapons: Query<(), With<NpcWeapon>>,
    mut fighters: Query<(Entity, &GlobalTransform, &mut Fighter)>,
) {
    for (e, gt, mut f) in &mut fighters {
        if f.home.is_none() && gt.translation() != Vec3::ZERO {
            f.home = Some(gt.translation());
        }
        if f.linked {
            continue;
        }
        f.weapons.clear();
        for d in children.iter_descendants(e) {
            if arms.contains(d) {
                f.arm = Some(d);
            }
            if weapons.contains(d) {
                f.weapons.push(d);
            }
        }
        f.linked = f.arm.is_some();
    }
}

/// Đòn của người chơi (cận chiến + đạn) trúng dân làng; báo động cả làng.
pub fn villager_hit_system(
    mut commands: Commands,
    loadout: Res<Loadout>,
    generator: Option<Res<WorldGenerator>>,
    player: Query<(&Transform, &Character, &MeleeAttacker)>,
    projs: Query<(Entity, &Transform, &Projectile)>,
    mut villagers: Query<(Entity, &GlobalTransform, &Villager, &mut Fighter, Option<&Hostile>)>,
    mut log: ResMut<QuestLog>,
    mut swung: Local<bool>,
) {
    let mut hits: Vec<(Entity, f32, Vec3)> = Vec::new();

    // Cận chiến: quét một lần mỗi nhát, cùng hình dạng vùng đánh với `melee_hit_detection_system`
    if let Ok((ptf, ch, att)) = player.single() {
        let def = loadout.current();
        let melee = matches!(def.kind, WeaponKind::Sword | WeaponKind::Axe | WeaponKind::Hammer);
        match ch.state {
            ActionState::Attacking(_) if melee => {
                let progress = 1.0 - ch.attack_timer / ch.attack_len.max(0.05);
                if progress >= def.release && !*swung {
                    *swung = true;
                    let ppos = ptf.translation;
                    let fwd = *ptf.forward();
                    let (centre, cone, push) = match def.kind {
                        WeaponKind::Sword => (ppos, Some(0.3), 6.0),
                        WeaponKind::Axe => (ppos, None, 7.0),
                        _ => (ppos + fwd * 2.0, None, 9.0),
                    };
                    for (e, gt, _, f, _) in &villagers {
                        if f.is_dead() {
                            continue;
                        }
                        let vp = gt.translation();
                        if (vp - centre).length() > att.range {
                            continue;
                        }
                        let dir = (vp - ppos).normalize_or_zero();
                        if let Some(min_dot) = cone {
                            if fwd.dot(dir) <= min_dot {
                                continue;
                            }
                        }
                        hits.push((e, att.damage, dir * push));
                    }
                }
            }
            _ => *swung = false,
        }
    }

    // Tên / cầu lửa của người chơi
    for (pe, ptf, pr) in &projs {
        if pr.visual || pr.kind == ProjKind::HealBolt {
            continue;
        }
        for (e, gt, _, f, _) in &villagers {
            if f.is_dead() {
                continue;
            }
            if (gt.translation() + Vec3::Y * 0.9).distance(ptf.translation) < 1.0 {
                hits.push((e, pr.damage, pr.vel.normalize_or_zero() * 5.0));
                commands.entity(pe).try_despawn();
                break;
            }
        }
    }
    if hits.is_empty() {
        return;
    }

    let mut alarms: Vec<Vec3> = Vec::new();
    for (e, dmg, push) in hits {
        let Ok((_, gt, v, mut f, _)) = villagers.get_mut(e) else {
            continue;
        };
        if f.is_dead() {
            continue;
        }
        f.hp = (f.hp - dmg).max(0.0);
        f.knock = Vec3::new(push.x, 0.0, push.z);
        f.calm_t = 0.0;
        if f.hp <= 0.0 {
            f.state = FightState::Dead(0.0);
            commands.entity(e).insert(Hostile);
            log.say(format!("{} has fallen.", v.name));
        } else {
            log.say(format!("{} ({}): {} / {} HP", v.name, v.job.label(), f.hp as i32, f.profile.hp as i32));
        }
        alarms.push(gt.translation());
    }

    for p in alarms {
        let mut newly = 0;
        for (e, gt, _, mut f, hostile) in &mut villagers {
            if f.is_dead() || gt.translation().distance(p) > ALARM_RADIUS {
                continue;
            }
            f.calm_t = 0.0;
            if hostile.is_none() {
                commands.entity(e).insert(Hostile).remove::<Defending>();
                f.target = None;
                let line = pick_line(ANGRY_LINES, e, newly);
                commands.entity(e).insert(Speech::new(line));
                newly += 1;
            }
            if f.state == FightState::Idle {
                f.state = FightState::Chase;
            }
        }
        if newly == 0 {
            continue;
        }
        // Cả làng nổi giận: trừ uy tín
        let site = generator.as_ref().and_then(|g| {
            let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
            g.sites_in_area(x - 1, z - 1, x + 1, z + 1)
                .into_iter()
                .find(|s| s.kind == SiteKind::Village && s.contains(x, z, 8))
        });
        if let Some(site) = site {
            let rep = log.reputation.entry(site.id).or_insert(0);
            *rep = rep.saturating_sub(1);
            let rep = *rep;
            log.say(format!("The people of {} take up arms against you!  (reputation {})", site.name(), rep));
        } else {
            log.say("The villagers take up arms against you!".to_string());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Lời thoại
// ---------------------------------------------------------------------------------------------

/// Tìm thú dữ lọt vào làng cho dân làng đánh; bỏ mục tiêu khi nó chết / chạy xa.
pub fn villager_defend_system(
    mut commands: Commands,
    time: Res<Time>,
    player: Query<&GlobalTransform, With<Character>>,
    animals: Query<(Entity, &Animal, &Agent, &GlobalTransform)>,
    mut villagers: Query<(Entity, &GlobalTransform, &Villager, &mut Fighter, Option<&Hostile>)>,
) {
    let dt = time.delta_secs();
    let ppos = player.single().ok().map(|p| p.translation());
    for (e, gt, v, mut f, hostile) in &mut villagers {
        if hostile.is_some() || f.is_dead() {
            continue;
        }
        f.scan_t -= dt;
        if f.scan_t > 0.0 {
            continue;
        }
        f.scan_t = 0.4;
        let vpos = gt.translation();
        let home = f.home.unwrap_or(vpos);
        let player_near = ppos.map_or(false, |p| p.distance(vpos) < 25.0);

        if let Some(t) = f.target {
            let alive = animals.get(t).ok().filter(|(_, _, ag, _)| ag.mood != Mood::Dead);
            match alive {
                Some((_, _, _, agt)) if agt.translation().distance(home) <= LEASH => continue,
                _ => {
                    // Mục tiêu đã chết (hoặc chạy mất): cất vũ khí, nói một câu
                    let killed = animals.get(t).map_or(false, |(_, _, ag, _)| ag.mood == Mood::Dead);
                    f.target = None;
                    f.state = FightState::Idle;
                    commands.entity(e).remove::<Defending>();
                    if killed {
                        let lines = if player_near { THANKS_LINES } else { KILL_LINES };
                        commands.entity(e).insert(Speech::new(pick_line(lines, e, 3)));
                    }
                    continue;
                }
            }
        }

        // Thú dữ còn sống gần nhất trong tầm nhìn, ở trong khu làng
        let found = animals
            .iter()
            .filter(|(_, a, ag, _)| a.species.alignment() == Alignment::Enemy && ag.mood != Mood::Dead)
            .map(|(ae, _, _, agt)| (ae, agt.translation()))
            .filter(|(_, ap)| ap.distance(vpos) <= DEFEND_SIGHT && ap.distance(home) <= DEFEND_AREA)
            .min_by(|a, b| a.1.distance(vpos).total_cmp(&b.1.distance(vpos)));
        if let Some((ae, _)) = found {
            f.target = Some(ae);
            f.state = FightState::Chase;
            f.calm_t = 0.0;
            let line = if player_near {
                pick_line(HELP_LINES, e, 1)
            } else {
                job_shout(v.job).unwrap_or_else(|| pick_line(ALARM_LINES, e, 2))
            };
            commands.entity(e).insert((Defending, Speech::new(line)));
        }
    }
}

/// Bước đi có kiểm tra địa hình (như `villager_wander`): trả về độ dời thế giới, hoặc None nếu vướng.
pub(super) fn step(map: &VoxelWorldMap, from: Vec3, dir: Vec3, dist: f32) -> Option<Vec3> {
    let nx = from.x + dir.x * dist;
    let nz = from.z + dir.z * dist;
    let (h, blocked) = map.get_walk_height(nx, nz, from.y);
    if blocked || h < 0.5 || h - from.y > 1.2 || from.y - h > 3.0 {
        return None;
    }
    Some(Vec3::new(nx - from.x, h - from.y, nz - from.z))
}

/// Dân làng chiến đấu: nổi giận thì đánh người chơi, đang bảo vệ làng thì đánh thú dữ.
/// Đuổi, vung vũ khí, bắn; nguôi giận khi người chơi đi xa.
#[allow(clippy::too_many_arguments)]
pub fn villager_combat_ai(
    mut commands: Commands,
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    fx: Option<Res<VillagerFx>>,
    mut player: Query<(&Transform, &mut Health, &Character), (Without<Villager>, Without<Animal>)>,
    mut villagers: Query<
        (Entity, &GlobalTransform, &mut Transform, &mut Fighter, Option<&Hostile>),
        (With<Villager>, Or<(With<Hostile>, With<Defending>)>, Without<Character>, Without<Animal>),
    >,
    mut animals: Query<
        (&Transform, &mut Health, Option<&mut Knockback>, Option<&mut HitFlash>),
        (With<Animal>, Without<Villager>, Without<Character>),
    >,
    mut arms: Query<&mut Transform, (With<NpcArmRight>, Without<Villager>, Without<Character>, Without<Animal>)>,
) {
    let (Some(map), Some(fx)) = (map, fx) else {
        return;
    };
    let Ok((ptf, mut php, pch)) = player.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let ppos = ptf.translation;
    // Đang lăn thì né được đòn (i-frame như Veloren)
    let dodging = pch.rolling();

    for (e, gt, mut tf, mut f, hostile) in &mut villagers {
        let gpos = gt.translation();
        f.cooldown -= dt;

        // Bị đẩy lùi
        if f.knock.length_squared() > 0.04 {
            let k = f.knock;
            if let Some(d) = step(&map, gpos, k.normalize(), k.length() * dt) {
                tf.translation += d;
            }
            f.knock *= (1.0 - 8.0 * dt).max(0.0);
        }

        // Ngã xuống rồi biến mất
        if let FightState::Dead(t) = f.state {
            let t = t + dt;
            f.state = FightState::Dead(t);
            let fall = (t / 0.6).min(1.0);
            tf.rotation = Quat::from_rotation_y(f.yaw) * Quat::from_rotation_x(-fall * std::f32::consts::FRAC_PI_2);
            if t > DEATH_TIME {
                commands.entity(e).despawn();
            }
            continue;
        }

        // Mục tiêu: người chơi (nổi giận) hoặc thú dữ (bảo vệ làng)
        let at_player = hostile.is_some();
        let tpos = if at_player {
            ppos
        } else {
            match f.target.and_then(|t| animals.get(t).ok()) {
                Some((atf, ..)) => atf.translation,
                None => continue, // `villager_defend_system` sẽ dọn
            }
        };

        let to = Vec3::new(tpos.x - gpos.x, 0.0, tpos.z - gpos.z);
        let dist = to.length();
        let dir = if dist > 0.01 { to / dist } else { Vec3::Z };
        let from_home = f.home.map_or(0.0, |h| Vec2::new(h.x - gpos.x, h.z - gpos.z).length());

        // Nguôi giận (chỉ với người chơi)
        if at_player {
            if dist > CALM_DIST || from_home > LEASH {
                f.calm_t += if from_home > LEASH { dt * 3.0 } else { dt };
                if f.calm_t > CALM_TIME {
                    f.calm_t = 0.0;
                    f.state = FightState::Idle;
                    f.hp = f.profile.hp;
                    if let Some(arm) = f.arm {
                        if let Ok(mut a) = arms.get_mut(arm) {
                            a.rotation = Quat::IDENTITY;
                        }
                    }
                    commands.entity(e).remove::<Hostile>().insert(Speech::new(pick_line(CALM_LINES, e, 4)));
                    continue;
                }
            } else {
                f.calm_t = 0.0;
            }
        }

        // Quay mặt về mục tiêu
        f.yaw = (-dir.x).atan2(-dir.z);
        tf.rotation = tf.rotation.slerp(Quat::from_rotation_y(f.yaw), (dt * 10.0).min(1.0));

        let ranged = f.profile.attack != NpcAttack::Melee;
        let range = f.profile.range;
        let mut arm_angle = -0.5;

        match f.state {
            FightState::Idle | FightState::Chase => {
                f.state = FightState::Chase;
                // Di chuyển: cận chiến áp sát, đánh xa giữ khoảng cách
                let speed = f.profile.speed;
                let move_dir = if ranged {
                    if dist > range * 0.85 {
                        Some(dir)
                    } else if dist < KITE_DIST {
                        Some(-dir)
                    } else {
                        None
                    }
                } else if dist > range * 0.8 {
                    Some(dir)
                } else {
                    None
                };
                if let Some(md) = move_dir {
                    if let Some(d) = step(&map, gpos, md, speed * dt) {
                        tf.translation += d;
                    }
                }
                if dist <= range && f.cooldown <= 0.0 && (!at_player || dist < CALM_DIST) {
                    f.state = FightState::Windup(0.0);
                }
            }
            FightState::Windup(t) => {
                let t = t + dt;
                let len = if ranged { WINDUP * 1.4 } else { WINDUP };
                arm_angle = -0.5 - 2.1 * (t / len).min(1.0);
                if t >= len {
                    if ranged {
                        let origin = gpos + Vec3::Y * 1.4 + dir * 0.6;
                        let aim = tpos + Vec3::Y * if at_player { 0.9 } else { 0.5 };
                        spawn_shot(&mut commands, &fx, f.profile.attack, origin, aim, f.profile.damage, at_player);
                    } else if dist <= range + 0.8 {
                        if at_player {
                            if !dodging {
                                php.current = (php.current - f.profile.damage).max(0.0);
                            }
                        } else if let Some(t) = f.target {
                            if let Ok((_, mut hp, kb, flash)) = animals.get_mut(t) {
                                hit_animal(&mut commands, t, &mut hp, kb, flash, dir, f.profile.damage);
                            }
                        }
                    }
                    f.cooldown = f.profile.cooldown;
                    f.state = FightState::Recover(0.0);
                } else {
                    f.state = FightState::Windup(t);
                }
            }
            FightState::Recover(t) => {
                let t = t + dt;
                let k = (t / RECOVER).min(1.0);
                // Vung xuống nhanh rồi trở về thế thủ
                arm_angle = if k < 0.4 { -2.6 + 3.4 * (k / 0.4) } else { 0.8 - 1.3 * ((k - 0.4) / 0.6) };
                f.state = if t >= RECOVER { FightState::Chase } else { FightState::Recover(t) };
            }
            FightState::Dead(_) => {}
        }

        if let Some(arm) = f.arm {
            if let Ok(mut a) = arms.get_mut(arm) {
                a.rotation = Quat::from_rotation_x(arm_angle);
            }
        }
    }
}

/// Trừ máu thú + đẩy lùi + chớp đỏ (AI thú tự nhận biết bị đánh qua `last_hp`).
pub(super) fn hit_animal(
    commands: &mut Commands,
    e: Entity,
    hp: &mut Health,
    kb: Option<Mut<Knockback>>,
    flash: Option<Mut<HitFlash>>,
    dir: Vec3,
    damage: f32,
) {
    hp.current = (hp.current - damage).max(0.0);
    let v = (dir + Vec3::Y * 0.4).normalize_or_zero() * 8.0;
    match kb {
        Some(mut k) => k.velocity = v,
        None => {
            commands.entity(e).insert(Knockback { velocity: v });
        }
    }
    match flash {
        Some(mut f) => f.timer = 0.15,
        None => {
            commands.entity(e).insert(HitFlash { timer: 0.15 });
        }
    }
}

pub(super) fn spawn_shot(
    commands: &mut Commands,
    fx: &VillagerFx,
    kind: NpcAttack,
    origin: Vec3,
    target: Vec3,
    damage: f32,
    at_player: bool,
) {
    let (speed, gravity, mesh, mat) = match kind {
        NpcAttack::Arrow => (32.0, 9.0, fx.arrow.clone(), fx.arrow_mat.clone()),
        NpcAttack::Fire => (18.0, 0.0, fx.orb.clone(), fx.fire_mat.clone()),
        _ => (22.0, 0.0, fx.orb.clone(), fx.holy_mat.clone()),
    };
    let delta = target - origin;
    let t = (delta.length() / speed).max(0.05);
    // Bù độ rơi để tên bay trúng đích
    let vel = delta / t + Vec3::Y * 0.5 * gravity * t;
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform { translation: origin, rotation: Transform::default().looking_to(vel, Vec3::Y).rotation, ..default() },
        NotShadowCaster,
        VillagerShot { at_player, vel, gravity, damage, life: 3.0 },
    ));
}

/// Đạn của dân làng: bay, chạm khối thì mất; trúng người chơi (nếu bắn người chơi) hoặc thú dữ thì trừ máu.
pub fn villager_shot_system(
    mut commands: Commands,
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    mut player: Query<(&Transform, &mut Health, &Character), (Without<VillagerShot>, Without<Animal>)>,
    mut animals: Query<
        (Entity, &Transform, &mut Health, &Animal, Option<&mut Knockback>, Option<&mut HitFlash>),
        (Without<VillagerShot>, Without<Character>),
    >,
    mut shots: Query<(Entity, &mut Transform, &mut VillagerShot), (Without<Character>, Without<Animal>)>,
) {
    let dt = time.delta_secs();
    let mut player = player.single_mut().ok();
    for (e, mut tf, mut s) in &mut shots {
        s.life -= dt;
        let g = s.gravity;
        s.vel.y -= g * dt;
        tf.translation += s.vel * dt;
        if s.vel.length_squared() > 0.01 {
            tf.rotation = Transform::default().looking_to(s.vel, Vec3::Y).rotation;
        }
        let mut gone = s.life <= 0.0;
        if let Some(m) = map.as_ref() {
            if matches!(m.block_at(tf.translation.x, tf.translation.y, tf.translation.z), Some(b) if b != BlockType::Air && b != BlockType::Water) {
                gone = true;
            }
        }
        if s.at_player {
            if let Some((ptf, hp, pch)) = player.as_mut() {
                if !pch.rolling() && (ptf.translation + Vec3::Y * 0.9).distance(tf.translation) < 0.9 {
                    hp.current = (hp.current - s.damage).max(0.0);
                    gone = true;
                }
            }
        } else {
            for (ae, atf, mut hp, animal, kb, flash) in &mut animals {
                if animal.species.alignment() != Alignment::Enemy || hp.current <= 0.0 {
                    continue;
                }
                if (atf.translation + Vec3::Y * 0.5).distance(tf.translation) < 1.0 {
                    let dir = s.vel.normalize_or_zero();
                    hit_animal(&mut commands, ae, &mut hp, kb, flash, dir, s.damage);
                    gone = true;
                    break;
                }
            }
        }
        if gone {
            commands.entity(e).try_despawn();
        }
    }
}

/// Hiện vũ khí khi đánh nhau (nổi giận hoặc bảo vệ làng), cất đi khi thôi.
pub fn villager_weapon_visibility(
    fighters: Query<(&Fighter, Option<&Hostile>, Option<&Defending>)>,
    mut vis: Query<&mut Visibility, With<NpcWeapon>>,
) {
    for (f, hostile, defending) in &fighters {
        let fighting = hostile.is_some() || defending.is_some();
        let want = if fighting && !f.is_dead() { Visibility::Inherited } else { Visibility::Hidden };
        for w in f.weapons.iter() {
            if let Ok(mut v) = vis.get_mut(*w) {
                if *v != want {
                    *v = want;
                }
            }
        }
    }
}
