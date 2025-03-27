use bevy_flycam::NoCameraPlayerPlugin;
use bevy_lines::prelude::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext, Winding, new_color},
    resolver::Resolver,
};
use weldr::{Command, Mat4, SourceMap};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        RenderPlugin,
        camera::Exposure,
        mesh::PrimitiveTopology,
        settings::{Backends, RenderCreation, WgpuSettings},
        view::VisibilitySystems,
    },
    utils::HashMap,
};

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
        ))
        .insert_resource(bevy_flycam::MovementSettings {
            sensitivity: 0.00012,
            speed: 15.0,
        })
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, vis.after(VisibilitySystems::CheckVisibility))
        .run();
}

fn make_polyline(lines: impl IntoIterator<Item = [Vec3; 2]>) -> Polyline {
    Polyline {
        vertices: lines.into_iter().flatten().collect(),
    }
}

fn setup(
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut lines: ResMut<Assets<Polyline>>,
    mut line_materials: ResMut<Assets<PolylineMaterial>>,
) {
    let resolver = Resolver::new(dirs::document_dir().unwrap().join("lego/aria/HQ.io")).unwrap();
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse("HQ.io", &resolver, &mut source_map).unwrap();

    let color_map = ColorMap::load("C:/Program Files/Studio 2.0/ldraw/LDConfig.ldr").unwrap();

    let mut ctx = GeometryContext::new();
    ctx.transform = Mat4::from_rotation_z(std::f32::consts::PI)
        * glam::Mat4::from_scale(glam::Vec3::splat(0.05));

    let mut parts = Vec::new();
    traverse_design(&source_map, &main_model_name, ctx.clone(), &mut parts);

    let mut mesh_handles = HashMap::<String, Handle<Mesh>>::new();
    let mut polyline_handles = HashMap::<String, Handle<Polyline>>::new();
    let mut opt_polyline_handles = HashMap::<String, Handle<Polyline>>::new();
    let mut mat_handles = HashMap::<ColorCode, Handle<StandardMaterial>>::new();

    let line_material = line_materials.add(PolylineMaterial {
        width: 3.0,
        color: Color::BLACK.into(),
        ..default()
    });

    let opt_line_material = line_materials.add(PolylineMaterial {
        width: 6.0,
        color: Color::BLACK.into(),
        depth_bias: 0.1,
        ..default()
    });

    ctx.transform = Mat4::IDENTITY;

    for part in &parts {
        if !mesh_handles.contains_key(&part.id) {
            let mut primitives = Default::default();
            traverse_part(&source_map, &part.id, ctx.clone(), &mut primitives);

            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                primitives
                    .triangles
                    .iter()
                    .flatten()
                    .map(Vec3::to_array)
                    .collect::<Vec<_>>(),
            )
            .with_computed_normals();

            mesh_handles.insert(part.id.clone(), meshes.add(mesh));

            let polyline = make_polyline(primitives.lines);
            polyline_handles.insert(part.id.clone(), lines.add(polyline));

            let opt_polyline = make_polyline(primitives.opt_lines.iter().map(|&(a, _b)| a));
            opt_polyline_handles.insert(part.id.clone(), lines.add(opt_polyline));
        }

        if !mat_handles.contains_key(&part.color) {
            let ldraw_color = color_map.by_code(part.color);
            let rgb = ldraw_color.value;
            let alpha = ldraw_color.alpha.unwrap_or(0xFF);
            let [r, g, b, a] = [rgb.red, rgb.green, rgb.blue, alpha].map(|n| n as f32 / 255.0);
            let color = Color::srgba(r, g, b, a);
            mat_handles.insert(part.color, materials.add(color));
        }

        commands
            .spawn((
                Mesh3d(mesh_handles[&part.id].clone()),
                MeshMaterial3d(mat_handles[&part.color].clone()),
                Transform::from_matrix(part.transform),
            ))
            .with_children(|parent| {
                parent.spawn(PolylineBundle {
                    polyline: PolylineHandle(polyline_handles[&part.id].clone()),
                    material: PolylineMaterialHandle(line_material.clone()),
                    ..default()
                });

                parent.spawn(PolylineBundle {
                    polyline: PolylineHandle(opt_polyline_handles[&part.id].clone()),
                    material: PolylineMaterialHandle(opt_line_material.clone()),
                    ..default()
                });
            });
    }

    commands.spawn((
        PointLight {
            radius: 1.0,
            ..default()
        },
        Transform::from_xyz(6.0, 8.0, -10.0),
    ));

    commands.spawn((
        Camera3d::default(),
        PerspectiveProjection {
            far: 0.01,
            ..default()
        },
        Exposure::INDOOR,
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy_flycam::FlyCam,
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

fn bevy_from_glam(a: glam::Vec3) -> Vec3 {
    Vec3::from_array(a.to_array())
}

#[derive(Default)]
struct Primitives {
    triangles: Vec<[Vec3; 3]>,
    lines: Vec<[Vec3; 2]>,
    opt_lines: Vec<([Vec3; 2], [Vec3; 2])>,
}

fn traverse_part(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Primitives,
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
            Command::Line(l) => output
                .lines
                .push(ctx.project(l.vertices).map(bevy_from_glam)),

            Command::OptLine(l) => {
                let [vertices, control_points] =
                    [l.vertices, l.control_points].map(|x| ctx.project(x).map(bevy_from_glam));
                output.opt_lines.push((vertices, control_points));
            }
            Command::Triangle(t) => {
                assert!(!invert_next);

                // TODO: color of individual polygons
                let [a, b, c] = ctx.project(t.vertices).map(bevy_from_glam);
                let to_push = if effective_winding == Winding::Ccw {
                    [a, b, c]
                } else {
                    [c, b, a]
                };
                output.triangles.push(to_push);
            }
            Command::Quad(q) => {
                assert!(!invert_next);

                let [a, b, c, d] = ctx.project(q.vertices).map(bevy_from_glam);
                let to_push = if effective_winding == Winding::Ccw {
                    [[a, b, c], [c, d, a]]
                } else {
                    [[c, b, a], [a, d, c]]
                };
                output.triangles.extend(to_push);
            }
            _ => {}
        }
    }
}

fn vis(
    meshes: Query<&ViewVisibility, With<Mesh3d>>,
    mut polylines: Query<(&mut Visibility, &Parent), With<PolylineHandle>>,
) {
    for (mut child_vis, parent_id) in polylines.iter_mut() {
        let Ok(parent_vis) = meshes.get(parent_id.get()) else {
            continue;
        };

        *child_vis = if parent_vis.get() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
