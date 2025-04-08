use bevy_lines::prelude::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext, new_color},
    resolver::Resolver,
};
use std::collections::HashMap;
use weldr::{Command, SourceMap};

use bevy::{prelude::*, render::camera::Exposure};

use crate::{material::MyMaterial, primitives::Primitives};

pub fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,

    mut lines: ResMut<Assets<Polyline>>,
    mut line_materials: ResMut<Assets<PolylineMaterial>>,
) {
    let path = dirs::document_dir().unwrap().join("lego/aria/HQ.io");
    let file = path.file_name().unwrap();

    let resolver = Resolver::new(&path).unwrap();
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse(&file, &resolver, &mut source_map).unwrap();

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
        handles.load_part(&source_map, &color_map, part, &mut meshes, &mut lines);
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

    #[cfg(feature = "overlay")]
    commands.spawn(iyes_perf_ui::entries::PerfUiAllEntries::default());
}

#[derive(Default)]
struct Handles {
    part: HashMap<String, PartHandles>,
    material: HashMap<ColorCode, Handle<MyMaterial>>,
}

#[derive(Clone)]
struct PartHandles {
    mesh: Handle<Mesh>,
    line: Handle<Polyline>,
    opt_line: Option<Handle<Polyline>>,
}

impl Handles {
    fn load_part(
        &mut self,
        source_map: &SourceMap,
        color_map: &ColorMap,
        part: &Part,
        meshes: &mut Assets<Mesh>,
        lines: &mut Assets<Polyline>,
    ) {
        if self.part.contains_key(&part.id) {
            return;
        }

        let primitives = Primitives::of_part(&source_map, &part.id);

        self.part.insert(
            part.id.clone(),
            PartHandles {
                mesh: meshes.add(primitives.build_mesh(&color_map)),
                line: lines.add(primitives.build_lines()),
                opt_line: primitives.build_opt_lines().map(|l| lines.add(l)),
            },
        );
    }

    fn load_material(
        &mut self,
        color_map: &ColorMap,
        part_color: ColorCode,
        materials: &mut Assets<MyMaterial>,
    ) {
        if self.material.contains_key(&part_color) {
            return;
        }

        let ldraw_color = color_map.by_code(part_color);
        let rgb = ldraw_color.value;
        let alpha = ldraw_color.alpha.unwrap_or(0xFF);
        let [r, g, b, a] = [rgb.red, rgb.green, rgb.blue, alpha].map(|n| n as f32 / 255.0);
        let material = MyMaterial {
            base: StandardMaterial::from_color(Color::srgba(r, g, b, a)),
            extension: Default::default(),
        };
        self.material.insert(part_color, materials.add(material));
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

                if let Some(opt_line) = ph.opt_line {
                    parent.spawn(PolylineBundle {
                        polyline: PolylineHandle(opt_line),
                        material: opt_line_material.clone(),
                        ..default()
                    });
                }
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
                        color: new_color(child_ctx.color, sfrc.color),
                        transform: Mat4::from_cols_array(&child_ctx.transform.to_cols_array()),
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
