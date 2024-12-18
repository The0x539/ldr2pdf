use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};

pub struct Instructions {
    pub page_setup: PageSetup,
}

pub struct PageSetup {
    pub paper_type: PaperType,
    pub length_unit: LengthUnit,
    pub custom_size: Vec2,
    pub portrait: bool,
    pub margins: Vec4,
    pub use_cmyk_color_table: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum LengthUnit {
    Inches,
    Millimeters,
    #[default]
    Pixels,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum PaperType {
    Letter,
    Legal,
    Tabloid,
    A2,
    A3,
    #[default]
    A4,
    Custom,
}

pub mod style;

mod helpers;
pub use helpers::Color;
