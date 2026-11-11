#![allow(unused_imports)]
use super::*;

/// Tầm bắt đầu tương tác.
pub(super) const INTERACT_RANGE: f32 = 3.5;

/// Tầm NPC quay mặt về phía người chơi.
pub(super) const FACE_RANGE: f32 = 8.0;

/// Dân làng đi dạo trong bán kính này quanh chỗ đứng ban đầu (khi không tìm được làng).
pub(super) const WANDER_RADIUS: f32 = 6.0;

/// Tỉ lệ lần đi dạo chọn điểm bất kỳ trong làng (còn lại: quanh chỗ đứng).
pub(super) const ROAM_CHANCE: f32 = 0.65;

pub(super) const WALK_SPEED: f32 = 1.3;

/// Người chơi lại gần hơn khoảng này thì dân làng dừng lại.
pub(super) const STOP_RANGE: f32 = 4.5;

pub(super) fn rnd(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

pub(super) fn init_villagers(
    mut commands: Commands,
    new: Query<(Entity, &VillagerSpawn), Added<VillagerSpawn>>,
    catalog: Res<AppearanceCatalog>,
    asset_server: Res<AssetServer>,
) {
    for (entity, spawn) in &new {
        let job = Profession::from_index(spawn.job);
        let name = person_name(spawn.seed);
        let mut appearance = catalog.villager_appearance(spawn.seed);
        appearance.outfit = crate::outfits::outfit_for(job, spawn.seed);
        appearance.npc_weapon = Some(combat_profile(job).weapon.to_string());
        let age = age_of(spawn.seed, job);
        if age == 2 {
            // Người già: tóc bạc
            let g = 0.72 + (spawn.seed % 20) as f32 * 0.01;
            appearance.hair_tint = [g, g, g + 0.02];
        }
        spawn_humanoid(&mut commands, entity, &appearance, &catalog, &asset_server, false);

        let title = format!("{name} ({})", job.label());
        commands.entity(entity).insert((
            Villager { name, job, female: appearance.female, age },
            Wander {
                ready: false,
                home: Vec3::ZERO,
                center: Vec3::ZERO,
                roam: 0.0,
                target: Vec3::ZERO,
                wait: 1.0 + (spawn.seed % 7) as f32,
                walking: false,
                phase: 0.0,
                rng: spawn.seed | 1,
            },
        ));

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
                        n.spawn(ui_text("Q  Quest", 16.0, Color::srgb(0.6, 0.9, 1.0)));
                    });
            });
    }
}

pub(super) fn find_nearby(
    player: Query<&GlobalTransform, With<Character>>,
    villagers: Query<(Entity, &GlobalTransform), (With<Villager>, Without<Hostile>)>,
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

pub(super) fn dialog_input(
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
/// Đi dạo quanh khu của mình; đứng lại khi người chơi lại gần hoặc đang nói chuyện / giao dịch.
pub(super) fn villager_wander(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    dialog: Res<NpcDialog>,
    generator: Option<Res<WorldGenerator>>,
    player: Query<&GlobalTransform, With<Character>>,
    mut villagers: Query<
        (Entity, &GlobalTransform, &mut Transform, &mut Wander),
        (With<Villager>, Without<Hostile>, Without<Defending>),
    >,
) {
    let Some(map) = map else { return };
    let dt = time.delta_secs();
    let ppos = player.single().ok().map(|p| p.translation());
    for (entity, gt, mut tf, mut w) in &mut villagers {
        // Dân làng là con của chunk nên Transform là toạ độ cục bộ: tính toán bằng toạ độ thế giới
        let gpos = gt.translation();
        if !w.ready {
            w.ready = true;
            w.home = gpos;
            w.target = gpos;
            w.center = gpos;
            w.roam = WANDER_RADIUS;
            // Tìm làng chứa chỗ đứng để đi khắp làng
            if let Some(world_gen) = &generator {
                let (x, z) = (gpos.x.floor() as i32, gpos.z.floor() as i32);
                if let Some(site) = world_gen
                    .sites_in_area(x - 1, z - 1, x + 1, z + 1)
                    .into_iter()
                    .find(|s| s.kind == SiteKind::Village && s.contains(x, z, 8))
                {
                    w.center = Vec3::new(site.x as f32 + 0.5, gpos.y, site.z as f32 + 0.5);
                    w.roam = (site.radius as f32 - 4.0).max(WANDER_RADIUS);
                }
            }
        }
        let talking = dialog.is_open() && dialog.target == Some(entity);
        let near_player = ppos.map_or(false, |p| p.distance(gpos) < STOP_RANGE);
        if talking || near_player {
            w.walking = false;
            continue;
        }
        if !w.walking {
            w.wait -= dt;
            if w.wait <= 0.0 {
                let a = rnd(&mut w.rng) * std::f32::consts::TAU;
                w.target = if rnd(&mut w.rng) < ROAM_CHANCE {
                    // Điểm bất kỳ trong làng (căn bậc hai để rải đều trên hình tròn)
                    let r = rnd(&mut w.rng).sqrt() * w.roam;
                    w.center + Vec3::new(a.cos() * r, 0.0, a.sin() * r)
                } else {
                    let r = 1.5 + rnd(&mut w.rng) * (WANDER_RADIUS - 1.5);
                    w.home + Vec3::new(a.cos() * r, 0.0, a.sin() * r)
                };
                w.walking = true;
            }
            continue;
        }
        let to = Vec3::new(w.target.x - gpos.x, 0.0, w.target.z - gpos.z);
        let dist = to.length();
        if dist < 0.3 {
            w.walking = false;
            w.wait = 2.0 + rnd(&mut w.rng) * 6.0;
            continue;
        }
        let dir = to / dist;
        let nx = gpos.x + dir.x * WALK_SPEED * dt;
        let nz = gpos.z + dir.z * WALK_SPEED * dt;
        let (h, blocked) = map.get_walk_height(nx, nz, gpos.y);
        if blocked || h < 0.5 || (h - gpos.y).abs() > 1.1 {
            // Vướng tường / hàng rào / chunk chưa tải: nghỉ rồi chọn hướng khác
            w.walking = false;
            w.wait = 0.4 + rnd(&mut w.rng) * 1.2;
            continue;
        }
        w.phase += dt * 9.0;
        // Chuyển độ dời thế giới thành độ dời cục bộ (chunk chỉ tịnh tiến, không xoay)
        tf.translation += Vec3::new(nx - gpos.x, h + w.phase.sin().abs() * 0.05 - gpos.y, nz - gpos.z);
        let target = Quat::from_rotation_y((-dir.x).atan2(-dir.z));
        tf.rotation = tf.rotation.slerp(target, 0.2);
    }
}

pub(super) fn villager_face_player(
    player: Query<&GlobalTransform, With<Character>>,
    mut villagers: Query<(&GlobalTransform, &mut Transform, Option<&Wander>), (With<Villager>, Without<Hostile>, Without<Defending>)>,
) {
    let Ok(p) = player.single() else { return };
    let pos = p.translation();
    for (g, mut t, wander) in &mut villagers {
        if wander.map_or(false, |w| w.walking) {
            continue;
        }
        let d = pos - g.translation();
        let flat = d.x * d.x + d.z * d.z;
        if flat < 0.01 || flat > FACE_RANGE * FACE_RANGE {
            continue;
        }
        let target = Quat::from_rotation_y((-d.x).atan2(-d.z));
        t.rotation = t.rotation.slerp(target, 0.15);
    }
}
