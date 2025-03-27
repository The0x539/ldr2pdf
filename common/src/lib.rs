use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

pub mod ldr;
pub mod resolver;

use ldr::ColorCode;

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub fn read_model_ins(path: impl AsRef<std::path::Path>) -> Result<String, zip::result::ZipError> {
    let f = File::open(path)?;
    let mut zip = ZipArchive::new(f)?;
    let mut ins = zip.by_name_decrypt("model.ins", b"soho0909")?;
    let mut buf = String::new();
    ins.read_to_string(&mut buf)?;
    Ok(buf)
}

pub type Point = glam::Vec3;

#[derive(Copy, Clone)]
pub enum Poly {
    Tri([Point; 3]),
    Quad([Point; 4]),
}

impl Poly {
    pub fn as_slice(&self) -> &[Point] {
        match self {
            Poly::Tri(s) => s,
            Poly::Quad(s) => s,
        }
    }
    pub fn as_mut_slice(&mut self) -> &mut [Point] {
        match self {
            Poly::Tri(s) => s,
            Poly::Quad(s) => s,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Primitive {
    Line([Point; 2]),
    Polygon(Poly, ColorCode),
}

impl Primitive {
    pub fn as_slice(&self) -> &[Point] {
        match self {
            Self::Line(l) => l,
            Self::Polygon(p, _) => p.as_slice(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [Point] {
        match self {
            Self::Line(l) => l,
            Self::Polygon(p, _) => p.as_mut_slice(),
        }
    }

    pub fn center(&self) -> Point {
        self.as_slice().iter().copied().sum::<Point>() / self.as_slice().len() as f32
    }
}
