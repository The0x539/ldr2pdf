use crate::{
    ldr::{new_color, ColorCode, ColorMap, GeometryContext, Winding},
    resolver::Resolver,
};
use weldr::{Command, Mat4, SourceMap};

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::*,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        mesh::PrimitiveTopology,
        settings::{Backends, RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    utils::HashMap,
};

pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    backends: Some(Backends::VULKAN),
                    ..default()
                }),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: WHITE.into(),
        })
        .insert_resource(bevy_flycam::MovementSettings {
            sensitivity: 0.00012,
            speed: 8.0,
        })
        .add_plugins(bevy_flycam::PlayerPlugin)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let resolver = Resolver::new(dirs::document_dir().unwrap().join("lego/penbu/ket.io")).unwrap();
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse("ket.io", &resolver, &mut source_map).unwrap();
    weldr::parse("3005.dat", &resolver, &mut source_map).unwrap();

    let color_map = ColorMap::load("C:/Program Files/Studio 2.0/ldraw/LDConfig.ldr").unwrap();

    let mut ctx = GeometryContext::new();
    ctx.transform = Mat4::from_rotation_z(std::f32::consts::PI)
        * glam::Mat4::from_scale(glam::Vec3::splat(0.05));

    let mut parts = Vec::new();
    traverse_design(&source_map, &main_model_name, ctx.clone(), &mut parts);

    let mut mesh_handles = HashMap::<String, Handle<Mesh>>::new();
    let mut mat_handles = HashMap::<ColorCode, Handle<StandardMaterial>>::new();

    ctx.transform = Mat4::IDENTITY;

    for part in &parts {
        if !mesh_handles.contains_key(&part.id) {
            let mut triangles = Default::default();
            traverse_part(&source_map, &part.id, ctx.clone(), &mut triangles);

            let vertices = triangles
                .iter()
                .flatten()
                .map(glam::Vec3::to_array)
                .collect::<Vec<_>>();

            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_computed_normals();

            mesh_handles.insert(part.id.clone(), meshes.add(mesh));
        }

        if !mat_handles.contains_key(&part.color) {
            let rgb = color_map.by_code(part.color).value;
            let [r, g, b] = [rgb.red, rgb.green, rgb.blue].map(|n| n as f32 / 255.0);
            let color = Color::srgb(r, g, b);
            mat_handles.insert(part.color, materials.add(color));
        }

        commands.spawn((
            Mesh3d(mesh_handles[&part.id].clone()),
            MeshMaterial3d(mat_handles[&part.color].clone()),
            Transform::from_matrix(part.transform),
        ));
    }

    commands.spawn((
        PointLight {
            intensity: 5_000_000.0,
            radius: 1.0,
            ..default()
        },
        Transform::from_xyz(6.0, 8.0, -10.0),
    ));
}

struct Part {
    id: String,
    color: ColorCode,
    transform: bevy::prelude::Mat4,
}

fn traverse_design(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Vec<Part>,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    for cmd in &model.cmds {
        match cmd {
            Command::Comment(..) => {}
            Command::SubFileRef(sfrc) => {
                let child_ctx = ctx.child(sfrc, false);
                if sfrc.file.ends_with(".dat") {
                    let transform =
                        bevy::prelude::Mat4::from_cols_array(&child_ctx.transform.to_cols_array());
                    let part = Part {
                        id: sfrc.file.clone(),
                        // TODO: respect currentcolor
                        color: new_color(child_ctx.color, sfrc.color),
                        transform,
                    };
                    output.push(part);
                } else {
                    traverse_design(source_map, &sfrc.file, child_ctx, output);
                }
            }
            Command::Line(_) | Command::OptLine(_) => panic!("line in {model_name}"),
            Command::Triangle(_) | Command::Quad(_) => panic!("polygon in {model_name}"),
            _ => {}
        }
    }
}

pub fn traverse_part(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Vec<[glam::Vec3; 3]>,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    let mut current_winding = Winding::Ccw;
    let mut current_inverted = ctx.inverted;

    if ctx.transform.determinant() < 0.0 {
        current_inverted = !current_inverted;
    }

    let mut invert_next = false;

    for cmd in &model.cmds {
        let effective_winding = if current_inverted {
            !current_winding
        } else {
            current_winding
        };

        match cmd {
            Command::Comment(c) => {
                if c.text.starts_with("BFC CERTIFY") {
                    current_winding = match &*c.text {
                        "BFC CERTIFY CCW" => Winding::Ccw,
                        "BFC CERTIFY CW" => Winding::Cw,
                        _ => panic!("{}", c.text),
                    };
                } else if c.text.contains("BFC INVERTNEXT") {
                    invert_next = true;
                }
            }
            Command::SubFileRef(sfrc) => {
                let child = ctx.child(sfrc, invert_next);
                traverse_part(source_map, &sfrc.file, child, output);
                invert_next = false;
            }
            Command::Line(_line) => {}
            Command::Triangle(t) => {
                assert!(!invert_next);

                // TODO: color of individual polygons
                let [a, b, c] = ctx.project(t.vertices);
                if effective_winding == Winding::Ccw {
                    output.push([a, b, c]);
                } else {
                    output.push([c, b, a]);
                }
            }
            Command::Quad(q) => {
                assert!(!invert_next);

                let [a, b, c, d] = ctx.project(q.vertices);
                if effective_winding == Winding::Ccw {
                    output.push([a, b, c]);
                    output.push([c, d, a]);
                } else {
                    output.push([c, b, a]);
                    output.push([a, d, c]);
                }
            }
            _ => {}
        }
    }
}
