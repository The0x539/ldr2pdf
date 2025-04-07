use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub type MyMaterial = ExtendedMaterial<StandardMaterial, MyExtension>;

#[derive(Debug, Default, Copy, Clone, Asset, Reflect, AsBindGroup)]
pub struct MyExtension {}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        "../src/custom_material.wgsl".into()
    }
}
