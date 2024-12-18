use crate::{ldr::ColorMap, VectorData};

use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use weldr::Color;

pub fn build_pdf(
    pages: u32,
    width: u32,
    height: u32,
    drawing: &VectorData,
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

    fn push_op<T, const N: usize>(content: &mut Content, op: &str, vs: [T; N])
    where
        Object: From<T>,
    {
        content
            .operations
            .push(Operation::new(op, vs.map(Object::from).to_vec()));
    }

    push_op(&mut content, "w", [0.1]);
    push_op(&mut content, "cs", [Object::Name("DeviceRGB".into())]);
    push_op(&mut content, "CS", [Object::Name("DeviceRGB".into())]);
    push_op(&mut content, "J", [1u8]);

    let mut current_color = Color::new(255, 255, 255);

    for (polygon, color_code) in &drawing.polygons {
        let rgb = colors.by_code(*color_code).value;
        let points = polygon.as_slice();

        push_op(&mut content, "m", points[0]);
        for p in &points[1..] {
            push_op(&mut content, "l", *p);
        }

        if rgb != current_color {
            push_op(
                &mut content,
                "rg",
                [rgb.red, rgb.green, rgb.blue].map(|n| n as f32 / 255.0),
            );
            current_color = rgb;
        }

        push_op(&mut content, "f", [0u8; 0]);
    }

    for line in &drawing.lines {
        push_op(&mut content, "m", line[0]);
        push_op(&mut content, "l", line[1]);
        push_op(&mut content, "S", [0u8; 0]);
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
