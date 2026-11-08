use bevy::prelude::*;
use crate::{ActionState, Character};

/// Đánh dấu model body để hệ thống lọc mesh nhận diện
#[derive(Component)]
pub struct CharacterModelRoot;

/// Hệ thống tự động dọn dẹp các mesh thừa (khung ba lô trôi nổi) trong file GLB
pub fn clean_character_glb_hierarchy(
    model_roots: Query<Entity, Added<CharacterModelRoot>>,
    children_query: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut commands: Commands,
) {
    for root in &model_roots {
        // Duyệt toàn bộ các node con sinh ra từ GLTF Scene
        clean_recursive(root, &children_query, &names, &transforms, &mut commands);
    }
}

fn clean_recursive(
    entity: Entity,
    children_query: &Query<&Children>,
    names: &Query<&Name>,
    transforms: &Query<&Transform>,
    commands: &mut Commands,
) {
    if let Ok(name) = names.get(entity) {
        let name_str = name.as_str().to_lowercase();
        // Lọc các node thường gây rác như frame, backpack, collision rác
        if name_str.contains("frame") || name_str.contains("backpack") || name_str.contains("bounding") {
            commands.entity(entity).despawn();
            return;
        }
    }

    // Nếu một mesh con bị đặt translation lệch quá xa khỏi tâm nhân vật (như cái khung gỗ cách 1.5m)
    if let Ok(transform) = transforms.get(entity) {
        if transform.translation.x.abs() > 1.2 || transform.translation.z.abs() > 1.2 {
            commands.entity(entity).despawn();
            return;
        }
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            clean_recursive(child, children_query, names, transforms, commands);
        }
    }
}

// Thêm animation nhún nhảy cơ thể khi chạy (Bobbing)
pub fn character_procedural_animation(
    time: Res<Time>,
    mut query: Query<(&Character, &mut Transform), Without<crate::OrbitCamera>>,
    model_child_query: Query<&Children, With<Character>>,
    mut model_transforms: Query<&mut Transform, (With<CharacterModelRoot>, Without<Character>)>,
) {
    let t = time.elapsed_secs();

    for (character, root_transform) in &mut query {
        if let Ok(children) = model_child_query.single() {
            for child in children.iter() {
                if let Ok(mut model_trans) = model_transforms.get_mut(child) {
                    match character.state {
                        ActionState::Running => {
                            // Nhún lên xuống theo bước chân
                            let bounce = (t * 14.0).sin().abs() * 0.12;
                            // Nghiêng nhẹ người sang 2 bên
                            let roll = (t * 7.0).sin() * 0.08;

                            model_trans.translation.y = bounce;
                            model_trans.rotation = Quat::from_rotation_z(roll);
                        }
                        ActionState::Idle => {
                            // Thở nhẹ nhàng khi đứng yên
                            let breathe = (t * 3.0).sin() * 0.02;
                            model_trans.translation.y = breathe;
                            model_trans.rotation = Quat::IDENTITY;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

