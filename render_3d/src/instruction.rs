use bevy::prelude::*;
use bevy_blendy_cameras::{
    FlyCameraController, OrbitCameraController, SwitchToFlyController, SwitchToOrbitController,
};

use crate::setup::{DoublyLinked, Model, MyOverlay, Step};

pub fn instruction_plugin(app: &mut App) {
    app.init_resource::<KeyBindings>()
        .init_resource::<CurrentStep>()
        .add_systems(Update, update)
        .add_systems(Update, camera_control);
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
    pub(crate) fresh: bool,
}

impl FromWorld for CurrentStep {
    fn from_world(_world: &mut World) -> Self {
        Self {
            id: Entity::PLACEHOLDER,
            fresh: true,
        }
    }
}

fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key_bindings: Res<KeyBindings>,
    models: Query<&Model>,
    steps: Query<(&Step, &Parent)>,
    step_sequence: Query<&DoublyLinked>,
    mut vis: Query<&mut Visibility>,
    mut current_step: ResMut<CurrentStep>,
    #[cfg(feature = "outline")] children: Query<&Children, Or<(With<Model>, With<Step>)>>,
    #[cfg(feature = "overlay")] parents: Query<&Parent, Or<(With<Model>, With<Step>)>>,
    #[cfg(feature = "overlay")] mut text: Single<&mut Text, With<MyOverlay>>,
) {
    // borrowck doesn't like my usage pattern with the smart pointer
    let current_step = &mut *current_step;

    let step_id = &mut current_step.id;
    let old_step_id = *step_id;

    if keys.just_pressed(key_bindings.previous_step)
        || keys.pressed(KeyCode::KeyH)
        || mouse.just_pressed(MouseButton::Back)
    {
        if let Some(previous) = step_sequence.get(*step_id).unwrap().previous {
            *vis.get_mut(*step_id).unwrap() = Visibility::Hidden;
            *step_id = previous;
        }
    }

    if keys.just_pressed(key_bindings.next_step)
        || keys.pressed(KeyCode::KeyL)
        || mouse.just_pressed(MouseButton::Forward)
    {
        if let Some(next) = step_sequence.get(*step_id).unwrap().next {
            *step_id = next;
            *vis.get_mut(*step_id).unwrap() = Visibility::Inherited;
        }
    }

    if *step_id == old_step_id && !current_step.fresh {
        return;
    }

    current_step.fresh = false;

    let old_model_id = steps.get(old_step_id).unwrap().1.get();
    let model_id = steps.get(*step_id).unwrap().1.get();

    // WIP feature: this hides everything but the current submodel,
    // but I would also like for it to not apply the parent's transform
    const FOCUS_SUBMODELS: bool = false;

    if FOCUS_SUBMODELS {
        *vis.get_mut(old_model_id).unwrap() = Visibility::Inherited;
        *vis.get_mut(model_id).unwrap() = Visibility::Visible;
    }

    let show_up_to = steps.get(*step_id).unwrap().0.index;

    #[cfg(feature = "outline")]
    {
        use bevy_mod_outline::{ComputedOutline, InheritOutline, OutlineVolume};

        type AnyOutline = (OutlineVolume, InheritOutline, ComputedOutline);

        commands.entity(old_step_id).remove::<AnyOutline>();
        for child in children.iter_descendants(old_step_id) {
            commands.entity(child).remove::<AnyOutline>();
        }

        let outline = OutlineVolume {
            visible: true,
            width: 4.0,
            colour: Color::srgb(1.0, 0.0, 0.0),
        };

        commands.entity(*step_id).insert(outline);
        for child in children.iter_descendants(*step_id) {
            commands.entity(child).insert(InheritOutline);
        }
    }

    #[cfg(feature = "overlay")]
    {
        let mut lines = vec![format!("step {}", show_up_to + 1)];
        for id in parents.iter_ancestors(*step_id) {
            if let Ok(model) = models.get(id) {
                lines.push(model.name.clone());
            } else if let Ok((step, _)) = steps.get(id) {
                lines.push(format!("step {}", step.index + 1));
            }
        }
        lines.reverse();
        text.0 = lines.join("\n");
    }

    let model = models.get(model_id).unwrap();

    for (i, step_id) in model.steps.iter().enumerate() {
        *vis.get_mut(*step_id).unwrap() = if i <= show_up_to {
            if FOCUS_SUBMODELS {
                Visibility::Inherited
            } else {
                Visibility::Visible
            }
        } else {
            Visibility::Hidden
        };
    }
}

fn camera_control(
    camera: Single<(Entity, &OrbitCameraController, &FlyCameraController)>,
    keys: Res<ButtonInput<KeyCode>>,
    key_bindings: Res<KeyBindings>,
    mut to_fly: EventWriter<SwitchToFlyController>,
    mut to_orbit: EventWriter<SwitchToOrbitController>,
) {
    let (camera_entity, orbit, _fly) = *camera;

    let kjp = keys.get_just_pressed().collect::<Vec<_>>();
    if !kjp.is_empty() {
        println!("{kjp:?}");
    }

    if keys.just_pressed(key_bindings.toggle_camera_mode) {
        if orbit.is_enabled {
            to_fly.send(SwitchToFlyController { camera_entity });
        } else {
            to_orbit.send(SwitchToOrbitController { camera_entity });
        };
    }
}
