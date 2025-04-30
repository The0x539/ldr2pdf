use bevy_lines::prelude::*;

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
};

mod instruction;
mod material;
mod primitives;
mod setup;
mod traverse;
mod watch;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }),
                ..default()
            }),
            PolylinePlugin,
            bevy_blendy_cameras::BlendyCamerasPlugin,
            MaterialPlugin::<material::MyMaterial> {
                prepass_enabled: false,
                shadows_enabled: false,
                ..default()
            },
            #[cfg(feature = "overlay")]
            (
                bevy::diagnostic::FrameTimeDiagnosticsPlugin,
                bevy::diagnostic::EntityCountDiagnosticsPlugin,
                bevy::render::diagnostic::RenderDiagnosticsPlugin,
                iyes_perf_ui::PerfUiPlugin,
            ),
            #[cfg(feature = "outline")]
            (
                bevy_mod_outline::OutlinePlugin,
                bevy_mod_outline::AutoGenerateOutlineNormalsPlugin::default(),
            ),
        ))
        .add_plugins(setup::setup_plugin)
        .add_plugins(instruction::instruction_plugin)
        .run();
}
