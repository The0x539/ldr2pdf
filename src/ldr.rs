use std::collections::HashMap;

use crate::{Point, Poly, Primitive};
use slab::Slab;
use weldr::{ColourCmd, Command, Mat4, SourceMap, Vec3};

pub fn traverse(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Vec<Primitive>,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    for cmd in &model.cmds {
        match cmd {
            Command::Comment(..) => {}
            Command::SubFileRef(sfrc) => {
                traverse(source_map, &sfrc.file, ctx.child(sfrc), output);
            }
            Command::Line(line) => {
                output.push(Primitive::Line(ctx.project(line.vertices)));
            }
            Command::Triangle(t) => {
                let color = new_color(ctx.color, t.color);
                let poly = Poly::Tri(ctx.project(t.vertices));
                output.push(Primitive::Polygon(poly, color));
            }
            Command::Quad(q) => {
                let color = new_color(ctx.color, q.color);
                let poly = Poly::Quad(ctx.project(q.vertices));
                output.push(Primitive::Polygon(poly, color));
            }
            _ => {}
        }
    }
}

pub type ColorCode = u32;
// Special color code that "inherits" the existing color.
const CURRENT_COLOR: ColorCode = 16;

pub struct GeometryContext {
    transform: Mat4,
    color: ColorCode,
}

impl GeometryContext {
    pub fn new() -> Self {
        let alpha = 30.0_f32.to_radians().tan().asin();
        let beta = 45.0_f32.to_radians();
        let transform = Mat4::from_rotation_x(alpha) * Mat4::from_rotation_y(beta);
        Self {
            transform,
            color: CURRENT_COLOR,
        }
    }

    pub fn child(&self, subfile: &weldr::SubFileRefCmd) -> Self {
        Self {
            transform: self.transform * subfile.matrix(),
            color: new_color(self.color, subfile.color),
        }
    }

    pub fn project<const N: usize>(&self, vertices: [Vec3; N]) -> [Point; N] {
        vertices.map(|v| self.transform.transform_point3(v))
    }
}

fn new_color(current: ColorCode, new: ColorCode) -> ColorCode {
    if new == CURRENT_COLOR {
        current
    } else {
        new
    }
}

#[derive(Default)]
pub struct ColorMap {
    codes: HashMap<ColorCode, usize>,
    names: HashMap<String, usize>,
    values: Slab<ColourCmd>,
}

impl ColorMap {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        let cmds = weldr::parse_raw(&data)?;
        let mut map = ColorMap::default();

        for cmd in cmds {
            let Command::Colour(c) = cmd else { continue };
            let key = map.values.insert(c.clone());
            map.codes.insert(c.code, key);
            map.names.insert(c.name, key);
        }

        Ok(map)
    }

    // pub fn by_name(&self, name: &str) -> &ColourCmd {
    //     &self.values[self.names[name]]
    // }

    pub fn by_code(&self, code: ColorCode) -> &ColourCmd {
        &self.values[*self.codes.get(&code).unwrap_or(&0)]
    }
}
