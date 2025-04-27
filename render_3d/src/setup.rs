use bevy_lines::prelude::*;
use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext},
    resolver::Resolver,
};
use std::collections::HashMap;
use weldr::SourceMap;

use bevy::{ecs::system::SystemParam, prelude::*, render::camera::Exposure};

use crate::{instruction::CurrentStep, material::MyMaterial, primitives::Primitives, traverse};

#[derive(SystemParam)]
pub struct ModelAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<MyMaterial>>,
    lines: ResMut<'w, Assets<Polyline>>,
    line_materials: ResMut<'w, Assets<PolylineMaterial>>,
}

#[derive(Component)]
#[require(Transform)]
pub struct ModelRoot;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct ShadowRealm;

#[derive(Debug, Component)]
#[require(Transform, Visibility)]
pub struct Model {
    pub name: String,
    pub steps: Vec<Entity>,
    // pub true_parent: Entity,
}

#[derive(Debug, Component)]
#[require(Transform, Visibility, DoublyLinked)]
pub struct Step {
    pub items: Vec<Entity>,
    pub index: usize,
}

// TODO: make this a relationship in bevy 0.16
#[derive(Debug, Component, Default)]
pub struct DoublyLinked {
    pub previous: Option<Entity>,
    pub next: Option<Entity>,
}

#[derive(Component)]
pub struct MyOverlay;

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

    // A hidden entity that serves as the root of the hierarchy,
    // so that descendants must opt IN to being visible.
    // Originally intended as a "holding area" for whatever isn't the current submodel.
    let shadow_realm = commands.spawn((
        ShadowRealm,
        Visibility::Hidden,
        Transform::from_matrix(base_transform),
    ));

    let model_root = handles.spawn_model(shadow_realm, &mut model_assets, &model);
    commands.entity(model_root).insert(ModelRoot);
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
        Transform::from_xyz(90.0, 50.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy_flycam::FlyCam,
    ));

    #[cfg(feature = "overlay")]
    commands.spawn(iyes_perf_ui::entries::PerfUiAllEntries::default());

    #[cfg(feature = "overlay")]
    commands.spawn((Text::new(""), MyOverlay));
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

pub fn link_steps(
    steps: Query<&Step>,
    models: Query<&Model>,
    root: Query<&Model, With<ModelRoot>>,
    mut links: Query<&mut DoublyLinked>,
    mut current_step: ResMut<CurrentStep>,
) {
    let mut sequence: Vec<Entity> = vec![];
    traverse_hierarchy(&steps, &models, root.single(), &mut sequence);

    for pair in sequence.windows(2) {
        let a = pair[0];
        let b = pair[1];
        links.get_mut(a).unwrap().next = Some(b);
        links.get_mut(b).unwrap().previous = Some(a);
    }

    current_step.id = sequence[0];
    current_step.fresh = true;
}

fn traverse_hierarchy(
    steps: &Query<&Step>,
    models: &Query<&Model>,

    current_model: &Model,
    sequence: &mut Vec<Entity>,
) {
    for &step_id in &current_model.steps {
        let step = steps.get(step_id).unwrap();
        for &item_id in &step.items {
            if let Ok(submodel) = models.get(item_id) {
                traverse_hierarchy(steps, models, submodel, sequence);
            }
        }
        sequence.push(step_id);
    }
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

    fn spawn_part(&self, mut parent: EntityCommands, part: &traverse::Part) -> Entity {
        let ph = self.part[&part.id].clone();

        let material = MeshMaterial3d(self.material[&part.color].clone());

        let transform = Transform::from_matrix(part.transform);

        let bundle = (Mesh3d(ph.mesh), material.clone(), transform);
        let parent_id = parent.id();
        let mut part_entity = parent.commands_mut().spawn(bundle);
        part_entity.set_parent(parent_id);

        #[cfg(feature = "outline")]
        part_entity.insert(bevy_mod_outline::InheritOutline);

        part_entity.with_children(|c| {
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

        part_entity.id()
    }

    fn spawn_model(
        &mut self,
        mut parent: EntityCommands,
        assets: &mut ModelAssets,
        model: &traverse::Model,
    ) -> Entity {
        let parent_id = parent.id();

        let transform = Transform::from_matrix(model.transform);
        let mut model_entity = parent.commands_mut().spawn(transform);
        model_entity.set_parent(parent_id);

        let mut model_component = Model {
            name: model.name.clone(),
            steps: vec![],
            // true_parent: parent_id,
        };

        for (index, step) in model.steps.iter().enumerate() {
            let step_entity = self.spawn_step(model_entity.reborrow(), assets, step, index);
            model_component.steps.push(step_entity);
        }

        #[cfg(feature = "outline")]
        model_entity.insert(bevy_mod_outline::InheritOutline);

        model_entity.insert(model_component);
        model_entity.id()
    }

    fn spawn_step(
        &mut self,
        mut parent: EntityCommands,
        assets: &mut ModelAssets,
        step: &traverse::Step,
        index: usize,
    ) -> Entity {
        let parent_id = parent.id();

        let mut step_entity = parent.commands_mut().spawn_empty();
        step_entity.set_parent(parent_id);

        let mut step_component = Step {
            items: vec![],
            index,
        };

        for item in &step.items {
            let step_entity = step_entity.reborrow();
            match item {
                traverse::StepItem::Part(part) => {
                    self.load_part(part, assets);
                    self.load_material(part.color, assets);
                    let part_entity = self.spawn_part(step_entity, part);
                    step_component.items.push(part_entity);
                }
                traverse::StepItem::Submodel(submodel) => {
                    let model_entity = self.spawn_model(step_entity, assets, submodel);
                    step_component.items.push(model_entity);
                }
            }
        }

        step_entity.insert(step_component);
        step_entity.id()
    }
}
