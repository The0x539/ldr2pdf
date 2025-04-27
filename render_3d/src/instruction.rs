use bevy::prelude::*;

use crate::setup::{DoublyLinked, Model, Step};

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
    steps: Query<(&Step, &Parent)>,
    step_sequence: Query<&DoublyLinked>,
    mut vis: Query<&mut Visibility>,
    mut current_step: ResMut<CurrentStep>,
    #[cfg(feature = "outline")] child_steps: Query<&Children, With<Step>>,
) {
    let step_id = &mut current_step.0;
    let old_step_id = *step_id;

    if keys.just_pressed(key_bindings.previous_step) || keys.pressed(KeyCode::KeyH) {
        if let Some(previous) = step_sequence.get(*step_id).unwrap().previous {
            *vis.get_mut(*step_id).unwrap() = Visibility::Hidden;
            *step_id = previous;
        }
    }

    if keys.just_pressed(key_bindings.next_step) || keys.pressed(KeyCode::KeyL) {
        if let Some(next) = step_sequence.get(*step_id).unwrap().next {
            *step_id = next;
            *vis.get_mut(*step_id).unwrap() = Visibility::Inherited;
        }
    }

    if *step_id == old_step_id {
        return;
    }

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
        for child in child_steps.iter_descendants(old_step_id) {
            commands.entity(child).remove::<AnyOutline>();
        }

        let outline = OutlineVolume {
            visible: true,
            width: 4.0,
            colour: Color::srgb(1.0, 0.0, 0.0),
        };

        commands.entity(*step_id).insert(outline);
        for child in child_steps.iter_descendants(*step_id) {
            commands.entity(child).insert(InheritOutline);
        }
    }

    let model = models.get(model_id).unwrap();

    for (i, step_id) in model.steps.iter().enumerate() {
        *vis.get_mut(*step_id).unwrap() = if i <= show_up_to {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
