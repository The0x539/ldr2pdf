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

// TODO: put this in bevy state properly
fn model_path() -> std::path::PathBuf {
    std::env::args_os()
        .nth(1)
        .map(From::from)
        .unwrap_or_else(|| dirs::document_dir().unwrap().join("lego/aria/HQ.io"))
}

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
        .add_systems(Startup, (setup::setup, setup::link_steps).chain())
        .add_systems(
            Update,
            (setup::reset, setup::link_steps)
                .chain()
                .run_if(watch::file_touched(&model_path())),
        )
        .init_resource::<instruction::KeyBindings>()
        .init_resource::<instruction::CurrentStep>()
        .add_systems(Update, instruction::update)
        .add_systems(Update, instruction::camera_control)
        .run();
}
