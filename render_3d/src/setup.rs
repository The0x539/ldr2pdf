use ldr2pdf_common::{
    ldr::{ColorCode, ColorMap, GeometryContext},
    resolver::Resolver,
};
use std::collections::{HashMap, HashSet};
use weldr::SourceMap;

use bevy::{ecs::system::SystemParam, prelude::*, render::camera::Exposure};
use bevy_blendy_cameras::{FlyCameraController, OrbitCameraController};
#[cfg(feature = "line")]
use bevy_lines::prelude::*;

use crate::{
    ModelPath, ViewerConfig,
    instruction::{CurrentStep, SavedTransform},
    material::MyMaterial,
    primitives::PartData,
    traverse,
};

pub fn setup_plugin(app: &mut App) {
    let on_startup = (initial_setup, load_model, finalize_loading).chain();

    // TODO: set this up so that we can respond to a new path being selected.
    // this is likely to involve moving the file-touched check to a system?
    // in any case, having to get this in this fashion feels wrong
    let model_path = app.world().get_resource::<ModelPath>().unwrap();

    let on_update = (unload_model, load_model, finalize_loading)
        .chain()
        .run_if(crate::watch::file_touched(&model_path.0));

    app.add_systems(Startup, on_startup);
    app.add_systems(Update, on_update);
}

#[derive(SystemParam)]
struct ModelAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<MyMaterial>>,
    #[cfg(feature = "line")]
    lines: ResMut<'w, Assets<Polyline>>,
    #[cfg(feature = "line")]
    line_materials: ResMut<'w, Assets<PolylineMaterial>>,
}

#[derive(Component)]
#[require(Transform)]
pub struct ModelRoot;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct SceneRoot;

#[derive(Debug, Component)]
#[require(Transform, Visibility)]
pub struct Model;

#[derive(Debug, Component)]
#[require(Transform, Visibility, DoublyLinked)]
pub struct Step {
    pub index: usize,
}

// TODO: make this a relationship in bevy 0.16
#[derive(Debug, Component, Default)]
pub struct DoublyLinked {
    pub previous: Option<Entity>,
    pub next: Option<Entity>,
}

#[cfg(feature = "overlay")]
#[derive(Component)]
pub struct MyOverlay;

fn load_model(
    mut commands: Commands,
    mut model_assets: ModelAssets,
    model_path: Res<ModelPath>,
    viewer_config: Res<ViewerConfig>,
) {
    let path = &model_path.0;
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
        #[cfg(feature = "line")]
        line_material: PolylineMaterialHandle(model_assets.line_materials.add(PolylineMaterial {
            width: 3.0,
            color: Color::BLACK.into(),
            ..default()
        })),
        #[cfg(feature = "line")]
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

    let scene_root = commands.spawn((
        SceneRoot,
        if viewer_config.steps {
            Visibility::Hidden
        } else {
            Visibility::Visible
        },
        Transform::from_matrix(base_transform()),
    ));

    let model_root = handles.spawn_model(scene_root, &mut model_assets, &model);
    commands.entity(model_root).insert(ModelRoot);
}

pub fn base_transform() -> Mat4 {
    Mat4::from_rotation_z(std::f32::consts::PI) * Mat4::from_scale(Vec3::splat(0.05))
}

fn initial_setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,
    #[cfg(feature = "overlay")] viewer_config: Res<ViewerConfig>,
) {
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
        Projection::from(PerspectiveProjection {
            far: 1000.0,
            ..default()
        }),
        Exposure::INDOOR,
        Transform::from_xyz(90.0, 50.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCameraController {
            button_orbit: MouseButton::Right,
            button_pan: MouseButton::Middle,
            modifier_pan: None,
            ..default()
        },
        FlyCameraController {
            key_move_forward: KeyCode::KeyW,
            key_move_backward: KeyCode::KeyS,
            key_move_left: KeyCode::KeyA,
            key_move_right: KeyCode::KeyD,
            key_move_up: KeyCode::Space,
            key_move_down: KeyCode::ShiftLeft,
            button_rotate: MouseButton::Right,
            grab_cursor: true,
            is_enabled: false,
            speed: 20.0,
            ..default()
        },
    ));

    #[cfg(feature = "overlay")]
    {
        if viewer_config.show_fps {
            commands.spawn(iyes_perf_ui::entries::PerfUiAllEntries::default());
        }

        commands.spawn((Text::new(""), MyOverlay));
    }
}

fn unload_model(root: Option<Single<Entity, With<SceneRoot>>>, mut commands: Commands) {
    if let Some(root) = root {
        commands.entity(*root).despawn();
    }
}

fn finalize_loading(
    mut commands: Commands,
    steps: Query<&Step>,
    model_names: Query<&Name, With<Model>>,
    children: Query<&Children>,
    root: Query<Entity, With<ModelRoot>>,
    mut links: Query<&mut DoublyLinked>,
    mut current_step: ResMut<CurrentStep>,
    model_entities: Query<(Entity, &Transform), With<Model>>,
    transform_helper: TransformHelper,
) {
    let mut sequence: Vec<Entity> = vec![];
    traverse_hierarchy(
        &steps,
        &model_names,
        &children,
        root.single().unwrap(),
        &mut sequence,
    );

    for pair in sequence.windows(2) {
        let a = pair[0];
        let b = pair[1];
        links.get_mut(a).unwrap().next = Some(b);
        links.get_mut(b).unwrap().previous = Some(a);
    }

    // TODO: I tried many approaches to setting this up.
    // Many were foiled by GlobalTransform not propagating until LateUpdate.
    // Now that the math is figured out, determine if this is still the best approach.
    for (entity, local_t) in model_entities.iter() {
        let global = transform_helper
            .compute_global_transform(entity)
            .unwrap()
            .compute_matrix();

        let local = local_t.compute_matrix();
        let parent = global * local.inverse();
        let undo_parent = parent.inverse() * base_transform();

        commands
            .entity(entity)
            .insert(SavedTransform(Transform::from_matrix(undo_parent)));
    }

    // TODO: figure out how to remember step position across reloads of a model.
    // current naive attempts do not behave properly with respect to the visibility toggling of parts and submodels
    current_step.id = sequence[0];
    commands.trigger(crate::instruction::ChangeStep::Refresh);
}

fn traverse_hierarchy(
    steps: &Query<&Step>,
    model_names: &Query<&Name, With<Model>>,
    children: &Query<&Children>,

    current_model: Entity,
    sequence: &mut Vec<Entity>,
) {
    // to avoid including multiple copies of instructions for the same model in the same set
    let mut seen = HashSet::new();

    for &step_id in children.get(current_model).unwrap() {
        seen.clear();
        let Ok(step_items) = children.get(step_id) else {
            continue;
        };
        for &item_id in step_items {
            let Ok(submodel_name) = model_names.get(item_id) else {
                // this step item is not a submodel (i.e., it is a part)
                // this means that we don't need to recurse any deeper
                continue;
            };

            if !seen.insert(submodel_name) {
                // this isn't the first copy of this exact submodel within this same step,
                // so don't bother going over the steps a second/third/etc time
                continue;
            };

            traverse_hierarchy(steps, model_names, children, item_id, sequence);
        }
        sequence.push(step_id);
    }
}

struct Handles {
    part: HashMap<String, PartHandles>,
    material: HashMap<ColorCode, Handle<MyMaterial>>,
    #[cfg(feature = "line")]
    line_material: PolylineMaterialHandle,
    #[cfg(feature = "line")]
    opt_line_material: PolylineMaterialHandle,
    source_map: SourceMap,
    color_map: ColorMap,
}

#[derive(Clone)]
struct PartHandles {
    name: String,
    mesh: Handle<Mesh>,
    #[cfg(feature = "line")]
    line: Handle<Polyline>,
    #[cfg(feature = "line")]
    opt_line: Option<Handle<Polyline>>,
}

impl Handles {
    fn load_part(&mut self, part: &traverse::Part, assets: &mut ModelAssets) {
        if self.part.contains_key(&part.id) {
            return;
        }

        let data = PartData::load(&self.source_map, &part.id);

        self.part.insert(
            part.id.clone(),
            PartHandles {
                mesh: assets.meshes.add(data.build_mesh(&self.color_map)),
                #[cfg(feature = "line")]
                line: assets.lines.add(data.build_lines()),
                #[cfg(feature = "line")]
                opt_line: data.build_opt_lines().map(|l| assets.lines.add(l)),
                name: data.name,
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

        let part_name = ph.name.replace("  ", " ");
        let color_name = self
            .color_map
            .by_code(part.color)
            .name
            .replace("Trans_", "Trans-")
            .replace("_", " ");

        let bundle = (
            Mesh3d(ph.mesh),
            material.clone(),
            transform,
            Name::new(format!("{color_name} {part_name}")),
            ChildOf(parent.id()),
            Visibility::Inherited,
        );
        #[allow(unused_mut)]
        let mut part_entity = parent.commands_mut().spawn(bundle);

        #[cfg(feature = "outline")]
        part_entity.insert(bevy_mod_outline::InheritOutline);

        #[cfg(feature = "line")]
        part_entity.with_children(|c| {
            c.spawn(PolylineBundle {
                polyline: PolylineHandle(ph.line),
                material: self.line_material.clone(),
                transform: Transform::IDENTITY,
                ..default()
            });

            if let Some(opt_line) = ph.opt_line {
                c.spawn(PolylineBundle {
                    polyline: PolylineHandle(opt_line),
                    material: self.opt_line_material.clone(),
                    transform: Transform::IDENTITY,
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
        let bundle = (
            Transform::from_matrix(model.transform),
            ChildOf(parent.id()),
            Name::new(model.name.clone()),
            Model,
            #[cfg(feature = "outline")]
            bevy_mod_outline::InheritOutline,
        );
        let mut model_entity = parent.commands_mut().spawn(bundle);

        for (index, step) in model.steps.iter().enumerate() {
            self.spawn_step(model_entity.reborrow(), assets, step, index);
        }

        model_entity.id()
    }

    fn spawn_step(
        &mut self,
        mut parent: EntityCommands,
        assets: &mut ModelAssets,
        step: &traverse::Step,
        index: usize,
    ) -> Entity {
        let bundle = (ChildOf(parent.id()), Step { index });

        let mut step_entity = parent.commands_mut().spawn(bundle);

        if let Some(name) = &step.name {
            step_entity.insert(Name::new(name.to_owned()));
        }

        for item in &step.items {
            let step_entity = step_entity.reborrow();
            match item {
                traverse::StepItem::Part(part) => {
                    self.load_part(part, assets);
                    self.load_material(part.color, assets);
                    self.spawn_part(step_entity, part);
                }
                traverse::StepItem::Submodel(submodel) => {
                    self.spawn_model(step_entity, assets, submodel);
                }
            }
        }

        step_entity.id()
    }
}
