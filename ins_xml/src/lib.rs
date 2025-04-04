use glam::Vec2;
use helpers::Rect;
use serde::{Deserialize, Serialize};

pub mod page;
pub mod style;

mod helpers;
pub use helpers::Color;
use serde_with::skip_serializing_none;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Instruction {
    pub global_setting: GlobalSettings,
    pub pages: Pages,
    pub custom_layouts: (),
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GlobalSettings {
    pub page_setup: PageSetup,
    pub global_style: style::Style,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pages {
    #[serde(rename = "Page", default)]
    pub inner: Vec<page::Page>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PageSetup {
    pub paper_type: PaperType,
    pub length_unit: LengthUnit,
    #[serde(with = "helpers::Vec2SpaceOpt", default)]
    pub custom_size: Option<Vec2>,
    #[serde(rename = "IsPortrait", with = "helpers::UpperBool")]
    pub portrait: bool,
    pub margins: Rect,
    #[serde(rename = "UseCMYKColorTable", with = "helpers::UpperBool")]
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
