use bevy::prelude::*;

use crate::setup::{Model, ModelRoot, Step};

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

pub fn update(
    keys: Res<ButtonInput<KeyCode>>,
    key_bindings: Res<KeyBindings>,
    root: Single<(&Model, Entity), With<ModelRoot>>,
    models: Query<(&Model, Option<&Parent>)>,
    mut steps: Query<(&Step, &mut Visibility, &Parent)>,
    mut current_model_id: Local<Option<Entity>>,
    mut current_step: Local<usize>,
) {
    let mut update = false;

    let current_model_id: &mut Entity = current_model_id.get_or_insert_with(|| {
        update = true;
        *current_step = 1;
        root.1
    });

    if keys.just_pressed(key_bindings.previous_step) {
        update = true;
        *current_step = current_step.saturating_sub(1);
    }
    if keys.just_pressed(key_bindings.next_step) {
        update = true;
        *current_step += 1;
    }

    if !update {
        return;
    }

    let (current_model, _) = models.get(*current_model_id).unwrap();

    for (i, step_id) in current_model.steps.iter().enumerate() {
        let (_step, mut vis, _parent) = steps.get_mut(*step_id).unwrap();
        *vis = if i < *current_step {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
