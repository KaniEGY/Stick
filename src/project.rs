use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

pub const PROJECT_SCHEMA_VERSION: u32 = 7;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum KritaBrushPreset {
    GPenInk,
    Watercolor,
    ChiselMarker,
    SoftAirbrush,
    SketchPencil,
    DryBrush,
    Chalk,
    Charcoal,
    SoftPastel,
    Fineliner,
    SmoothInk,
    TaperedPen,
    Calligraphy,
    SoftSketchPencil,
    /// Real graphite-style pencil. Appended to preserve existing enum order.
    GraphitePencil,
    /// Broad natural sketching brush. Appended to preserve old serialized presets.
    SketchBrush,
}

impl Default for KritaBrushPreset {
    fn default() -> Self {
        KritaBrushPreset::GPenInk
    }
}

impl KritaBrushPreset {
    pub const MENU: [(Self, &'static str, &'static str); 16] = [
        (Self::GPenInk, "2D Pen", "قلم عادي"),
        (Self::Watercolor, "Watercolor", "مائية"),
        (Self::ChiselMarker, "Marker", "ماركر"),
        (Self::SoftAirbrush, "Airbrush", "بخاخ"),
        (Self::SketchPencil, "Sketch Pencil", "سكتش"),
        (Self::DryBrush, "DryBrush", "جافة"),
        (Self::Chalk, "Chalk", "طباشير"),
        (Self::Charcoal, "Charcoal", "فحم"),
        (Self::SoftPastel, "Soft Pastel", "باستيل"),
        (Self::Fineliner, "Fineliner", "قلم تحديد"),
        (Self::SmoothInk, "Smooth Ink", "حبر ناعم"),
        (Self::TaperedPen, "Tapered Pen", "قلم مدبب"),
        (Self::Calligraphy, "Calligraphy", "خطاط"),
        (Self::SoftSketchPencil, "Soft Sketch", "سكتش ناعم"),
        (Self::GraphitePencil, "Graphite Pencil", "قلم رصاص"),
        (Self::SketchBrush, "Sketch Brush", "فرشة الاسكتش"),
    ];

    pub fn label(&self) -> &'static str {
        match self {
            KritaBrushPreset::GPenInk => "2D Pen (حبر دائري ناعم وثابت للرسم ثنائي الأبعاد)",
            KritaBrushPreset::Watercolor => "Watercolor (ألوان مائية متصلة)",
            KritaBrushPreset::ChiselMarker => "Marker (ماركر شفاف متراكم)",
            KritaBrushPreset::SoftAirbrush => "Airbrush (بخاخ ناعم)",
            KritaBrushPreset::SketchPencil => "Sketch Pencil (قلم سكتش خشن وخفيف)",
            Self::DryBrush => "DryBrush (فرشاة جافة)",
            Self::Chalk => "Chalk (طباشير)",
            Self::Charcoal => "Charcoal (قلم فحم خشن)",
            Self::SoftPastel => "Soft Pastel (باستيل ناعم)",
            Self::Fineliner => "Fineliner (خط دقيق ثابت السُمك)",
            Self::SmoothInk => "Smooth Ink (حبر متصل بنهايات مستديرة)",
            Self::TaperedPen => "Tapered Pen (ضغط يتدرج عند بداية ونهاية الخط)",
            Self::Calligraphy => "Calligraphy (سن مائل بعرض واتجاه ثابتين)",
            Self::SoftSketchPencil => "Soft Sketch Pencil (رصاص محبّب وناعم)",
            Self::GraphitePencil => "Graphite Pencil (قلم رصاص بجرافيت حقيقي وشفافية حسب الضغط)",
            Self::SketchBrush => "Sketch Brush (فرشة اسكتش طبيعية عريضة بخامة ورق خفيفة)",
        }
    }

    pub fn default_opacity(&self) -> f32 {
        match self {
            KritaBrushPreset::GPenInk => 1.0,
            KritaBrushPreset::Watercolor => 0.55,
            KritaBrushPreset::ChiselMarker => 0.42,
            KritaBrushPreset::SoftAirbrush => 0.40,
            KritaBrushPreset::SketchPencil => 0.75,
            Self::DryBrush => 0.84,
            Self::Chalk => 0.64,
            Self::Charcoal => 0.82,
            Self::SoftPastel => 0.68,
            Self::Fineliner => 1.0,
            Self::SmoothInk => 1.0,
            Self::TaperedPen => 1.0,
            Self::Calligraphy => 0.90,
            Self::SoftSketchPencil => 0.55,
            Self::GraphitePencil => 0.62,
            Self::SketchBrush => 0.72,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum AudioSyncMode {
    Normal,
    Stream,
}

impl Default for AudioSyncMode {
    fn default() -> Self {
        AudioSyncMode::Stream
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum TweenMode {
    None,
    Classic,
    Shape,
}

impl Default for TweenMode {
    fn default() -> Self {
        TweenMode::None
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum EasingCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Default for EasingCurve {
    fn default() -> Self {
        EasingCurve::EaseInOut
    }
}

impl EasingCurve {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingCurve::Linear => t,
            EasingCurve::EaseIn => t * t,
            EasingCurve::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingCurve::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) * 0.5
                }
            }
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EasingCurve::Linear => "Linear (ثابت)",
            EasingCurve::EaseIn => "Ease In (تسارع تدريجي)",
            EasingCurve::EaseOut => "Ease Out (تباطؤ ناعم)",
            EasingCurve::EaseInOut => "Ease In-Out (سينمائي)",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_pressure")]
    pub pressure: f32,
}

fn default_pressure() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BrushTipShape {
    #[default] Round,
    Square,
    Triangle,
    Pentagon,
    TallRectangle,
    WideRectangle,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stroke {
    #[serde(default)]
    pub points: Vec<StrokePoint>,
    #[serde(default = "default_black")]
    pub color: [u8; 4],
    #[serde(default)]
    pub fill_color: Option<[u8; 4]>,
    #[serde(default = "default_width")]
    pub base_width: f32,
    #[serde(default)]
    pub is_vector: bool,
    #[serde(default)]
    pub brush_preset: KritaBrushPreset,
    #[serde(default = "default_pressure")]
    pub opacity: f32,
    #[serde(default)]
    pub brush_texture_id: Option<String>,
    #[serde(default)]
    pub tip_shape: BrushTipShape,
}

fn default_black() -> [u8; 4] {
    [0, 0, 0, 255]
}

fn default_width() -> f32 {
    6.0
}

impl Stroke {
    pub fn fit_cubic_bezier_spline(&mut self, subdivisions: usize) {
        if self.points.len() < 3 {
            return;
        }
        let mut dense = Vec::with_capacity(self.points.len() * subdivisions);
        let n = self.points.len();
        dense.push(self.points[0].clone());

        for i in 0..(n - 1) {
            let p0 = &self.points[i.saturating_sub(1)];
            let p1 = &self.points[i];
            let p2 = &self.points[(i + 1).min(n - 1)];
            let p3 = &self.points[(i + 2).min(n - 1)];

            for step in 1..=subdivisions {
                let t = step as f32 / subdivisions as f32;
                let t2 = t * t;
                let t3 = t2 * t;

                let x = 0.5
                    * ((2.0 * p1.x)
                        + (-p0.x + p2.x) * t
                        + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                        + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);
                let y = 0.5
                    * ((2.0 * p1.y)
                        + (-p0.y + p2.y) * t
                        + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                        + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);
                let pressure = p1.pressure + (p2.pressure - p1.pressure) * t;

                dense.push(StrokePoint { x, y, pressure });
            }
        }
        self.points = dense;
    }

    pub fn simplify_vector_nodes(&mut self, tolerance: f32) {
        if self.points.len() <= 3 {
            return;
        }
        self.points = rdp_simplify(&self.points, tolerance);
    }

    pub fn resample_points(&self, target_count: usize) -> Vec<StrokePoint> {
        if self.points.is_empty() || target_count == 0 {
            return Vec::new();
        }
        if self.points.len() == 1 || target_count == 1 {
            return vec![self.points[0].clone(); target_count];
        }
        let mut out = Vec::with_capacity(target_count);
        let max_idx = (self.points.len() - 1) as f32;
        for i in 0..target_count {
            let pos = (i as f32 / (target_count - 1).max(1) as f32) * max_idx;
            let idx0 = pos.floor() as usize;
            let idx1 = (idx0 + 1).min(self.points.len() - 1);
            let frac = pos - idx0 as f32;
            let p0 = &self.points[idx0];
            let p1 = &self.points[idx1];
            out.push(StrokePoint {
                x: p0.x + (p1.x - p0.x) * frac,
                y: p0.y + (p1.y - p0.y) * frac,
                pressure: p0.pressure + (p1.pressure - p0.pressure) * frac,
            });
        }
        out
    }
}

fn rdp_simplify(pts: &[StrokePoint], epsilon: f32) -> Vec<StrokePoint> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut dmax = 0.0f32;
    let mut index = 0;
    let end = pts.len() - 1;
    for i in 1..end {
        let d = perpendicular_distance(&pts[i], &pts[0], &pts[end]);
        if d > dmax {
            index = i;
            dmax = d;
        }
    }
    if dmax > epsilon {
        let mut rec1 = rdp_simplify(&pts[..=index], epsilon);
        let rec2 = rdp_simplify(&pts[index..], epsilon);
        rec1.pop();
        rec1.extend(rec2);
        rec1
    } else {
        vec![pts[0].clone(), pts[end].clone()]
    }
}

fn perpendicular_distance(p: &StrokePoint, a: &StrokePoint, b: &StrokePoint) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = a.x + t * dx;
    let proj_y = a.y + t * dy;
    ((p.x - proj_x).powi(2) + (p.y - proj_y).powi(2)).sqrt()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextItem {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_black")]
    pub color: [u8; 4],
    #[serde(default)]
    pub rotation_deg: f32,
}

fn default_font_size() -> f32 {
    32.0
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum MediaType {
    Image,
    VideoFrame,
}

impl Default for MediaType {
    fn default() -> Self {
        MediaType::Image
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaItem {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub media_type: MediaType,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_media_dim")]
    pub width: f32,
    #[serde(default = "default_media_dim")]
    pub height: f32,
    #[serde(default)]
    pub rotation_deg: f32,
    #[serde(default)]
    pub flip_h: bool,
    #[serde(default)]
    pub flip_v: bool,
}

fn default_media_dim() -> f32 {
    300.0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Symbol {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub strokes: Vec<Stroke>,
    #[serde(default)]
    pub nested_keyframes: HashMap<usize, Vec<Stroke>>,
    #[serde(default = "default_one")]
    pub nested_total_frames: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SymbolInstance {
    #[serde(default)]
    pub symbol_name: String,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default = "default_pressure")]
    pub scale: f32,
    #[serde(default)]
    pub rotation_deg: f32,
    #[serde(default)]
    pub anchor_offset_x: f32,
    #[serde(default)]
    pub anchor_offset_y: f32,
}

impl SymbolInstance {
    /// Map a symbol-local point into stage coordinates using the same transform
    /// order as raster/video export: pivot -> scale -> rotate -> translate.
    /// Invalid numeric values are sanitized so a corrupt instance cannot poison
    /// the editor/timeline painter with NaN coordinates.
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let finite_or = |value: f32, fallback: f32| {
            if value.is_finite() { value } else { fallback }
        };
        let x = finite_or(x, 0.0);
        let y = finite_or(y, 0.0);
        let anchor_x = finite_or(self.anchor_offset_x, 0.0);
        let anchor_y = finite_or(self.anchor_offset_y, 0.0);
        let scale = finite_or(self.scale, 1.0);
        let rotation = finite_or(self.rotation_deg, 0.0).to_radians();
        let translate_x = finite_or(self.x, 0.0);
        let translate_y = finite_or(self.y, 0.0);

        let local_x = (x - anchor_x) * scale;
        let local_y = (y - anchor_y) * scale;
        let (sin_r, cos_r) = rotation.sin_cos();
        (
            local_x * cos_r - local_y * sin_r + translate_x + anchor_x,
            local_x * sin_r + local_y * cos_r + translate_y + anchor_y,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrameMarkerKind {
    /// Existing projects and ordinary authored keyframes keep the legacy marker.
    #[default]
    Legacy,
    /// A newly inserted empty cel/frame.
    InsertedEmpty,
    /// A keyframe/duplicate created from the currently visible previous pose.
    CopiedPose,
    /// An explicitly blank keyframe that stops the preceding exposure.
    Blank,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct FrameData {
    /// UI-only provenance for timeline markers. Old .stick files deserialize as Legacy.
    #[serde(default)]
    pub timeline_marker: FrameMarkerKind,
    #[serde(default)]
    pub strokes: Vec<Stroke>,
    #[serde(default)]
    pub media_items: Vec<MediaItem>,
    #[serde(default)]
    pub symbol_instances: Vec<SymbolInstance>,
    #[serde(default)]
    pub text_items: Vec<TextItem>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub motion_tween: bool,
    #[serde(default)]
    pub tween_mode: TweenMode,
    #[serde(default)]
    pub easing: EasingCurve,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerEffectPreset {
    None,
    Warm,
    Cool,
    Rose,
    Mint,
    Night,
    Vintage,
    Monochrome,
}
impl Default for LayerEffectPreset {
    fn default()->Self { Self::None }
}
impl LayerEffectPreset {
    pub fn rgb_multiplier(self)->[f32;3] {
        match self {
            Self::None | Self::Monochrome => [1.0,1.0,1.0],
            Self::Warm => [1.0,0.90,0.78],
            Self::Cool => [0.78,0.90,1.0],
            Self::Rose => [1.0,0.78,0.88],
            Self::Mint => [0.78,1.0,0.88],
            Self::Night => [0.62,0.72,1.0],
            Self::Vintage => [0.92,0.82,0.66],
        }
    }
    pub fn apply_rgba(self,color:[u8;4])->[u8;4] {
        if self==Self::Monochrome {
            let gray=(color[0] as f32*0.2126
                +color[1] as f32*0.7152
                +color[2] as f32*0.0722).round().clamp(0.0,255.0) as u8;
            return [gray,gray,gray,color[3]];
        }
        let m=self.rgb_multiplier();
        [
            (color[0] as f32*m[0]).round().clamp(0.0,255.0) as u8,
            (color[1] as f32*m[1]).round().clamp(0.0,255.0) as u8,
            (color[2] as f32*m[2]).round().clamp(0.0,255.0) as u8,
            color[3],
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Layer {
    #[serde(default = "default_layer_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub is_guide: bool,
    /// Editor-only articulated pose guide; kept in the stack for front/back ordering.
    #[serde(default)]
    pub is_pose_rig: bool,
    /// Reference layers participate in bucket boundaries without forcing the
    /// user to sample every visible paint layer.
    #[serde(default)]
    pub is_reference: bool,
    /// Whether a real stroke has already earned this layer's one-time credit.
    #[serde(default)]
    pub xp_first_paint_rewarded: bool,
    #[serde(default = "default_pressure")]
    pub opacity: f32,
    #[serde(default)]
    pub effect: LayerEffectPreset,
    #[serde(default)]
    pub keyframes: HashMap<usize, FrameData>,
    /// Optional flat color behind this layer's artwork; None is transparent.
    #[serde(default)]
    pub background: Option<[u8; 4]>,
}

fn default_layer_name() -> String {
    "Layer 1".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AudioTrack {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default = "default_one")]
    pub start_frame: usize,
    #[serde(default = "default_end_frame")]
    pub end_frame: usize,
    #[serde(default)]
    pub sync_mode: AudioSyncMode,
    #[serde(default)]
    pub waveform_peaks: Vec<f32>,
    #[serde(default = "default_audio_gain")]
    pub gain: f32,
    #[serde(default)]
    pub muted: bool,
}

fn default_audio_gain() -> f32 { 1.0 }

fn default_one() -> usize {
    1
}

fn default_end_frame() -> usize {
    120
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OnionSkinSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_two")]
    pub frames_before: usize,
    #[serde(default = "default_two")]
    pub frames_after: usize,
    #[serde(default = "default_onion_before")]
    pub color_before: [u8; 4],
    #[serde(default = "default_onion_after")]
    pub color_after: [u8; 4],
    /// Percentage of opacity removed for each additional previous ghost.
    #[serde(default = "default_onion_fade_percent")]
    pub fade_before_percent: f32,
    /// Percentage of opacity removed for each additional next ghost.
    #[serde(default = "default_onion_fade_percent")]
    pub fade_after_percent: f32,
}

fn default_two() -> usize {
    2
}

fn default_onion_fade_percent() -> f32 {
    25.0
}

fn default_onion_before() -> [u8; 4] {
    [255, 80, 80, 90]
}

fn default_onion_after() -> [u8; 4] {
    [80, 255, 100, 90]
}

impl Default for OnionSkinSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            frames_before: 2,
            frames_after: 2,
            color_before: [255, 80, 80, 90],
            color_after: [80, 255, 100, 90],
            fade_before_percent: default_onion_fade_percent(),
            fade_after_percent: default_onion_fade_percent(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AniProject {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default = "default_w")]
    pub width: u32,
    #[serde(default = "default_h")]
    pub height: u32,
    #[serde(default = "default_background_color")]
    pub background_color: [u8; 4],
    #[serde(default)]
    pub background_transparent: bool,
    #[serde(default)]
    pub pose_guide: crate::pose_guide::PoseGuide,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_one")]
    pub total_frames: usize,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub audio_tracks: Vec<AudioTrack>,
    #[serde(default)]
    pub library_symbols: HashMap<String, Symbol>,
    #[serde(default)]
    pub onion_skin: OnionSkinSettings,
    /// Saved frame numbers already credited for profile XP.
    #[serde(default)]
    pub xp_credited_frames: Vec<usize>,
}

fn default_schema() -> u32 { PROJECT_SCHEMA_VERSION }
fn default_w() -> u32 { 1920 }
fn default_h() -> u32 { 1080 }
fn default_background_color() -> [u8; 4] { [255, 255, 255, 255] }
fn default_fps() -> u32 { 24 }

impl Default for AniProject {
    fn default() -> Self {
        let mut keyframes = HashMap::new();
        keyframes.insert(1, FrameData::default());
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            width: 1920,
            height: 1080,
            background_color: default_background_color(),
            background_transparent: false,
            pose_guide: crate::pose_guide::PoseGuide::default(),
            fps: 24,
            total_frames: 1,
            layers: vec![Layer {
                name: "Layer 1".to_string(),
                visible: true,
                locked: false,
                is_guide: false,
                is_pose_rig: false, background: None,
                is_reference: false,
                xp_first_paint_rewarded: false,
                opacity: 1.0,
                effect: LayerEffectPreset::None,
                keyframes,
            }],
            audio_tracks: Vec::new(),
            library_symbols: HashMap::new(),
            onion_skin: OnionSkinSettings::default(),
            xp_credited_frames: Vec::new(),
        }
    }
}

impl AniProject {
    pub fn evaluate_layer_at_frame(&self, layer: &Layer, frame: usize) -> FrameData {
        self.layer_frame_view(layer, frame).into_owned()
    }

    /// Borrow the actual drawing for a keyframe or hold. Only a genuine
    /// interpolated tween needs an owned, freshly evaluated drawing.
    pub fn layer_frame_view<'a>(&self, layer: &'a Layer, frame: usize) -> std::borrow::Cow<'a, FrameData> {
        use std::borrow::Cow;
        let prev_k = match layer.keyframes.keys().filter(|&&k| k <= frame).max().copied() {
            Some(k) => k,
            None => return Cow::Owned(FrameData::default()),
        };
        let prev_fd = match layer.keyframes.get(&prev_k) {
            Some(fd) => fd,
            None => return Cow::Owned(FrameData::default()),
        };

        let effective_tween = if prev_fd.tween_mode != TweenMode::None {
            prev_fd.tween_mode
        } else if prev_fd.motion_tween {
            TweenMode::Classic
        } else {
            TweenMode::None
        };

        if effective_tween == TweenMode::None || prev_k == frame {
            return Cow::Borrowed(prev_fd);
        }

        if let Some(&next_k) = layer.keyframes.keys().filter(|&&k| k > prev_k).min() {
            if frame <= next_k {
                if let Some(next_fd) = layer.keyframes.get(&next_k) {
                    let raw_t = (frame - prev_k) as f32 / (next_k - prev_k).max(1) as f32;
                    let t = prev_fd.easing.apply(raw_t);

                    let mut evaluated = prev_fd.clone();

                    let mut interp_syms = Vec::with_capacity(prev_fd.symbol_instances.len());
                    for (idx, inst) in prev_fd.symbol_instances.iter().enumerate() {
                        if let Some(target) = next_fd.symbol_instances.get(idx) {
                            if target.symbol_name == inst.symbol_name {
                                interp_syms.push(SymbolInstance {
                                    symbol_name: inst.symbol_name.clone(),
                                    x: inst.x + (target.x - inst.x) * t,
                                    y: inst.y + (target.y - inst.y) * t,
                                    scale: inst.scale + (target.scale - inst.scale) * t,
                                    rotation_deg: inst.rotation_deg + (target.rotation_deg - inst.rotation_deg) * t,
                                    anchor_offset_x: inst.anchor_offset_x,
                                    anchor_offset_y: inst.anchor_offset_y,
                                });
                                continue;
                            }
                        }
                        interp_syms.push(inst.clone());
                    }
                    evaluated.symbol_instances = interp_syms;

                    if effective_tween == TweenMode::Shape {
                        let mut morphed_strokes = Vec::with_capacity(prev_fd.strokes.len());
                        for (idx, s0) in prev_fd.strokes.iter().enumerate() {
                            if let Some(s1) = next_fd.strokes.get(idx) {
                                // Bitmap patches cannot be resampled as vector paths.
                                // Hold the source patch until the next keyframe.
                                if crate::raster_patch::is_patch(s0) || crate::raster_patch::is_patch(s1) {
                                    morphed_strokes.push(s0.clone());
                                    continue;
                                }
                                let n = s0.points.len().max(s1.points.len()).clamp(2, 96);
                                let pts0 = s0.resample_points(n);
                                let pts1 = s1.resample_points(n);
                                let mut blended = Vec::with_capacity(n);
                                for (p0, p1) in pts0.iter().zip(pts1.iter()) {
                                    blended.push(StrokePoint {
                                        x: p0.x + (p1.x - p0.x) * t,
                                        y: p0.y + (p1.y - p0.y) * t,
                                        pressure: p0.pressure + (p1.pressure - p0.pressure) * t,
                                    });
                                }
                                let c = [
                                    (s0.color[0] as f32 + (s1.color[0] as f32 - s0.color[0] as f32) * t) as u8,
                                    (s0.color[1] as f32 + (s1.color[1] as f32 - s0.color[1] as f32) * t) as u8,
                                    (s0.color[2] as f32 + (s1.color[2] as f32 - s0.color[2] as f32) * t) as u8,
                                    (s0.color[3] as f32 + (s1.color[3] as f32 - s0.color[3] as f32) * t) as u8,
                                ];
                                morphed_strokes.push(Stroke {
                                    points: blended,
                                    color: c,
                                    fill_color: s0.fill_color.or(s1.fill_color),
                                    base_width: s0.base_width + (s1.base_width - s0.base_width) * t,
                                    is_vector: true,
                                    brush_preset: s0.brush_preset,
                                    opacity: s0.opacity + (s1.opacity - s0.opacity) * t,
                                    brush_texture_id: None,
                                    tip_shape: s0.tip_shape,
                                });
                            } else {
                                morphed_strokes.push(s0.clone());
                            }
                        }
                        evaluated.strokes = morphed_strokes;
                    }

                    return Cow::Owned(evaluated);
                }
            }
        }
        Cow::Borrowed(prev_fd)
    }

    pub fn evaluate_symbol_strokes<'a>(&'a self, sym: &'a Symbol, global_frame: usize) -> &'a [Stroke] {
        if sym.nested_keyframes.is_empty() {
            return &sym.strokes;
        }
        let local_f = ((global_frame.saturating_sub(1)) % sym.nested_total_frames.max(1)) + 1;
        if let Some(&k) = sym.nested_keyframes.keys().filter(|&&k| k <= local_f).max() {
            if let Some(st) = sym.nested_keyframes.get(&k) {
                return st;
            }
        }
        &sym.strokes
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        crate::persistence::save_project_safe(self, path)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        crate::persistence::open_project_safe(path).map(|o| o.project)
    }
}

pub fn parse_svg_to_strokes(svg_path: &Path) -> Result<Vec<Stroke>, String> {
    let content = fs::read_to_string(svg_path).map_err(|e| format!("SVG Read Error: {}", e))?;
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("SVG Parse Error: {}", e))?;
    let mut result = Vec::new();

    for node in doc.descendants() {
        let tag = node.tag_name().name();
        if tag == "path" {
            if let Some(d) = node.attribute("d") {
                let stroke_col = parse_hex_color(node.attribute("stroke")).unwrap_or([0, 0, 0, 255]);
                let fill_col = parse_hex_color(node.attribute("fill"));
                let width = node
                    .attribute("stroke-width")
                    .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
                    .unwrap_or(4.0);
                let pts = parse_svg_path_d(d);
                if pts.len() >= 2 {
                    result.push(Stroke {
                        points: pts,
                        color: stroke_col,
                        fill_color: fill_col,
                        base_width: width,
                        is_vector: true,
                        brush_preset: KritaBrushPreset::GPenInk,
                        opacity: 1.0,
                        brush_texture_id: None,
                                    tip_shape: BrushTipShape::Round,
                    });
                }
            }
        }
    }

    if result.is_empty() {
        return Err("لم يتم العثور على مسارات فيكتور صالحة داخل ملف SVG.".to_string());
    }
    Ok(result)
}

fn parse_hex_color(attr: Option<&str>) -> Option<[u8; 4]> {
    let s = attr?.trim();
    if s == "none" || s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some([r, g, b, 255]);
        }
    }
    None
}

fn parse_svg_path_d(d: &str) -> Vec<StrokePoint> {
    let mut pts = Vec::new();
    let cleaned = d.replace(',', " ");
    let mut tokens = Vec::new();
    let mut buf = String::new();
    for ch in cleaned.chars() {
        if ch.is_alphabetic() {
            if !buf.trim().is_empty() {
                tokens.push(buf.trim().to_string());
                buf.clear();
            }
            tokens.push(ch.to_string());
        } else if ch.is_whitespace() {
            if !buf.trim().is_empty() {
                tokens.push(buf.trim().to_string());
                buf.clear();
            }
        } else if ch == '-' && !buf.is_empty() && !buf.ends_with('e') && !buf.ends_with('E') {
            tokens.push(buf.trim().to_string());
            buf.clear();
            buf.push(ch);
        } else {
            buf.push(ch);
        }
    }
    if !buf.trim().is_empty() {
        tokens.push(buf.trim().to_string());
    }

    let mut idx = 0;
    let mut cmd = 'M';
    let mut cur_x = 0.0f32;
    let mut cur_y = 0.0f32;
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;

    while idx < tokens.len() {
        if let Some(next_cmd)=tokens[idx].chars().next()
            .filter(|ch|tokens[idx].len()==1 && ch.is_ascii_alphabetic())
        {
            cmd=next_cmd;
            idx += 1;
            if cmd == 'Z' || cmd == 'z' {
                pts.push(StrokePoint { x: start_x, y: start_y, pressure: 1.0 });
                cur_x = start_x;
                cur_y = start_y;
                continue;
            }
        }
        let read_f = |i: &mut usize| -> Option<f32> {
            let v = tokens.get(*i)?.parse::<f32>().ok()?;
            *i += 1;
            Some(v)
        };
        match cmd {
            'M' => {
                if let (Some(x), Some(y)) = (read_f(&mut idx), read_f(&mut idx)) {
                    cur_x = x;
                    cur_y = y;
                    start_x = x;
                    start_y = y;
                    pts.push(StrokePoint { x: cur_x, y: cur_y, pressure: 1.0 });
                    cmd = 'L';
                } else {
                    idx += 1;
                }
            }
            'L' => {
                if let (Some(x), Some(y)) = (read_f(&mut idx), read_f(&mut idx)) {
                    cur_x = x;
                    cur_y = y;
                    pts.push(StrokePoint { x: cur_x, y: cur_y, pressure: 1.0 });
                } else {
                    idx += 1;
                }
            }
            _ => {
                idx += 1;
            }
        }
    }
    pts
}

pub fn import_adobe_animate_project(path: &Path) -> Result<AniProject, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mut xml_content = String::new();

    if ext == "xfl" {
        let parent = path.parent().unwrap_or(Path::new("."));
        let dom_path = parent.join("DOMDocument.xml");
        xml_content = fs::read_to_string(&dom_path)
            .map_err(|_| "لم يتم العثور على DOMDocument.xml بجوار ملف XFL.".to_string())?;
    } else if ext == "fla" {
        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|_| "ملف FLA قديم غير مضغوط بصيغة XFL، قم بحفظه كـ XFL من Animate.".to_string())?;
        for i in 0..archive.len() {
            if let Ok(mut entry) = archive.by_index(i) {
                if entry.name().ends_with("DOMDocument.xml") {
                    entry.read_to_string(&mut xml_content).map_err(|e| e.to_string())?;
                    break;
                }
            }
        }
    }

    if xml_content.is_empty() {
        return Err("تعذر قراءة بيانات Adobe Animate.".to_string());
    }

    let doc = roxmltree::Document::parse(&xml_content).map_err(|e| e.to_string())?;
    let mut proj = AniProject::default();
    let root = doc.root_element();
    if let Some(w) = root.attribute("width").and_then(|v| v.parse::<u32>().ok()) {
        proj.width = w;
    }
    if let Some(h) = root.attribute("height").and_then(|v| v.parse::<u32>().ok()) {
        proj.height = h;
    }
    if let Some(fps) = root.attribute("frameRate").and_then(|v| v.parse::<u32>().ok()) {
        proj.fps = fps;
    }

    let mut imported_layers = Vec::new();
    for node in doc.descendants() {
        if node.tag_name().name() == "DOMLayer" {
            let l_name = node.attribute("name").unwrap_or("Animate_Layer").to_string();
            let mut kf_map = HashMap::new();
            for frame_node in node.descendants().filter(|n| n.tag_name().name() == "DOMFrame") {
                let idx = frame_node.attribute("index").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0) + 1;
                let mut fd = FrameData::default();
                for edge in frame_node.descendants().filter(|n| n.tag_name().name() == "Edge") {
                    if let Some(edges_str) = edge.attribute("edges") {
                        let pts = parse_animate_edges(edges_str);
                        if pts.len() >= 2 {
                            fd.strokes.push(Stroke {
                                points: pts,
                                color: [0, 0, 0, 255],
                                fill_color: None,
                                base_width: 3.0,
                                is_vector: true,
                                brush_preset: KritaBrushPreset::GPenInk,
                                opacity: 1.0,
                                brush_texture_id: None,
                                    tip_shape: BrushTipShape::Round,
                            });
                        }
                    }
                }
                kf_map.insert(idx, fd);
            }
            if kf_map.is_empty() {
                kf_map.insert(1, FrameData::default());
            }
            imported_layers.push(Layer {
                name: l_name,
                visible: true,
                locked: false,
                is_guide: false,
                is_pose_rig: false, background: None,
                is_reference: false,
                xp_first_paint_rewarded: false,
                opacity: 1.0,
                effect: LayerEffectPreset::None,
                keyframes: kf_map,
            });
        }
    }

    if !imported_layers.is_empty() {
        let max_f = imported_layers.iter().flat_map(|l| l.keyframes.keys().copied()).max().unwrap_or(1);
        proj.layers = imported_layers;
        proj.total_frames = max_f;
    }
    Ok(proj)
}

fn parse_animate_edges(edges: &str) -> Vec<StrokePoint> {
    let mut pts = Vec::new();
    let nums: Vec<f32> = edges
        .split(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    for pair in nums.chunks(2) {
        if pair.len() == 2 {
            pts.push(StrokePoint {
                x: pair[0] / 20.0,
                y: pair[1] / 20.0,
                pressure: 1.0,
            });
        }
    }
    pts
}

#[cfg(test)]
mod brush_tip_compat_tests {
    use super::*;

    #[test]
    fn old_layer_without_effect_defaults_to_none() {
        let layer:Layer=serde_json::from_str(
            r#"{"name":"Old","visible":true,"locked":false,"opacity":1.0,"keyframes":{}}"#
        ).unwrap();
        assert_eq!(layer.effect,LayerEffectPreset::None);
    }

    #[test]
    fn layer_effect_presets_transform_only_rgb() {
        let c=LayerEffectPreset::Warm.apply_rgba([100,120,140,77]);
        assert_eq!(c[3],77);
        assert_eq!(c[0],100);
        assert!(c[1]<120 && c[2]<140);
    }

    #[test]
    fn monochrome_layer_effect_preserves_alpha_and_equalizes_rgb() {
        let c=LayerEffectPreset::Monochrome.apply_rgba([220,80,20,91]);
        assert_eq!(c[3],91);
        assert_eq!(c[0],c[1]);
        assert_eq!(c[1],c[2]);
        assert!(c[0]>80 && c[0]<220);
    }

    #[test]
    fn legacy_strokes_default_to_round_tip() {
        let legacy = r#"{
          "points":[{"x":2.0,"y":3.0}],
          "color":[12,34,56,255],
          "fill_color":null,
          "base_width":7.0,
          "is_vector":false,
          "brush_preset":"GPenInk",
          "opacity":1.0,
          "brush_texture_id":null
        }"#;
        let old: Stroke = serde_json::from_str(legacy).expect("old stroke format");
        assert_eq!(old.tip_shape, BrushTipShape::Round);
        let encoded = serde_json::to_string(&old).unwrap();
        let restored: Stroke = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.tip_shape, BrushTipShape::Round);
        assert_eq!(restored.points.len(), 1);
        assert_eq!(restored.color, [12, 34, 56, 255]);
    }

    #[test]
    fn graphite_pencil_is_a_new_serializable_preset_without_reordering_old_ones() {
        let json=serde_json::to_string(&KritaBrushPreset::GraphitePencil).unwrap();
        let restored:KritaBrushPreset=serde_json::from_str(&json).unwrap();
        assert_eq!(restored,KritaBrushPreset::GraphitePencil);
        assert_eq!(KritaBrushPreset::MENU.len(),16);
        assert!(KritaBrushPreset::MENU.iter().any(|(preset,_,ar)|
            *preset==KritaBrushPreset::GraphitePencil && *ar=="قلم رصاص"));
        assert!(KritaBrushPreset::MENU.iter().any(|(preset,en,ar)|
            *preset==KritaBrushPreset::SketchBrush && *en=="Sketch Brush" && *ar=="فرشة الاسكتش"));
        let sketch_json=serde_json::to_string(&KritaBrushPreset::SketchBrush).unwrap();
        assert_eq!(serde_json::from_str::<KritaBrushPreset>(&sketch_json).unwrap(),
            KritaBrushPreset::SketchBrush);
    }

    #[test]
    fn old_onion_skin_settings_get_fade_defaults() {
        let settings: OnionSkinSettings = serde_json::from_str(
            r#"{"enabled":true,"frames_before":2,"frames_after":2,"color_before":[255,80,80,90],"color_after":[80,255,100,90]}"#
        ).unwrap();
        assert_eq!(settings.fade_before_percent,25.0);
        assert_eq!(settings.fade_after_percent,25.0);
    }

    #[test]
    fn old_frame_data_loads_with_legacy_timeline_marker() {
        let frame: FrameData = serde_json::from_str(
            r#"{"strokes":[],"media_items":[],"symbol_instances":[],"text_items":[]}"#
        ).expect("old frame data");
        assert_eq!(frame.timeline_marker, FrameMarkerKind::Legacy);
    }

    #[test]
    fn timeline_marker_round_trips_without_changing_project_schema() {
        let mut frame = FrameData::default();
        frame.timeline_marker = FrameMarkerKind::CopiedPose;
        let json = serde_json::to_string(&frame).unwrap();
        let restored: FrameData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.timeline_marker, FrameMarkerKind::CopiedPose);
    }
    #[test]
    fn symbol_instance_transform_matches_export_transform_order() {
        let instance = SymbolInstance {
            symbol_name: "bird".into(),
            x: 10.0,
            y: 20.0,
            scale: 2.0,
            rotation_deg: 90.0,
            anchor_offset_x: 1.0,
            anchor_offset_y: 1.0,
        };
        let (x, y) = instance.transform_point(2.0, 1.0);
        assert!((x - 11.0).abs() < 0.001, "x={x}");
        assert!((y - 23.0).abs() < 0.001, "y={y}");
    }

    #[test]
    fn symbol_instance_transform_sanitizes_non_finite_values() {
        let instance = SymbolInstance {
            symbol_name: "bad".into(),
            x: f32::NAN,
            y: 0.0,
            scale: f32::INFINITY,
            rotation_deg: f32::NAN,
            anchor_offset_x: 0.0,
            anchor_offset_y: 0.0,
        };
        let point = instance.transform_point(f32::NAN, 5.0);
        assert!(point.0.is_finite() && point.1.is_finite());
    }

    #[test]
    fn old_text_without_rotation_loads_at_zero_degrees() {
        let text: TextItem=serde_json::from_str(
            r#"{"content":"hello","x":10.0,"y":20.0,"font_size":30.0,"color":[0,0,0,255]}"#
        ).unwrap();
        assert_eq!(text.rotation_deg,0.0);
    }

    #[test]
    fn old_media_and_audio_load_without_new_transform_fields() {
        let old_media: MediaItem = serde_json::from_str(
            r#"{"path":"art.png","media_type":"Image","x":20.0,"y":30.0,"width":100.0,"height":80.0}"#
        ).expect("legacy media item");
        assert_eq!(old_media.rotation_deg, 0.0);
        assert!(!old_media.flip_h && !old_media.flip_v);
        let old_audio: AudioTrack = serde_json::from_str(
            r#"{"name":"voice","file_path":"voice.wav","start_frame":10,"end_frame":30,"sync_mode":"Stream","waveform_peaks":[0.1,0.5]}"#
        ).expect("legacy audio track");
        assert_eq!(old_audio.gain,1.0);
        assert!(!old_audio.muted);
        assert_eq!((old_audio.start_frame,old_audio.end_frame),(10,30));
        let roundtrip: MediaItem=serde_json::from_str(&serde_json::to_string(&old_media).unwrap()).unwrap();
        assert_eq!(old_media,roundtrip);
    }

    #[test]
    fn polygonal_brush_tips_roundtrip() {
        let mut stroke: Stroke = serde_json::from_str(
            r#"{"points":[],"color":[0,0,0,255],"brush_preset":"GPenInk"}"#
        ).unwrap();
        stroke.tip_shape = BrushTipShape::Pentagon;
        let serialized = serde_json::to_string(&stroke).unwrap();
        let opened: Stroke = serde_json::from_str(&serialized).unwrap();
        assert_eq!(opened.tip_shape, BrushTipShape::Pentagon);
    }
}

#[cfg(test)]
mod layer_frame_view_tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn held_frames_borrow_the_authored_key_and_empty_frames_are_owned() {
        let mut project = AniProject::default();
        project.total_frames = 10;
        let layer = &mut project.layers[0];
        layer.keyframes.get_mut(&1).unwrap().label = "source".into();
        let layer = &project.layers[0];
        assert!(matches!(project.layer_frame_view(layer, 0), Cow::Owned(_)));
        for frame in [1, 2, 10] {
            let view = project.layer_frame_view(layer, frame);
            assert!(matches!(&view, Cow::Borrowed(_)));
            assert!(std::ptr::eq(view.as_ref(), &layer.keyframes[&1]));
        }
    }

    #[test]
    fn real_tweens_are_owned_but_endpoints_and_trailing_holds_are_borrowed() {
        let mut project = AniProject::default();
        project.total_frames = 10;
        let layer = &mut project.layers[0];
        layer.keyframes.get_mut(&1).unwrap().tween_mode = TweenMode::Classic;
        layer.keyframes.insert(5, FrameData::default());
        let layer = &project.layers[0];
        assert!(matches!(project.layer_frame_view(layer, 1), Cow::Borrowed(_)));
        assert!(matches!(project.layer_frame_view(layer, 3), Cow::Owned(_)));
        assert!(matches!(project.layer_frame_view(layer, 5), Cow::Borrowed(_)));
        assert!(matches!(project.layer_frame_view(layer, 8), Cow::Borrowed(_)));
        assert_eq!(project.layer_frame_view(layer, 3).into_owned(), project.evaluate_layer_at_frame(layer, 3));
    }
}
