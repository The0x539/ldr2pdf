use bevy_lines::prelude::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext},
    resolver::Resolver,
};
use std::collections::HashMap;
use weldr::SourceMap;

use bevy::{ecs::system::SystemParam, prelude::*, render::camera::Exposure};

use crate::{material::MyMaterial, primitives::Primitives, traverse};

#[derive(SystemParam)]
pub struct ModelAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<MyMaterial>>,
    lines: ResMut<'w, Assets<Polyline>>,
    line_materials: ResMut<'w, Assets<PolylineMaterial>>,
}

#[derive(Component)]
pub struct ModelRoot;

#[derive(Default, Debug, Component)]
#[require(InheritedVisibility, Transform)]
pub struct Model {
    name: String,
    steps: Vec<Entity>,
}

#[derive(Default, Debug, Component)]
#[require(InheritedVisibility, Transform)]
pub struct Step {
    items: Vec<Entity>,
}

fn load_model(mut commands: Commands, mut model_assets: ModelAssets) {
    let path = crate::model_path();
    if !path.exists() {
        return;
    }
    let file = path.file_name().unwrap();

    let resolver = Resolver::new(&path).unwrap();
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse(&file, &resolver, &mut source_map).unwrap();

    let color_map = ColorMap::load("C:/Program Files/Studio 2.0/ldraw/LDConfig.ldr").unwrap();

    let mut ctx = GeometryContext::new();
    ctx.transform = weldr::Mat4::from_rotation_z(std::f32::consts::PI)
        * weldr::Mat4::from_scale(weldr::Vec3::splat(0.05));

    let mut model = traverse::Model {
        name: file.to_string_lossy().into_owned(),
        transform: Mat4::IDENTITY,
        steps: vec![],
    };

    traverse::traverse_design(&source_map, &main_model_name, ctx.clone(), &mut model);

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
            ModelRoot,
        ))
        .with_children(|root| {
            handles.spawn_model(root, &model, &mut model_assets);
        });
}

fn initial_setup(mut commands: Commands, mut ambient_light: ResMut<AmbientLight>) {
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

pub fn setup(
    mut commands: Commands,
    ambient_light: ResMut<AmbientLight>,
    model_assets: ModelAssets<'_>,
) {
    load_model(commands.reborrow(), model_assets);
    initial_setup(commands.reborrow(), ambient_light);
}

pub fn reset(
    root: Option<Single<Entity, With<ModelRoot>>>,
    mut commands: Commands,
    model_assets: ModelAssets,
) {
    if let Some(root) = root {
        commands.entity(*root).despawn_recursive();
    }
    load_model(commands.reborrow(), model_assets);
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
    fn load_part(&mut self, part: &traverse::Part, assets: &mut ModelAssets) {
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

    fn spawn_part(&self, parent: &mut ChildBuilder<'_>, part: &traverse::Part) -> Entity {
        let ph = self.part[&part.id].clone();

        let material = MeshMaterial3d(self.material[&part.color].clone());

        let transform = Transform::from_matrix(part.transform);

        let mut entity = parent.spawn((Mesh3d(ph.mesh), material.clone(), transform));

        #[cfg(feature = "outline")]
        entity.insert(bevy_mod_outline::InheritOutline);

        entity.with_children(|c| {
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

        entity.id()
    }

    fn spawn_model(
        &mut self,
        parent_model: &mut ChildBuilder,
        model: &traverse::Model,
        assets: &mut ModelAssets,
    ) -> Model {
        let mut model_component = Model::default();
        model_component.name = model.name.clone();

        for step in &model.steps {
            let mut step_entity = parent_model.spawn((Transform::IDENTITY,));
            let mut step_component = Default::default();
            step_entity.with_children(|parent_step| {
                step_component = self.spawn_step(parent_step, assets, step);
            });
            step_entity.insert(step_component);
            model_component.steps.push(step_entity.id());
        }

        model_component
    }

    fn spawn_step(
        &mut self,
        parent_step: &mut ChildBuilder,
        assets: &mut ModelAssets,
        step: &traverse::Step,
    ) -> Step {
        let mut step_component = Step::default();

        for item in &step.items {
            match item {
                traverse::StepItem::Part(part) => {
                    self.load_part(part, assets);
                    self.load_material(part.color, assets);
                    let part_entity = self.spawn_part(parent_step, part);
                    step_component.items.push(part_entity);
                }
                traverse::StepItem::Submodel(submodel) => {
                    let mut model_entity =
                        parent_step.spawn(Transform::from_matrix(submodel.transform));

                    let mut model_component = Default::default();

                    model_entity.with_children(|parent_model| {
                        model_component = self.spawn_model(parent_model, submodel, assets);
                    });

                    model_entity.insert(model_component);

                    #[cfg(feature = "outline")]
                    if submodel.name == "office level" {
                        let outline = bevy_mod_outline::OutlineVolume {
                            visible: true,
                            width: 4.0,
                            colour: Color::srgb(1.0, 0.0, 0.0),
                        };
                        model_entity.insert(outline);
                    } else {
                        model_entity.insert(bevy_mod_outline::InheritOutline);
                    }

                    step_component.items.push(model_entity.id());
                }
            }
        }

        step_component
    }
}
