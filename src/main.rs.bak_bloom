use bevy::camera::primitives::Aabb;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use redstoric_combat_ai::{CombatAiPlugin, Health, MeleeAttacker};
use redstoric_core::{Character, CorePlugin, OrbitCamera};
use redstoric_voxel::{VoxelViewer, VoxelWorldPlugin};
use bevy::window::PresentMode;
use redstoric_core::fps_counter::FpsOverlayPlugin;
use redstoric_core::graphics::GraphicsPlugin;
use redstoric_core::graphics_menu::GraphicsMenuPlugin;
use redstoric_core::character_creator::CharacterCreatorPlugin;
use redstoric_core::minimap::MinimapPlugin;
use redstoric_net::NetClientPlugin;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Redstoric".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        // --- CÁC KIỂU DỮ LIỆU ĐÃ ĐĂNG KÝ TRƯỚC ĐÓ ---
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<bevy::transform::components::TransformTreeChanged>()
        .register_type::<Visibility>()
        .register_type::<InheritedVisibility>()
        .register_type::<ViewVisibility>()
        .register_type::<Name>()
        .register_type::<Camera3d>()
        .register_type::<DirectionalLight>()
        .register_type::<DistanceFog>()
        .register_type::<Children>()
        .register_type::<ChildOf>()
        .register_type::<Mesh3d>()
        .register_type::<MeshMaterial3d<StandardMaterial>>()

        .register_type::<Aabb>()


        // ------------------------------------

        .add_plugins(FpsOverlayPlugin)
        .add_plugins((CorePlugin, CombatAiPlugin, VoxelWorldPlugin))
        .add_plugins((GraphicsPlugin, GraphicsMenuPlugin, MinimapPlugin, CharacterCreatorPlugin))
        .add_plugins(NetClientPlugin)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.9, 0.95, 1.0),
            brightness: 180.0,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.68, 0.78, 0.90)))
        .add_systems(Startup, (setup_environment, spawn_player))
        .add_systems(Update, fix_voxel_character_materials)

        .run();
}


fn setup_environment(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 10_000.0,
            shadow_depth_bias: 0.02,   // Giảm bias để tránh đứt bóng ở mép khối
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::from_xyz(30.0, 50.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off, //Rang cua: On
        OrbitCamera {
            distance: 9.0,
            ..default()
        },

        // Tăng tầm nhìn sương mù cho địa hình mới
        DistanceFog {
            color: Color::srgba(0.68, 0.78, 0.90, 1.0),
            falloff: FogFalloff::Linear {
                start: 60.0,
                end: 130.0,
            },
            ..default()
        },
    ));
}

fn fix_voxel_character_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, (Added<MeshMaterial3d<StandardMaterial>>, Without<redstoric_voxel::systems::ChunkComponent>)>,
) {
    for handle in &query {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.alpha_mode = AlphaMode::Opaque;
            mat.cull_mode = None;
            mat.double_sided = true;
        }
    }
}

/// Chỉ tạo entity người chơi; bộ model nhân vật được dựng (và dựng lại khi đổi ngoại hình)
/// bởi `CharacterCreatorPlugin` trong `redstoric_core::character_creator`.
fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, 25.0, 0.0),
        Visibility::default(),
        Character::default(),
        MeleeAttacker::default(),
        Health::new(100.0),
        VoxelViewer,
        Name::new("Player"),
    ));
}
