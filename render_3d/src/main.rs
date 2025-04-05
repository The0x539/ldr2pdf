use bevy_flycam::NoCameraPlayerPlugin;
use bevy_lines::prelude::*;
use iyes_perf_ui::prelude::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext, Winding, new_color},
    resolver::Resolver,
};
use weldr::{Command, SourceMap};

use bevy::{
    asset::RenderAssetUsages,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        RenderPlugin,
        camera::Exposure,
        diagnostic::RenderDiagnosticsPlugin,
        mesh::PrimitiveTopology,
        settings::{Backends, RenderCreation, WgpuSettings},
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
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            RenderDiagnosticsPlugin,
            PerfUiPlugin,
        ))
        .insert_resource(bevy_flycam::MovementSettings {
            sensitivity: 0.00012,
            speed: 15.0,
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,

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
    ctx.transform = weldr::Mat4::from_rotation_z(std::f32::consts::PI)
        * weldr::Mat4::from_scale(weldr::Vec3::splat(0.05));

    let mut parts = Vec::new();
    traverse_design(&source_map, &main_model_name, ctx.clone(), &mut parts);

    let mut handles = Handles::default();
    let line_material = line_materials.add(PolylineMaterial {
        width: 3.0,
        color: Color::BLACK.into(),
        ..default()
    });
    let opt_line_material = line_materials.add(PolylineMaterial {
        width: 6.0,
        color: Color::BLACK.into(),
        ..default()
    });

    for part in &parts {
        handles.load_part(&source_map, &part.id, &mut meshes, &mut lines);
        handles.load_material(&color_map, part.color, &mut materials);
        handles.spawn_part(
            &mut commands,
            part,
            line_material.clone(),
            opt_line_material.clone(),
        );
    }

    commands.spawn((
        PointLight {
            radius: 1.0,
            ..default()
        },
        Transform::from_xyz(6.0, 8.0, -10.0),
    ));
    ambient_light.brightness = 220.0;

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

    commands.spawn(PerfUiAllEntries::default());
}

#[derive(Default)]
struct Handles {
    part: HashMap<String, PartHandles>,
    material: HashMap<ColorCode, Handle<StandardMaterial>>,
}

#[derive(Clone)]
struct PartHandles {
    mesh: Handle<Mesh>,
    line: Handle<Polyline>,
    opt_line: Handle<Polyline>,
}

impl Handles {
    fn load_part(
        &mut self,
        source_map: &SourceMap,
        part_id: &str,
        meshes: &mut Assets<Mesh>,
        lines: &mut Assets<Polyline>,
    ) {
        if self.part.contains_key(part_id) {
            return;
        }

        let primitives = build_part_mesh(&source_map, &part_id);

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

        let line = Polyline {
            vertices: primitives.lines.into_iter().flatten().collect(),
            control_vertices: None,
        };
        let opt_line = Polyline {
            vertices: primitives.opt_lines.iter().flat_map(|v| v.0).collect(),
            control_vertices: Some(primitives.opt_lines.iter().flat_map(|v| v.1).collect()),
        };

        self.part.insert(
            part_id.to_owned(),
            PartHandles {
                mesh: meshes.add(mesh),
                line: lines.add(line),
                opt_line: lines.add(opt_line),
            },
        );
    }

    fn load_material(
        &mut self,
        color_map: &ColorMap,
        part_color: ColorCode,
        materials: &mut Assets<StandardMaterial>,
    ) {
        if self.material.contains_key(&part_color) {
            return;
        }

        let ldraw_color = color_map.by_code(part_color);
        let rgb = ldraw_color.value;
        let alpha = ldraw_color.alpha.unwrap_or(0xFF);
        let [r, g, b, a] = [rgb.red, rgb.green, rgb.blue, alpha].map(|n| n as f32 / 255.0);
        let color = Color::srgba(r, g, b, a);
        self.material.insert(part_color, materials.add(color));
    }

    fn spawn_part(
        &self,
        commands: &mut Commands,
        part: &Part,
        line_material: Handle<PolylineMaterial>,
        opt_line_material: Handle<PolylineMaterial>,
    ) {
        let ph = self.part[&part.id].clone();

        let material = MeshMaterial3d(self.material[&part.color].clone());
        let line_material = PolylineMaterialHandle(line_material.clone());
        let opt_line_material = PolylineMaterialHandle(opt_line_material.clone());

        let transform = Transform::from_matrix(part.transform);

        commands
            .spawn((Mesh3d(ph.mesh), material.clone(), transform))
            .with_children(|parent| {
                parent.spawn(PolylineBundle {
                    polyline: PolylineHandle(ph.line),
                    material: line_material.clone(),
                    ..default()
                });

                parent.spawn(PolylineBundle {
                    polyline: PolylineHandle(ph.opt_line),
                    material: opt_line_material.clone(),
                    ..default()
                });
            });
    }
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
                    let part = Part {
                        id: sfrc.file.clone(),
                        // TODO: respect currentcolor
                        color: new_color(child_ctx.color, sfrc.color),
                        transform: bevy_from_weldr_mat(child_ctx.transform),
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

fn bevy_from_weldr(a: weldr::Vec3) -> bevy::prelude::Vec3 {
    bevy::prelude::Vec3::from_array(a.to_array())
}

fn bevy_from_weldr_mat(a: weldr::Mat4) -> bevy::prelude::Mat4 {
    bevy::prelude::Mat4::from_cols_array(&a.to_cols_array())
}

#[derive(Default)]
struct Primitives {
    triangles: Vec<[Vec3; 3]>,
    lines: Vec<[Vec3; 2]>,
    opt_lines: Vec<([Vec3; 2], [Vec3; 2])>,
}

fn build_part_mesh(source_map: &SourceMap, model_name: &str) -> Primitives {
    let mut primitives = Primitives::default();
    let mut ctx = GeometryContext::new();
    ctx.transform = weldr::Mat4::IDENTITY;
    traverse_part(source_map, model_name, ctx, &mut primitives);
    primitives
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
                .push(ctx.project(l.vertices).map(bevy_from_weldr)),

            Command::OptLine(l) => {
                let [vertices, control_points] =
                    [l.vertices, l.control_points].map(|x| ctx.project(x).map(bevy_from_weldr));
                output.opt_lines.push((vertices, control_points));
            }
            Command::Triangle(t) => {
                assert!(!invert_next);

                // TODO: color of individual polygons
                let [a, b, c] = ctx.project(t.vertices).map(bevy_from_weldr);
                let to_push = if effective_winding == Winding::Ccw {
                    [a, b, c]
                } else {
                    [c, b, a]
                };
                output.triangles.push(to_push);
            }
            Command::Quad(q) => {
                assert!(!invert_next);

                let [a, b, c, d] = ctx.project(q.vertices).map(bevy_from_weldr);
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
