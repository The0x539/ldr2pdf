use super::helpers::*;
use super::style::{
    ArrowStyle, BoxStyle, CalloutDividerStyle, CalloutMultiplierStyle, Font, Padding, Spacing,
    StepItemLayout,
};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Page {
    #[serde(rename = "@template")]
    pub template: PageTemplate,
    #[serde(rename = "@resizeBars", with = "resize_bar_list")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resize_bars: Vec<ResizeBar>,
    #[serde(rename = "@IsLocked", with = "UpperBool")]
    pub locked: bool,
    #[serde(rename = "Slot")]
    pub slots: Vec<Slot>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum PageTemplate {
    Custom,
    Empty,
    #[default]
    OneByOne,
    OneByTwo,
    TwoByOne,
    OneByThree,
    ThreeByOne,
    TwoByTwo,
    #[serde(rename = "TwoByTwo_Col")]
    TwoByTwoCol,
    TwoByThree,
    ThreeByTwo,
}

#[derive(Debug, Default)]
pub struct ResizeBar {
    pub vertical: bool,
    pub ref_index_1: isize,
    pub ref_index_2: isize,
    pub offset: f32,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Slot {
    #[serde(rename = "@refResizeBarLeft")]
    pub ref_resize_bar_left: Option<u8>,
    #[serde(rename = "@refResizeBarRight")]
    pub ref_resize_bar_right: Option<u8>,
    #[serde(rename = "@refResizeBarTop")]
    pub ref_resize_bar_top: Option<u8>,
    #[serde(rename = "@refResizeBarBottom")]
    pub ref_resize_bar_bottom: Option<u8>,

    #[serde(rename = "$value")]
    pub content: Vec<SlotContent>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum SlotContent {
    #[serde(rename = "StepCompLayout")]
    Layout(StepItemLayout),
    Step(Step),
    #[serde(rename = "BOM")]
    Bom(Bom),
    Image(Image),
    #[default]
    #[serde(other)]
    Other,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Step {
    #[serde(rename = "@SerializedIndex")]
    pub serialized_index: u32,
    #[serde(rename = "@RectOffset", with = "Arr4Space")]
    pub rect_offset: [f32; 4],
    #[serde(rename = "@IsVisibleStepNumber")]
    #[serde(with = "UpperBoolOpt", default)]
    pub visible_step_number: Option<bool>,
    #[serde(rename = "@IsVisiblePartsList")]
    #[serde(with = "UpperBoolOpt", default)]
    pub visible_parts_list: Option<bool>,
    #[serde(rename = "@IsVisibleSubModelPreview")]
    #[serde(with = "UpperBoolOpt", default)]
    pub visible_submodel_preview: Option<bool>,

    pub step_preview: StepPreview,
    pub step_number: StepNumber,
    pub part_list: PartList,
    pub submodel_preview: SubmodelPreview,
    pub call_out: Option<Callout>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StepPreview {
    #[serde(rename = "@Depth")]
    pub depth: i32,
    #[serde(rename = "@targetPosOffset", with = "Vec3Space")]
    pub target_pos_offset: Vec3,
    #[serde(rename = "@IsForcedTargetPosOffset", with = "UpperBool")]
    pub forced_target_pos_offset: bool,
    #[serde(flatten)]
    pub camera_control: Option<CameraControl>,
    #[serde(flatten)]
    pub default_camera_control: Option<DefaultCameraControl>,
}

#[serde_as]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CameraControl {
    #[serde(rename = "@cameraScale")]
    #[serde_as(as = "DisplayFromStr")]
    pub scale: f32,
    #[serde(rename = "@TargetPos", with = "Vec3Space")]
    pub pos: Vec3,
    #[serde(rename = "@cameraAngle", with = "Vec2Space")]
    pub camera_angle: Vec2,
    #[serde(default, rename = "@modelAngle", with = "Vec3SpaceOpt")]
    pub model_angle: Option<Vec3>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefaultCameraControl {
    #[serde(default, rename = "@DefaultCameraControlInfo_cameraScale")]
    #[serde_as(as = "DisplayFromStr")]
    pub scale: f32,
    #[serde(rename = "@DefaultCameraControlInfo_TargetPos", with = "Vec3Space")]
    pub pos: Vec3,
    #[serde(rename = "@DefaultCameraControlInfo_cameraAngle", with = "Vec2Space")]
    pub camera_angle: Vec2,
    #[serde(
        default,
        rename = "@DefaultCameraControlInfo_modelAngle",
        with = "Vec3SpaceOpt"
    )]
    pub model_angle: Option<Vec3>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StepNumber {
    #[serde(rename = "@Depth")]
    pub depth: i32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartList {
    #[serde(rename = "@Rect", with = "Arr4Space")]
    pub rect: [f32; 4],
    #[serde(rename = "@Depth")]
    pub depth: i32,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubmodelPreview {
    #[serde(rename = "@Depth")]
    pub depth: i32,
    #[serde(rename = "@Position", with = "Vec2Space")]
    pub position: Vec2,
    #[serde(rename = "Orientation")]
    pub orientation: Option<CameraControl>,
    #[serde(rename = "Multiplier")]
    pub multiplier: Multiplier,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Multiplier {
    #[serde(rename = "@Position", with = "Vec2Space")]
    pub position: Vec2,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Image {
    #[serde(rename = "@depth")]
    pub depth: i32,
    #[serde(rename = "@rect", with = "Arr4Space")]
    pub rect: [f32; 4],
    #[serde(rename = "@rotation")]
    pub rotation: i32,
    #[serde(rename = "@fliped_H", with = "UpperBool")]
    pub flipped_h: bool,
    #[serde(rename = "@fliped_V", with = "UpperBool")]
    pub flipped_v: bool,
    #[serde(rename = "@opacity")]
    pub opacity: f32,
    #[serde(rename = "@imagePath")]
    pub image_path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Callout {
    #[serde(rename = "CallOutItemData")]
    pub item_data: Vec<CalloutItemData>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CalloutItemData {
    #[serde(rename = "@Depth")]
    pub depth: i32,
    #[serde(rename = "@Rect", with = "Arr4Space")]
    pub rect: [f32; 4],
    #[serde(rename = "@CalloutGridMaxPerLine")]
    pub max_per_line: u32,
    #[serde(rename = "@IsCalloutAsRows", with = "UpperBool")]
    pub as_rows: bool,
    #[serde(rename = "@MultiplierPosition", with = "Vec2Space")]
    pub multiplier_position: Vec2,
    #[serde(rename = "@MultiplierValue")]
    pub multiplier_value: u32,
    #[serde(rename = "@IsUseGlobalStyle", with = "UpperBool")]
    pub use_global_style: bool,

    pub colors: Option<BoxStyle>,
    pub divider: Option<CalloutDividerStyle>,
    pub step_number: Option<Font>,
    pub multiplier: Option<CalloutMultiplierStyle>,
    pub arrow: Option<ArrowStyle>,
    pub padding: Option<Padding>,
    pub margin: Option<Spacing>,

    #[serde(rename = "CallOutStepItemData")]
    pub steps: Vec<CalloutStepItemData>,
    #[serde(rename = "CallOutArrowItemData")]
    pub arrows: Vec<CalloutArrowItemData>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CalloutStepItemData {
    pub step: Step,
    pub orientation: Option<CameraControl>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CalloutArrowItemData {
    #[serde(rename = "@ArrowPosition", with = "Arr4Space")]
    pub arrow_position: [f32; 4],
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Bom {}
