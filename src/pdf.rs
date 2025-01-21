use crate::{ldr::ColorMap, Point, Primitive};

use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use weldr::Color;

pub fn build_pdf(
    pages: u32,
    width: u32,
    height: u32,
    drawing: &[Primitive],
    colors: &ColorMap,
) -> Document {
    let mut doc = Document::new();

    let pages_id = doc.new_object_id();
    let content_id = doc.add_object(Stream::new(dictionary! {}, vec![]));
    let mut page_ids = vec![];

    for _ in 0..pages {
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id);
    }

    let kids: Vec<Object> = page_ids.iter().copied().map(From::from).collect();
    let pages = dictionary! {
        "Type" => "Pages",
        "Count" => kids.len() as u32,
        "Kids" => kids,
        "MediaBox" => [0, 0, width, height].map(From::from).to_vec(),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let mut content = doc.get_and_decode_page_content(page_ids[0]).unwrap();

    content.push_op("w", [0.1]);
    content.push_op("cs", ["DeviceRGB"]);
    content.push_op("CS", ["DeviceRGB"]);
    content.push_op("J", [1u8]);

    let mut current_color = Color::new(255, 255, 255);

    let mut drawing = drawing.to_vec();
    drawing.sort_by(|a, b| b.center().z.total_cmp(&a.center().z));

    for shape in drawing {
        match shape {
            Primitive::Line(l) => content.push_line(l),
            Primitive::Polygon(polygon, color_code) => {
                let rgb = colors.by_code(color_code).value;
                content.push_polygon(polygon.as_slice(), (rgb != current_color).then_some(rgb));
                current_color = rgb;
            }
        }
    }

    doc.change_page_content(page_ids[0], content.encode().unwrap())
        .unwrap();

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc
}

trait ContentExt {
    fn push_op<T>(&mut self, op: &str, vs: impl IntoIterator<Item = T>)
    where
        Object: From<T>;

    fn push_void_op(&mut self, op: &str) {
        self.push_op::<Object>(op, []);
    }

    fn push_polygon(&mut self, points: &[Point], color: Option<weldr::Color>) {
        self.push_op("m", [points[0].x, points[0].y]);
        for p in &points[1..] {
            self.push_op("l", [p.x, p.y]);
        }

        if let Some(rgb) = color {
            self.push_op(
                "rg",
                [rgb.red, rgb.green, rgb.blue].map(|n| n as f32 / 255.0),
            );
        }

        self.push_void_op("f");
    }

    fn push_line(&mut self, line: [Point; 2]) {
        self.push_op("m", [line[0].x, line[0].y]);
        self.push_op("l", [line[1].x, line[1].y]);
        self.push_void_op("S");
    }
}

impl ContentExt for Content {
    fn push_op<T>(&mut self, op: &str, vs: impl IntoIterator<Item = T>)
    where
        Object: From<T>,
    {
        self.operations.push(Operation::new(
            op,
            vs.into_iter().map(Object::from).collect(),
        ));
    }
}
