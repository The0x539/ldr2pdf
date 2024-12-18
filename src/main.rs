mod pdf;
mod resolver;

use std::{fs::File, io::Read};

use ldr::{ColorCode, ColorMap};
use resolver::Resolver;
use weldr::SourceMap;
use zip::ZipArchive;

pub mod instruction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = {
        let f = File::open("/home/the0x539/winhome/documents/lego/penbu/penbu.io")?;
        let mut zip = ZipArchive::new(f)?;
        let mut ins = zip.by_name("model.ins")?;
        let mut buf = String::new();
        ins.read_to_string(&mut buf)?;
        buf
    };

    let xml = tidier::format(xml, true, &Default::default())?;

    let page_design: instruction::Instruction = quick_xml::de::from_str(&xml)?;

    // let mut page_design = instruction::Instructions::default();

    // page_design.pages.inner.push(Default::default());
    // page_design.pages.inner[0].slots.push(Default::default());

    let roundtrip = quick_xml::se::to_string(&page_design)?;
    let roundtrip = tidier::format(roundtrip, true, &Default::default())?;

    pretty_assertions::assert_str_eq!(&xml, &roundtrip);

    Ok(())
}

#[allow(dead_code)]
fn render_main() -> Result<(), Box<dyn std::error::Error>> {
    // tracing_subscriber::fmt::init();

    let resolver = Resolver::new("/home/the0x539/winhome/Documents/lego/mario/Star World.io")?;
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse("Star World.io", &resolver, &mut source_map)?;

    let color_map = ColorMap::load("/mnt/c/Program Files/Studio 2.0/ldraw/LDConfig.ldr")?;

    let mut vector_data = VectorData::default();

    let ctx = ldr::GeometryContext::new();
    ldr::traverse(&source_map, &main_model_name, ctx, &mut vector_data);

    vector_data.normalize();
    for v in vector_data.points_mut() {
        v[1] = 600.0 - v[1];
    }

    pdf::build_pdf(1, 800, 600, &vector_data, &color_map).save("out.pdf")?;

    Ok(())
}

#[derive(Default)]
struct VectorData {
    lines: Vec<[[f32; 2]; 2]>,
    polygons: Vec<(Poly, ColorCode)>,
}

#[derive(Copy, Clone)]
enum Poly {
    Tri([[f32; 2]; 3]),
    Quad([[f32; 2]; 4]),
}

impl Poly {
    fn as_slice(&self) -> &[[f32; 2]] {
        match self {
            Poly::Tri(s) => s,
            Poly::Quad(s) => s,
        }
    }
    fn as_mut_slice(&mut self) -> &mut [[f32; 2]] {
        match self {
            Poly::Tri(s) => s,
            Poly::Quad(s) => s,
        }
    }
}

impl VectorData {
    fn points(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        let polygon_points = self.polygons.iter().flat_map(|(p, _c)| p.as_slice());
        let line_points = self.lines.iter().flatten();
        polygon_points.chain(line_points).copied()
    }

    fn points_mut(&mut self) -> impl Iterator<Item = &mut [f32; 2]> {
        let polygon_points = self
            .polygons
            .iter_mut()
            .flat_map(|(p, _c)| p.as_mut_slice());
        let line_points = self.lines.iter_mut().flatten();
        polygon_points.chain(line_points)
    }

    fn normalize(&mut self) {
        let [mut dx, mut dy] = [0.0_f32; 2];

        for [x, y] in self.points() {
            if x < 0.0 {
                dx = dx.max(-x);
            }
            if y < 0.0 {
                dy = dy.max(-y);
            }
        }

        for [x, y] in self.points_mut() {
            *x += dx;
            *y += dy;
        }
    }
}

mod ldr;
