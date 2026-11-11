#![allow(unused_imports)]
use super::*;

/// Vật phẩm/cột mốc nhiệm vụ trong thế giới.
#[derive(Component)]
pub struct QuestMarker {
    /// true: vật phẩm nhặt được (Fetch); false: cột mốc phát sáng.
    pub pickup: bool,
    /// Cột mốc chạm vào thì tính (Patrol).
    pub touch: bool,
    pub(super) grounded: bool,
    pub(super) base_y: f32,
    pub(super) phase: f32,
}

pub(super) fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    generator: &WorldGenerator,
    x: f32,
    z: f32,
    pickup: bool,
    touch: bool,
    phase: f32,
) {
    let y = generator.get_surface_height(x as f64, z as f64) as f32 + 1.0;
    let (mesh, base, glow) = if pickup {
        (Cuboid::new(0.45, 0.45, 0.45), Color::srgb(1.0, 0.85, 0.3), LinearRgba::rgb(4.0, 2.8, 0.5))
    } else {
        (Cuboid::new(0.3, 6.0, 0.3), Color::srgb(0.4, 0.85, 1.0), LinearRgba::rgb(0.5, 2.2, 4.0))
    };
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: base, emissive: glow, unlit: true, ..default() })),
        Transform::from_xyz(x + 0.5, y, z + 0.5),
        Visibility::default(),
        QuestMarker { pickup, touch, grounded: false, base_y: y, phase },
    ));
}

/// Đặt vật phẩm/cột mốc cho nhiệm vụ vừa nhận.
pub(super) fn spawn_quest_markers(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    generator: &WorldGenerator,
    q: &Quest,
) {
    let h = name_hash(&q.giver_name) ^ q.giver.wrapping_mul(977);
    match q.goal {
        QuestGoal::Fetch { x, z, need } => {
            for i in 0..need {
                let a = (mix(h, i) & 0xffff) as f32 / 65536.0 * std::f32::consts::TAU;
                let r = 2.0 + (mix(h, i + 7) % 7) as f32;
                let (px, pz) = (x as f32 + a.cos() * r, z as f32 + a.sin() * r);
                spawn_marker(commands, meshes, mats, generator, px.floor(), pz.floor(), true, false, i as f32);
            }
        }
        QuestGoal::Patrol { site, need } => {
            if let Some(s) = generator.site_by_id(site) {
                for (i, (px, pz)) in patrol_points(s, h, need).into_iter().enumerate() {
                    spawn_marker(commands, meshes, mats, generator, px as f32, pz as f32, false, true, i as f32);
                }
            }
        }
        QuestGoal::Courier { to: site } | QuestGoal::Scout { site } | QuestGoal::SlayBoss { site } => {
            if let Some(s) = generator.site_by_id(site) {
                spawn_marker(commands, meshes, mats, generator, s.x as f32, s.z as f32, false, false, 0.0);
            }
        }
        QuestGoal::Hunt { .. } => {}
    }
}

pub(super) fn clear_markers(commands: &mut Commands, markers: &Query<(Entity, &QuestMarker, &GlobalTransform)>) {
    for (e, _, _) in markers.iter() {
        commands.entity(e).despawn();
    }
}

/// Đặt vật phẩm/cột mốc lên mặt đất thật (khi chunk đã tải) và cho chúng nhấp nhô.
pub fn quest_marker_system(
    time: Res<Time>,
    map: Option<Res<VoxelWorldMap>>,
    mut markers: Query<(&mut Transform, &mut QuestMarker)>,
) {
    let t = time.elapsed_secs();
    for (mut tf, mut m) in &mut markers {
        if !m.grounded {
            if let Some(map) = &map {
                let (h, _) = map.get_walk_height(tf.translation.x, tf.translation.z, 63.0);
                if h > 0.5 {
                    m.base_y = h;
                    m.grounded = true;
                }
            }
        }
        if m.pickup {
            tf.translation.y = m.base_y + 0.7 + (t * 2.0 + m.phase).sin() * 0.15;
            tf.rotate_y(time.delta_secs() * 1.5);
        } else {
            tf.translation.y = m.base_y + 3.0;
        }
    }
}
