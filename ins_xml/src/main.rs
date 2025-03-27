use ldr2pdf_common::{read_model_ins, Result};

use ldr2pdf_ins_xml as instruction;

fn main() -> Result<()> {
    std::env::set_current_dir("/home/the0x539/winhome/documents/lego/")?;

    let paths = walkdir::WalkDir::new(".")
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension() == Some("io".as_ref()))
        .map(|e| e.into_path())
        .collect::<Vec<_>>();

    for path in paths {
        do_roundtrip(path)?;
    }

    Ok(())
}

fn do_roundtrip(path: impl AsRef<std::path::Path>) -> Result<()> {
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
