use crate::persistence::sniff::{DetectedFormat, SniffResult};
use crate::project::{
    AniProject, FrameData, Layer, LayerEffectPreset, OnionSkinSettings, PROJECT_SCHEMA_VERSION,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct EnvelopePayload {
    magic: String,
    schema_version: u32,
    project: AniProject,
}

#[derive(Deserialize)]
struct LegacyLayerV1 {
    #[serde(default = "default_layer_name")]
    name: String,
    #[serde(default = "default_true")]
    visible: bool,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    frames: Vec<FrameData>,
}

#[derive(Deserialize)]
struct LegacyProjectV1 {
    #[serde(default = "default_w")]
    width: u32,
    #[serde(default = "default_h")]
    height: u32,
    #[serde(default = "default_fps")]
    fps: u32,
    #[serde(default)]
    layers: Vec<LegacyLayerV1>,
}

fn default_layer_name() -> String { "Layer 1".to_string() }
fn default_true() -> bool { true }
fn default_w() -> u32 { 1920 }
fn default_h() -> u32 { 1080 }
fn default_fps() -> u32 { 24 }

#[derive(Debug, Clone)]
pub struct ProjectInventory {
    pub layer_count: usize,
    pub keyframe_count: usize,
    pub stroke_count: usize,
    pub total_points: usize,
    pub media_count: usize,
    pub audio_count: usize,
    pub symbol_count: usize,
}

impl ProjectInventory {
    pub fn from_project(proj: &AniProject) -> Self {
        let mut kf_count = 0;
        let mut strokes = 0;
        let mut points = 0;
        let mut media = 0;
        for l in &proj.layers {
            kf_count += l.keyframes.len();
            for fd in l.keyframes.values() {
                strokes += fd.strokes.len();
                media += fd.media_items.len();
                for s in &fd.strokes {
                    points += s.points.len();
                }
            }
        }
        Self {
            layer_count: proj.layers.len(),
            keyframe_count: kf_count,
            stroke_count: strokes,
            total_points: points,
            media_count: media,
            audio_count: proj.audio_tracks.len(),
            symbol_count: proj.library_symbols.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub source_format_label: String,
    pub old_schema: u32,
    pub new_schema: u32,
    pub was_migrated: bool,
    pub inventory: ProjectInventory,
    pub warnings: Vec<String>,
}

pub fn decode_and_migrate(sniffed: SniffResult) -> Result<(AniProject, MigrationReport), String> {
    match sniffed.format {
        DetectedFormat::NewerUnsupportedSchema { schema_version } => {
            Err(format!(
                "هذا المشروع تم إنشاؤه بإصدار أحدث من Stick (Schema v{} بينما نسختك تدعم حتى v{}). يرجى تحديث البرنامج أولاً لحماية بيانات المشروع.",
                schema_version, PROJECT_SCHEMA_VERSION
            ))
        }
        DetectedFormat::EnvelopeZstd { schema_version } | DetectedFormat::EnvelopeJson { schema_version } => {
            let env: EnvelopePayload = serde_json::from_slice(&sniffed.payload_bytes)
                .map_err(|e| format!("فشل قراءة بيانات مغلف المشروع: {}", e))?;
            if env.magic != "STICK_STUDIO_PROJECT" {
                return Err("توقيع المشروع غير متطابق.".to_string());
            }
            let before_inv = ProjectInventory::from_project(&env.project);
            let migrated = migrate_step_by_step(env.project, schema_version)?;
            validate_no_data_loss(&before_inv, &migrated)?;
            let after_inv = ProjectInventory::from_project(&migrated);
            Ok((
                migrated,
                MigrationReport {
                    source_format_label: if sniffed.was_zstd {
                        format!("Stick Envelope Zstd (v{})", schema_version)
                    } else {
                        format!("Stick Envelope JSON (v{})", schema_version)
                    },
                    old_schema: schema_version,
                    new_schema: PROJECT_SCHEMA_VERSION,
                    was_migrated: schema_version < PROJECT_SCHEMA_VERSION,
                    inventory: after_inv,
                    warnings: Vec::new(),
                },
            ))
        }
        DetectedFormat::LegacyV1Json => {
            let leg: LegacyProjectV1 = serde_json::from_slice(&sniffed.payload_bytes)
                .map_err(|e| format!("فشل قراءة مشروع Legacy V1 JSON: {}", e))?;
            convert_legacy_v1(leg, "Legacy V1 JSON")
        }
        DetectedFormat::LegacyV1MsgPack => {
            let leg: LegacyProjectV1 = rmp_serde::from_slice(&sniffed.payload_bytes)
                .map_err(|e| format!("فشل قراءة مشروع Legacy V1 MessagePack: {}", e))?;
            convert_legacy_v1(leg, "Legacy V1 MessagePack")
        }
        DetectedFormat::BareProjectJson { inferred_schema } => {
            let proj: AniProject = serde_json::from_slice(&sniffed.payload_bytes)
                .map_err(|e| format!("فشل قراءة مشروع JSON القديم: {}", e))?;
            let before_inv = ProjectInventory::from_project(&proj);
            let migrated = migrate_step_by_step(proj, inferred_schema)?;
            validate_no_data_loss(&before_inv, &migrated)?;
            let after_inv = ProjectInventory::from_project(&migrated);
            Ok((
                migrated,
                MigrationReport {
                    source_format_label: format!("Legacy Bare JSON (v{})", inferred_schema),
                    old_schema: inferred_schema,
                    new_schema: PROJECT_SCHEMA_VERSION,
                    was_migrated: true,
                    inventory: after_inv,
                    warnings: Vec::new(),
                },
            ))
        }
        DetectedFormat::BareProjectMsgPack { inferred_schema } => {
            let proj: AniProject = rmp_serde::from_slice(&sniffed.payload_bytes)
                .map_err(|e| format!("فشل قراءة مشروع MessagePack القديم: {}", e))?;
            let before_inv = ProjectInventory::from_project(&proj);
            let migrated = migrate_step_by_step(proj, inferred_schema)?;
            validate_no_data_loss(&before_inv, &migrated)?;
            let after_inv = ProjectInventory::from_project(&migrated);
            Ok((
                migrated,
                MigrationReport {
                    source_format_label: format!("Legacy Bare MessagePack (v{})", inferred_schema),
                    old_schema: inferred_schema,
                    new_schema: PROJECT_SCHEMA_VERSION,
                    was_migrated: true,
                    inventory: after_inv,
                    warnings: Vec::new(),
                },
            ))
        }
    }
}

fn convert_legacy_v1(leg: LegacyProjectV1, label: &str) -> Result<(AniProject, MigrationReport), String> {
    let expected_layers = leg.layers.len();
    let expected_strokes: usize = leg
        .layers
        .iter()
        .flat_map(|l| l.frames.iter())
        .map(|f| f.strokes.len())
        .sum();

    let mut converted_layers = Vec::with_capacity(leg.layers.len());
    let mut max_len = 1usize;

    for l in leg.layers {
        let mut kf_map = HashMap::new();
        for (idx, fd) in l.frames.into_iter().enumerate() {
            kf_map.insert(idx + 1, fd);
        }
        if kf_map.is_empty() {
            kf_map.insert(1, FrameData::default());
        }
        max_len = max_len.max(kf_map.len());
        converted_layers.push(Layer {
            name: l.name,
            visible: l.visible,
            locked: l.locked,
            is_guide: false,
            is_pose_rig: false,
            background: None,
            is_reference: false,
            xp_first_paint_rewarded: false,
            opacity: 1.0,

            effect: LayerEffectPreset::None,
            keyframes: kf_map,
        });
    }

    let proj = AniProject {
        schema_version: 1,
        width: leg.width.max(1),
        height: leg.height.max(1),
        background_color: [255,255,255,255],
        background_transparent: false,
        pose_guide: crate::pose_guide::PoseGuide::default(),
        fps: leg.fps.max(1),
        total_frames: max_len,
        layers: converted_layers,
        audio_tracks: Vec::new(),
        library_symbols: HashMap::new(),
        onion_skin: OnionSkinSettings::default(),
        xp_credited_frames: Vec::new(),
    };

    let migrated = migrate_step_by_step(proj, 1)?;
    let inv = ProjectInventory::from_project(&migrated);

    if inv.layer_count != expected_layers || inv.stroke_count != expected_strokes {
        return Err("فشل التحقق من سلامة التحويل: حدث عدم تطابق في عدد الطبقات أو الرسومات أثناء تحويل V1.".to_string());
    }

    Ok((
        migrated,
        MigrationReport {
            source_format_label: label.to_string(),
            old_schema: 1,
            new_schema: PROJECT_SCHEMA_VERSION,
            was_migrated: true,
            inventory: inv,
            warnings: Vec::new(),
        },
    ))
}

fn migrate_step_by_step(mut proj: AniProject, from_schema: u32) -> Result<AniProject, String> {
    let mut current = from_schema;
    while current < PROJECT_SCHEMA_VERSION {
        match current {
            1 => {
                // v1 -> v2: ضمان وجود طبقة واحدة على الأقل وفريم 1
                if proj.layers.is_empty() {
                    let mut kf = HashMap::new();
                    kf.insert(1, FrameData::default());
                    proj.layers.push(Layer {
                        name: "Layer 1".to_string(),
                        visible: true,
                        locked: false,
                        is_guide: false,
            is_pose_rig: false,
            background: None,
            is_reference: false,
                        xp_first_paint_rewarded: false,
                        opacity: 1.0,

                        effect: LayerEffectPreset::None,
                        keyframes: kf,
                    });
                }
                current = 2;
            }
            2 => {
                // v2 -> v3: ضبط الشفافية الافتراضية للطبقات القديمة إذا كانت 0 بالخطأ
                for l in &mut proj.layers {
                    if l.opacity <= 0.0 || l.opacity.is_nan() {
                        l.opacity = 1.0;
                    }
                }
                current = 3;
            }
            3 => {
                // v3 -> v4: ضبط أبعاد الكانفا والـ FPS لمنع القسمة على صفر
                if proj.width == 0 { proj.width = 1920; }
                if proj.height == 0 { proj.height = 1080; }
                if proj.fps == 0 { proj.fps = 24; }
                current = 4;
            }
            4 => {
                // v4 -> v5: حساب total_frames الفعلي من أعلى Keyframe
                let max_kf = proj
                    .layers
                    .iter()
                    .flat_map(|l| l.keyframes.keys().copied())
                    .max()
                    .unwrap_or(1);
                proj.total_frames = proj.total_frames.max(max_kf).max(1);
                current = 5;
            }
            5 => {
                // v5 -> v6: persist canvas background and editor-only pose-rig settings.
                // Missing fields are supplied by serde defaults (white background, hidden rig).
                current = 6;
            }
            6 => {
                // Embedded raster patches use the existing optional texture field.
                current = 7;
            }
            _ => {
                return Err(format!("لا يوجد مسار تحويل معروف من الإصدار v{}.", current));
            }
        }
    }
    proj.schema_version = PROJECT_SCHEMA_VERSION;
    Ok(proj)
}

fn validate_no_data_loss(before: &ProjectInventory, after: &AniProject) -> Result<(), String> {
    let now = ProjectInventory::from_project(after);
    if before.layer_count > 0 && now.layer_count < before.layer_count {
        return Err("تم إيقاف التحويل: فقدان غير متوقع في عدد الطبقات.".to_string());
    }
    if before.stroke_count > 0 && now.stroke_count < before.stroke_count {
        return Err("تم إيقاف التحويل: فقدان غير متوقع في الرسومات.".to_string());
    }
    if before.total_points > 0 && now.total_points < before.total_points {
        return Err("تم إيقاف التحويل: فقدان غير متوقع في نقاط الفرش.".to_string());
    }
    Ok(())
}
