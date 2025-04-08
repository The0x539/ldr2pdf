use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionPipeline},
    prelude::*,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_resource::*,
    },
};

pub const ATTRIBUTE_FLAGS: MeshVertexAttribute =
    MeshVertexAttribute::new("flags", 1000, VertexFormat::Uint32);

pub type MyMaterial = ExtendedMaterial<StandardMaterial, MyExtension>;

#[derive(Debug, Default, Copy, Clone, Asset, Reflect, AsBindGroup)]
pub struct MyExtension {}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        "../src/custom_material.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "../src/custom_material.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let to_add = layout
            .0
            .get_layout(&[ATTRIBUTE_FLAGS.at_shader_location(20)])
            .unwrap();

        descriptor.vertex.buffers[0]
            .attributes
            .extend(to_add.attributes);

        Ok(())
    }
}
