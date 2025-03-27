mod pdf;

use ldr2pdf_common::ldr::{self, ColorMap, GeometryContext};
use ldr2pdf_common::resolver::Resolver;
use ldr2pdf_common::{Primitive, Result};

use weldr::SourceMap;

fn main() -> Result<()> {
    // tracing_subscriber::fmt::init();

    let resolver = Resolver::new("/home/the0x539/winhome/Documents/lego/penbu/ket.io")?;
    let mut source_map = SourceMap::new();
    let main_model_name = weldr::parse("ket.io", &resolver, &mut source_map)?;

    let color_map = ColorMap::load("/mnt/c/Program Files/Studio 2.0/ldraw/LDConfig.ldr")?;

    let mut shapes = Vec::new();

    let ctx = GeometryContext::new();
    ldr::traverse(&source_map, &main_model_name, ctx, &mut shapes);

    normalize(&mut shapes);
    for v in shapes.iter_mut().flat_map(|p| p.as_mut_slice()) {
        v[1] = 600.0 - v[1];
    }

    pdf::build_pdf(1, 800, 600, &shapes, &color_map).save("out.pdf")?;

    Ok(())
}

fn normalize(shapes: &mut [Primitive]) {
    let [mut dx, mut dy] = [0.0_f32; 2];

    for point in shapes.iter().flat_map(Primitive::as_slice) {
        let (x, y) = (point.x, point.y);
        if x < 0.0 {
            dx = dx.max(-x);
        }
        if y < 0.0 {
            dy = dy.max(-y);
        }
    }

    for point in shapes.iter_mut().flat_map(Primitive::as_mut_slice) {
        point.x += dx;
        point.y += dy;
    }
}
