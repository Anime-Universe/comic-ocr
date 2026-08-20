use serde::{Deserialize, Serialize};

pub type UUID = String;
pub type LanguageCode = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub type Polygon = Vec<Point>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumericRange {
    pub min: f64,
    pub preferred: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpatialValue {
    pub px: f64,
    pub normalized: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DualPoint {
    pub px: Point,
    pub normalized: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DualSize {
    pub px: Size,
    pub normalized: Size,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DualRect {
    pub px: Rect,
    pub normalized: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WritingMode {
    #[serde(rename = "horizontal-tb")]
    HorizontalTb,
    #[serde(rename = "vertical-rl")]
    VerticalRl,
    #[serde(rename = "vertical-lr")]
    VerticalLr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WritingFlow {
    pub mode: WritingMode,
    #[serde(rename = "characterDirection")]
    pub character_direction: String,
    #[serde(rename = "columnDirection")]
    pub column_direction: Option<String>,
    #[serde(rename = "lineDirection")]
    pub line_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanelFlow {
    pub strategy: String,
    #[serde(rename = "primaryAxis")]
    pub primary_axis: String,
    #[serde(rename = "secondaryAxis")]
    pub secondary_axis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentReadingModel {
    pub binding: String,
    #[serde(rename = "pageDirection")]
    pub page_direction: String,
    #[serde(rename = "defaultPanelFlow")]
    pub default_panel_flow: PanelFlow,
    #[serde(rename = "sourceWriting")]
    pub source_writing: WritingFlow,
    #[serde(rename = "targetWriting")]
    pub target_writing: WritingFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MangaDocument {
    pub id: UUID,
    pub metadata: DocumentMetadata,
    pub reading: DocumentReadingModel,
    pub pages: Vec<MangaPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    #[serde(rename = "sourceLanguage")]
    pub source_language: LanguageCode,
    #[serde(rename = "targetLanguage")]
    pub target_language: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MangaPage {
    pub id: UUID,
    #[serde(rename = "pageNumber")]
    pub page_number: Option<u32>,
    pub source: PageSource,
    pub bands: Option<Vec<PanelBand>>,
    pub panels: Vec<Panel>,
    pub containers: Vec<TextContainer>,
    pub text_regions: Vec<TextRegion>,
    pub art_regions: Option<Vec<ArtRegion>>,
    pub masks: Option<Vec<MaskRegion>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageSource {
    #[serde(rename = "imageId")]
    pub image_id: UUID,
    pub filename: Option<String>,
    #[serde(rename = "nativeSize")]
    pub native_size: Size,
    pub dpi: Option<f64>,
    #[serde(rename = "colorSpace")]
    pub color_space: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanelBand {
    pub id: UUID,
    pub order: u32,
    pub direction: String,
    pub bounds: Option<DualRect>,
    #[serde(rename = "panelIds")]
    pub panel_ids: Vec<UUID>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Panel {
    pub id: UUID,
    #[serde(rename = "logicalOrder")]
    pub logical_order: u32,
    pub frame: PanelFrame,
    #[serde(rename = "contentBounds")]
    pub content_bounds: Option<DualRect>,
    #[serde(rename = "safeBounds")]
    pub safe_bounds: Option<DualRect>,
    #[serde(rename = "bleedBounds")]
    pub bleed_bounds: Option<DualRect>,
    #[serde(rename = "zIndex")]
    pub z_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanelFrame {
    pub bounds: DualRect,
    pub polygon: Option<Polygon>,
    #[serde(rename = "borderWidth")]
    pub border_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContainer {
    pub id: UUID,
    #[serde(rename = "panelId")]
    pub panel_id: Option<UUID>,
    #[serde(rename = "type")]
    pub container_type: String,
    pub geometry: ContainerGeometry,
    pub padding: Option<EdgeInsets>,
    #[serde(rename = "opticalCenter")]
    pub optical_center: Option<DualPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerGeometry {
    pub shape: String,
    pub bounds: DualRect,
    pub polygon: Option<Polygon>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextRegion {
    pub id: UUID,
    #[serde(rename = "panelId")]
    pub panel_id: Option<UUID>,
    #[serde(rename = "containerId")]
    pub container_id: Option<UUID>,
    #[serde(rename = "placementMode")]
    pub placement_mode: String,
    pub role: String,
    pub source: TextContent,
    pub translation: TranslationContent,
    pub geometry: ObjectGeometry,
    pub layout: TextLayout,
    pub typography: Typography,
    #[serde(rename = "logicalOrder")]
    pub logical_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContent {
    pub language: LanguageCode,
    pub raw: String,
    pub normalized: Option<String>,
    pub reading: Option<String>,
    pub writing: WritingFlow,
    pub columns: Option<Vec<SourceTextColumn>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceTextColumn {
    pub order: u32,
    pub text: String,
    pub bounds: Option<DualRect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationContent {
    pub language: LanguageCode,
    pub literal: Option<String>,
    pub localized: String,
    #[serde(rename = "displayText")]
    pub display_text: String,
    pub tone: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectGeometry {
    pub bounds: SpatialBounds,
    pub transform: TextTransform,
    #[serde(rename = "zIndex")]
    pub z_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpatialBounds {
    pub preferred: DualRect,
    pub min: Option<DualRect>,
    pub max: Option<DualRect>,
    pub hard: Option<DualRect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextTransform {
    pub position: Point,
    pub rotation: f64,
    pub scale: Point,
    pub anchor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextLayout {
    #[serde(rename = "writingMode")]
    pub writing_mode: WritingMode,
    #[serde(rename = "textAlign")]
    pub text_align: String,
    #[serde(rename = "verticalAlign")]
    pub vertical_align: String,
    pub flow: String,
    pub lines: Option<Vec<LineLayout>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineLayout {
    pub text: String,
    #[serde(rename = "scaleX")]
    pub scale_x: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Typography {
    pub font: FontReference,
    #[serde(rename = "fontSize")]
    pub font_size: f64,
    #[serde(rename = "fontWeight")]
    pub font_weight: Option<u32>,
    #[serde(rename = "lineHeight")]
    pub line_height: Option<f64>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontReference {
    pub family: String,
    pub fallback: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtRegion {
    pub id: UUID,
    #[serde(rename = "panelId")]
    pub panel_id: Option<UUID>,
    pub role: String,
    pub protection: String,
    pub penalty: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaskRegion {
    pub id: UUID,
    #[serde(rename = "panelId")]
    pub panel_id: Option<UUID>,
    #[serde(rename = "textRegionId")]
    pub text_region_id: Option<UUID>,
    #[serde(rename = "type")]
    pub mask_type: String,
    pub expansion: Option<f64>,
    pub feather: Option<f64>,
}
