use bevy_lines::prelude::*;
use bevy_mod_outline::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext, new_color},
    resolver::Resolver,
};
use std::collections::HashMap;
use weldr::{Command, SourceMap};

use bevy::{ecs::system::SystemParam, prelude::*, render::camera::Exposure};

use crate::{material::MyMaterial, primitives::Primitives};

#[derive(SystemParam)]
pub struct ModelAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<MyMaterial>>,
    lines: ResMut<'w, Assets<Polyline>>,
    line_materials: ResMut<'w, Assets<PolylineMaterial>>,
}

pub fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,
    mut model_assets: ModelAssets<'_>,
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

    let mut model = Model {
        name: file.to_string_lossy().into_owned(),
        transform: Mat4::IDENTITY,
        steps: vec![],
    };

    traverse_design(&source_map, &main_model_name, ctx.clone(), &mut model);

    let mut handles = Handles {
        part: HashMap::new(),
        material: HashMap::new(),
        line_material: PolylineMaterialHandle(model_assets.line_materials.add(PolylineMaterial {
            width: 3.0,
            color: Color::BLACK.into(),
            ..default()
        })),
        opt_line_material: PolylineMaterialHandle(model_assets.line_materials.add(
            PolylineMaterial {
                width: 6.0,
                color: Color::BLACK.into(),
                ..default()
            },
        )),
        source_map,
        color_map,
    };

    let base_transform =
        Mat4::from_rotation_z(std::f32::consts::PI) * Mat4::from_scale(Vec3::splat(0.05));

    commands
        .spawn((
            Transform::from_matrix(base_transform),
            InheritedVisibility::VISIBLE,
            OutlineVolume {
                visible: true,
                colour: Color::srgb(1.0, 0.0, 0.0),
                width: 4.0,
            },
        ))
        .with_children(|root| {
            handles.spawn_model(root, &model, &mut model_assets);
        });

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

struct Handles {
    part: HashMap<String, PartHandles>,
    material: HashMap<ColorCode, Handle<MyMaterial>>,
    line_material: PolylineMaterialHandle,
    opt_line_material: PolylineMaterialHandle,
    source_map: SourceMap,
    color_map: ColorMap,
}

#[derive(Clone)]
struct PartHandles {
    mesh: Handle<Mesh>,
    line: Handle<Polyline>,
    opt_line: Option<Handle<Polyline>>,
}

impl Handles {
    fn load_part(&mut self, part: &Part, assets: &mut ModelAssets) {
        if self.part.contains_key(&part.id) {
            return;
        }

        let primitives = Primitives::of_part(&self.source_map, &part.id);

        self.part.insert(
            part.id.clone(),
            PartHandles {
                mesh: assets.meshes.add(primitives.build_mesh(&self.color_map)),
                line: assets.lines.add(primitives.build_lines()),
                opt_line: primitives.build_opt_lines().map(|l| assets.lines.add(l)),
            },
        );
    }

    fn load_material(&mut self, part_color: ColorCode, assets: &mut ModelAssets) {
        if self.material.contains_key(&part_color) {
            return;
        }

        let ldraw_color = self.color_map.by_code(part_color);
        let rgb = ldraw_color.value;
        let alpha = ldraw_color.alpha.unwrap_or(0xFF);
        let [r, g, b, a] = [rgb.red, rgb.green, rgb.blue, alpha].map(|n| n as f32 / 255.0);
        let material = MyMaterial {
            base: StandardMaterial::from_color(Color::srgba(r, g, b, a)),
            extension: Default::default(),
        };
        self.material
            .insert(part_color, assets.materials.add(material));
    }

    fn spawn_part(&self, parent: &mut ChildBuilder<'_>, part: &Part) {
        let ph = self.part[&part.id].clone();

        let material = MeshMaterial3d(self.material[&part.color].clone());

        let transform = Transform::from_matrix(part.transform);

        parent
            .spawn((Mesh3d(ph.mesh), material.clone(), transform, InheritOutline))
            .with_children(|c| {
                c.spawn(PolylineBundle {
                    polyline: PolylineHandle(ph.line),
                    material: self.line_material.clone(),
                    ..default()
                });

                if let Some(opt_line) = ph.opt_line {
                    c.spawn(PolylineBundle {
                        polyline: PolylineHandle(opt_line),
                        material: self.opt_line_material.clone(),
                        ..default()
                    });
                }
            });
    }

    fn spawn_model(
        &mut self,
        parent: &mut ChildBuilder<'_>,
        model: &Model,
        assets: &mut ModelAssets<'_>,
    ) {
        for step in &model.steps {
            for item in &step.items {
                match item {
                    StepItem::Part(part) => {
                        self.load_part(part, assets);
                        self.load_material(part.color, assets);
                        self.spawn_part(parent, part);
                    }
                    StepItem::Submodel(submodel) => {
                        let bundle = (
                            Transform::from_matrix(submodel.transform),
                            InheritedVisibility::VISIBLE,
                            InheritOutline,
                        );
                        parent.spawn(bundle).with_children(|subparent| {
                            self.spawn_model(subparent, submodel, assets)
                        });
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct Part {
    id: String,
    color: ColorCode,
    transform: Mat4,
}

struct Model {
    #[allow(dead_code)]
    name: String,
    steps: Vec<Step>,
    transform: Mat4,
}

#[derive(Default)]
struct Step {
    items: Vec<StepItem>,
}

impl Model {
    fn new_step(&mut self) -> &mut Step {
        self.steps.push(Default::default());
        self.steps.last_mut().unwrap()
    }
}

impl Step {
    fn add_part(&mut self, part: Part) {
        self.items.push(StepItem::Part(part))
    }

    fn new_submodel(&mut self, name: String, transform: Mat4) -> &mut Model {
        self.items.push(StepItem::Submodel(Model {
            name,
            transform,
            steps: vec![],
        }));
        match self.items.last_mut() {
            Some(StepItem::Submodel(m)) => m,
            _ => unreachable!(),
        }
    }
}

enum StepItem {
    Part(Part),
    Submodel(Model),
}

fn traverse_design(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Model,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    let mut step = output.new_step();

    for cmd in &model.cmds {
        match cmd {
            Command::Comment(c) => {
                if c.text == "STEP" {
                    step = output.new_step();
                }
            }
            Command::SubFileRef(sfrc) => {
                let transform = Mat4::from_cols_array(&sfrc.matrix().to_cols_array());

                let child_ctx = ctx.child(sfrc, false);
                if sfrc.file.ends_with(".dat") {
                    let part = Part {
                        id: sfrc.file.clone(),
                        color: new_color(child_ctx.color, sfrc.color),
                        transform,
                    };
                    step.add_part(part);
                } else {
                    let submodel = step.new_submodel(sfrc.file.clone(), transform);
                    traverse_design(source_map, &sfrc.file, child_ctx, submodel);
                }
            }
            Command::Line(_) | Command::OptLine(_) => panic!("line in {model_name}"),
            Command::Triangle(_) | Command::Quad(_) => panic!("polygon in {model_name}"),
            _ => {}
        }
    }
}
