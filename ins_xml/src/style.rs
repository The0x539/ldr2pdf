use glam::Vec2;
use glam::Vec3;
use serde::Deserialize;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use super::helpers::*;
use super::LengthUnit;

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Style {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@PaddingUnit")]
    pub padding_unit: LengthUnit,
    pub page: PageStyle,
    pub step_item_layout: Option<StepItemLayoutOuter>,
    pub step_number: StepNumberStyle,
    pub parts_list: PartsListStyle,
    pub new_part_highlight: NewPartHighlightStyle,
    pub sub_model_preview: SubModelPreviewStyle,
    pub size_guide: SizeGuideStyle,
    pub color_guide: ColorGuideStyle,
    pub call_out: CalloutStyle,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PageStyle {
    #[serde(rename = "@SlotSpacing", with = "Vec2Space")]
    pub slot_spacing: Vec2,
    #[serde(rename = "Colors" /* I have no idea. */)]
    pub inner: PageStyleInner,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PageStyleInner {
    #[serde(rename = "@IsUseBgColor", with = "UpperBool")]
    pub use_bg_color: bool,
    #[serde(rename = "@BgColor")]
    pub bg_color: Option<Color>,
    #[serde(rename = "@BgImage")]
    pub bg_image: Option<String>,
    #[serde(rename = "@BgImageDisplayT")]
    pub bg_image_display_type: Option<ImageDisplay>,
    #[serde(
        rename = "@BgImageScale",
        default,
        deserialize_with = "option_from_str"
    )]
    pub bg_image_scale: Option<f32>,

    #[serde(rename = "@IsUseBorder", with = "UpperBool")]
    pub use_border: bool,
    #[serde(rename = "@BorderColor")]
    pub border_color: Option<Color>,
    #[serde(
        rename = "@BorderThickness",
        default,
        deserialize_with = "option_from_str"
    )]
    pub border_thickness: Option<u32>,

    // "corner" radius - used even when border is absent?
    #[serde(rename = "@BorderRadius")]
    #[serde_as(as = "DisplayFromStr")]
    pub border_radius: u32,

    #[serde(rename = "@IsUseLineSeparatorColumns", with = "UpperBool")]
    pub use_line_separator_columns: bool,
    #[serde(rename = "@IsUseLineSeparatorRows", with = "UpperBool")]
    pub use_line_separator_rows: bool,
    #[serde(rename = "@LineSeperatorColor")]
    pub line_separator_color: Option<Color>,
    #[serde(
        rename = "@LineSeperatorThickness",
        default,
        deserialize_with = "option_from_str"
    )]
    pub line_separator_thickness: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum ImageDisplay {
    Fill,
    Fit,
    Stretch,
    #[default]
    Center,
    Tile,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StepItemLayoutOuter {
    #[serde(rename = "StepCompLayout")]
    pub inner: StepItemLayout,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StepItemLayout {
    #[serde(rename = "@refCount")]
    pub ref_count: u32,
    pub left_top: SnappableComponentList,
    pub left_bottom: SnappableComponentList,
    pub right_top: SnappableComponentList,
    pub right_bottom: SnappableComponentList,
    pub none: SnappableComponentList,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SnappableComponentList {
    #[serde(default, rename = "StepCompLayoutElem")]
    pub inner: Vec<SnappableComponent>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SnappableComponent {
    #[serde(rename = "@ComponentT")]
    pub component_type: SnappableComponentType,
    #[serde(rename = "@IsVertical", with = "UpperBool")]
    pub vertical: bool,
    #[serde(rename = "@Padding", with = "Vec2Space")]
    pub padding: Vec2,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum SnappableComponentType {
    #[default]
    StepNumber,
    PartsList,
    SubModelPreview,
    StepPreview,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StepNumberStyle {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    #[serde(rename = "StepNumber")]
    pub font: StepNumberFont,
    #[serde(rename = "Padding")]
    pub padding: Padding,
}

// fields have a different order
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StepNumberFont {
    #[serde(rename = "@FontFamily")]
    pub family: String,
    #[serde(rename = "@FontStyle")]
    pub style: FontStyle,
    #[serde(rename = "@FontColor")]
    pub color: Color,
    #[serde(rename = "@FontSize")]
    pub size: u32,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Font {
    #[serde(rename = "@FontSize")]
    #[serde_as(as = "DisplayFromStr")]
    pub size: u32,
    #[serde(rename = "@FontColor")]
    pub color: Color,
    #[serde(rename = "@FontFamily")]
    pub family: String,
    #[serde(rename = "@FontStyle")]
    pub style: FontStyle,
    #[serde(rename = "@FontOffset", with = "Vec2SpaceOpt", default)]
    pub offset: Option<Vec2>,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TextboxFont {
    #[serde(rename = "@FontFamily")]
    pub family: String,
    #[serde(rename = "@FontStyle")]
    pub style: FontStyle,
    #[serde(rename = "@FontSize")]
    #[serde_as(as = "DisplayFromStr")]
    pub size: u32,
    #[serde(rename = "@FontColor", with = "Vec3Space")]
    pub color: Vec3,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum FontStyle {
    #[default]
    Normal,
    Bold,
    Italic,
    BoldAndItalic,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Padding {
    // TODO: figure out if the order is correct and consistent
    #[serde(rename = "@LRBT")]
    pub lrbt: Rect,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PartsListStyle {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    pub colors: BoxStyle,
    pub part_size: Scale,
    pub part_count: Font,
    pub padding: Padding,
    pub spacing: Spacing,
}

#[skip_serializing_none]
#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BoxStyle {
    #[serde(rename = "@IsUseBgColor", with = "UpperBool")]
    pub use_bg_color: bool,
    #[serde(rename = "@BgColor", default)]
    pub bg_color: Option<Color>,
    #[serde(rename = "@IsUseBorder", with = "UpperBool")]
    pub use_border: bool,
    #[serde(rename = "@BorderColor", default)]
    pub border_color: Option<Color>,
    #[serde(
        rename = "@BorderThickness",
        default,
        deserialize_with = "option_from_str"
    )]
    pub border_thickness: Option<u32>,
    #[serde(
        rename = "@BorderRadius",
        default,
        deserialize_with = "option_from_str"
    )]
    pub border_radius: Option<u32>,
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Scale {
    #[serde(rename = "@Scale")]
    #[serde_as(as = "DisplayFromStr")]
    pub scale: f32,
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Spacing {
    #[serde(rename = "@Horizontal")]
    #[serde_as(as = "DisplayFromStr")]
    pub horizontal: f32,
    #[serde(rename = "@Vertical")]
    #[serde_as(as = "DisplayFromStr")]
    pub vertical: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NewPartHighlightStyle {
    pub highlight: Highlight,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Highlight {
    #[serde(rename = "@IsUseHighlight", with = "UpperBool")]
    pub use_highlight: bool,
    #[serde(rename = "@Thickness")]
    pub thickness: u32,
    #[serde(rename = "@Color")]
    pub color: Color,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SubModelPreviewStyle {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    pub colors: Option<BoxStyle>,
    pub multiplier: Option<MultiplierStyle>,
    pub padding: Option<Padding>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MultiplierStyle {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    #[serde(flatten)]
    pub font: Font,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CalloutMultiplierStyle {
    #[serde(rename = "@IsShowMultiplier", with = "UpperBool")]
    pub visible: bool,
    #[serde(flatten)]
    pub font: Font,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SizeGuideStyle {
    pub colors: BoxStyle,
    pub font: Font,
    pub padding: Padding,
    pub assembly_margin: Spacing,
    pub length_indicator: Option<LengthIndicatorStyle>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LengthIndicatorStyle {
    #[serde(rename = "@IsUseIndicator", with = "UpperBool")]
    pub use_indicator: bool,
    #[serde(rename = "@IsUseLengthBgColor", with = "UpperBool")]
    pub use_length_bg_color: bool,
    #[serde(rename = "@BgColor")]
    pub bg_color: Color,
    #[serde(rename = "@IsUseBorder", with = "UpperBool")]
    pub use_border: bool,
    #[serde(rename = "@BorderColor")]
    pub border_color: Color,
    #[serde(rename = "@BorderThickness")]
    pub border_thickness: u32,
    #[serde(flatten)]
    pub font: Font,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ColorGuideStyle {
    pub colors: BoxStyle,
    pub font: ColorGuideFont,
    pub padding: Padding,
    pub assembly_margin: Spacing,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ColorGuideFont {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    #[serde(flatten)]
    pub font: Font,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CalloutStyle {
    pub colors: BoxStyle,
    pub divider: DividerStyle,
    pub step_number: Font,
    pub multiplier: MultiplierStyle,
    pub arrow_style: ArrowStyle,
    pub padding: Padding,
    pub margin: Spacing,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DividerStyle {
    #[serde(rename = "@IsVisible", with = "UpperBool")]
    pub visible: bool,
    #[serde(rename = "@DividerColor")]
    pub color: Color,
    #[serde(rename = "@DividerThickness")]
    pub thickness: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CalloutDividerStyle {
    #[serde(rename = "@IsShowDivider", with = "UpperBool")]
    pub visible: bool,
    #[serde(rename = "@DividerColor")]
    pub color: Color,
    #[serde(rename = "@DividerThickness")]
    pub thickness: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArrowStyle {
    #[serde(rename = "@Style")]
    pub style: Arrowhead,
    #[serde(rename = "@Thickness")]
    pub thickness: u32,
    #[serde(rename = "@Color")]
    pub color: Color,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(i8)]
pub enum Arrowhead {
    None = -1,
    NoHeads,
    TriangleEmpty,
    #[default]
    TriangleFilled,
    RectangleEmpty,
    RectangleFilled,
    CircleEmpty,
    CircleFilled,
    Line,
}
