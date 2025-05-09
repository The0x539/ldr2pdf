use std::path::PathBuf;

use bevy::{
    asset::UnapprovedPathMode,
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

#[derive(Resource)]
struct ViewerConfig {
    steps: bool,
    focus_submodels: bool,
    #[cfg(feature = "overlay")]
    show_fps: bool,
}

#[derive(FromArgs, Debug)]
struct Args {
    #[argh(positional, default = "default_path()")]
    model_path: PathBuf,
    #[argh(switch)]
    no_steps: bool,
    #[argh(switch)]
    no_focus_submodels: bool,
    #[cfg(feature = "overlay")]
    #[argh(switch)]
    show_fps: bool,
}

fn default_path() -> PathBuf {
    dirs::document_dir().unwrap().join("lego/aria/HQ.io")
}

fn main() {
    let args = argh::from_env::<Args>();
    let viewer_config = ViewerConfig {
        steps: !args.no_steps,
        focus_submodels: !args.no_focus_submodels,
        #[cfg(feature = "overlay")]
        show_fps: args.show_fps,
    };

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // TODO: figure out the correct file_path
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
            #[cfg(feature = "line")]
            bevy_lines::PolylinePlugin,
            bevy_blendy_cameras::BlendyCamerasPlugin,
            MaterialPlugin::<material::MyMaterial> {
                prepass_enabled: false,
                shadows_enabled: false,
                ..default()
            },
            #[cfg(feature = "overlay")]
            (
                bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
                bevy::diagnostic::EntityCountDiagnosticsPlugin,
                bevy::render::diagnostic::RenderDiagnosticsPlugin,
                iyes_perf_ui::PerfUiPlugin,
            ),
            #[cfg(feature = "outline")]
            bevy_mod_outline::OutlinePlugin,
        ))
        .insert_resource(ModelPath(args.model_path))
        .insert_resource(viewer_config)
        .add_plugins(setup::setup_plugin)
        .add_plugins(instruction::instruction_plugin)
        .run();
}
