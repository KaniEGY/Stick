use crate::persistence::sniff::sniff_project_bytes;
use crate::project::{AniProject, PROJECT_SCHEMA_VERSION};
use serde::Serialize;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct EnvelopeWrite<'a> {
    magic: &'static str,
    schema_version: u32,
    project: &'a AniProject,
}

#[derive(Debug, Clone)]
pub struct RecoveryEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub timestamp_secs: u64,
    pub size_bytes: u64,
}

pub fn get_recovery_dir() -> PathBuf {
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app).join("Stick").join("Recovery");
    }
    std::env::temp_dir().join("Stick_Recovery")
}

pub fn serialize_and_compress_project(project: &AniProject) -> Result<Vec<u8>, String> {
    super::validate_project_bounds(project)?;

    let mut clean_proj = project.clone();
    clean_proj.schema_version = PROJECT_SCHEMA_VERSION;

    let env = EnvelopeWrite {
        magic: "STICK_STUDIO_PROJECT",
        schema_version: PROJECT_SCHEMA_VERSION,
        project: &clean_proj,
    };

    let json = serde_json::to_vec(&env)
        .map_err(|e| format!("فشل تحويل بيانات المشروع إلى JSON: {}", e))?;
    let compressed = zstd::encode_all(&json[..], 3)
        .map_err(|e| format!("فشل ضغط ملف المشروع: {}", e))?;

    // التحقق الذاتي قبل لمس القرص: تأكد أن البيانات المضغوطة قابلة للقراءة
    sniff_project_bytes(&compressed)
        .map_err(|e| format!("فشل التحقق الداخلي من سلامة البيانات قبل الحفظ: {}", e))?;

    Ok(compressed)
}

/// الحفظ الذري الآمن (Atomic Save):
/// 1. يكتب إلى ملف مؤقت .tmp في نفس مجلد الهدف
/// 2. يفرغ البيانات فعلياً إلى القرص (flush + sync_all)
/// 3. يتحقق من سلامة الملف المؤقت بقراءته وفحصه
/// 4. ينشئ نسخة احتياطية .bak من الملف القديم إن وجد
/// 5. يستبدل الملف الأصلي بأمان دون تدميره عند حدوث خطأ
pub fn save_project_atomic(project: &AniProject, target_path: &Path) -> Result<(), String> {
    let compressed = serialize_and_compress_project(project)?;

    let parent = target_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|e| format!("تعذر الوصول إلى مجلد الحفظ '{}': {}", parent.display(), e))?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file_stem = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project.stick");
    let tmp_path = parent.join(format!(".{}.{}.{}.tmp", file_stem, std::process::id(), nonce));

    // كتابة إلى الملف المؤقت مع sync_all
    {
        let mut tmp_file = File::options().write(true).create_new(true).open(&tmp_path)
            .map_err(|e| format!("فشل إنشاء الملف المؤقت للحفظ (تحقق من المساحة والصلاحيات): {}", e))?;
        if let Err(e) = tmp_file.write_all(&compressed) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("انقطعت الكتابة أثناء الحفظ (تم الحفاظ على ملفك الأصلي سليماً): {}", e));
        }
        if let Err(e) = tmp_file.flush() {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("فشل تفريغ البيانات إلى القرص: {}", e));
        }
        if let Err(e) = tmp_file.sync_all() {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("فشل مزامنة الملف المؤقت مع القرص: {}", e));
        }
    }

    // قراءة الملف المؤقت من القرص للتحقق من عدم تلفه أثناء الكتابة
    let written_bytes = fs::read(&tmp_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("فشل التحقق من الملف المؤقت بعد كتابته: {}", e)
    })?;
    if written_bytes != compressed {
        let _ = fs::remove_file(&tmp_path);
        return Err("تم إلغاء الاستبدال: الملف المؤقت المكتوب على القرص غير مكتمل.".to_string());
    }

    // إذا كان الملف الأصلي موجوداً، أنشئ نسخة احتياطية .bak قبل الاستبدال
    let bak_path = target_path.with_extension("stick.bak");
    if target_path.exists() {
        // لا نتابع الاستبدال إن فشل النسخ الاحتياطي: الكتابة لا تستحق المخاطرة
        // بضياع النسخة الموجودة لدى المستخدم.
        if let Err(e) = fs::copy(target_path, &bak_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("تم إلغاء الحفظ لأن إنشاء النسخة الاحتياطية فشل: {}", e));
        }
    }

    // الاستبدال الآمن المتوافق مع Windows
    if let Err(rename_err) = fs::rename(&tmp_path, target_path) {
        if target_path.exists() {
            let swap_old = parent.join(format!(".{}.{}.{}.old", file_stem, std::process::id(), nonce));
            if fs::rename(target_path, &swap_old).is_ok() {
                match fs::rename(&tmp_path, target_path) {
                    Ok(_) => {
                        let _ = fs::remove_file(&swap_old);
                        return Ok(());
                    }
                    Err(e2) => {
                        // استرجاع الملف الأصلي فوراً في حال فشل التبديل
                        let restored = fs::rename(&swap_old, target_path);
                        let _ = fs::remove_file(&tmp_path);
                        return match restored {
                            Ok(()) => Err(format!("فشل استبدال الملف الأصلي (تمت استعادة النسخة الأصلية): {e2}")),
                            Err(restore_err) => Err(format!("فشل الاستبدال والاستعادة: {e2}; {restore_err}. النسخة الأصلية محفوظة في {} والاحتياطية في {}.", swap_old.display(), bak_path.display())),
                        };
                    }
                }
            }
        }
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("تعذر حفظ الملف (ربما الملف مفتوح في برنامج آخر): {}", rename_err));
    }

    Ok(())
}

/// إنشاء نسخة احتياطية صريحة قبل ترقية ملف قديم حتى لا يُمس الأصل أبداً
pub fn create_legacy_backup_copy(original_path: &Path) -> Result<PathBuf, String> {
    let parent = original_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let ext = original_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("stick");
    let backup_path = parent.join(format!("{}.legacy-backup.{}", stem, ext));
    if !backup_path.exists() {
        fs::copy(original_path, &backup_path)
            .map_err(|e| format!("فشل إنشاء نسخة احتياطية من الملف القديم: {}", e))?;
    }
    Ok(backup_path)
}

/// حفظ لقطة استرجاع تلقائية (Auto-Save / Recovery) في مجلد مستقل مع سياسة احتفاظ (آخر 10 نسخ)
pub fn write_recovery_snapshot(project: &AniProject, project_label: &str) -> Result<PathBuf, String> {
    let dir = get_recovery_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let safe_label: String = project_label
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .take(32)
        .collect();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let snap_path = dir.join(format!("autosave_{}_{}.stick", safe_label, ts));
    save_project_atomic(project, &snap_path)?;

    // الاحتفاظ بآخر 10 نسخ استرجاع فقط لتوفير المساحة
    let mut entries = list_recovery_snapshots();
    if entries.len() > 10 {
        for old in entries.drain(10..) {
            let _ = fs::remove_file(old.path);
        }
    }

    Ok(snap_path)
}

pub fn list_recovery_snapshots() -> Vec<RecoveryEntry> {
    let dir = get_recovery_dir();
    let mut list = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("stick") {
                if let Ok(meta) = entry.metadata() {
                    let ts = meta
                        .modified()
                        .ok()
                        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    list.push(RecoveryEntry {
                        file_name: entry.file_name().to_string_lossy().to_string(),
                        path: p,
                        timestamp_secs: ts,
                        size_bytes: meta.len(),
                    });
                }
            }
        }
    }
    list.sort_by(|a, b| b.timestamp_secs.cmp(&a.timestamp_secs));
    list
}
