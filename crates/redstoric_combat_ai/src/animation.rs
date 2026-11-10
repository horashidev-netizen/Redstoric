use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use redstoric_core::biped::BoneShoulderRight;
use redstoric_core::weapons::{Loadout, WeaponKind};
use redstoric_core::{ActionState, Character, WeaponHand};
use crate::ranged::{spawn_burst, FxAssets, FxKind};
use crate::types::MeleeAttacker;

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// (pitch, roll) của vai phải theo vũ khí và tiến độ nhát đánh p (0..1); pitch âm = giơ tay lên.
fn arm_pose(kind: WeaponKind, p: f32) -> (f32, f32) {
    match kind {
        // Giơ cao ra sau, chém nhanh chéo xuống từ phải sang trái, thu về
        WeaponKind::Sword => {
            if p < 0.3 {
                let t = ease_out(p / 0.3);
                (lerp(-0.2, -2.7, t), lerp(-0.15, -0.65, t))
            } else if p < 0.7 {
                let t = smooth((p - 0.3) / 0.4);
                (lerp(-2.7, -0.7, t), lerp(-0.65, 0.45, t))
            } else {
                let t = ease_out((p - 0.7) / 0.3);
                (lerp(-0.7, -0.1, t), lerp(0.45, -0.05, t))
            }
        }
        // Quét ngang một vòng: tay giang ngang, roll quét từ phải sang trái
        WeaponKind::Axe => {
            if p < 0.25 {
                let t = ease_out(p / 0.25);
                (lerp(-0.2, -1.5, t), lerp(-0.1, -1.7, t))
            } else if p < 0.7 {
                let t = smooth((p - 0.25) / 0.45);
                (-1.5, lerp(-1.7, 1.5, t))
            } else {
                let t = ease_out((p - 0.7) / 0.3);
                (lerp(-1.5, -0.1, t), lerp(1.5, 0.0, t))
            }
        }
        // Giơ búa lên cao thật chậm, nện thẳng xuống, giữ một nhịp rồi thu về
        WeaponKind::Hammer => {
            if p < 0.5 {
                let t = ease_out(p / 0.5);
                (lerp(-0.2, -3.0, t), lerp(-0.1, -0.2, t))
            } else if p < 0.62 {
                let t = smooth((p - 0.5) / 0.12);
                (lerp(-3.0, -0.3, t), -0.2)
            } else {
                let t = smooth((p - 0.62) / 0.38);
                (lerp(-0.3, -0.15, t), -0.2)
            }
        }
        // Dương cung: tay đưa ra trước, giữ, giật nhẹ khi thả
        WeaponKind::Bow => {
            if p < 0.25 {
                let t = ease_out(p / 0.25);
                (lerp(-0.2, -1.45, t), lerp(-0.1, 0.1, t))
            } else if p < 0.45 {
                (-1.45, 0.1)
            } else {
                let t = smooth((p - 0.45) / 0.55);
                (lerp(-1.3, -0.2, t), lerp(0.1, -0.1, t))
            }
        }
        // Giơ trượng lên rồi chỉ thẳng ra trước khi tung phép
        WeaponKind::FireStaff | WeaponKind::Sceptre => {
            if p < 0.4 {
                let t = ease_out(p / 0.4);
                (lerp(-0.2, -2.3, t), lerp(-0.1, -0.2, t))
            } else if p < 0.6 {
                let t = smooth((p - 0.4) / 0.2);
                (lerp(-2.3, -1.3, t), -0.2)
            } else {
                let t = smooth((p - 0.6) / 0.4);
                (lerp(-1.3, -0.2, t), lerp(-0.2, -0.1, t))
            }
        }
    }
}

/// Hoạt ảnh đánh theo vũ khí: cả cánh tay phải vung (vai xoay), vũ khí giữ tư thế trong bàn tay.
pub fn weapon_swing_animation(
    time: Res<Time>,
    loadout: Res<Loadout>,
    character_query: Query<&Character>,
    mut weapon_query: Query<&mut Transform, (With<WeaponHand>, Without<BoneShoulderRight>)>,
    mut shoulder_query: Query<&mut Transform, With<BoneShoulderRight>>,
) {
    let Ok(character) = character_query.single() else { return; };
    let def = loadout.current();
    let rest_weapon = Quat::from_rotation_x(def.rest_pitch) * Quat::from_rotation_z(0.15);

    if let ActionState::Attacking(_) = character.state {
        let p = (1.0 - (character.attack_timer / character.attack_len.max(0.05))).clamp(0.0, 1.0);
        let (pitch, roll) = arm_pose(def.kind, p);
        let target = Quat::from_rotation_x(pitch) * Quat::from_rotation_z(roll);
        let k = (30.0 * time.delta_secs()).min(1.0);
        for mut tf in &mut shoulder_query {
            tf.rotation = tf.rotation.slerp(target, k);
        }

        // Kiếm: cổ tay bật nhẹ ở giữa nhát chém
        let flick = if def.kind == WeaponKind::Sword && (0.3..0.7).contains(&p) { -0.5 * ((p - 0.3) / 0.4) } else { 0.0 };
        for mut w in &mut weapon_query {
            w.rotation = rest_weapon * Quat::from_rotation_x(flick);
        }
    } else {
        for mut w in &mut weapon_query {
            w.rotation = rest_weapon;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Vệt chém
// ---------------------------------------------------------------------------------------------

const FX_LIFE: f32 = 0.20;

#[derive(Component)]
pub struct SlashFx {
    age: f32,
    material: Handle<StandardMaterial>,
}

/// Dải cung trăng khuyết: đầu cung sáng và dày, đuôi mỏng và mờ dần.
fn slash_mesh() -> Mesh {
    const SEG: usize = 24;
    let (a0, a1) = (0.35f32, 2.55f32); // đuôi ở phải, đầu ở trái (rad)
    let r_out = 1.55f32;
    let mut pos = Vec::new();
    let mut col = Vec::new();
    let mut nor = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=SEG {
        let s = i as f32 / SEG as f32;
        let a = a0 + (a1 - a0) * s;
        let width = 0.42 * (std::f32::consts::PI * s).sin().max(0.04) * (0.4 + 0.6 * s);
        let (c, sn) = (a.cos(), a.sin());
        pos.push([c * r_out, sn * r_out, 0.0]);
        pos.push([c * (r_out - width), sn * (r_out - width), 0.0]);
        let alpha = (s * s).clamp(0.0, 1.0);
        col.push([1.0, 1.0, 1.0, alpha]);
        col.push([0.6, 0.85, 1.0, alpha * 0.2]);
        nor.push([0.0, 0.0, 1.0]);
        nor.push([0.0, 0.0, 1.0]);
        if i < SEG {
            let b = (i * 2) as u32;
            idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
    }
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_indices(Indices::U32(idx));
    m
}

/// Sinh vệt chém khi nhát chém đến giữa pha vung, rồi nở ra và mờ dần.
pub fn slash_fx_system(
    mut commands: Commands,
    time: Res<Time>,
    loadout: Res<Loadout>,
    fx_assets: Option<Res<FxAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<(&Transform, &Character), With<MeleeAttacker>>,
    mut fx: Query<(Entity, &mut SlashFx, &mut Transform), Without<MeleeAttacker>>,
    mut armed: Local<bool>,
    mut mesh_handle: Local<Option<Handle<Mesh>>>,
) {
    let dt = time.delta_secs();

    for (tf, ch) in &players {
        if let ActionState::Attacking(_) = ch.state {
            let def = loadout.current();
            let p = 1.0 - ch.attack_timer / ch.attack_len.max(0.05);
            if p >= def.release && *armed {
                *armed = false;
                match def.kind {
                    WeaponKind::Axe => {
                        if let Some(a) = fx_assets.as_ref() {
                            // Vòng gió quét ngang quanh người
                            spawn_burst(&mut commands, &mut materials, a, FxKind::Spin, tf.translation + Vec3::Y * 1.0,
                                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2), true, (1.0, 3.6, 0.28), (1.0, 0.9, 0.7));
                        }
                        continue;
                    }
                    WeaponKind::Hammer => {
                        if let Some(a) = fx_assets.as_ref() {
                            // Sóng xung kích trên mặt đất phía trước + bụi đất
                            let c = tf.translation + *tf.forward() * 2.0 + Vec3::Y * 0.15;
                            spawn_burst(&mut commands, &mut materials, a, FxKind::Slam, c,
                                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2), true, (0.7, 4.2, 0.4), (1.0, 0.7, 0.4));
                            spawn_burst(&mut commands, &mut materials, a, FxKind::Dust, c + Vec3::Y * 0.3,
                                Quat::IDENTITY, false, (0.3, 1.4, 0.3), (0.8, 0.6, 0.4));
                        }
                        continue;
                    }
                    WeaponKind::Sword => {}
                    _ => continue, // cung / trượng: dùng đạn
                }
                let mh = mesh_handle.get_or_insert_with(|| meshes.add(slash_mesh())).clone();
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                });
                // Vệt chém đứng trước người chơi, ngang tầm ngực, nghiêng chéo theo hướng vung
                let forward = tf.forward();
                let pos = tf.translation + Vec3::Y * 1.0 + *forward * 0.9;
                let face = Quat::from_rotation_arc(Vec3::Z, -*forward);
                let rot = face * Quat::from_rotation_z(0.55);
                commands.spawn((
                    Mesh3d(mh),
                    MeshMaterial3d(mat.clone()),
                    Transform { translation: pos, rotation: rot, scale: Vec3::splat(0.8) },
                    NotShadowCaster,
                    SlashFx { age: 0.0, material: mat },
                ));
            }
        } else {
            *armed = true;
        }
    }

    for (e, mut f, mut tf) in &mut fx {
        f.age += dt;
        let t = (f.age / FX_LIFE).clamp(0.0, 1.0);
        tf.scale = Vec3::splat(0.8 + 0.4 * ease_out(t));
        if let Some(mut m) = materials.get_mut(&f.material) {
            m.base_color = Color::srgba(1.0, 1.0, 1.0, 0.9 * (1.0 - t));
        }
        if t >= 1.0 {
            commands.entity(e).try_despawn();
        }
    }
}
