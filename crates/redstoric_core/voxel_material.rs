use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct PackedVoxelMaterial {}

impl Material for PackedVoxelMaterial {
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/voxel_figure.wgsl".into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/voxel_figure.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Layout 8 bytes tương ứng với struct PackedVoxelVertex
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0), // packed_pos_norm (Uint32)
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),    // packed_color (Uint32)
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}