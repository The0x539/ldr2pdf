use bevy::prelude::Mat4;
use ldr2pdf_common::ldr::{ColorCode, GeometryContext, new_color};
use weldr::{Command, SourceMap};

#[derive(Clone)]
pub(crate) struct Part {
    pub id: String,
    pub color: ColorCode,
    pub transform: Mat4,
}

pub(crate) struct Model {
    #[allow(dead_code)]
    pub name: String,
    pub steps: Vec<Step>,
    pub transform: Mat4,
}

#[derive(Default)]
pub(crate) struct Step {
    pub name: Option<String>,
    pub items: Vec<StepItem>,
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

pub(crate) enum StepItem {
    Part(Part),
    Submodel(Model),
}

pub(crate) fn traverse_design(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Model,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    let mut step = output.new_step();

    // LDraw puts `0 STEP` at the *end* of each step,
    // so we don't know that there's actually another step after until we see something afterwards.
    // Sometimes a designer may intentionally end a step sequence with a blank step,
    // in which case this approach leaves that untouched,
    // as opposed to the more obvious approach of just removing a trailing blank step.
    let mut step_is_real = false;

    for cmd in &model.cmds {
        step_is_real = true;
        match cmd {
            Command::Comment(c) => {
                if c.text == "STEP" {
                    step = output.new_step();
                    step_is_real = false;
                } else if let Some(step_name) = c.text.strip_prefix("STUDIOSTEPDESC") {
                    step.name = Some(step_name.trim().to_owned());
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

    if !step_is_real {
        assert!(step.items.is_empty());
        output.steps.pop();
    }
}
