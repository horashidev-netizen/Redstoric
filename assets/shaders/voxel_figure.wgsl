#import bevy_pbr::mesh_functions

struct VertexInput {
    @location(0) packed_pos_norm: u32,
    @location(1) packed_color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

// Bảng tra pháp tuyến 6 mặt chuẩn: 0:+Y, 1:-Y, 2:+X, 3:-X, 4:-Z, 5:+Z
const NORMALS = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, -1.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(-1.0, 0.0, 0.0),
    vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(0.0, 0.0, 1.0)
);

@vertex
fn vertex(input: VertexInput, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // 1. Giải nén tọa độ X, Y, Z (trừ 128.0 để đưa gốc tọa độ về lại tâm mô hình)
    let raw_x = f32(input.packed_pos_norm & 255u) - 128.0;
    let raw_y = f32((input.packed_pos_norm >> 8u) & 255u) - 128.0;
    let raw_z = f32((input.packed_pos_norm >> 16u) & 255u) - 128.0;
    let local_pos = vec3<f32>(raw_x, raw_y, raw_z);

    // 2. Giải nén hướng mặt Normal
    let norm_idx = (input.packed_pos_norm >> 24u) & 7u;
    let local_normal = NORMALS[norm_idx];

    // 3. Giải nén màu RGBA (từ u8 sang dải 0.0 - 1.0)
    let r = f32(input.packed_color & 255u) / 255.0;
    let g = f32((input.packed_color >> 8u) & 255u) / 255.0;
    let b = f32((input.packed_color >> 16u) & 255u) / 255.0;
    out.color = vec4<f32>(pow(r, 2.2), pow(g, 2.2), pow(b, 2.2), 1.0);

    // 4. Biến đổi tọa độ không gian thế giới thông qua Transform của Bevy
    let model = mesh_functions::get_world_from_local(instance_index);
    let world_pos = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(local_pos, 1.0));
    out.clip_position = mesh_functions::mesh_position_world_to_clip(world_pos);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(local_normal, instance_index);

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Chiếu sáng theo hướng mặt trời + ambient nhẹ
    let sun_dir = normalize(vec3<f32>(0.5, 1.0, 0.4));
    let ndotl = max(dot(in.world_normal, sun_dir), 0.35);
    return vec4<f32>(in.color.rgb * ndotl, 1.0);
}