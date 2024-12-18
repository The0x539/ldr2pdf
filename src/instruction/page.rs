use super::{helpers::*, style::StepItemLayout};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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
    #[serde(rename = "@refResizeBarBottom")]
    pub ref_resize_bar_bottom: Option<u8>,
    #[serde(rename = "@refResizeBarTop")]
    pub ref_resize_bar_top: Option<u8>,

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
    #[default]
    #[serde(other)]
    Other,
}

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
}

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
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CameraControl {
    #[serde(rename = "@DefaultCameraControlInfo_cameraScale")]
    pub scale: f32,
    #[serde(rename = "@DefaultCameraControlInfo_TargetPos", with = "Vec3Space")]
    pub pos: Vec3,
    #[serde(rename = "@DefaultCameraControlInfo_cameraAngle", with = "Vec2Space")]
    pub camera_angle: Vec2,
    #[serde(rename = "@DefaultCameraControlInfo_modelAngle", with = "Vec3Space")]
    pub model_angle: Vec3,
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubmodelPreview {
    #[serde(rename = "@Depth")]
    pub depth: i32,
    #[serde(rename = "@Position", with = "Vec2Space")]
    pub position: Vec2,
    #[serde(rename = "Multiplier")]
    pub multiplier: Multiplier,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Multiplier {
    #[serde(rename = "@Position", with = "Vec2Space")]
    pub position: Vec2,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Bom {}
