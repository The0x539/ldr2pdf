use bevy_lines::prelude::*;
use bytemuck::NoUninit;
use flat_zip::FlatZipExt;
use ldr2pdf_common::ldr::{
    CURRENT_COLOR, ColorCode, ColorMap, GeometryContext, Winding, new_color,
};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use weldr::{Command, SourceMap};

use bevy::{prelude::*, render::mesh::Indices};

use crate::material::ATTRIBUTE_FLAGS;

#[derive(Default)]
pub struct PartData {
    pub name: String,
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

    fn edges(&self) -> TriOrQuad<[T; 2]>
    where
        T: Copy,
    {
        match *self {
            Self::Tri([a, b, c]) => TriOrQuad::Tri([[a, b], [b, c], [c, a]]),
            Self::Quad([a, b, c, d]) => TriOrQuad::Quad([[a, b], [b, c], [c, d], [d, a]]),
        }
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

struct AttributeFace {
    color: Option<[u8; 3]>,
    flags: u32,
    verts: TriOrQuad<AttributeVertex>,
}

#[derive(Copy, Clone, NoUninit)]
#[repr(C)]
struct AttributeVertex {
    position: Vec3,
    normal: Vec3,
}

trait AsKey {
    type Key: Hash + Eq + Ord;
    fn as_key(&self) -> &Self::Key;
    fn to_key(&self) -> Self::Key;
}

macro_rules! as_key {
    ($T:ty as $U:ty) => {
        impl AsKey for $T {
            type Key = $U;
            fn as_key(&self) -> &Self::Key {
                bytemuck::cast_ref(self)
            }
            fn to_key(&self) -> Self::Key {
                bytemuck::cast(*self)
            }
        }
    };
}

as_key!(Vec3 as [u32; 3]);
as_key!([Vec3; 2] as [u32; 6]);
as_key!(AttributeVertex as [u32; 6]);

impl PartData {
    pub fn load(source_map: &SourceMap, model_name: &str) -> Self {
        let mut output = Self::default();

        // we hope to find a human-friendly name at the start of the root file, but that's not guaranteed
        output.name = model_name.to_owned();

        let mut ctx = GeometryContext::new();
        ctx.transform = weldr::Mat4::IDENTITY;
        traverse_part(source_map, model_name, ctx, &mut output, true);
        output
    }

    pub fn build_mesh(&self, color_map: &ColorMap) -> Mesh {
        let mut attr_faces = vec![];

        for (face_index, face) in self.faces.iter().enumerate() {
            let color = self
                .face_colors
                .get(&face_index)
                .map(|c| color_map.by_code(*c).value)
                .map(|c| [c.red, c.green, c.blue]);

            let is_contrast = self.contrast_faces.contains(&face_index);

            let normal = face.normal();
            let verts = face.map(|position| AttributeVertex { position, normal });

            attr_faces.push(AttributeFace {
                color,
                flags: is_contrast as u32,
                verts,
            });
        }

        self.compute_smooth_normals(&mut attr_faces);

        let include_colors = !self.face_colors.is_empty();

        let IndexedVertices {
            indices,
            positions,
            normals,
            colors,
            flags,
        } = generate_indices(&attr_faces, include_colors);

        let mut mesh = Mesh::new(default(), default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if include_colors {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        mesh.insert_attribute(ATTRIBUTE_FLAGS, flags);
        mesh.insert_indices(indices);

        mesh
    }

    fn compute_smooth_normals(&self, attr_faces: &mut [AttributeFace]) {
        let mut vert_to_edge = HashMap::<_, Vec<usize>>::new();
        for (i, (points, _)) in self.opt_lines.iter().enumerate() {
            for point in points {
                vert_to_edge.entry(point.to_key()).or_default().push(i);
            }
        }

        let mut edge_to_face = HashMap::<_, Vec<usize>>::new();
        for (i, face) in self.faces.iter().enumerate() {
            for edge in &face.edges() {
                edge_to_face.entry(edge.to_key()).or_default().push(i);
            }
        }

        for (i, vert) in attr_faces
            .iter_mut()
            .map(|face| &mut face.verts)
            .enumerate()
            .flat_zip()
        {
            let Some(edge_indices) = vert_to_edge.get(vert.position.as_key()) else {
                continue;
            };

            let face_indices = edge_indices
                .iter()
                .map(|j| self.opt_lines[*j].0)
                .flat_map(|[a, b]| [[a, b], [b, a]])
                .filter_map(|edge| edge_to_face.get(edge.as_key()))
                .flatten()
                .copied();

            let smooth = face_indices.clone().any(|j| j == i);
            if smooth {
                vert.normal = face_indices
                    .map(|j| self.faces[j].normal())
                    .sum::<Vec3>()
                    .normalize();
            }
        }
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

struct IndexedVertices {
    indices: Indices,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    colors: Vec<[f32; 4]>,
    flags: Vec<u32>,
}

fn generate_indices(faces: &[AttributeFace], include_colors: bool) -> IndexedVertices {
    let mut indices = Indices::U16(vec![]);
    let (mut positions, mut normals, mut colors, mut flags) = (vec![], vec![], vec![], vec![]);

    let mut seen = HashMap::new();

    for (face, vert) in faces
        .iter()
        .map(|face| (face, face.verts.as_flat_tris()))
        .flat_zip()
    {
        let key = (
            vert.position.as_key(),
            vert.normal.as_key(),
            face.flags,
            face.color,
        );

        let index = *seen.entry(key).or_insert_with(|| {
            let i = positions.len() as u32;

            positions.push(vert.position);
            normals.push(vert.normal);
            flags.push(face.flags);

            if include_colors {
                let color = match face.color {
                    Some([r, g, b]) => Srgba::rgb_u8(r, g, b),
                    None => Srgba::NONE,
                };
                colors.push(color.to_f32_array());
            }

            i
        });

        indices.push(index);
    }

    IndexedVertices {
        indices,
        positions,
        normals,
        colors,
        flags,
    }
}

fn traverse_part(
    source_map: &SourceMap,
    model_name: &str,
    ctx: GeometryContext,
    output: &mut PartData,
    root: bool,
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

    let mut first_line_is_file_command = false;

    for (i, cmd) in model.cmds.iter().enumerate() {
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
                } else if root && (i == 0 || (i == 1 && first_line_is_file_command)) {
                    output.name = c.text.clone();
                }
            }
            Command::SubFileRef(sfrc) => {
                let child = ctx.child(sfrc, invert_next);
                traverse_part(source_map, &sfrc.file, child, output, false);
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
            Command::File(_) if root && i == 0 => first_line_is_file_command = true,
            _ => {}
        }
    }
}
