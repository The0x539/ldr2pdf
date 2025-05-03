use std::path::PathBuf;

use bevy_lines::prelude::*;

use bevy::{
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
};

use argh::FromArgs;

mod instruction;
mod material;
mod primitives;
mod setup;
mod traverse;
mod watch;

#[derive(Resource)]
struct ModelPath(PathBuf);

#[derive(FromArgs, Debug)]
struct Args {
    #[argh(positional, default = "default_path()")]
    model_path: PathBuf,
}

fn default_path() -> PathBuf {
    dirs::document_dir().unwrap().join("lego/aria/HQ.io")
}

fn main() {
    let args = argh::from_env::<Args>();

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
        .insert_resource(ModelPath(args.model_path))
        .add_plugins(setup::setup_plugin)
        .add_plugins(instruction::instruction_plugin)
        .run();
}
