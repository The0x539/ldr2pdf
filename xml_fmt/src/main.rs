use ldr2pdf_common::{read_model_ins, Result};

fn main() -> Result<()> {
    let Some(path) = std::env::args().find(|a| a.ends_with(".io")) else {
        eprintln!("no path specified");
        return Ok(());
    };

    let xml = read_model_ins(&path)?;
    let pretty = tidier::format(&xml, true, &Default::default())?;
    println!("{pretty}");

    Ok(())
}
