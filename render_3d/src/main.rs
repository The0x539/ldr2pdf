use bevy_flycam::NoCameraPlayerPlugin;
use bevy_lines::prelude::*;
use iyes_perf_ui::prelude::*;

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        RenderPlugin,
        diagnostic::RenderDiagnosticsPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
};

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
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            RenderDiagnosticsPlugin,
            PerfUiPlugin,
            MaterialPlugin::<material::MyMaterial>::default(),
        ))
        .insert_resource(bevy_flycam::MovementSettings {
            sensitivity: 0.00012,
            speed: 15.0,
        })
        .add_systems(Startup, setup::setup)
        .run();
}

fn bevy_from_weldr(a: weldr::Vec3) -> bevy::prelude::Vec3 {
    bevy::prelude::Vec3::from_array(a.to_array())
}

fn bevy_from_weldr_mat(a: weldr::Mat4) -> bevy::prelude::Mat4 {
    bevy::prelude::Mat4::from_cols_array(&a.to_cols_array())
}
