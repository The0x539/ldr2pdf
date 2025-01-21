pub mod instruction;
mod ldr;
mod pdf;
mod resolver;

use std::{collections::HashSet, fs::File, io::Read};

use ldr::{ColorCode, ColorMap};
use resolver::Resolver;
use weldr::SourceMap;
use zip::ZipArchive;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let args = std::env::args().collect::<HashSet<_>>();
    if args.contains("xml") {
        std::env::set_current_dir("/home/the0x539/winhome/documents/lego/")?;

        let paths = walkdir::WalkDir::new(".")
            .into_iter()
            .map(Result::unwrap)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension() == Some("io".as_ref()))
            .map(|e| e.into_path())
            .collect::<Vec<_>>();

        for path in paths {
            xml_main(path)?;
        }
    }
    if args.contains("pdf") {
        render_main()?;
    }
    if args.contains("fmt") {
        fmt_main()?;
    }
    Ok(())
}

fn read_model_ins(path: impl AsRef<std::path::Path>) -> Result<String, zip::result::ZipError> {
    let f = File::open(path)?;
    let mut zip = ZipArchive::new(f)?;
    let mut ins = zip.by_name_decrypt("model.ins", b"soho0909")?;
    let mut buf = String::new();
    ins.read_to_string(&mut buf)?;
    Ok(buf)
}

fn xml_main(path: impl AsRef<std::path::Path>) -> Result<()> {
    let path = path.as_ref();
    let xml = match read_model_ins(path) {
        Err(zip::result::ZipError::FileNotFound) => {
            return Ok(());
        }
        x => x.inspect_err(|e| println!("failed to open zip at {}: {e}", path.display()))?,
    };
    let path = path.display();

    let xml = tidier::format(xml, true, &Default::default())?;

    let page_design: instruction::Instruction =
        quick_xml::de::from_str(&xml).inspect_err(|_| println!("failed to deserialize {path}"))?;
    let roundtrip = quick_xml::se::to_string(&page_design)?;
    let roundtrip = tidier::format(roundtrip, true, &Default::default())?;

    if xml != roundtrip {
        let line_index = xml
            .lines()
            .zip(roundtrip.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(0)
            .saturating_sub(10);

        println!("round trip failed for {path} at line {line_index}");
        let byte_index = xml.lines().take(line_index).map(|l| l.len() + 1).sum();

        pretty_assertions::assert_str_eq!(&xml[byte_index..], &roundtrip[byte_index..]);
    }

    println!("round trip successful for {path}");

    Ok(())
}

fn render_main() -> Result<()> {
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

fn fmt_main() -> Result<()> {
    let Some(path) = std::env::args().find(|a| a.ends_with(".io")) else {
        eprintln!("no path specified");
        return Ok(());
    };

    let xml = read_model_ins(&path)?;
    let pretty = tidier::format(&xml, true, &Default::default())?;
    println!("{pretty}");

    Ok(())
}
