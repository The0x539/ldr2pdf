use bevy::prelude::*;
use bevy_blendy_cameras::{
    FlyCameraController, OrbitCameraController, SwitchToFlyController, SwitchToOrbitController,
};

use crate::{
    ViewerConfig,
    setup::{DoublyLinked, Model, Step},
};

#[cfg(feature = "overlay")]
use {crate::setup::MyOverlay, indexmap::IndexMap, std::fmt::Write};

pub fn instruction_plugin(app: &mut App) {
    app.init_resource::<KeyBindings>()
        .init_resource::<CurrentStep>()
        .add_systems(Update, (input, update_focus).chain());

    app.world_mut().add_observer(change_step);
}

#[derive(Resource)]
struct KeyBindings {
    previous_step: KeyCode,
    next_step: KeyCode,
    toggle_camera_mode: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            previous_step: KeyCode::BrowserBack,
            next_step: KeyCode::BrowserForward,
            toggle_camera_mode: KeyCode::Backquote,
        }
    }
}

#[derive(Resource)]
pub struct CurrentStep {
    pub(crate) id: Entity,
}

impl FromWorld for CurrentStep {
    fn from_world(_world: &mut World) -> Self {
        Self {
            id: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Event, PartialEq, Eq)]
pub enum ChangeStep {
    Next,
    Previous,
    Refresh,
}

fn input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key_bindings: Res<KeyBindings>,
    camera: Single<(Entity, &OrbitCameraController, &FlyCameraController)>,
    mut fly_event: EventWriter<SwitchToFlyController>,
    mut orbit_event: EventWriter<SwitchToOrbitController>,
) {
    if keys.just_pressed(key_bindings.previous_step)
        || keys.pressed(KeyCode::KeyH)
        || mouse.just_pressed(MouseButton::Back)
    {
        commands.trigger(ChangeStep::Previous);
    }

    if keys.just_pressed(key_bindings.next_step)
        || keys.pressed(KeyCode::KeyL)
        || mouse.just_pressed(MouseButton::Forward)
    {
        commands.trigger(ChangeStep::Next);
    }

    if keys.just_pressed(key_bindings.toggle_camera_mode) {
        let (camera_entity, orbit, _fly) = *camera;
        if orbit.is_enabled {
            fly_event.write(SwitchToFlyController { camera_entity });
        } else {
            orbit_event.write(SwitchToOrbitController { camera_entity });
        }
    }
}

type WithModelOrStep = Or<(With<Model>, With<Step>)>;
#[cfg(any(feature = "outline", feature = "overlay"))]
type WithSolid = Or<(With<Model>, With<Step>, With<Mesh3d>)>;

fn change_step(
    trigger: Trigger<ChangeStep>,
    mut commands: Commands,
    models: Query<&Model>,
    steps: Query<(&Step, &ChildOf)>,
    step_sequence: Query<&DoublyLinked>,
    mut current_step: ResMut<CurrentStep>,
    mut vis: Query<&mut Visibility, WithModelOrStep>,
    viewer_config: Res<ViewerConfig>,
    #[cfg(any(feature = "outline", feature = "overlay"))] children: Query<&Children, WithSolid>,
    #[cfg(feature = "overlay")] parents: Query<&ChildOf, WithModelOrStep>,
    #[cfg(feature = "overlay")] mut text: Single<&mut Text, With<MyOverlay>>,
    #[cfg(feature = "overlay")] names: Query<&Name>,
) {
    if !viewer_config.steps {
        return;
    }

    // borrowck doesn't like my usage pattern with the smart pointer
    let current_step = &mut *current_step;

    let step_id = &mut current_step.id;
    let old_step_id = *step_id;

    match trigger.event() {
        ChangeStep::Previous => {
            if let Some(previous) = step_sequence.get(*step_id).unwrap().previous {
                *vis.get_mut(*step_id).unwrap() = Visibility::Hidden;
                *step_id = previous;
            }
        }
        ChangeStep::Next => {
            if let Some(next) = step_sequence.get(*step_id).unwrap().next {
                *step_id = next;
                *vis.get_mut(*step_id).unwrap() = Visibility::Inherited;
            }
        }
        ChangeStep::Refresh => {}
    }

    let old_model_id = steps.get(old_step_id).unwrap().1.parent();
    let model_id = steps.get(*step_id).unwrap().1.parent();

    if viewer_config.focus_submodels {
        if old_model_id != model_id {
            *vis.get_mut(old_model_id).unwrap() = Visibility::Inherited;
            *vis.get_mut(model_id).unwrap() = Visibility::Visible;

            commands.entity(old_model_id).insert(ToggleFocus);
            commands.entity(model_id).insert(ToggleFocus);
        } else if *trigger.event() == ChangeStep::Refresh {
            *vis.get_mut(model_id).unwrap() = Visibility::Visible;
            commands.entity(model_id).insert(ToggleFocus);
        }
    }

    let show_up_to = steps.get(*step_id).unwrap().0.index;

    #[cfg(feature = "outline")]
    {
        use bevy_mod_outline::{ComputedOutline, InheritOutline, OutlineMode, OutlineVolume};

        type AnyOutline = (OutlineVolume, InheritOutline, ComputedOutline);

        commands.entity(old_step_id).remove::<AnyOutline>();
        for child in children.iter_descendants(old_step_id) {
            commands.entity(child).remove::<AnyOutline>();
        }

        let outline = (
            OutlineVolume {
                visible: true,
                width: 4.0,
                colour: Color::srgb(1.0, 0.0, 0.0),
            },
            OutlineMode::FloodFlat,
        );

        commands.entity(*step_id).insert(outline);
        for child in children.iter_descendants(*step_id) {
            commands.entity(child).insert(InheritOutline);
        }
    }

    #[cfg(feature = "overlay")]
    {
        let ids = std::iter::once(*step_id)
            .chain(parents.iter_ancestors(*step_id))
            .collect::<Vec<_>>();

        let buf = &mut text.0;
        buf.clear();

        for id in ids.into_iter().rev() {
            if let Ok(model) = models.get(id) {
                buf.push_str(&model.name);
            } else if let Ok((step, _)) = steps.get(id) {
                write!(buf, ", step {}", step.index + 1).unwrap();
                if let Ok(name) = names.get(id) {
                    write!(buf, ": {name}").unwrap();
                }
                buf.push('\n');
            }
        }

        if let Ok(step_items) = children.get(*step_id) {
            let mut counts = IndexMap::new();
            for &step_item_id in step_items {
                let name = names.get(step_item_id).unwrap();
                *counts.entry(name).or_insert(0) += 1;
            }

            for (name, count) in counts {
                if count == 1 {
                    writeln!(buf, " - {name}").unwrap()
                } else {
                    writeln!(buf, " - {count}x {name}").unwrap()
                }
            }
        } else {
            buf.push_str("(empty step)\n");
        }
    }

    let model = models.get(model_id).unwrap();

    for (i, step_id) in model.steps.iter().enumerate() {
        *vis.get_mut(*step_id).unwrap() = if i <= show_up_to {
            if viewer_config.focus_submodels {
                Visibility::Inherited
            } else {
                Visibility::Visible
            }
        } else {
            Visibility::Hidden
        };
    }
}

#[derive(Component)]
struct ToggleFocus;

#[derive(Component)]
pub struct SavedTransform(pub Transform);

fn update_focus(
    mut commands: Commands,
    mut transforms: Query<(Entity, &mut Transform, &mut SavedTransform), With<ToggleFocus>>,
) {
    for (entity, mut active, mut saved) in transforms.iter_mut() {
        std::mem::swap(&mut *active, &mut saved.0);
        commands.entity(entity).remove::<ToggleFocus>();
    }
}
