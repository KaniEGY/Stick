pub mod atomic_save;
pub mod migration;
pub mod sniff;

use crate::project::AniProject;
use atomic_save::{create_legacy_backup_copy, save_project_atomic};
use migration::{decode_and_migrate, MigrationReport};
use sniff::sniff_project_bytes;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LoadedProjectOutcome {
    pub project: AniProject,
    pub report: MigrationReport,
    pub missing_assets: Vec<String>,
    pub legacy_backup_path: Option<PathBuf>,
}

pub fn open_project_safe(path: &Path) -> Result<LoadedProjectOutcome, String> {
    if !path.exists() {
        return Err(format!("الملف غير موجود: {}", path.display()));
    }
    let file = fs::File::open(path).map_err(|e| format!("تعذر قراءة الملف من القرص: {e}"))?;
    let limit = sniff::MAX_PROJECT_RAW_BYTES as u64;
    if file.metadata().map_err(|e| e.to_string())?.len() > limit {
        return Err("حجم ملف المشروع يتجاوز الحد الأقصى (250MB).".into());
    }
    let mut raw = Vec::new();
    file.take(limit + 1).read_to_end(&mut raw)
        .map_err(|e| format!("تعذر قراءة الملف من القرص: {e}"))?;
    let sniffed = sniff_project_bytes(&raw)?;
    let (project, report) = decode_and_migrate(sniffed)?;
    validate_project_bounds(&project)?;

    let legacy_backup_path = if report.was_migrated {
        Some(create_legacy_backup_copy(path).map_err(|e| {
            format!("تعذر فتح المشروع القديم بأمان لأن إنشاء النسخة الاحتياطية فشل: {}", e)
        })?)
    } else {
        None
    };

    let missing_assets = collect_missing_assets(&project);

    Ok(LoadedProjectOutcome {
        project,
        report,
        missing_assets,
        legacy_backup_path,
    })
}

pub(crate) fn validate_project_bounds(project: &AniProject) -> Result<(), String> {
    let pixels = u64::from(project.width) * u64::from(project.height);
    if project.width == 0 || project.height == 0
        || project.width > crate::canvas_sizes::MAX_DIMENSION
        || project.height > crate::canvas_sizes::MAX_DIMENSION
        || pixels > crate::canvas_sizes::MAX_PIXELS
        || project.fps == 0 || project.total_frames == 0
        || project.layers.is_empty()
    {
        return Err("أبعاد أو بيانات المشروع غير صالحة أو تتجاوز حدود الذاكرة الآمنة. لم يتم تعديل ملفك.".into());
    }
    Ok(())
}

pub fn save_project_safe(project: &AniProject, target_path: &Path) -> Result<(), String> {
    save_project_atomic(project, target_path)
}

pub fn collect_missing_assets(project: &AniProject) -> Vec<String> {
    let mut missing = Vec::new();
    for layer in &project.layers {
        for fd in layer.keyframes.values() {
            for m in &fd.media_items {
                if !m.path.is_empty() && !Path::new(&m.path).exists() && !missing.contains(&m.path) {
                    missing.push(m.path.clone());
                }
            }
        }
    }
    for track in &project.audio_tracks {
        if !track.file_path.is_empty()
            && !Path::new(&track.file_path).exists()
            && !missing.contains(&track.file_path)
        {
            missing.push(track.file_path.clone());
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{KritaBrushPreset, Stroke, StrokePoint};

    #[test]
    fn rejects_unsafe_project_dimensions_without_allocating_canvas() {
        let mut p = AniProject::default();
        p.width = u32::MAX;
        assert!(validate_project_bounds(&p).is_err());
        p.width = 3840;
        p.height = 2160;
        assert!(validate_project_bounds(&p).is_ok());
        p.fps = 0;
        assert!(validate_project_bounds(&p).is_err());
    }

    #[test]
    fn cmp_002_legacy_v1_preserves_layers_and_strokes() {
        let legacy_v1_json = r#"{
            "width": 1280,
            "height": 720,
            "fps": 24,
            "layers": [
                {
                    "name": "طبقة_قديمة",
                    "visible": true,
                    "locked": false,
                    "frames": [
                        {
                            "strokes": [
                                {
                                    "points": [{"x": 10.0, "y": 20.0}, {"x": 100.0, "y": 150.0}],
                                    "color": [255, 0, 0, 255],
                                    "base_width": 5.0
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let sniffed = sniff_project_bytes(legacy_v1_json.as_bytes()).expect("Should detect Legacy V1");
        let (proj, report) = decode_and_migrate(sniffed).expect("Should migrate Legacy V1");
        assert!(report.was_migrated);
        assert_eq!(report.old_schema, 1);
        assert_eq!(proj.layers.len(), 1);
        assert_eq!(proj.layers[0].name, "طبقة_قديمة");
        let f1 = proj.layers[0].keyframes.get(&1).expect("Frame 1 must exist");
        assert_eq!(f1.strokes.len(), 1);
        assert_eq!(f1.strokes[0].points.len(), 2);
    }

    #[test]
    fn cmp_003_rejects_unknown_json_instead_of_empty_project() {
        let unknown = br#"{"hello": 1, "random": "data"}"#;
        let res = sniff_project_bytes(unknown);
        assert!(res.is_err(), "Unknown JSON must be rejected");
    }

    #[test]
    fn cmp_005_rejects_newer_future_schema() {
        let future = br#"{"magic":"STICK_STUDIO_PROJECT","schema_version":999,"project":{}}"#;
        let sniffed = sniff_project_bytes(future).expect("Sniff detects newer schema");
        let res = decode_and_migrate(sniffed);
        assert!(res.is_err(), "Future schema must not be silently downgraded");
    }


    #[test]
    fn cmp_009_legacy_open_preserves_original_and_backup() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Clock should be after epoch")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!(
            "stick_legacy_open_{}_{}", std::process::id(), suffix
        ));
        fs::create_dir_all(&test_dir).expect("Create test directory");
        let original_path = test_dir.join("old_project.stick");
        let original_bytes = br#"{
            "width": 640, "height": 480, "fps": 24,
            "layers": [{"name": "Legacy", "frames": [
                {"strokes": [{"points": [{"x": 1.0, "y": 2.0}], "base_width": 2.0}]}
            ]}]
        }"#;
        fs::write(&original_path, original_bytes).expect("Write legacy fixture");

        let outcome = open_project_safe(&original_path).expect("Legacy project should open");
        assert!(outcome.report.was_migrated);
        assert_eq!(outcome.project.layers.len(), 1);
        assert_eq!(outcome.project.layers[0].keyframes[&1].strokes.len(), 1);
        assert_eq!(fs::read(&original_path).unwrap(), original_bytes);

        let backup_path = outcome.legacy_backup_path.expect("Backup must be created");
        assert_eq!(fs::read(&backup_path).unwrap(), original_bytes);
        fs::remove_dir_all(test_dir).expect("Clean up test directory");
    }

    #[test]
    fn embedded_raster_survives_save_reload_and_export() {
        let mut proj=AniProject::default();
        proj.width=8;proj.height=8;
        let mut pix=tiny_skia::Pixmap::new(2,2).unwrap();
        pix.data_mut()[0..4].copy_from_slice(&[200,50,30,255]);
        let patch=crate::raster_patch::from_pixmap(&pix,(2,3)).unwrap();
        proj.layers[0].keyframes.get_mut(&1).unwrap().strokes.push(patch.clone());
        let bytes=atomic_save::serialize_and_compress_project(&proj).unwrap();
        let (loaded,_)=decode_and_migrate(sniff_project_bytes(&bytes).unwrap()).unwrap();
        assert_eq!(loaded.layers[0].keyframes[&1].strokes[0],patch);
        let rendered=crate::exporter::render_frame_pixmap(&loaded,1,true,[0;4]);
        let i=(3*8+2)*4;
        assert_eq!(&rendered.data()[i..i+4],&[200,50,30,255]);
    }

    #[test]
    fn cmp_004_and_008_atomic_save_and_reload_roundtrip() {
        let tmp_dir = std::env::temp_dir().join(format!("stick_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp_dir);
        let file_path = tmp_dir.join("مشروع_اختبار.stick");

        let mut proj = AniProject::default();
        proj.layers[0].name = "رسم_عربي".to_string();
        proj.layers[0].keyframes.get_mut(&1).unwrap().strokes.push(Stroke {
            points: vec![
                StrokePoint { x: 12.0, y: 34.0, pressure: 1.0 },
                StrokePoint { x: 56.0, y: 78.0, pressure: 1.0 },
            ],
            color: [10, 20, 30, 255],
            fill_color: None,
            base_width: 4.0,
            is_vector: false,
            brush_preset: KritaBrushPreset::GPenInk,
            opacity: 1.0,
            brush_texture_id: None,
            tip_shape: crate::project::BrushTipShape::Round,
        });

        save_project_safe(&proj, &file_path).expect("Atomic save should succeed");
        let loaded = open_project_safe(&file_path).expect("Reload should succeed");
        assert_eq!(loaded.project.layers[0].name, "رسم_عربي");
        assert_eq!(loaded.project.layers[0].keyframes[&1].strokes.len(), 1);
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
