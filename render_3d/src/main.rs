use bevy_flycam::NoCameraPlayerPlugin;
use bevy_lines::prelude::*;

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
};
use bevy_mod_outline::{AutoGenerateOutlineNormalsPlugin, OutlinePlugin};

mod material;
mod primitives;
mod setup;

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
            NoCameraPlayerPlugin,
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
            OutlinePlugin,
            AutoGenerateOutlineNormalsPlugin::default(),
        ))
        .insert_resource(bevy_flycam::MovementSettings {
            sensitivity: 0.00012,
            speed: 15.0,
        })
        .add_systems(Startup, setup::setup)
        .run();
}
