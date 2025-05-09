use bevy::{
    asset::{load_internal_asset, weak_handle},
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionPipeline},
    prelude::*,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_resource::*,
    },
};

const SHADER_HANDLE: Handle<Shader> = weak_handle!("3f143a7b-f598-4e2a-8b52-4413e556bc0a");

pub fn my_material_plugin(app: &mut App) {
    load_internal_asset!(
        app,
        SHADER_HANDLE,
        "custom_material.wgsl",
        Shader::from_wgsl
    );

    app.add_plugins(MaterialPlugin::<MyMaterial> {
        prepass_enabled: false,
        shadows_enabled: false,
        ..default()
    });
}

pub const ATTRIBUTE_FLAGS: MeshVertexAttribute =
    MeshVertexAttribute::new("flags", 1000, VertexFormat::Uint32);

pub type MyMaterial = ExtendedMaterial<StandardMaterial, MyExtension>;

#[derive(Debug, Default, Copy, Clone, Asset, Reflect, AsBindGroup)]
pub struct MyExtension {}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_HANDLE.into()
    }

    fn vertex_shader() -> ShaderRef {
        SHADER_HANDLE.into()
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
