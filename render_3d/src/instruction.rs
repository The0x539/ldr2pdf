use bevy::prelude::*;

use crate::setup::{DisplayRoot, DoublyLinked, Model, Step};

#[derive(Resource)]
pub struct KeyBindings {
    pub previous_step: KeyCode,
    pub next_step: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            previous_step: KeyCode::ArrowLeft,
            next_step: KeyCode::ArrowRight,
        }
    }
}

#[derive(Resource)]
pub struct CurrentStep(pub Entity);

impl FromWorld for CurrentStep {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

pub fn update(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    key_bindings: Res<KeyBindings>,
    models: Query<&Model>,
    display_root: Single<Entity, With<DisplayRoot>>,
    mut steps: Query<(&Step, &mut Visibility, &Parent)>,
    step_sequence: Query<&DoublyLinked>,
    mut current_step: ResMut<CurrentStep>,
) {
    let step_id = &mut current_step.0;
    let old_step_id = *step_id;

    if keys.just_pressed(key_bindings.previous_step) || keys.pressed(KeyCode::KeyH) {
        if let Some(previous) = step_sequence.get(*step_id).unwrap().previous {
            *step_id = previous;
        }
    }

    if keys.just_pressed(key_bindings.next_step) || keys.pressed(KeyCode::KeyL) {
        if let Some(next) = step_sequence.get(*step_id).unwrap().next {
            *step_id = next;
        }
    }

    if *step_id == old_step_id {
        return;
    }

    let old_model_id = steps.get(old_step_id).unwrap().2.get();
    let model_id = steps.get(*step_id).unwrap().2.get();

    println!(
        "{}[{}] -> {}[{}]",
        models.get(old_model_id).unwrap().name,
        steps.get(old_step_id).unwrap().0.index,
        models.get(model_id).unwrap().name,
        steps.get(*step_id).unwrap().0.index,
    );

    if model_id != old_model_id {
        let true_parent_id = models.get(old_model_id).unwrap().true_parent;
        commands.entity(old_model_id).set_parent(true_parent_id);
        commands.entity(model_id).set_parent(*display_root);
    }

    let show_up_to = steps.get(*step_id).unwrap().0.index;

    let model = models.get(model_id).unwrap();
    for (i, step_id) in model.steps.iter().enumerate() {
        let (_step, mut vis, _parent) = steps.get_mut(*step_id).unwrap();
        *vis = if i <= show_up_to {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
