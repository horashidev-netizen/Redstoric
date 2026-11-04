use bevy::math::primitives::Cuboid;
use bevy::prelude::*;

mod world;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Sun;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.6, 0.8, 0.95)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 650.0,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Redstoric".into(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (player_movement, daylight_cycle))
        .run();
}

fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(8.0, 12.0, 24.0).looking_at(Vec3::new(8.0, 0.0, 8.0), Vec3::Y),
            ..default()
        },
        Player,
    ));

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                illuminance: 9_000.0,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 16.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        Sun,
    ));

    let block_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.76, 0.37),
        perceptual_roughness: 0.92,
        ..default()
    });
    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.52, 0.56),
        perceptual_roughness: 0.95,
        ..default()
    });

    for (x, y, z) in world::flat_world_positions(16, 16, 0) {
        let material = if (x + z) % 2 == 0 {
            grass_material.clone()
        } else {
            stone_material.clone()
        };

        commands.spawn(PbrBundle {
            mesh: block_mesh.clone(),
            material,
            transform: Transform::from_xyz(x as f32, y as f32, z as f32),
            ..default()
        });
    }
}

fn player_movement(
    keyboard: Res<'_, ButtonInput<KeyCode>>,
    time: Res<'_, Time>,
    mut query: Query<'_, '_, &mut Transform, With<Player>>,
) {
    let mut transform = query.single_mut();
    let mut movement = Vec3::ZERO;
    let speed = 8.0;

    if keyboard.pressed(KeyCode::KeyW) {
        movement += *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        movement -= *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        movement -= *transform.right();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += *transform.right();
    }
    if keyboard.pressed(KeyCode::Space) {
        movement += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        movement -= Vec3::Y;
    }

    if movement != Vec3::ZERO {
        transform.translation += movement.normalize() * speed * time.delta_seconds();
    }
}

fn daylight_cycle(time: Res<'_, Time>, mut sun_query: Query<'_, '_, &mut Transform, With<Sun>>) {
    let mut sun = sun_query.single_mut();
    sun.rotate_x(0.15 * time.delta_seconds());
}
