use bevy_lines::prelude::*;
use bytemuck::NoUninit;
use flat_zip::FlatZipExt;
use ldr2pdf_common::ldr::{
    CURRENT_COLOR, ColorCode, ColorMap, GeometryContext, Winding, new_color,
};
use std::collections::{HashMap, HashSet};
use weldr::{Command, SourceMap};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::material::ATTRIBUTE_FLAGS;

#[derive(Default)]
pub struct Primitives {
    faces: Vec<TriOrQuad<Vec3>>,
    face_colors: HashMap<usize, ColorCode>,
    contrast_faces: HashSet<usize>,
    lines: Vec<[Vec3; 2]>,
    opt_lines: Vec<([Vec3; 2], [Vec3; 2])>,
}

#[derive(Copy, Clone)]
enum TriOrQuad<T> {
    Tri([T; 3]),
    Quad([T; 4]),
}

impl<T> TriOrQuad<T> {
    fn as_slice(&self) -> &[T] {
        match self {
            Self::Tri(x) => x,
            Self::Quad(x) => x,
        }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        match self {
            Self::Tri(x) => x,
            Self::Quad(x) => x,
        }
    }

    fn map<U>(self, f: impl FnMut(T) -> U) -> TriOrQuad<U> {
        match self {
            Self::Tri(x) => TriOrQuad::Tri(x.map(f)),
            Self::Quad(x) => TriOrQuad::Quad(x.map(f)),
        }
    }

    fn as_flat_tris<'a>(&'a self) -> impl Iterator<Item = &'a T> {
        let (first, second) = match self {
            Self::Tri([a, b, c]) => ([a, b, c], None),
            Self::Quad([a, b, c, d]) => ([a, b, c], Some([c, d, a])),
        };

        std::iter::once(first).chain(second).flatten()
    }
}

impl<'a, T> IntoIterator for &'a mut TriOrQuad<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a TriOrQuad<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().into_iter()
    }
}

impl TriOrQuad<Vec3> {
    fn first_triangle(&self) -> Triangle3d {
        let (Self::Tri([a, b, c]) | Self::Quad([a, b, c, _])) = *self;
        Triangle3d::new(a, b, c)
    }

    fn normal(&self) -> Vec3 {
        self.first_triangle().normal().unwrap_or(Dir3::X).as_vec3()
    }
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
        #[derive(Copy, Clone, NoUninit)]
        #[repr(C)]
        struct Vertex {
            position: Vec3,
            normal: Vec3,
            color: [f32; 4],
            flags: u32,
        }

        impl Vertex {
            fn key(&self) -> &[u32; 11] {
                bytemuck::cast_ref(self)
            }
        }

        let mut tris = vec![];

        for (triangle_index, triangle) in self.faces.iter().enumerate() {
            let color_code = *self
                .face_colors
                .get(&triangle_index)
                .unwrap_or(&CURRENT_COLOR);

            let mut color = [0.0; 4];
            if color_code != CURRENT_COLOR {
                let c = color_map.by_code(color_code).value;
                color = Color::srgb_u8(c.red, c.green, c.blue)
                    .to_srgba()
                    .to_f32_array();
            }

            let is_contrast = self.contrast_faces.contains(&triangle_index);

            let normal = triangle.normal();

            let tri = triangle.map(|position| Vertex {
                position,
                normal,
                color,
                flags: is_contrast as u32,
            });
            tris.push(tri);
        }

        let mut vert_to_edge = HashMap::<[u32; 3], Vec<usize>>::new();
        for (i, (points, _)) in self.opt_lines.iter().enumerate() {
            for point in points {
                let key: [u32; 3] = bytemuck::cast(*point);
                vert_to_edge.entry(key).or_default().push(i);
            }
        }

        let mut edge_to_face = HashMap::<[[u32; 3]; 2], Vec<usize>>::new();
        for (i, face) in self.faces.iter().enumerate() {
            let keys: &[[Vec3; 2]] = match *face {
                TriOrQuad::Tri([a, b, c]) => &[[a, b], [b, c], [c, a]],
                TriOrQuad::Quad([a, b, c, d]) => &[[a, b], [b, c], [c, d], [d, a]],
            };

            for key in keys.iter().copied().map(bytemuck::cast) {
                edge_to_face.entry(key).or_default().push(i);
            }
        }

        for (i, vert) in tris.iter_mut().enumerate().flat_zip() {
            let edge_indices = vert_to_edge
                .get(bytemuck::cast_ref::<_, [u32; 3]>(&vert.position))
                .map(Vec::as_slice)
                .unwrap_or_default();

            let face_indices = edge_indices.iter().flat_map(|j| {
                let [a, b] = self.opt_lines[*j].0.map(bytemuck::cast::<_, [u32; 3]>);
                let foo = edge_to_face.get(&[a, b]);
                let bar = edge_to_face.get(&[b, a]);
                foo.into_iter().chain(bar).flatten().copied()
            });

            let smooth = face_indices.clone().any(|j| j == i);
            if smooth {
                vert.normal = face_indices
                    .map(|j| self.faces[j].normal())
                    .sum::<Vec3>()
                    .normalize();
            }
        }

        let mut dedup = HashMap::new();
        let mut indices = Indices::U16(vec![]);

        let (mut positions, mut normals, mut colors, mut flags) = (vec![], vec![], vec![], vec![]);

        for vert in tris.iter().flat_map(TriOrQuad::as_flat_tris) {
            let index = *dedup.entry(vert.key()).or_insert_with(|| {
                let i = positions.len() as u32;

                positions.push(vert.position);
                normals.push(vert.normal);
                flags.push(vert.flags);
                if !self.face_colors.is_empty() {
                    colors.push(vert.color);
                }

                i
            });

            indices.push(index);
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if !self.face_colors.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        mesh.insert_attribute(ATTRIBUTE_FLAGS, flags);
        mesh.insert_indices(indices);
        mesh
    }

    pub fn build_lines(&self) -> Polyline {
        Polyline {
            vertices: self.lines.iter().copied().flatten().collect(),
            control_vertices: None,
        }
    }

    pub fn build_opt_lines(&self) -> Option<Polyline> {
        if self.opt_lines.is_empty() {
            return None;
        }
        Some(Polyline {
            vertices: self.opt_lines.iter().flat_map(|v| v.0).collect(),
            control_vertices: Some(self.opt_lines.iter().flat_map(|v| v.1).collect()),
        })
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

    fn is_stud_name(name: &&str) -> bool {
        [
            "stud.dat",
            "studa.dat",
            "stud26.dat",
            "stud2.dat",
            "stud2a.dat",
            "stud3.dat",
            "stud3a.dat",
            "stud4.dat",
            "stud4a.dat",
        ]
        .iter()
        .any(|x| name.ends_with(x))
    }

    // Are we the cylindrical part of a stud, to be drawn with high contrast?
    // True iff this polygon is descendant of the "4-4cyli.dat" submodel of a stud.
    let contrast = ctx
        .names
        .iter()
        .skip(1)
        .position(is_stud_name)
        .is_some_and(|i| ctx.names[i].ends_with("4-4cyli.dat"));

    for cmd in &model.cmds {
        let effective_winding = if current_inverted {
            !current_winding
        } else {
            current_winding
        };

        let mut push_triangle = |vertices: TriOrQuad<weldr::Vec3>, color| {
            let color = new_color(ctx.color, color);

            let mut vertices = vertices.map(|v| {
                let [v] = ctx.project([v]);
                Vec3::from_array(v.to_array())
            });

            if effective_winding != Winding::Ccw {
                vertices.as_mut_slice().reverse();
            }

            if color != CURRENT_COLOR {
                output.face_colors.insert(output.faces.len(), color);
            }
            if contrast {
                output.contrast_faces.insert(output.faces.len());
            }

            output.faces.push(vertices);
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
            Command::OptLine(l) if !contrast => {
                output
                    .opt_lines
                    .push((project(&ctx, l.vertices), project(&ctx, l.control_points)));
            }
            Command::Triangle(t) => {
                assert!(!invert_next);
                push_triangle(TriOrQuad::Tri(t.vertices), t.color);
            }
            Command::Quad(q) => {
                assert!(!invert_next);
                push_triangle(TriOrQuad::Quad(q.vertices), q.color);
                // push_triangle([c, d, a], q.color);
            }
            _ => {}
        }
    }
}
