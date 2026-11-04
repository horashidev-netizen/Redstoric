use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin; // <-- Cửa sổ kéo thả trực quan
use redstoric_combat_ai::{CombatAiPlugin, MeleeAttacker};
use redstoric_core::{
    biped::*, Character, CorePlugin, OrbitCamera, WeaponHand,
};
use redstoric_voxel::{VoxelViewer, VoxelWorldPlugin};
use std::f32::consts::FRAC_PI_2;
use bevy_inspector_egui::bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Redstoric Engine - Inspector Rig Tool".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 700.0,
            ..default()
        })
        .add_plugins((CorePlugin, CombatAiPlugin, VoxelWorldPlugin))
        // 1. Phải thêm EguiPlugin trước:
        .add_plugins(EguiPlugin::default())
        // 2. Rồi mới đến WorldInspectorPlugin:
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, (setup_environment, spawn_player))
        .add_systems(Update, fix_voxel_character_materials)
        .run();
}

fn setup_environment(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 16_000.0,
            shadow_depth_bias: 0.08,
            shadow_normal_bias: 1.5,
            ..default()
        },
        Transform::from_xyz(40.0, 60.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            distance: 6.0,
            ..default()
        },
    ));
}

fn fix_voxel_character_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, Added<MeshMaterial3d<StandardMaterial>>>,
) {
    for handle in &query {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.alpha_mode = AlphaMode::Opaque;
            mat.cull_mode = None;
            mat.double_sided = true;
        }
    }
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let p = "placeholders/glb/npc/adlet/elder/male";

    // 1. Nạp toàn bộ 13 file GLB đã được chuẩn hóa trục từ tool batch_vox
    let torso_upper = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/torso_upper.glb")));
    let torso_lower = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/torso_lower.glb")));
    let tail = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/tail.glb")));
    let head = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/head.glb")));
    let jaw = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/jaw.glb")));

    let shoulder_l = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/shoulder_l.glb")));
    let shoulder_r = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/shoulder_r.glb")));
    let hand_l = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/hand_l.glb")));
    let hand_r = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/hand_r.glb")));

    let leg_l = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/leg_l.glb")));
    let leg_r = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/leg_r.glb")));
    let foot_l = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/foot_l.glb")));
    let foot_r = asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("{p}/foot_r.glb")));

    let weapon_scene = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("placeholders/glb/weapon/sword/caladbolg.glb"),
    );

    // Entity gốc quản lý Di chuyển, Va chạm & Tải map
    commands
        .spawn((
            Transform::from_xyz(0.0, 25.0, 0.0),
            Visibility::default(),
            Character::default(),
            MeleeAttacker::default(),
            VoxelViewer,
            Name::new("Player"),
        ))
        .with_children(|root| {
            // Container gốc: KHÔNG xoay âm 90 độ nữa, đứng thẳng tự nhiên!
            root.spawn((
                Transform::from_xyz(0.0, 0.0, 0.0)
                    .with_scale(Vec3::splat(0.065)),
                Visibility::default(),
            ))
                .with_children(|biped| {
                    // =========================================================================
                    // 1. NGỰC (UPPER TORSO) - GỐC CHÍNH CƠ THỂ Ở ĐỘ CAO Y = 16.0
                    // =========================================================================
                    biped.spawn((
                        Transform::from_xyz(0.0, 16.0, 0.0),
                        Visibility::default(),
                        BoneChest,
                    ))
                        .with_children(|chest| {
                            chest.spawn((WorldAssetRoot(torso_upper), Transform::IDENTITY));

                            // =====================================================================
                            // 2. HÔNG (LOWER TORSO) & ĐUÔI (NẰM DƯỚI NGỰC Y = -5.0)
                            // =====================================================================
                            chest.spawn((
                                WorldAssetRoot(torso_lower),
                                Transform::from_xyz(0.0, -5.0, 0.5),
                            ))
                                .with_children(|pelvis| {
                                    // Đuôi sói gắn phía sau mông (Z dương là lùi về sau)
                                    pelvis.spawn((
                                        Transform::from_xyz(0.0, -1.5, 3.5),
                                        Visibility::default(),
                                        BoneTail,
                                    ))
                                        .with_children(|t| {
                                            t.spawn((WorldAssetRoot(tail), Transform::IDENTITY));
                                        });

                                    // CHÂN TRÁI & BÀN CHÂN (Hạ xuống dưới hông theo trục Y)
                                    pelvis.spawn((
                                        Transform::from_xyz(-3.8, -4.5, 0.0),
                                        Visibility::default(),
                                        BoneLegLeft,
                                    ))
                                        .with_children(|leg| {
                                            leg.spawn((WorldAssetRoot(leg_l), Transform::IDENTITY));
                                            // Bàn chân nhô ra trước theo trục Z âm (-Z là hướng nhìn)
                                            leg.spawn((WorldAssetRoot(foot_l), Transform::from_xyz(0.0, -5.0, -1.0)));
                                        });

                                    // CHÂN PHẢI & BÀN CHÂN
                                    pelvis.spawn((
                                        Transform::from_xyz(3.8, -4.5, 0.0),
                                        Visibility::default(),
                                        BoneLegRight,
                                    ))
                                        .with_children(|leg| {
                                            leg.spawn((WorldAssetRoot(leg_r), Transform::IDENTITY));
                                            leg.spawn((WorldAssetRoot(foot_r), Transform::from_xyz(0.0, -5.0, -1.0)));
                                        });
                                });

                            // =====================================================================
                            // 3. ĐẦU & MÕM SÓI (CAO HƠN NGỰC Y = +7.0, NHÔ RA TRƯỚC Z = -1.5)
                            // =====================================================================
                            chest.spawn((
                                Transform::from_xyz(0.0, 7.5, -1.5),
                                Visibility::default(),
                                BoneHead,
                            ))
                                .with_children(|h| {
                                    h.spawn((WorldAssetRoot(head), Transform::IDENTITY));
                                    // Mõm sói nhe nanh khớp khít ngay dưới mũi
                                    h.spawn((WorldAssetRoot(jaw), Transform::from_xyz(0.0, -1.5, -1.0), BoneJaw));
                                });

                            // =====================================================================
                            // 4. VAI TRÁI & TAY TRÁI (ÁP SÁT HÔNG SƯỜN X = -6.0)
                            // =====================================================================
                            chest.spawn((
                                Transform::from_xyz(-6.0, 2.0, 0.0),
                                Visibility::default(),
                                BoneShoulderLeft,
                            ))
                                .with_children(|sh| {
                                    sh.spawn((WorldAssetRoot(shoulder_l), Transform::IDENTITY));
                                    // Tay thõng xuống dưới vai theo trục Y
                                    sh.spawn((
                                        WorldAssetRoot(hand_l),
                                        Transform::from_xyz(0.0, -6.5, 0.0),
                                        BoneHandLeft,
                                    ));
                                });

                            // =====================================================================
                            // 5. VAI PHẢI, TAY PHẢI & THANH KIẾM (X = +6.0)
                            // =====================================================================
                            chest.spawn((
                                Transform::from_xyz(6.0, 2.0, 0.0),
                                Visibility::default(),
                                BoneShoulderRight,
                            ))
                                .with_children(|sh| {
                                    sh.spawn((WorldAssetRoot(shoulder_r), Transform::IDENTITY));

                                    // Bàn tay phải
                                    sh.spawn((
                                        Transform::from_xyz(0.0, -6.5, 0.0),
                                        Visibility::default(),
                                        BoneHandRight,
                                    ))
                                        .with_children(|hand| {
                                            hand.spawn((WorldAssetRoot(hand_r), Transform::IDENTITY));

                                            // Kiếm Caladbolg nằm gọn trong lòng bàn tay phải, chĩa ra trước (-Z)
                                            hand.spawn((
                                                WorldAssetRoot(weapon_scene),
                                                Transform::from_xyz(0.5, 0.5, -2.5),
                                                WeaponHand,
                                            ));
                                        });
                                });
                        });
                });
        });
}