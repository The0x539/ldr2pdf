use bevy_lines::prelude::*;
use ldr2pdf_common::ldr::{
    CURRENT_COLOR, ColorCode, ColorMap, GeometryContext, Winding, new_color,
};
use std::collections::HashMap;
use weldr::{Command, SourceMap};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

#[derive(Default)]
pub struct Primitives {
    triangles: Vec<Triangle3d>,
    triangle_colors: HashMap<usize, ColorCode>,
    lines: Vec<[Vec3; 2]>,
    opt_lines: Vec<([Vec3; 2], [Vec3; 2])>,
}

impl Primitives {
    pub fn of_part(source_map: &SourceMap, model_name: &str) -> Self {
        let mut primitives = Self::default();
        let mut ctx = GeometryContext::new();
        ctx.transform = weldr::Mat4::IDENTITY;
        traverse_part(source_map, model_name, ctx, &mut primitives);
        primitives
    }

    pub fn build_mesh(&self, color_map: &ColorMap) -> Mesh {
        let mut positions = Vec::<Vec3>::new();
        let mut normals = Vec::<Vec3>::new();
        let mut colors = Vec::<Vec4>::new();
        let mut indices = Indices::U16(vec![]);
        let mut dedup = HashMap::<([u32; 3], [u32; 3], u32), u32>::new();

        // We want to use indexed vertices for memory efficiency,
        // but we also (usually?) want flat normals,
        // so we need to compute flat normals ourselves
        // and duplicate the vertex for each face it belongs to
        // TODO: Identify cases where we do want smooth normals

        for (triangle_index, triangle) in self.triangles.iter().enumerate() {
            let color_code = *self
                .triangle_colors
                .get(&triangle_index)
                .unwrap_or(&CURRENT_COLOR);

            let mut color = Vec4::ZERO;
            if color_code != CURRENT_COLOR {
                let c = color_map.by_code(color_code).value;
                color = Color::srgb_u8(c.red, c.green, c.blue).to_srgba().to_vec4();
            }

            let normal = triangle.normal().unwrap_or(Dir3::X).as_vec3();
            for vertex in triangle.vertices {
                let key = (bytemuck::cast(vertex), bytemuck::cast(normal), color_code);

                let index = *dedup.entry(key).or_insert_with(|| {
                    let i = positions.len() as u32;
                    positions.push(vertex);
                    normals.push(normal);
                    if !self.triangle_colors.is_empty() {
                        colors.push(color);
                    }
                    i
                });

                indices.push(index);
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if !self.triangle_colors.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        mesh.insert_indices(indices);
        mesh
    }

    pub fn build_lines(&self) -> Polyline {
        Polyline {
            vertices: self.lines.iter().copied().flatten().collect(),
            control_vertices: None,
        }
    }

    pub fn build_opt_lines(&self) -> Polyline {
        Polyline {
            vertices: self.opt_lines.iter().flat_map(|v| v.0).collect(),
            control_vertices: Some(self.opt_lines.iter().flat_map(|v| v.1).collect()),
        }
    }
}

fn traverse_part(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut Primitives,
) {
    let Some(model) = source_map.get(model_name) else {
        panic!("{model_name}");
    };

    let mut current_winding = Winding::Ccw;
    let mut current_inverted = ctx.inverted;

    if ctx.transform.determinant() < 0.0 {
        current_inverted = !current_inverted;
    }

    let mut invert_next = false;

    fn project<const N: usize>(ctx: &GeometryContext, vertices: [weldr::Vec3; N]) -> [Vec3; N] {
        ctx.project(vertices)
            .map(|v| Vec3::from_array(v.to_array()))
    }

    for cmd in &model.cmds {
        let effective_winding = if current_inverted {
            !current_winding
        } else {
            current_winding
        };

        let mut push_triangle = |vertices, color| {
            let color = new_color(ctx.color, color);
            let vertices = project(&ctx, vertices);
            let mut tri = Triangle3d { vertices };
            if effective_winding != Winding::Ccw {
                tri.reverse();
            }

            if color != CURRENT_COLOR {
                output.triangle_colors.insert(output.triangles.len(), color);
            }

            output.triangles.push(tri);
        };

        match cmd {
            Command::Comment(c) => {
                if c.text.starts_with("BFC CERTIFY") {
                    current_winding = match &*c.text {
                        "BFC CERTIFY CCW" => Winding::Ccw,
                        "BFC CERTIFY CW" => Winding::Cw,
                        _ => panic!("{}", c.text),
                    };
                } else if c.text.contains("BFC INVERTNEXT") {
                    invert_next = true;
                }
            }
            Command::SubFileRef(sfrc) => {
                let child = ctx.child(sfrc, invert_next);
                traverse_part(source_map, &sfrc.file, child, output);
                invert_next = false;
            }
            Command::Line(l) => output.lines.push(project(&ctx, l.vertices)),
            Command::OptLine(l) => {
                output
                    .opt_lines
                    .push((project(&ctx, l.vertices), project(&ctx, l.control_points)));
            }
            Command::Triangle(t) => {
                assert!(!invert_next);
                push_triangle(t.vertices, t.color);
            }
            Command::Quad(q) => {
                assert!(!invert_next);
                let [a, b, c, d] = q.vertices;
                push_triangle([a, b, c], q.color);
                push_triangle([c, d, a], q.color);
            }
            _ => {}
        }
    }
}
