//! Persistent imported raster brush library for Stick.
//!
//! Imported vendor brushes are normalized into Stick's raster stamp model.
//! The original file is kept beside the normalized tip(s) so future versions
//! can improve the mapping without making users re-import their libraries.

use crate::brush_engine::{visit_pressure_dabs_spaced_dense, visit_pressure_dabs_spaced_dense_bounded};
use crate::project::Stroke;
use base64::Engine;
use eframe::egui;
use flate2::read::ZlibDecoder;
use image::{DynamicImage, GrayImage, ImageFormat, RgbaImage};
use rusqlite::{types::Value, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform};

const MAX_TIPS: usize = 32;
const MAX_IMPORT_BRUSHES: usize = 128;
const MAX_TIP_SIDE: u32 = 512;
const CUSTOM_BRUSH_ENGINE_VERSION: u32 = 6;

fn default_tip_scale() -> f32 { 1.0 }
fn default_brush_size() -> f32 { 30.0 }
fn default_engine_version() -> u32 { 0 }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomBrushRenderMode {
    #[default]
    Stamp,
    Continuous,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CustomBrushMeta {
    pub id: String,
    pub name: String,
    pub source_format: String,
    pub source_path: String,
    pub icon_path: String,
    pub tip_paths: Vec<String>,
    #[serde(default = "default_brush_size")]
    pub default_size: f32,
    pub spacing: f32,
    pub opacity: f32,
    pub flow: f32,
    pub size_pressure: f32,
    pub opacity_pressure: f32,
    pub angle_deg: f32,
    pub follow_stroke: bool,
    pub scatter: f32,
    pub random_tip: bool,
    #[serde(default = "default_tip_scale")]
    pub tip_scale: f32,
    #[serde(default)]
    pub render_mode: CustomBrushRenderMode,
    #[serde(default = "default_engine_version")]
    pub engine_version: u32,
    #[serde(default)]
    pub disabled_reason: String,
    #[serde(default)]
    pub compatibility_note: String,
}

#[derive(Clone, Debug)]
pub struct ImportedBrushDraft {
    pub suggested_name: String,
    pub source_format: String,
    pub source_path: PathBuf,
    pub tips: Vec<TipMask>,
    pub icon: Option<IconImage>,
    pub default_size: f32,
    pub spacing: f32,
    pub opacity: f32,
    pub flow: f32,
    pub size_pressure: f32,
    pub opacity_pressure: f32,
    pub angle_deg: f32,
    pub follow_stroke: bool,
    pub scatter: f32,
    pub random_tip: bool,
    pub tip_scale: f32,
    pub render_mode: CustomBrushRenderMode,
    pub auxiliary_resource_path: Option<PathBuf>,
    pub compatibility_note: String,
}

#[derive(Clone, Debug)]
pub struct TipMask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone)]
struct RuntimeBrush {
    meta: CustomBrushMeta,
    tips: Vec<TipMask>,
    icon: Option<IconImage>,
}

static RUNTIME: OnceLock<RwLock<HashMap<String, RuntimeBrush>>> = OnceLock::new();

fn runtime() -> &'static RwLock<HashMap<String, RuntimeBrush>> {
    RUNTIME.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn accepted_extensions() -> &'static [&'static str] {
    &[
        "kpp", "bundle", "myb", "gbr", "gih", "vbr", "abr",
        "sut", "sutg", "blend", "brush", "brushset", "zip",
        "afbrushes", "brushpack",
        "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "tga", "gif",
        "pbm", "pgm", "ppm", "pnm", "pam",
    ]
}

pub fn brush_root() -> PathBuf {
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base).join("StickStudio").join("brushes");
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("brushes");
        }
    }
    PathBuf::from("brushes")
}

fn library_path() -> PathBuf {
    brush_root().join("custom_brushes.json")
}

pub fn load_saved_library() -> Vec<CustomBrushMeta> {
    let path = library_path();
    let mut list = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<CustomBrushMeta>>(&s).ok())
        .unwrap_or_default();

    let mut changed=false;
    for meta in &mut list {
        if meta.engine_version >= CUSTOM_BRUSH_ENGINE_VERSION { continue; }
        match rebuild_saved_brush(meta) {
            Ok(true) => changed=true,
            Ok(false) => {
                // Multi-brush containers that cannot be matched safely are left
                // untouched instead of guessing the wrong entry.
            }
            Err(error) => {
                let ext=Path::new(&meta.source_path).extension()
                    .and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
                if ext=="kpp" {
                    // Old KPP imports could silently become a fake round brush.
                    // Disable that stale conversion until the user reimports the
                    // complete preset/resources instead of drawing incorrectly.
                    meta.disabled_reason=format!("Needs re-import: {error}");
                    meta.engine_version=CUSTOM_BRUSH_ENGINE_VERSION;
                    changed=true;
                }
            }
        }
    }
    if changed {
        let _=write_library_file(&list);
    }
    refresh_runtime(&list);
    list
}

fn write_library_file(list:&[CustomBrushMeta])->Result<(),String>{
    let root=brush_root();
    fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    let data=serde_json::to_string_pretty(list).map_err(|e|e.to_string())?;
    let dst=library_path();
    let tmp=dst.with_extension("json.tmp");
    fs::write(&tmp,data).map_err(|e|e.to_string())?;
    if dst.exists(){let _=fs::remove_file(&dst);}
    fs::rename(tmp,dst).map_err(|e|e.to_string())
}

fn rebuild_saved_brush(meta:&mut CustomBrushMeta)->Result<bool,String>{
    let source=PathBuf::from(&meta.source_path);
    if !source.is_file(){return Err("stored source file is missing".into())}
    let drafts=import_file(&source)?;
    let selected=if drafts.len()==1 {
        drafts.into_iter().next()
    } else {
        drafts.into_iter().find(|d|d.suggested_name.eq_ignore_ascii_case(&meta.name))
    };
    let Some(draft)=selected else {return Ok(false)};

    let dir=brush_root().join(&meta.id);
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    for entry in fs::read_dir(&dir).map_err(|e|e.to_string())? {
        let Ok(entry)=entry else {continue};
        let name=entry.file_name().to_string_lossy().to_string();
        if name.starts_with("tip_") && name.ends_with(".png") {
            let _=fs::remove_file(entry.path());
        }
    }
    let mut tip_paths=Vec::new();
    for (i,tip) in draft.tips.iter().take(MAX_TIPS).enumerate() {
        let path=dir.join(format!("tip_{i}.png"));
        save_tip_png(tip,&path)?;
        tip_paths.push(path.to_string_lossy().to_string());
    }
    if tip_paths.is_empty(){return Err("import produced no usable brush tips".into())}

    meta.tip_paths=tip_paths;
    meta.source_format=draft.source_format;
    meta.default_size=draft.default_size.clamp(1.0,2000.0);
    meta.spacing=draft.spacing.clamp(0.005,2.0);
    meta.opacity=draft.opacity.clamp(0.05,1.0);
    meta.flow=draft.flow.clamp(0.02,1.0);
    meta.size_pressure=draft.size_pressure.clamp(0.0,1.0);
    meta.opacity_pressure=draft.opacity_pressure.clamp(0.0,1.0);
    meta.angle_deg=draft.angle_deg;
    meta.follow_stroke=draft.follow_stroke;
    meta.scatter=draft.scatter.clamp(0.0,3.0);
    meta.random_tip=draft.random_tip;
    meta.tip_scale=draft.tip_scale.clamp(0.05,8.0);
    meta.render_mode=draft.render_mode;
    meta.compatibility_note=draft.compatibility_note;
    meta.disabled_reason.clear();
    meta.engine_version=CUSTOM_BRUSH_ENGINE_VERSION;
    Ok(true)
}

pub fn save_library(list: &[CustomBrushMeta]) -> Result<(), String> {
    write_library_file(list)?;
    refresh_runtime(list);
    Ok(())
}

pub fn delete_brush(id: &str) {
    let _ = fs::remove_dir_all(brush_root().join(id));
    if let Ok(mut map) = runtime().write() {
        map.remove(id);
    }
}

pub fn install_draft(
    draft: &ImportedBrushDraft,
    display_name: &str,
    custom_icon: Option<&Path>,
) -> Result<CustomBrushMeta, String> {
    let clean_name = display_name.trim();
    if clean_name.is_empty() {
        return Err("Brush name cannot be empty".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    let mut h = Sha256::new();
    h.update(draft.source_path.to_string_lossy().as_bytes());
    h.update(clean_name.as_bytes());
    h.update(now.to_le_bytes());
    let id = format!("{:x}", h.finalize());
    let dir = brush_root().join(&id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ext = draft.source_path.extension().and_then(|s| s.to_str()).unwrap_or("brush");
    let source_dst = dir.join(format!("source.{ext}"));
    fs::copy(&draft.source_path, &source_dst).map_err(|e| e.to_string())?;
    if let Some(resource)=draft.auxiliary_resource_path.as_ref() {
        if let Some(name)=resource.file_name() {
            let resource_dst=dir.join(name);
            if resource_dst!=source_dst {
                fs::copy(resource,&resource_dst).map_err(|e|
                    format!("Couldn't save the required brush resource: {e}"))?;
            }
        }
    }

    let mut tip_paths = Vec::new();
    let tips = if draft.tips.is_empty() {
        vec![round_tip(96, 0.9)]
    } else {
        draft.tips.clone()
    };
    for (i, tip) in tips.iter().take(MAX_TIPS).enumerate() {
        let path = dir.join(format!("tip_{i}.png"));
        save_tip_png(tip, &path)?;
        tip_paths.push(path.to_string_lossy().to_string());
    }

    let icon_path = dir.join("icon.png");
    let icon = if let Some(path) = custom_icon {
        image::open(path).ok().map(icon_from_image)
    } else {
        draft.icon.clone()
    };
    if let Some(icon) = icon {
        save_icon_png(&icon, &icon_path)?;
    } else {
        save_tip_preview_png(&tips[0], &icon_path)?;
    }

    let meta = CustomBrushMeta {
        id,
        name: clean_name.to_owned(),
        source_format: draft.source_format.clone(),
        source_path: source_dst.to_string_lossy().to_string(),
        icon_path: icon_path.to_string_lossy().to_string(),
        tip_paths,
        default_size: draft.default_size.clamp(1.0,2000.0),
        spacing: draft.spacing.clamp(0.02, 2.0),
        opacity: draft.opacity.clamp(0.05, 1.0),
        flow: draft.flow.clamp(0.02, 1.0),
        size_pressure: draft.size_pressure.clamp(0.0, 1.0),
        opacity_pressure: draft.opacity_pressure.clamp(0.0, 1.0),
        angle_deg: draft.angle_deg,
        follow_stroke: draft.follow_stroke,
        scatter: draft.scatter.clamp(0.0, 3.0),
        random_tip: draft.random_tip,
        tip_scale: draft.tip_scale.clamp(0.05, 8.0),
        render_mode: draft.render_mode,
        engine_version: CUSTOM_BRUSH_ENGINE_VERSION,
        disabled_reason: String::new(),
        compatibility_note: draft.compatibility_note.clone(),
    };
    register_meta(&meta);
    Ok(meta)
}

fn register_meta(meta: &CustomBrushMeta) {
    let tips: Vec<_> = meta.tip_paths.iter().filter_map(|p| load_tip(Path::new(p))).collect();
    let icon = load_icon(Path::new(&meta.icon_path));
    if let Ok(mut map) = runtime().write() {
        map.insert(meta.id.clone(), RuntimeBrush { meta: meta.clone(), tips, icon });
    }
}

fn refresh_runtime(list: &[CustomBrushMeta]) {
    if let Ok(mut map) = runtime().write() {
        map.clear();
    }
    for meta in list {
        register_meta(meta);
    }
}

pub fn stroke_custom_id(stroke: &Stroke) -> Option<&str> {
    stroke.brush_texture_id.as_deref()?.strip_prefix("custom:")
}

pub fn draw_icon(painter: &egui::Painter, rect: egui::Rect, id: &str) -> bool {
    let Ok(map) = runtime().read() else { return false };
    let Some(brush) = map.get(id) else { return false };
    let Some(icon) = brush.icon.as_ref() else { return false };
    let ctx = painter.ctx();
    let key = egui::Id::new(("stick_custom_brush_icon", id.to_owned()));
    let cached = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key));
    let tex = cached.unwrap_or_else(|| {
        let pixels = icon.rgba.chunks_exact(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0],p[1],p[2],p[3]))
            .collect();
        let handle = ctx.load_texture(
            format!("custom brush icon {id}"),
            egui::ColorImage { size:[icon.width as usize,icon.height as usize], pixels },
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(key, handle.clone()));
        handle
    });
    painter.image(tex.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0,1.0)), egui::Color32::WHITE);
    true
}

pub fn preview(
    painter: &egui::Painter,
    stroke: &Stroke,
    rgba: [u8;4],
    to_screen: impl Fn(f32,f32)->egui::Pos2,
) -> bool {
    let Some(id)=stroke_custom_id(stroke) else { return false };
    let Ok(map)=runtime().read() else { return false };
    let Some(brush)=map.get(id) else { return false };
    if !brush.meta.disabled_reason.is_empty() { return true; }
    if brush.tips.is_empty() { return true; }

    let ctx=painter.ctx();
    let mut meshes=Vec::with_capacity(brush.tips.len());
    for (i,tip) in brush.tips.iter().enumerate() {
        // Match built-in texture brushes: cache the actual selected RGB in the
        // alpha mask, rather than multiplying a white texture by a colored
        // translucent vertex in the preview shader.
        let key=egui::Id::new(("stick_custom_brush_tip_rgb",id.to_owned(),i,rgba[0],rgba[1],rgba[2]));
        let cached=ctx.data(|d|d.get_temp::<egui::TextureHandle>(key));
        let tex=cached.unwrap_or_else(||{
            let pixels=tip.data.iter()
                .map(|&a|egui::Color32::from_rgba_unmultiplied(rgba[0],rgba[1],rgba[2],a))
                .collect();
            let handle=ctx.load_texture(
                format!("custom brush tip {id}:{i}"),
                egui::ColorImage{size:[tip.width as usize,tip.height as usize],pixels},
                egui::TextureOptions::LINEAR,
            );
            ctx.data_mut(|d|d.insert_temp(key,handle.clone()));
            handle
        });
        meshes.push(egui::Mesh::with_texture(tex.id()));
    }

    emit_stamps(stroke,brush,Some(8_000),|stamp|{
        let mesh=&mut meshes[stamp.tip_index];
        let start=mesh.vertices.len() as u32;
        let alpha=(rgba[3] as f32*stamp.flow).round().clamp(0.0,255.0) as u8;
        let color=egui::Color32::from_white_alpha(alpha);
        for (u,v) in [(0.,0.),(1.,0.),(1.,1.),(0.,1.)] {
            let [x,y]=stamp.corner(u,v);
            mesh.vertices.push(egui::epaint::Vertex{
                pos:to_screen(x,y),uv:egui::pos2(u,v),color,
            });
        }
        mesh.indices.extend([start,start+1,start+2,start,start+2,start+3]);
    });
    drop(map);
    for mesh in meshes {
        if !mesh.indices.is_empty() {
            painter.add(egui::Shape::mesh(mesh));
        }
    }
    true
}

pub fn export(
    target:&mut Pixmap,
    stroke:&Stroke,
    alpha:u8,
    transform:Transform,
)->bool {
    let Some(id)=stroke_custom_id(stroke) else {return false};
    let Ok(map)=runtime().read() else {return false};
    let Some(brush)=map.get(id) else {return false};
    if !brush.meta.disabled_reason.is_empty(){return true;}
    if brush.tips.is_empty(){return true;}

    let mut pixmaps=Vec::with_capacity(brush.tips.len());
    for tip in &brush.tips {
        let mut pm=Pixmap::new(tip.width,tip.height).unwrap();
        for (px,&a) in pm.data_mut().chunks_exact_mut(4).zip(&tip.data) {
            px[0]=((stroke.color[0] as u16*a as u16+127)/255) as u8;
            px[1]=((stroke.color[1] as u16*a as u16+127)/255) as u8;
            px[2]=((stroke.color[2] as u16*a as u16+127)/255) as u8;
            px[3]=a;
        }
        pixmaps.push(pm);
    }
    emit_stamps(stroke,brush,None,|stamp|{
        let tip=&pixmaps[stamp.tip_index];
        let origin=stamp.corner(0.,0.);
        let kx=2.0/tip.width() as f32;
        let ky=2.0/tip.height() as f32;
        let position=Transform::from_row(
            stamp.along[0]*kx,stamp.along[1]*kx,
            stamp.across[0]*ky,stamp.across[1]*ky,
            origin[0],origin[1],
        ).post_concat(transform);
        let paint=PixmapPaint{
            opacity:(alpha as f32*stamp.flow).round()/255.0,
            quality:FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        target.draw_pixmap(0,0,tip.as_ref(),&paint,position,None);
    });
    true
}

#[derive(Clone,Copy)]
struct Stamp {
    center:[f32;2],
    along:[f32;2],
    across:[f32;2],
    flow:f32,
    tip_index:usize,
}
impl Stamp {
    fn corner(self,u:f32,v:f32)->[f32;2]{
        let (u,v)=(2.0*u-1.0,2.0*v-1.0);
        [
            self.center[0]+self.along[0]*u+self.across[0]*v,
            self.center[1]+self.along[1]*u+self.across[1]*v,
        ]
    }
}

fn emit_stamps(stroke:&Stroke,brush:&RuntimeBrush,live_budget:Option<usize>,mut emit:impl FnMut(Stamp)){
    // Vendor spacing is stored as a fraction of the brush DIAMETER while the
    // shared sampler expects a fraction of RADIUS. Preserve that distinction:
    // Krita spacing=0.1 must become 0.2 radius, not 2.0.
    let spacing=match brush.meta.render_mode {
        // Imported continuous engines need overlap, but allowing vendor
        // spacing to approach zero creates thousands of redundant dabs and
        // makes the pointer feel heavy. Preserve the vendor's real spacing
        // whenever possible; only clamp pathological values for interactivity.
        CustomBrushRenderMode::Continuous => brush.meta.spacing.clamp(0.04,0.35),
        CustomBrushRenderMode::Stamp => brush.meta.spacing,
    };
    let ratio=(spacing*2.0*brush.meta.tip_scale).clamp(0.01,2.0);
    let mut ordinal=0u32;
    let mut paint_one=|dab:crate::brush_engine::BrushDab,[dx,dy]:[f32;2]|{
        let p=(2.0*dab.radius/stroke.base_width.max(0.001)).clamp(0.0,1.5);
        let size_factor=(1.0-brush.meta.size_pressure)+brush.meta.size_pressure*p;
        let r=(stroke.base_width*0.5*size_factor*brush.meta.tip_scale).max(0.25);
        let flow=brush.meta.flow*((1.0-brush.meta.opacity_pressure)+brush.meta.opacity_pressure*p)
            .clamp(0.02,1.5);
        let jitter=unit_noise(ordinal.wrapping_add(hash_id(&brush.meta.id)));
        let mut cx=dab.x;
        let mut cy=dab.y;
        if brush.meta.scatter>0.0 {
            let offset=(jitter-0.5)*2.0*brush.meta.scatter*stroke.base_width;
            cx+=-dy*offset;
            cy+=dx*offset;
        }
        let base_angle=brush.meta.angle_deg.to_radians();
        let angle=if brush.meta.follow_stroke {dy.atan2(dx)+base_angle}else{base_angle};
        let (sin,cos)=angle.sin_cos();
        let idx=if brush.meta.random_tip {
            (unit_noise(ordinal.wrapping_mul(1103515245).wrapping_add(12345))*brush.tips.len() as f32)
                .floor() as usize % brush.tips.len()
        } else {
            ordinal as usize%brush.tips.len()
        };
        let tip=&brush.tips[idx];
        let max_side=tip.width.max(tip.height).max(1) as f32;
        let hw=r*tip.width as f32/max_side;
        let hh=r*tip.height as f32/max_side;
        emit(Stamp{
            center:[cx,cy],
            along:[cos*hw,sin*hw],
            across:[-sin*hh,cos*hh],
            flow,
            tip_index:idx,
        });
        ordinal=ordinal.wrapping_add(1);
    };
    if let Some(max_dabs)=live_budget {
        visit_pressure_dabs_spaced_dense_bounded(
            &stroke.points,stroke.base_width,ratio,max_dabs,&mut paint_one);
    } else {
        visit_pressure_dabs_spaced_dense(
            &stroke.points,stroke.base_width,ratio,&mut paint_one);
    }
}

fn hash_id(id:&str)->u32 {
    id.bytes().fold(2166136261u32,|h,b|(h^b as u32).wrapping_mul(16777619))
}
fn unit_noise(mut x:u32)->f32{
    x^=x>>16;x=x.wrapping_mul(0x7feb352d);x^=x>>15;x=x.wrapping_mul(0x846ca68b);
    ((x^(x>>16))>>8) as f32/16_777_216.0
}

// -------------------------------------------------------------------------------------------------
// Importers
// -------------------------------------------------------------------------------------------------

// Bound compressed input and aggregate expanded archive data before allocating.
const MAX_BRUSH_INPUT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BRUSH_MEMBER_BYTES: u64 = 128 * 1024 * 1024;

fn check_brush_input(path: &Path) -> Result<(), String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_BRUSH_INPUT_BYTES {
        return Err("Brush file exceeds the 512 MiB import limit.".into());
    }
    Ok(())
}

pub fn import_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    check_brush_input(path)?;
    let ext=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    let direct=match ext.as_str() {
        ext if tip_image_format(ext).is_some()=>return import_image(path),
        "gbr"=>import_gbr_file(path),
        "gih"=>import_gih_file(path),
        "vbr"=>import_vbr_file(path),
        "abr"=>import_abr_file(path),
        // A KPP's embedded PNG is its thumbnail. Preserve the resource
        // error so the importer can ask for the actual missing tip.
        "kpp"=>return import_kpp_file(path),
        "bundle"=>import_krita_bundle(path),
        "myb"=>import_myb_file(path),
        "sut"=>import_sut_file(path),
        "sutg"=>import_sutg_file(path),
        "blend"=>import_blend_file(path),
        "brush"|"brushset"|"zip"=>return import_zip_brush_pack(path),
        "afbrushes"|"brushpack"=>import_archive_or_embedded_image_pack(path,&ext),
        _=>import_file_by_magic(path,&ext),
    };
    // Known vendor parsers are preferable, but a damaged/older/proprietary
    // wrapper may still contain real embedded PNG brush tips. Recover those
    // instead of silently producing a fake round brush.
    direct.or_else(|primary|{
        import_embedded_png_pack(path,&format!("Recovered embedded tip from .{ext}"))
            .map_err(|_|primary)
    })
}

fn import_file_by_magic(path:&Path,ext:&str)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
        return import_zip_brush_pack(path);
    }
    if data.starts_with(b"SQLite format 3\0") {
        return import_sut_file(path);
    }
    if data.starts_with(b"BLENDER") {
        return import_blend_file(path);
    }
    if data.starts_with(b"GIMP-VBR") {
        return import_vbr_file(path);
    }
    if data.len()>=24 && &data[20..24]==b"GIMP" {
        return import_gbr_file(path);
    }
    if image::guess_format(&data).is_ok() {
        return import_image(path);
    }
    import_embedded_png_pack(path,&format!("Recovered embedded brush data from .{ext}"))
        .map_err(|_|format!(
            "Unsupported or unreadable brush file: .{ext}. Stick also checked ZIP, SQLite/CSP, Blender, GIMP and embedded PNG brush data."
        ))
}

fn import_archive_or_embedded_image_pack(path:&Path,label:&str)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
        import_zip_brush_pack(path)
    } else {
        import_embedded_png_pack(path,&format!("{label} embedded image brush"))
    }
}

fn import_embedded_png_pack(path:&Path,label:&str)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    let images=find_png_images(&data,MAX_TIPS);
    let stem=path.file_stem().and_then(|s|s.to_str()).unwrap_or("Recovered Brush");
    let mut out=Vec::new();
    for (index,png) in images.into_iter().enumerate() {
        let Ok(img)=image::load_from_memory(png) else {continue};
        if img.width()<8 || img.height()<8 {continue}
        let tip=tip_from_image(&img);
        out.push(ImportedBrushDraft{
            suggested_name:if index==0 {stem.to_owned()} else {format!("{stem} {}",index+1)},
            source_format:label.to_owned(),source_path:path.to_path_buf(),
            default_size:tip.width.max(tip.height).clamp(1,160) as f32,
            tips:vec![tip],icon:Some(icon_from_image(img)),
            spacing:0.10,opacity:1.0,flow:1.0,size_pressure:1.0,opacity_pressure:0.0,
            angle_deg:0.0,follow_stroke:true,scatter:0.0,random_tip:false,tip_scale:1.0,
            render_mode:CustomBrushRenderMode::Stamp,auxiliary_resource_path:None,
            compatibility_note:"Recovered from embedded PNG data. Proprietary dynamics are unavailable, but the real embedded brush tip is preserved.".into(),
        });
    }
    if out.is_empty(){Err("No readable embedded PNG brush tips were found".into())}else{Ok(out)}
}

fn import_image(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let img=image::ImageReader::open(path).and_then(|r|r.with_guessed_format())
        .map_err(|e|e.to_string())?.decode().map_err(|e|e.to_string())?;
    let tip=tip_from_image(&img);
    Ok(vec![ImportedBrushDraft{
        suggested_name:path.file_stem().and_then(|s|s.to_str()).unwrap_or("Image Brush").to_owned(),
        source_format:"Image brush tip".into(),source_path:path.to_path_buf(),
        default_size:tip.width.max(tip.height).clamp(1,120) as f32,
        tips:vec![tip],icon:Some(icon_from_image(img)),spacing:0.10,opacity:1.0,flow:1.0,
        size_pressure:1.0,opacity_pressure:0.0,angle_deg:0.0,follow_stroke:true,
        scatter:0.0,random_tip:false,tip_scale:1.0,
        render_mode:CustomBrushRenderMode::Stamp,
        auxiliary_resource_path:None,compatibility_note:String::new(),
    }])
}

fn tip_image_format(ext: &str) -> Option<ImageFormat> {
    match ext {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" | "tga" | "gif"
        | "pbm" | "pgm" | "ppm" | "pnm" | "pam" => ImageFormat::from_extension(ext),
        _ => None,
    }
}


fn parse_vbr_text(text:&str,path:&Path)->Result<ImportedBrushDraft,String>{
    let lines:Vec<&str>=text.lines().map(str::trim).filter(|l|!l.is_empty()).collect();
    if lines.first().copied()!=Some("GIMP-VBR"){return Err("VBR magic missing".into())}
    let version=*lines.get(1).ok_or("VBR version missing")?;
    let shaped=version=="1.5";
    if version!="1.0" && !shaped{return Err(format!("Unsupported VBR version {version}"))}
    let name=lines.get(2).copied().unwrap_or("GIMP Generated Brush").to_owned();
    let (shape,offset)=if shaped {
        (lines.get(3).copied().unwrap_or("circle").to_ascii_lowercase(),1usize)
    } else {("circle".to_owned(),0usize)};
    let spacing:f32=lines.get(3+offset).and_then(|v|v.parse::<f32>().ok()).unwrap_or(10.0_f32);
    let radius:f32=lines.get(4+offset).and_then(|v|v.parse::<f32>().ok()).unwrap_or(15.0_f32)
        .clamp(0.5_f32,2048.0_f32);
    let (spikes,hard_index)=if shaped {
        (lines.get(5+offset).and_then(|v|v.parse::<usize>().ok()).unwrap_or(2),6+offset)
    } else {(2usize,5usize)};
    let hardness:f32=lines.get(hard_index).and_then(|v|v.parse::<f32>().ok()).unwrap_or(0.9_f32)
        .clamp(0.0_f32,0.999_f32);
    let aspect:f32=lines.get(hard_index+1).and_then(|v|v.parse::<f32>().ok()).unwrap_or(1.0_f32);
    let angle:f32=lines.get(hard_index+2).and_then(|v|v.parse::<f32>().ok()).unwrap_or(0.0_f32);
    let ratio=if aspect.abs()<=1.0 {aspect.abs().max(0.05)}
        else {(1.0/(1.0+aspect.abs()/20.0)).clamp(0.05,1.0)};
    let tip=match shape.as_str() {
        "square"=>square_tip(128,hardness),
        "diamond"=>diamond_tip(128,hardness),
        _ if ratio<0.98=>elliptical_tip(128,hardness,ratio,angle),
        _=>round_tip(128,hardness),
    };
    Ok(ImportedBrushDraft{
        suggested_name:if name.trim().is_empty(){
            path.file_stem().and_then(|s|s.to_str()).unwrap_or("GIMP Generated Brush").to_owned()
        }else{name},
        source_format:"GIMP generated brush .vbr".into(),source_path:path.to_path_buf(),
        default_size:(radius*2.0).clamp(1.0,2000.0),tips:vec![tip],icon:None,
        spacing:(spacing/100.0).clamp(0.005,2.0),opacity:1.0,flow:1.0,
        size_pressure:1.0,opacity_pressure:0.0,angle_deg:angle,follow_stroke:false,
        scatter:0.0,random_tip:false,tip_scale:1.0,render_mode:CustomBrushRenderMode::Stamp,
        auxiliary_resource_path:None,
        compatibility_note:format!(
            "GIMP VBR {version}: shape {shape}, radius {:.1}px, hardness {:.0}%, aspect {:.2}, angle {:.1}°, spikes {}.",
            radius,hardness*100.0,aspect,angle,spikes),
    })
}

fn import_vbr_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let text=fs::read_to_string(path).map_err(|e|format!("VBR: {e}"))?;
    Ok(vec![parse_vbr_text(&text,path)?])
}

fn import_zip_brush_pack(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let file=fs::File::open(path).map_err(|e|e.to_string())?;
    let mut zip=zip::ZipArchive::new(file).map_err(|e|format!("Brush archive: {e}"))?;
    let outer_ext=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    let procreate=outer_ext=="brush"||outer_ext=="brushset";
    let mut out=Vec::new();
    for i in 0..zip.len() {
        if out.len()>=MAX_IMPORT_BRUSHES{break}
        let mut entry=zip.by_index(i).map_err(|e|e.to_string())?;
        if entry.is_dir(){continue}
        let entry_name=entry.name().replace('\\',"/");
        let lower=entry_name.to_ascii_lowercase();
        if lower.split('/').any(|s|s=="__macosx" || s.starts_with('.')) { continue; }
        let member=Path::new(&entry_name);
        let ext=member.extension().and_then(|s|s.to_str())
            .unwrap_or("").to_ascii_lowercase();
        let format=tip_image_format(&ext);
        let is_shape = member.file_stem().and_then(|s|s.to_str())
            .is_some_and(|s|s.eq_ignore_ascii_case("shape")) || lower.contains("/shape/") || lower.contains("brushshape");
        if procreate && !(format.is_some() && is_shape) { continue; }
        if format.is_none() && !matches!(ext.as_str(),"gbr"|"gih"|"vbr"|"abr"|"myb") { continue; }
        const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;
        if entry.size() > MAX_MEMBER_BYTES { continue; }
        let mut data=Vec::new();
        entry.by_ref().take(MAX_MEMBER_BYTES + 1).read_to_end(&mut data).map_err(|e|e.to_string())?;
        if data.len() as u64 > MAX_MEMBER_BYTES { continue; }
        let display_name=if procreate {
            member.parent().and_then(Path::file_name).or_else(||path.file_stem())
        } else {member.file_stem()}.and_then(|s|s.to_str()).unwrap_or("Brush").to_owned();

        if ext=="abr" || ext=="myb" {
            let drafts = if ext=="abr" {
                parse_abr_bytes(&data, member)
            } else {
                parse_myb_bytes(&data, member)
            };
            if let Ok(drafts) = drafts {
                for mut draft in drafts.into_iter().take(MAX_IMPORT_BRUSHES-out.len()) {
                    draft.source_path=path.to_path_buf();
                    out.push(draft);
                }
            }
            continue;
        }

        if ext=="gbr" {
            if let Ok((name,tip,spacing,_))=parse_gbr(&data,0) {
                out.push(ImportedBrushDraft{
                    suggested_name:if name.is_empty(){display_name.clone()}else{name},
                    source_format:"Brush pack / GIMP .gbr".into(),source_path:path.to_path_buf(),
                    default_size:tip.width.max(tip.height).clamp(1,120) as f32,tips:vec![tip],
                    icon:None,spacing:(spacing as f32/100.0).clamp(0.005,2.0),
                    opacity:1.0,flow:1.0,size_pressure:1.0,opacity_pressure:0.0,
                    angle_deg:0.0,follow_stroke:false,scatter:0.0,random_tip:false,tip_scale:1.0,
                    render_mode:CustomBrushRenderMode::Stamp,auxiliary_resource_path:None,
                    compatibility_note:"Imported from a brush-pack archive.".into(),
                });
            }
            continue;
        }
        if ext=="gih" {
            if let Ok(mut drafts)=parse_gih_bytes(&data,Path::new(&entry_name)) {
                for draft in &mut drafts {
                    draft.source_path=path.to_path_buf();
                    draft.source_format="Brush pack / GIMP .gih".into();
                }
                out.extend(drafts.into_iter().take(MAX_IMPORT_BRUSHES-out.len()));
            }
            continue;
        }
        if ext=="vbr" {
            if let Ok(text)=std::str::from_utf8(&data) {
                if let Ok(mut draft)=parse_vbr_text(text,Path::new(&entry_name)) {
                    draft.source_path=path.to_path_buf();
                    draft.source_format="Brush pack / GIMP .vbr".into();
                    out.push(draft);
                }
            }
            continue;
        }

        if let Some(format)=format {
            if let Ok(img)=image::load_from_memory_with_format(&data,format) {
                let tip=if procreate {
                    let mask=img.to_luma_alpha8();
                    downsample_tip(TipMask { width:mask.width(), height:mask.height(),
                        data:mask.pixels().map(|p|((u16::from(p[0])*u16::from(p[1])+127)/255) as u8).collect() })
                } else {tip_from_image(&img)};
                out.push(ImportedBrushDraft{
                    suggested_name:display_name,
                    source_format:if procreate {
                        "Procreate .brush/.brushset shape tip".into()
                    } else {"ZIP image brush tip".into()},
                    source_path:path.to_path_buf(),
                    default_size:tip.width.max(tip.height).clamp(1,120) as f32,
                    tips:vec![tip],icon:Some(icon_from_image(img)),spacing:0.10,
                    opacity:1.0,flow:1.0,size_pressure:1.0,opacity_pressure:0.0,
                    angle_deg:0.0,follow_stroke:true,scatter:0.0,random_tip:false,tip_scale:1.0,
                    render_mode:CustomBrushRenderMode::Stamp,auxiliary_resource_path:None,
                    compatibility_note:if procreate {
                        "Procreate shape texture imported; proprietary dynamics are mapped to Stick defaults.".into()
                    } else {"Image tip imported from ZIP brush pack.".into()},
                });
            }
        }
    }
    if out.is_empty() {
        Err("The brush archive contained no readable brush presets or image tips.".into())
    } else {Ok(out)}
}

fn import_abr_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    parse_abr_bytes(&bytes,path)
}

fn parse_abr_bytes(bytes:&[u8],path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    use brushkit_preview::{preview_abr,PreviewOptions,TipPreview};
    let set=preview_abr(bytes,PreviewOptions{max_cell:MAX_TIP_SIDE})
        .map_err(|e|format!("ABR: {e}"))?;
    let mut out=Vec::new();
    for entry in set.entries.into_iter().take(MAX_IMPORT_BRUSHES) {
        if let TipPreview::Available(bitmap)=entry.tip {
            out.push(ImportedBrushDraft{
                suggested_name:if entry.name.trim().is_empty(){format!("ABR Brush {}",entry.index+1)}else{entry.name},
                source_format:"Photoshop / Krita / Clip Studio .abr".into(),
                source_path:path.to_path_buf(),
                default_size:bitmap.width.max(bitmap.height).clamp(1,120) as f32,
                tips:vec![TipMask{width:bitmap.width,height:bitmap.height,data:bitmap.data}],
                icon:None,spacing:0.10,opacity:1.0,flow:1.0,size_pressure:1.0,opacity_pressure:0.0,
                angle_deg:0.0,follow_stroke:false,scatter:0.0,random_tip:false,tip_scale:1.0,
                render_mode:CustomBrushRenderMode::Stamp,
                auxiliary_resource_path:None,
                compatibility_note:"ABR tip shape is preserved; vendor-only brush dynamics are mapped to Stick defaults.".into(),
            });
        }
    }
    if out.is_empty(){return Err("ABR contains no readable brush tips".into())}
    Ok(out)
}

fn import_gbr_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    let (name,tip,spacing,_)=parse_gbr(&data,0)?;
    Ok(vec![ImportedBrushDraft{
        suggested_name:if name.is_empty(){path.file_stem().and_then(|s|s.to_str()).unwrap_or("GBR Brush").to_owned()}else{name},
        source_format:"Krita / GIMP .gbr".into(),source_path:path.to_path_buf(),
        default_size:tip.width.max(tip.height).clamp(1,120) as f32,
        tips:vec![tip],icon:None,spacing:(spacing as f32/100.0).clamp(0.02,2.0),
        opacity:1.0,flow:1.0,size_pressure:1.0,opacity_pressure:0.0,angle_deg:0.0,
        follow_stroke:false,scatter:0.0,random_tip:false,tip_scale:1.0,
        render_mode:CustomBrushRenderMode::Stamp,
        auxiliary_resource_path:None,compatibility_note:String::new(),
    }])
}

fn import_gih_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    parse_gih_bytes(&data,path)
}

fn parse_gih_bytes(data:&[u8],path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let nl1=data.iter().position(|&b|b==b'\n').ok_or("GIH header missing")?;
    let nl2=data[nl1+1..].iter().position(|&b|b==b'\n').map(|i|i+nl1+1).ok_or("GIH header missing")?;
    let name=String::from_utf8_lossy(&data[..nl1]).trim().to_owned();
    let line=String::from_utf8_lossy(&data[nl1+1..nl2]);
    let count=line.split_whitespace().next().and_then(|v|v.parse::<usize>().ok()).ok_or("GIH cell count missing")?;
    let random=line.contains("sel0:random")||line.contains("sel0:pressure")||line.contains("sel0:velocity");
    let mut pos=nl2+1;
    let mut tips=Vec::new();
    let mut spacing=10u32;
    for _ in 0..count.min(MAX_TIPS) {
        let (_,tip,sp,next)=parse_gbr(&data[pos..],0)?;
        spacing=sp;
        tips.push(tip);
        pos=pos.saturating_add(next);
        if pos>=data.len(){break}
    }
    if tips.is_empty(){return Err("GIH contains no readable tips".into())}
    Ok(vec![ImportedBrushDraft{
        suggested_name:if name.is_empty(){path.file_stem().and_then(|s|s.to_str()).unwrap_or("GIH Brush").to_owned()}else{name},
        source_format:"Krita / GIMP .gih".into(),source_path:path.to_path_buf(),
        default_size:tips.first().map(|t|t.width.max(t.height).clamp(1,120) as f32).unwrap_or(30.0),
        tips,icon:None,
        spacing:(spacing as f32/100.0).clamp(0.02,2.0),opacity:1.0,flow:1.0,size_pressure:1.0,
        opacity_pressure:0.0,angle_deg:0.0,follow_stroke:false,scatter:0.0,random_tip:random,tip_scale:1.0,
        render_mode:CustomBrushRenderMode::Stamp,
        auxiliary_resource_path:None,
        compatibility_note:"Multi-tip GIH order is preserved as sequential/random in Stick.".into(),
    }])
}

fn parse_gbr(data:&[u8],offset:usize)->Result<(String,TipMask,u32,usize),String>{
    let d=data.get(offset..).ok_or("GBR offset out of range")?;
    if d.len()<20{return Err("GBR header truncated".into())}
    let be=|i:usize|u32::from_be_bytes(d[i..i+4].try_into().unwrap());
    let header=be(0) as usize;
    let version=be(4);
    let w=be(8);let h=be(12);let nbytes=be(16);
    if w==0||h==0||w>8192||h>8192{return Err("GBR dimensions are unsafe".into())}
    let (name_start,spacing)=if version==1 {
        (20,25)
    } else if version==2||version==3 {
        if d.len()<28||be(20)!=u32::from_be_bytes(*b"GIMP"){return Err("GBR magic missing".into())}
        (28,be(24))
    } else {return Err(format!("Unsupported GBR version {version}"))};
    if header<name_start||header>d.len(){return Err("GBR header size invalid".into())}
    let name=String::from_utf8_lossy(&d[name_start..header]).trim_end_matches('\0').to_string();
    let pixels=(w as usize).checked_mul(h as usize).ok_or("GBR too large")?;
    let need=match nbytes {1=>pixels,4=>pixels.checked_mul(4).ok_or("GBR too large")?,_=>return Err(format!("GBR depth {nbytes} is unsupported"))};
    if header+need>d.len(){return Err("GBR pixels truncated".into())}
    let raw=&d[header..header+need];
    let mask=if nbytes==1 {raw.to_vec()} else {raw.chunks_exact(4).map(|p|p[3]).collect()};
    Ok((name,downsample_tip(TipMask{width:w,height:h,data:mask}),spacing,header+need))
}

pub fn required_krita_resource(path:&Path)->Option<String>{
    if path.extension().and_then(|s|s.to_str())
        .is_none_or(|ext|!ext.eq_ignore_ascii_case("kpp")) { return None; }
    let data=fs::read(path).ok()?;
    let xml=png_text_chunks(&data).get("preset")?.clone();
    let doc=roxmltree::Document::parse(&xml).ok()?;
    for n in doc.descendants().filter(|n|n.has_tag_name("param")) {
        if n.attribute("name")==Some("requiredBrushFile") {
            let value=n.text().unwrap_or("").trim();
            if !value.is_empty(){return Some(value.to_owned())}
        }
    }
    let def=doc.descendants().find(|n|
        n.has_tag_name("param") && n.attribute("name")==Some("brush_definition"))
        .and_then(|n|n.text()).unwrap_or("");
    let cleaned=def.replace("<!DOCTYPE BrushTip>","");
    let bdoc=roxmltree::Document::parse(cleaned.trim()).ok()?;
    let brush=bdoc.descendants().find(|n|n.has_tag_name("Brush"))?;
    if brush.attribute("type").unwrap_or("auto_brush")=="auto_brush" {return None}
    brush.attribute("filename").map(str::trim)
        .filter(|s|!s.is_empty()).map(str::to_owned)
}

pub fn import_file_with_resource(
    path:&Path,
    resource:&Path,
)->Result<Vec<ImportedBrushDraft>,String>{
    check_brush_input(path)?;
    check_brush_input(resource)?;
    let required=required_krita_resource(path)
        .ok_or("This brush does not request an external Krita brush tip.")?;
    let selected_name=resource.file_name().and_then(|s|s.to_str()).unwrap_or("");
    let required_name=Path::new(&required).file_name().and_then(|s|s.to_str()).unwrap_or(&required);
    if !selected_name.eq_ignore_ascii_case(required_name) {
        return Err(format!(
            "This Krita preset needs '{required_name}', but you selected '{selected_name}'."
        ));
    }
    let raw_resource=fs::read(resource).map_err(|e|e.to_string())?;
    let data=fs::read(path).map_err(|e|e.to_string())?;
    let img=image::load_from_memory(&data).map_err(|e|format!("KPP preview PNG: {e}"))?;
    let xml=png_text_chunks(&data).get("preset").cloned().ok_or("KPP has no preset metadata")?;
    let mut draft=parse_krita_preset_xml(&xml,Some(icon_from_image(img)),path,|wanted|{
        let wanted_name=Path::new(wanted).file_name().and_then(|s|s.to_str()).unwrap_or(wanted);
        if wanted_name.eq_ignore_ascii_case(required_name) {
            Some(raw_resource.clone())
        } else {
            read_krita_resource_near(path,wanted)
        }
    })?;
    draft.auxiliary_resource_path=Some(resource.to_path_buf());
    Ok(vec![draft])
}

fn import_kpp_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let data=fs::read(path).map_err(|e|e.to_string())?;
    Ok(vec![parse_kpp_bytes(&data,path)?])
}

fn find_nearby_resource(root:&Path,file:&std::ffi::OsStr,max_depth:usize)->Option<Vec<u8>>{
    fn walk(dir:&Path,file:&std::ffi::OsStr,depth:usize,max_depth:usize,seen:&mut usize)->Option<Vec<u8>>{
        if depth>max_depth || *seen>4096 {return None}
        let entries=fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            *seen+=1;
            if *seen>4096 {return None}
            let path=entry.path();
            if path.is_file() && path.file_name().is_some_and(|name|
                name.to_string_lossy().eq_ignore_ascii_case(&file.to_string_lossy()))
            {
                if let Ok(bytes)=fs::read(&path){return Some(bytes)}
            }
            if path.is_dir() {
                if let Some(bytes)=walk(&path,file,depth+1,max_depth,seen){return Some(bytes)}
            }
        }
        None
    }
    let mut seen=0usize;
    walk(root,file,0,max_depth,&mut seen)
}

fn read_krita_resource_near(source_path:&Path,filename:&str)->Option<Vec<u8>>{
    let file=Path::new(filename).file_name()?.to_owned();
    let parent=source_path.parent().unwrap_or(Path::new("."));
    let mut candidates=vec![
        parent.join(&file),
        parent.join("brushes").join(&file),
        parent.join("resources").join(&file),
    ];
    if let Some(up)=parent.parent(){
        candidates.push(up.join("brushes").join(&file));
        candidates.push(up.join("resources").join("brushes").join(&file));
    }
    if let Some(appdata)=std::env::var_os("APPDATA"){
        candidates.push(PathBuf::from(&appdata).join("krita").join("brushes").join(&file));
        candidates.push(PathBuf::from(appdata).join("krita").join("resources").join("brushes").join(&file));
    }
    if let Some(local)=std::env::var_os("LOCALAPPDATA"){
        candidates.push(PathBuf::from(&local).join("krita").join("brushes").join(&file));
        candidates.push(PathBuf::from(local).join("krita").join("resources").join("brushes").join(&file));
    }
    if let Some(bytes)=candidates.into_iter().find_map(|p|fs::read(p).ok()){
        return Some(bytes);
    }

    // Extracted Krita packs often keep paintoppresets/ and brushes/ as siblings.
    // Search a tightly bounded nearby tree so users do not have to manually
    // locate every GBR/GIH dependency.
    if let Some(bytes)=find_nearby_resource(parent,&file,2){return Some(bytes)}
    if let Some(up)=parent.parent(){
        if let Some(bytes)=find_nearby_resource(up,&file,2){return Some(bytes)}
    }
    None
}

fn parse_kpp_bytes(data:&[u8],source_path:&Path)->Result<ImportedBrushDraft,String>{
    let img=image::load_from_memory(data).map_err(|e|format!("KPP preview PNG: {e}"))?;
    let icon=Some(icon_from_image(img));
    let text=png_text_chunks(data);
    let xml=text.get("preset").cloned().ok_or("KPP has no preset metadata")?;
    parse_krita_preset_xml(&xml,icon,source_path,|name|read_krita_resource_near(source_path,name))
}

fn parse_krita_preset_xml<F>(
    xml:&str,
    icon:Option<IconImage>,
    source_path:&Path,
    mut resolve_resource:F,
)->Result<ImportedBrushDraft,String>
where F:FnMut(&str)->Option<Vec<u8>>
{
    let doc=roxmltree::Document::parse(xml).map_err(|e|format!("KPP preset XML: {e}"))?;
    let root=doc.descendants().find(|n|n.has_tag_name("Preset")).ok_or("KPP Preset element missing")?;
    let name=root.attribute("name")
        .unwrap_or_else(||source_path.file_stem().and_then(|s|s.to_str()).unwrap_or("Krita Brush"))
        .replace('_'," ");
    let paintop=root.attribute("paintopid").unwrap_or("paintbrush").to_owned();

    let mut params=HashMap::<String,String>::new();
    for n in doc.descendants().filter(|n|n.has_tag_name("param")) {
        let Some(k)=n.attribute("name") else {continue};
        let raw=n.text().unwrap_or("");
        let value=if n.attribute("type")==Some("bytearray") {
            base64::engine::general_purpose::STANDARD
                .decode(raw.split_whitespace().collect::<String>())
                .ok()
                .map(|bytes|String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default()
        } else {
            raw.to_owned()
        };
        params.insert(k.to_owned(),value);
    }

    // Some modern KPPs embed resource data directly in the preset XML.
    let mut tips=Vec::new();
    for r in doc.descendants().filter(|n|n.has_tag_name("resource")).take(MAX_TIPS) {
        let rtype=r.attribute("type").unwrap_or("");
        if !rtype.to_ascii_lowercase().contains("brush") {continue}
        let filename=r.attribute("filename").or_else(||r.attribute("name")).unwrap_or("");
        let raw=r.text().and_then(|t|
            base64::engine::general_purpose::STANDARD
                .decode(t.split_whitespace().collect::<String>()).ok());
        if let Some(raw)=raw {
            tips.extend(tips_from_resource(filename,&raw));
        }
    }

    let mut spacing=0.10_f32;
    let mut angle_deg=0.0_f32;
    let mut auto_hardness=0.90_f32;
    let mut auto_ratio=1.0_f32;
    let mut default_size=30.0_f32;
    let mut tip_scale=1.0_f32;
    let mut brush_type="auto_brush".to_owned();
    let mut definition_resource=String::new();

    if let Some(def)=params.get("brush_definition") {
        let cleaned=def.replace("<!DOCTYPE BrushTip>", "");
        if let Ok(bdoc)=roxmltree::Document::parse(cleaned.trim()) {
            if let Some(brush)=bdoc.descendants().find(|n|n.has_tag_name("Brush")) {
                brush_type=brush.attribute("type").unwrap_or("auto_brush").to_ascii_lowercase();
                // IMPORTANT: this is the real brush-tip spacing. SpacingValue is
                // a dynamics multiplier and must never replace it.
                if let Some(v)=brush.attribute("spacing").and_then(|v|v.parse::<f32>().ok()) {
                    spacing=v.clamp(0.005,2.0);
                }
                tip_scale=brush.attribute("scale").and_then(|v|v.parse::<f32>().ok())
                    .unwrap_or(1.0).clamp(0.05,8.0);
                angle_deg=brush.attribute("angle").and_then(|v|v.parse::<f32>().ok())
                    .map(|r|r.to_degrees()).unwrap_or(0.0);
                definition_resource=brush.attribute("filename").unwrap_or("").trim().to_owned();
                if let Some(mask)=brush.descendants().find(|n|n.has_tag_name("MaskGenerator")) {
                    let h=mask.attribute("hfade").and_then(|v|v.parse::<f32>().ok()).unwrap_or(0.9);
                    let v=mask.attribute("vfade").and_then(|v|v.parse::<f32>().ok()).unwrap_or(h);
                    auto_hardness=((h+v)*0.5).clamp(0.02,0.999);
                    auto_ratio=mask.attribute("ratio").and_then(|v|v.parse::<f32>().ok())
                        .unwrap_or(1.0).clamp(0.05,1.0);
                    default_size=mask.attribute("diameter").and_then(|v|v.parse::<f32>().ok())
                        .unwrap_or(30.0).clamp(1.0,2000.0);
                }
            }
        }
    }

    let opacity=find_number_in_params(&params,&["OpacityValue","opacity"])
        .map(|v|if v>1.0{v/100.0}else{v}).unwrap_or(1.0).clamp(0.05,1.0);
    let flow=find_number_in_params(&params,&["FlowValue","flow"])
        .map(|v|if v>1.0{v/100.0}else{v}).unwrap_or(1.0).clamp(0.02,1.0);

    let pressure_size=params.get("PressureSize").is_some_and(|v|is_truthy(v));
    let size_sensor=params.get("SizeSensor").map(String::as_str).unwrap_or("");
    let size_pressure=pressure_size && size_sensor.to_ascii_lowercase().contains("pressure");

    let pressure_opacity=params.get("PressureOpacity").is_some_and(|v|is_truthy(v));
    let opacity_sensor=params.get("OpacitySensor").map(String::as_str).unwrap_or("");
    let opacity_pressure=if pressure_opacity
        && opacity_sensor.to_ascii_lowercase().contains("pressure"){1.0}else{0.0};

    let pressure_rotation=params.get("PressureRotation").is_some_and(|v|is_truthy(v));
    let rotation_sensor=params.get("RotationSensor").map(String::as_str).unwrap_or("").to_ascii_lowercase();
    let follow_stroke=pressure_rotation && rotation_sensor.contains("drawingangle");

    // Krita's own setPaintOpScatter() writes ScatterValue AND toggles
    // PressureScatter based on whether the value is non-zero. Old presets can
    // retain a stale ScatterValue while PressureScatter=false. Applying that
    // stale value was the cause of imported ink strokes wandering into dots.
    let pressure_scatter=params.get("PressureScatter").is_some_and(|v|is_truthy(v));
    let scatter=if pressure_scatter {
        find_number_in_params(&params,&["ScatterValue"]).unwrap_or(0.0).clamp(0.0,3.0)
    } else {
        0.0
    };

    if tips.is_empty() && brush_type!="auto_brush" {
        let required=params.get("requiredBrushFile")
            .map(|s|s.trim()).filter(|s|!s.is_empty())
            .unwrap_or(definition_resource.trim());
        if !required.is_empty() {
            if let Some(raw)=resolve_resource(required) {
                tips.extend(tips_from_resource(required,&raw));
            }
            if tips.is_empty() {
                return Err(format!(
                    "Krita brush '{name}' needs the external tip '{required}'. \
Put that .gih/.gbr beside the .kpp, keep the original 'brushes' folder next to \
'paintoppresets', or import the complete Krita .bundle. Stick will not replace \
a missing real tip with a fake round brush."
                ));
            }
        } else {
            return Err(format!(
                "Krita brush '{name}' uses '{brush_type}' but contains no readable brush tip. \
Stick refused the old round-tip fallback because it changes the brush."
            ));
        }
    }

    if tips.is_empty() {
        // auto_brush is self-contained: rebuild the actual generated mask from
        // the KPP brush_definition instead of substituting a generic hard dot.
        let tip=if auto_ratio<0.96 {
            elliptical_tip(128,auto_hardness,auto_ratio,angle_deg)
        } else {
            round_tip(128,auto_hardness)
        };
        tips.push(tip);
    }

    let note=if brush_type=="auto_brush" {
        format!(
            "Krita {paintop} auto-brush mapped from brush_definition: spacing {:.1}%, pressure size {}, pressure opacity {}, scatter {:.2}.",
            spacing*100.0,size_pressure,opacity_pressure>0.0,scatter
        )
    } else {
        format!(
            "Krita {paintop} {brush_type} imported with its real brush-tip resource; spacing {:.1}% and scale {:.2} preserved.",
            spacing*100.0,tip_scale
        )
    };

    Ok(ImportedBrushDraft{
        suggested_name:name,source_format:"Krita .kpp".into(),source_path:source_path.to_path_buf(),
        tips,icon,default_size:(default_size*tip_scale).clamp(1.0,2000.0),
        spacing,opacity,flow,size_pressure:if size_pressure{1.0}else{0.0},
        opacity_pressure,angle_deg,follow_stroke,scatter,random_tip:false,tip_scale,
        render_mode:if brush_type=="auto_brush" {
            CustomBrushRenderMode::Continuous
        } else {
            CustomBrushRenderMode::Stamp
        },
        auxiliary_resource_path:None,
        compatibility_note:note,
    })
}

fn import_krita_bundle(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    let cursor=Cursor::new(bytes);
    let mut zip=zip::ZipArchive::new(cursor).map_err(|e|format!("Krita bundle: {e}"))?;

    // Read the bundle once so every .kpp can resolve its sibling brush
    // resources (GBR/GIH/images) from the same archive.
    let mut entries=Vec::<(String,Vec<u8>)>::new();
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        let mut file=zip.by_index(i).map_err(|e|e.to_string())?;
        if file.is_dir(){continue}
        let name=file.name().replace('\\',"/");
        let remaining = MAX_BRUSH_INPUT_BYTES.saturating_sub(expanded).min(MAX_BRUSH_MEMBER_BYTES);
        if file.size() > remaining { return Err("Krita bundle exceeds safe expanded size.".into()); }
        let mut data=Vec::new();
        file.by_ref().take(remaining + 1).read_to_end(&mut data).map_err(|e|e.to_string())?;
        if data.len() as u64 > remaining { return Err("Krita bundle exceeds safe expanded size.".into()); }
        expanded += data.len() as u64;
        entries.push((name,data));
    }

    let mut out=Vec::new();
    for (entry_name,data) in entries.iter()
        .filter(|(name,_)|name.to_ascii_lowercase().ends_with(".kpp"))
        .take(MAX_IMPORT_BRUSHES)
    {
        let pseudo=PathBuf::from(entry_name);
        let parsed=parse_krita_preset_xml(
            &png_text_chunks(data).get("preset").cloned().unwrap_or_default(),
            image::load_from_memory(data).ok().map(icon_from_image),
            &pseudo,
            |wanted|{
                let wanted_name=Path::new(wanted).file_name()
                    .and_then(|s|s.to_str()).unwrap_or(wanted);
                entries.iter().find(|(name,_)|{
                    Path::new(name).file_name().and_then(|s|s.to_str())
                        .is_some_and(|n|n.eq_ignore_ascii_case(wanted_name))
                }).map(|(_,bytes)|bytes.clone())
            }
        );
        if let Ok(mut draft)=parsed {
            draft.source_path=path.to_path_buf();
            draft.source_format="Krita .bundle".into();
            out.push(draft);
        }
    }
    if out.is_empty(){return Err("Krita bundle contains no readable brush presets/resources".into())}
    Ok(out)
}

fn import_myb_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    parse_myb_bytes(&bytes,path)
}

fn parse_myb_bytes(bytes:&[u8],path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let v:serde_json::Value=serde_json::from_slice(bytes).map_err(|e|format!("MyPaint .myb: {e}"))?;
    let settings=v.get("settings").filter(|s|s.is_object()).ok_or("MyPaint preset is missing its settings")?;
    let setting=|name:&str|settings.get(name);
    let base=|name:&str,default:f32|setting(name).and_then(|x|x.get("base_value"))
        .and_then(|x|x.as_f64()).map(|x|x as f32).unwrap_or(default);
    let has_pressure=|name:&str|setting(name).and_then(|x|x.get("inputs"))
        .and_then(|x|x.get("pressure")).is_some();
    let opaque=(base("opaque",1.0)*base("opaque_multiply",1.0)).clamp(0.03,1.0);
    let hardness=base("hardness",0.8).clamp(0.0,0.999);
    let dabs=base("dabs_per_actual_radius",0.0)
        .max(base("dabs_per_basic_radius",4.0)).max(0.5);
    let default_size=(2.0*base("radius_logarithmic",2.0).exp()).clamp(1.0,2000.0);
    let ratio=base("elliptical_dab_ratio",1.0).abs().clamp(0.05,1.0);
    let angle=base("elliptical_dab_angle",0.0);
    let scatter=base("offset_by_random",0.0).abs().clamp(0.0,3.0);
    let size_pressure=if has_pressure("radius_logarithmic"){1.0}else{0.0};
    let opacity_pressure=if has_pressure("opaque")||has_pressure("opaque_multiply"){1.0}else{0.0};
    let tip=if ratio<0.98 {
        elliptical_tip(128,hardness,ratio,angle)
    } else {
        round_tip(128,hardness)
    };
    Ok(vec![ImportedBrushDraft{
        suggested_name:path.file_stem().and_then(|s|s.to_str()).unwrap_or("MyPaint Brush").to_owned(),
        source_format:"Krita / MyPaint .myb".into(),source_path:path.to_path_buf(),
        tips:vec![tip],icon:None,default_size,
        spacing:(0.5/dabs).clamp(0.02,0.5),
        opacity:opaque,flow:1.0,size_pressure,opacity_pressure,angle_deg:angle,
        follow_stroke:ratio<0.98,scatter,random_tip:false,tip_scale:1.0,
        render_mode:CustomBrushRenderMode::Continuous,
        auxiliary_resource_path:None,
        compatibility_note:format!(
            "MyPaint mapped: hardness {:.0}%, spacing {:.1}%, elliptic ratio {:.2}, scatter {:.2}, pressure size {}, pressure opacity {}.",
            hardness*100.0,(0.5/dabs).clamp(0.02,0.5)*100.0,ratio,scatter,
            size_pressure>0.0,opacity_pressure>0.0),
    }])
}

fn import_sut_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let flags=OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let con=Connection::open_with_flags(path,flags).map_err(|e|format!("Clip Studio .sut: {e}"))?;
    let tables:Vec<String>={
        let mut st=con.prepare("SELECT name FROM sqlite_master WHERE type='table'").map_err(|e|e.to_string())?;
        let rows=st.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
        let mut names=Vec::new();
        for row in rows { if let Ok(name)=row { names.push(name); } }
        names
    };
    if !tables.iter().any(|x|x=="Variant"){return Err("Clip Studio file has no Variant table".into())}

    // A .sut can contain init/current variants. The old importer always took
    // row #1, which makes unrelated CSP brushes collapse to similar defaults.
    // Follow Node.NodeVariantID to the actual active brush variant instead.
    let node_info:Option<(String,Value)>=if tables.iter().any(|x|x=="Node"){
        con.query_row(
            "SELECT COALESCE(NodeName,''), NodeVariantID
             FROM Node
             WHERE NodeVariantID IS NOT NULL
             LIMIT 1",
            [],
            |r|Ok((r.get::<_,String>(0)?,r.get::<_,Value>(1)?))
        ).ok()
    }else{None};
    let name=node_info.as_ref().map(|(n,_)|n.clone())
        .filter(|n|!n.trim().is_empty())
        .unwrap_or_else(||path.file_stem().and_then(|s|s.to_str())
            .unwrap_or("Clip Studio Brush").to_owned());
    let variant=if let Some((_,variant_id))=node_info.as_ref() {
        row_map_where(&con,"Variant","VariantID",variant_id)
            .or_else(|_|first_row_map(&con,"Variant"))?
    } else {
        first_row_map(&con,"Variant")?
    };

    let num=|k:&str,d:f32|value_f32(variant.get(k)).unwrap_or(d);
    let bool_num=|k:&str|num(k,0.0)>0.5;
    let opacity=(num("Opacity",100.0)/100.0).clamp(0.05,1.0);
    let flow=(num("BrushFlow",100.0)/100.0).clamp(0.02,1.0);
    let auto=num("BrushAutoIntervalType",0.0) as i32;
    let spacing=if auto==0{(num("BrushInterval",10.0)/100.0).clamp(0.01,2.0)}
        else{match auto{1=>0.05,2=>0.10,3=>0.25,_=>0.10}};
    let mut rotation=num("BrushRotation",0.0);
    if bool_num("BrushVerticalThicknes"){rotation+=90.0;}
    let follow_stroke=(num("BrushRotationEffector",1.0) as i32 & 0x02)!=0;
    let hardness=(num("BrushHardness",100.0)/100.0).clamp(0.02,0.999);
    let roundness=(num("BrushThickness",100.0)/100.0).clamp(0.05,1.0);
    let size_pressure=if csp_effector_uses_pressure(variant.get("BrushSizeEffector")){1.0}else{0.0};
    let opacity_pressure=if csp_effector_uses_pressure(variant.get("BrushOpacityEffector")){1.0}else{0.0};

    let mut tips=Vec::new();
    let mut icon=None;
    let uses_material_tip=bool_num("BrushUsePatternImage");
    if uses_material_tip && tables.iter().any(|x|x=="MaterialFile") {
        if let Ok(mut st)=con.prepare("SELECT FileData FROM MaterialFile WHERE FileData IS NOT NULL") {
            if let Ok(rows)=st.query_map([],|r|r.get::<_,Vec<u8>>(0)) {
                for row in rows.filter_map(Result::ok).take(MAX_TIPS) {
                    if let Some(png)=find_first_png(&row) {
                        if let Ok(img)=image::load_from_memory(png) {
                            if icon.is_none(){icon=Some(icon_from_image(img.clone()));}
                            tips.push(tip_from_image(&img));
                        }
                    }
                }
            }
        }
    }
    let has_embedded_tip=!tips.is_empty();

    let spray=bool_num("BrushUseSpray");
    let mut scatter=0.0;
    let mut tip_scale=1.0;
    if spray {
        let particle=num("BrushSpraySize",100.0).max(1.0);
        let area=num("BrushSize",30.0).max(1.0);
        tip_scale=(particle/100.0).clamp(0.05,4.0);
        scatter=(area/(particle.max(1.0)*2.0)).clamp(0.0,3.0);
    }
    if tips.is_empty(){
        let tip=if roundness<0.97 {
            elliptical_tip(128,hardness,roundness,rotation)
        } else {
            round_tip(128,hardness)
        };
        tips.push(tip);
    }

    Ok(vec![ImportedBrushDraft{
        suggested_name:name,source_format:"Clip Studio Paint .sut".into(),source_path:path.to_path_buf(),
        tips,icon,default_size:num("BrushSize",30.0).clamp(1.0,2000.0),
        spacing,opacity,flow,size_pressure,opacity_pressure,angle_deg:rotation,
        follow_stroke,scatter,random_tip:num("BrushPatternOrderType",0.0) as i32==3,tip_scale,
        render_mode:if has_embedded_tip || spray {
            CustomBrushRenderMode::Stamp
        } else {
            CustomBrushRenderMode::Continuous
        },
        auxiliary_resource_path:None,
        compatibility_note:format!(
            "CSP active Variant mapped: spacing {:.0}%, hardness {:.0}%, roundness {:.0}%, material tip {}, spray {}, pressure size {}, pressure opacity {}.",
            spacing*100.0,hardness*100.0,roundness*100.0,has_embedded_tip,spray,
            size_pressure>0.0,opacity_pressure>0.0),
    }])
}
fn import_sutg_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    if fs::read(path).map(|b|b.starts_with(b"SQLite format 3\0")).unwrap_or(false) {
        return import_sut_file(path);
    }
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if let Ok(mut zip)=zip::ZipArchive::new(Cursor::new(bytes)) {
        let mut out=Vec::new();
        for i in 0..zip.len() {
            if out.len()>=MAX_IMPORT_BRUSHES{break}
            let mut f=zip.by_index(i).map_err(|e|e.to_string())?;
            if !f.name().to_ascii_lowercase().ends_with(".sut"){continue}
            if f.size() > MAX_BRUSH_MEMBER_BYTES { return Err("Clip Studio brush archive member too large.".into()); }
            let mut buf=Vec::new();
            f.by_ref().take(MAX_BRUSH_MEMBER_BYTES + 1).read_to_end(&mut buf).map_err(|e|e.to_string())?;
            if buf.len() as u64 > MAX_BRUSH_MEMBER_BYTES { return Err("Clip Studio brush archive member too large.".into()); }
            let temp=std::env::temp_dir().join(format!("stick_sutg_{}_{}.sut",std::process::id(),i));
            fs::write(&temp,&buf).map_err(|e|e.to_string())?;
            if let Ok(mut brushes)=import_sut_file(&temp) {
                for b in &mut brushes {b.source_path=path.to_path_buf();b.source_format="Clip Studio Paint .sutg".into();}
                out.extend(brushes);
            }
            let _=fs::remove_file(temp);
        }
        if !out.is_empty(){return Ok(out)}
    }
    Err("Clip Studio .sutg could not be decoded into real .sut sub-tools. Export/import the individual .sut brush instead; Stick will not create a fake generic brush.".into())
}

fn row_map_where(
    con:&Connection,table:&str,column:&str,value:&Value
)->Result<HashMap<String,Value>,String>{
    let table=table.replace('"',"");
    let column=column.replace('"',"");
    let sql=format!("SELECT * FROM \"{table}\" WHERE \"{column}\"=?1 LIMIT 1");
    let mut st=con.prepare(&sql).map_err(|e|e.to_string())?;
    let names:Vec<String>=st.column_names().iter().map(|s|s.to_string()).collect();
    let mut rows=st.query([value]).map_err(|e|e.to_string())?;
    let Some(row)=rows.next().map_err(|e|e.to_string())? else{
        return Err("Requested Clip Studio Variant row was not found.".into())
    };
    let mut out=HashMap::new();
    for (i,name) in names.into_iter().enumerate(){
        if let Ok(v)=row.get::<_,Value>(i){out.insert(name,v);}
    }
    Ok(out)
}

fn first_row_map(con:&Connection,table:&str)->Result<HashMap<String,Value>,String>{
    let sql=format!("SELECT * FROM \"{}\" LIMIT 1",table.replace('"',""));
    let mut st=con.prepare(&sql).map_err(|e|e.to_string())?;
    let names:Vec<String>=st.column_names().iter().map(|s|s.to_string()).collect();
    let mut rows=st.query([]).map_err(|e|e.to_string())?;
    let Some(row)=rows.next().map_err(|e|e.to_string())? else{return Ok(HashMap::new())};
    let mut out=HashMap::new();
    for (i,name) in names.into_iter().enumerate() {
        if let Ok(v)=row.get::<_,Value>(i){out.insert(name,v);}
    }
    Ok(out)
}
fn csp_effector_uses_pressure(value:Option<&Value>)->bool{
    let Some(Value::Blob(blob))=value else{return false};
    if blob.len()<12{return false}
    let header=u32::from_be_bytes([blob[0],blob[1],blob[2],blob[3]]) as usize;
    if header<40 || header>blob.len(){return false}
    let enabled=i32::from_be_bytes([blob[8],blob[9],blob[10],blob[11]]);
    enabled&0x10!=0
}

fn value_f32(v:Option<&Value>)->Option<f32>{
    match v?{Value::Integer(x)=>Some(*x as f32),Value::Real(x)=>Some(*x as f32),Value::Text(s)=>s.parse().ok(),_=>None}
}

fn import_blend_file(path:&Path)->Result<Vec<ImportedBrushDraft>,String>{
    let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
        let blend_bytes=fs::read(path).map_err(|e|format!("Blender .blend read: {e}"))?;
        let png_sig=b"\x89PNG\r\n\x1a\n";
        let embedded_png_count=blend_bytes.windows(png_sig.len()).filter(|w|*w==png_sig).count();
        let packed_texture=if embedded_png_count==1 {
            find_first_png(&blend_bytes).and_then(|png|image::load_from_memory(png).ok())
        } else {None};
        let blend=blend::Blend::from_path(path).map_err(|e|format!("Blender .blend: {e:?}"))?;
        let mut out=Vec::new();

        let safe_i8=|inst:&blend::Instance<'_>,field:&str,default:i8| -> i8 {
            if !inst.is_valid(field){return default}
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(||inst.get_i8(field)))
                .unwrap_or(default)
        };
        let is_gp_draw_brush=|brush:&blend::Instance<'_>|->bool{
            if !brush.is_valid("gpencil_settings"){return false}
            let gp=match std::panic::catch_unwind(
                std::panic::AssertUnwindSafe(||brush.get("gpencil_settings"))) {
                Ok(settings)=>settings,
                Err(_)=>return false,
            };
            if !gp.is_valid("draw_strength") && !gp.is_valid("flag"){return false}
            let gp_type=if brush.is_valid("gpencil_brush_type"){
                safe_i8(brush,"gpencil_brush_type",0)
            }else{
                safe_i8(brush,"gpencil_tool",0)
            };
            gp_type==0
        };

        // Asset-browser .blend packs commonly contain normal Blender helper
        // brushes in addition to the brush the artist actually distributed.
        // Musubibi's Crayon is one such pack: importing every BR datablock
        // makes Airbrush/internal tools appear beside Crayon and they collapse
        // to near-identical results. If at least one Brush is explicitly marked
        // as a Blender Asset, import only asset-marked Brush datablocks.
        let is_asset_brush=|brush:&blend::Instance<'_>|->bool{
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
                if !brush.is_valid("id"){return false}
                let id=brush.get("id");
                if !id.is_valid("asset_data"){return false}
                let asset=id.get("asset_data");
                // catalog_simple_name is a fixed char[64] in Blender's
                // AssetMetaData DNA, so reading it is a reliable pointer probe
                // even when optional author/description pointers are null.
                if asset.is_valid("catalog_simple_name"){
                    let _=asset.get_string("catalog_simple_name");
                    true
                }else{false}
            })).unwrap_or(false)
        };
        let has_explicit_assets=blend.instances_with_code(*b"BR")
            .any(|b|is_gp_draw_brush(&b)&&is_asset_brush(&b));
        let has_custom_icon=|brush:&blend::Instance<'_>|->bool{
            if !brush.is_valid("icon_filepath"){return false}
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
                !brush.get_string("icon_filepath")
                    .trim_matches(char::from(0)).trim().is_empty()
            })).unwrap_or(false)
        };
        // Blender 3.6 brush-pack files often save the whole startup Brush list
        // and do not set ID.asset_data at all. Musubibi's file has 74 BR
        // datablocks, but the distributed Crayon is the one carrying a custom
        // icon/thumbnail path. If explicit Asset metadata is unavailable and
        // the file provides custom-icon brushes, treat those as pack entries
        // instead of importing Blender's factory Airbrush/Ink/Marker/etc.
        let has_custom_icon_brush=blend.instances_with_code(*b"BR")
            .any(|b|is_gp_draw_brush(&b)&&has_custom_icon(&b));

        for (index,brush) in blend.instances_with_code(*b"BR").enumerate() {
            if out.len()>=MAX_IMPORT_BRUSHES{break}
            if !is_gp_draw_brush(&brush){continue}
            if has_explicit_assets && !is_asset_brush(&brush){continue}
            if !has_explicit_assets && has_custom_icon_brush && !has_custom_icon(&brush){continue}

            let safe_f32=|inst:&blend::Instance<'_>,field:&str,default:f32| -> f32 {
                if !inst.is_valid(field){return default}
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(||inst.get_f32(field)))
                    .unwrap_or(default)
            };
            let safe_i32=|inst:&blend::Instance<'_>,field:&str,default:i32| -> i32 {
                if !inst.is_valid(field){return default}
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(||inst.get_i32(field)))
                    .unwrap_or(default)
            };
            let safe_i16=|inst:&blend::Instance<'_>,field:&str,default:i16| -> i16 {
                if !inst.is_valid(field){return default}
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(||inst.get_i16(field)))
                    .unwrap_or(default)
            };
            let safe_f32_vec=|inst:&blend::Instance<'_>,field:&str| -> Vec<f32> {
                if !inst.is_valid(field){return Vec::new()}
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(||inst.get_f32_vec(field)))
                    .unwrap_or_default()
            };

            // Candidate pre-pass already proved this is a real Grease Pencil
            // DRAW brush. Fetch the settings once more for the detailed mapping.
            let gp=match std::panic::catch_unwind(
                std::panic::AssertUnwindSafe(||brush.get("gpencil_settings"))) {
                Ok(settings)=>settings,
                Err(_)=>continue,
            };

            let raw_name=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{
                brush.get("id").get_string("name")
            })).unwrap_or_else(|_|format!("BRBrush {}",index+1));
            let name=raw_name.strip_prefix("BR").unwrap_or(&raw_name).trim().to_owned();

            let size=safe_i32(&brush,"size",30).clamp(1,5000);
            let alpha=safe_f32(&brush,"alpha",1.0).clamp(0.05,1.0);
            let raw_flow=safe_f32(&brush,"flow",1.0);
            let flow=(if raw_flow<=0.001{1.0}else{raw_flow}).clamp(0.02,1.0);
            let spacing=(safe_i32(&brush,"spacing",10) as f32/100.0).clamp(0.01,2.0);

            let gp_strength=safe_f32(&gp,"draw_strength",1.0).clamp(0.02,1.0);
            let jitter=safe_f32(&gp,"draw_jitter",
                safe_f32(&brush,"jitter",0.0)).clamp(0.0,3.0);
            let hardness=safe_f32(&gp,"hardness",
                safe_f32(&brush,"hardness",0.9)).clamp(0.02,0.999);
            let angle=safe_f32(&gp,"draw_angle",0.0).to_degrees();
            let angle_factor=safe_f32(&gp,"draw_angle_factor",0.0).abs().clamp(0.0,1.0);
            let mut aspect=safe_f32_vec(&gp,"aspect");
            if aspect.len()<2 {aspect=safe_f32_vec(&gp,"aspect_ratio");}
            let aspect_ratio=if aspect.len()>=2 {
                let a=aspect[0].abs().max(0.01);
                let b=aspect[1].abs().max(0.01);
                (a.min(b)/a.max(b)).clamp(0.08,1.0)
            } else {
                (safe_f32(&brush,"tip_scale_x",1.0).abs()*(1.0-angle_factor*0.55))
                    .clamp(0.08,1.0)
            };
            let smooth=safe_f32(&gp,"draw_smoothfac",
                safe_f32(&gp,"active_smooth",
                    safe_f32(&brush,"smooth_stroke_factor",0.0))).clamp(0.0,1.0);
            let smooth_level=safe_i32(&gp,"draw_smoothlvl",0).max(0);
            let subdivision=safe_i32(&gp,"draw_subdivide",0).max(0);
            let random_pressure=safe_f32(&gp,"draw_random_press",0.0).abs().clamp(0.0,1.0);
            let random_gp_strength=safe_f32(&gp,"draw_random_strength",0.0).abs().clamp(0.0,1.0);
            let uv_random=safe_f32(&gp,"uv_random",0.0).abs().clamp(0.0,1.0);
            let input_samples=safe_i32(&gp,"input_samples",
                safe_i32(&brush,"input_samples",1)).max(1);
            let gp_flags=safe_i32(&gp,"flag",0) as u32;
            let size_pressure=if gp_flags&(1<<0)!=0{1.0}else{0.0};
            let opacity_pressure=if gp_flags&(1<<1)!=0{1.0}else{0.0};
            let pressure_jitter=gp_flags&(1<<2)!=0;

            // Grease Pencil materials can change a brush from a plain line
            // into textured/dot/square media. Preserve that distinction and
            // load external //relative stroke textures when the asset ships them.
            let mut material_mode=0i32; // 0=line, 1=dot, 2=square
            let mut material_texture_path=String::new();
            let mut material_texture_tip:Option<TipMask>=None;
            let mut material_texture_source:Option<PathBuf>=None;
            let mut texture_scale=1.0f32;
            let mut random_size=0.0f32;
            let mut random_strength=0.0f32;
            let mut random_rotation=0.0f32;
            if gp.is_valid("material") {
                if let Ok(material)=std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(||gp.get("material"))) {
                    if material.is_valid("gp_style") {
                        if let Ok(style)=std::panic::catch_unwind(
                            std::panic::AssertUnwindSafe(||material.get("gp_style"))) {
                            material_mode=safe_i32(&style,"mode",0);
                            random_size=safe_f32(&style,"random_size_factor",0.0).clamp(0.0,1.0);
                            random_strength=safe_f32(&style,"random_strength_factor",0.0).clamp(0.0,1.0);
                            random_rotation=safe_f32(&style,"random_rotation_factor",0.0).clamp(0.0,1.0);
                            let tex_scale=safe_f32_vec(&style,"texture_scale");
                            if !tex_scale.is_empty() {
                                texture_scale=tex_scale.iter().map(|v|v.abs())
                                    .filter(|v|*v>0.001).fold(1.0f32,|a,b|a.min(b))
                                    .clamp(0.05,8.0);
                            }
                            let stroke_style=safe_i16(&style,"stroke_style",0);
                            if stroke_style==1 && style.is_valid("sima") {
                                if let Ok(img)=std::panic::catch_unwind(
                                    std::panic::AssertUnwindSafe(||style.get("sima"))) {
                                    material_texture_path=std::panic::catch_unwind(
                                        std::panic::AssertUnwindSafe(||img.get_string("filepath")))
                                        .unwrap_or_default();
                                    if let Some((texture,texture_path))=
                                        blender_external_image(path,&material_texture_path) {
                                        material_texture_tip=Some(tip_from_image(&texture));
                                        material_texture_source=Some(texture_path);
                                    } else if let Some(texture)=packed_texture.clone() {
                                        // Many distributed Blender brush packs embed the
                                        // texture and retain the creator's dead absolute
                                        // filepath. Use the packed PNG when it is unambiguous.
                                        material_texture_tip=Some(tip_from_image(&texture));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let tip=if let Some(texture_tip)=material_texture_tip.clone() {
                texture_tip
            } else if material_mode==2 {
                square_tip(128,hardness)
            } else if aspect_ratio<0.97 {
                elliptical_tip(128,hardness,aspect_ratio,angle)
            } else {
                round_tip(128,hardness)
            };
            let textured=material_texture_tip.is_some();
            // Preserve more of Blender's per-brush dynamics in Stick's current
            // stamp model. Rotation/random-strength do not have dedicated
            // channels yet, but folding a small amount into spatial variation
            // prevents distinct crayon presets from rendering as one generic tip.
            let final_jitter=(jitter+random_size*0.35+random_pressure*0.20
                +uv_random*0.15+random_rotation*0.12+random_strength*0.08
                +random_gp_strength*0.08).clamp(0.0,3.0);
            let material_note=if !material_texture_path.is_empty() && !textured {
                format!(" texture '{}' was referenced but not found beside the .blend;",material_texture_path)
            } else if textured && material_texture_source.is_none() {
                " packed stroke texture loaded from the .blend;".to_owned()
            } else if textured {
                " external stroke texture loaded;".to_owned()
            } else {
                String::new()
            };

            out.push(ImportedBrushDraft{
                suggested_name:if name.is_empty(){format!("Blender GP Brush {}",index+1)}else{name},
                source_format:"Blender Grease Pencil Draw Brush .blend/.asset.blend".into(),
                source_path:path.to_path_buf(),tips:vec![tip],icon:None,default_size:size as f32,
                spacing,opacity:(alpha*gp_strength).clamp(0.05,1.0),flow,
                size_pressure,opacity_pressure,angle_deg:angle,
                follow_stroke:material_mode!=0,scatter:final_jitter,random_tip:false,
                tip_scale:texture_scale,
                render_mode:if material_mode!=0 || textured {
                    CustomBrushRenderMode::Stamp
                } else {
                    CustomBrushRenderMode::Continuous
                },
                auxiliary_resource_path:material_texture_source,
                compatibility_note:format!(
                    "Blender GP Draw mapped: {}px, spacing {:.0}%, strength {:.0}%, hardness {:.0}%, aspect {:.2}, angle-factor {:.2}, jitter {:.2}{}, material mode {}, random size {:.2}, strength {:.2}/{:.2}, rotation {:.2}, uv-random {:.2},{} smoothing {:.2} (level {}), subdivision {}, input samples {}.",
                    size,spacing*100.0,gp_strength*100.0,hardness*100.0,aspect_ratio,angle_factor,
                    final_jitter,if pressure_jitter{" + pressure"}else{""},material_mode,random_size,
                    random_strength,random_gp_strength,random_rotation,uv_random,material_note,
                    smooth,smooth_level,subdivision,input_samples),
            });
        }
        Ok::<_,String>(out)
    })).map_err(|_|"Blender asset parser rejected this .blend safely".to_string())??;

    if result.is_empty() {
        Err("No Grease Pencil DRAW brushes were found in this .blend. Sculpt/Mask/Fill/Erase Blender tools are intentionally not imported as drawing brushes.".into())
    } else {
        Ok(result)
    }
}
fn tips_from_resource(filename:&str,data:&[u8])->Vec<TipMask>{
    let ext=Path::new(filename).extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str(){
        "gbr"=>parse_gbr(data,0).ok().map(|x|vec![x.1]).unwrap_or_default(),
        "gih"=>parse_gih_bytes(data,Path::new(filename)).ok()
            .and_then(|v|v.into_iter().next()).map(|d|d.tips).unwrap_or_default(),
        _=>tip_image_format(&ext).and_then(|format|image::load_from_memory_with_format(data,format).ok())
            .map(|i|vec![tip_from_image(&i)]).unwrap_or_default(),
    }
}

fn png_text_chunks(data:&[u8])->HashMap<String,String>{
    let mut out=HashMap::new();
    if !data.starts_with(b"\x89PNG\r\n\x1a\n"){return out}
    let mut pos=8usize;
    while pos+12<=data.len() {
        let len=u32::from_be_bytes(data[pos..pos+4].try_into().unwrap()) as usize;
        if pos+12+len>data.len(){break}
        let typ=&data[pos+4..pos+8];
        let chunk=&data[pos+8..pos+8+len];
        if typ==b"tEXt" {
            if let Some(z)=chunk.iter().position(|&b|b==0) {
                out.insert(String::from_utf8_lossy(&chunk[..z]).to_string(),String::from_utf8_lossy(&chunk[z+1..]).to_string());
            }
        } else if typ==b"zTXt" {
            if let Some(z)=chunk.iter().position(|&b|b==0) {
                if chunk.get(z+1)==Some(&0) {
                    let mut dec=ZlibDecoder::new(&chunk[z+2..]);
                    let mut text=String::new();
                    if dec.read_to_string(&mut text).is_ok(){out.insert(String::from_utf8_lossy(&chunk[..z]).to_string(),text);}
                }
            }
        } else if typ==b"iTXt" {
            if let Some(z)=chunk.iter().position(|&b|b==0) {
                let key=String::from_utf8_lossy(&chunk[..z]).to_string();
                if z+3<chunk.len() {
                    let compressed=chunk[z+1]==1;
                    let mut p=z+3;
                    for _ in 0..2 {
                        if let Some(n)=chunk[p..].iter().position(|&b|b==0){p+=n+1}else{p=chunk.len();break}
                    }
                    if p<=chunk.len() {
                        if compressed {
                            let mut dec=ZlibDecoder::new(&chunk[p..]);let mut text=String::new();
                            if dec.read_to_string(&mut text).is_ok(){out.insert(key,text);}
                        } else {out.insert(key,String::from_utf8_lossy(&chunk[p..]).to_string());}
                    }
                }
            }
        }
        pos+=12+len;
    }
    out
}

fn find_number_in_params(map:&HashMap<String,String>,names:&[&str])->Option<f32>{
    for wanted in names {
        for (k,v) in map {
            if k.eq_ignore_ascii_case(wanted)||k.to_ascii_lowercase().ends_with(&wanted.to_ascii_lowercase()) {
                if let Some(x)=first_number(v){return Some(x)}
            }
        }
    }
    None
}
fn first_number(s:&str)->Option<f32>{
    let mut buf=String::new();let mut started=false;
    for c in s.chars() {
        if c.is_ascii_digit()||c=='.'||c=='-'||c=='+' {
            buf.push(c);started=true;
        } else if started {break}
    }
    buf.parse().ok()
}
fn is_truthy(v:&str)->bool{
    matches!(v.trim().to_ascii_lowercase().as_str(),"1"|"true"|"yes"|"on")
}

fn find_png_images(data:&[u8],limit:usize)->Vec<&[u8]>{
    let mut out=Vec::new();
    let mut offset=0usize;
    while offset<data.len() && out.len()<limit {
        let Some(slice)=find_first_png(&data[offset..]) else {break};
        let absolute_start=data[offset..].as_ptr() as usize;
        let slice_start=slice.as_ptr() as usize;
        let rel=slice_start.saturating_sub(absolute_start);
        let start=offset.saturating_add(rel);
        let end=start.saturating_add(slice.len());
        if end>data.len() || end<=start {break}
        out.push(&data[start..end]);
        offset=end;
    }
    out
}

fn find_first_png(data:&[u8])->Option<&[u8]>{
    let sig=b"\x89PNG\r\n\x1a\n";
    let start=data.windows(sig.len()).position(|w|w==sig)?;
    let mut p=start+8;
    while p+12<=data.len() {
        let len=u32::from_be_bytes(data[p..p+4].try_into().ok()?) as usize;
        if p+12+len>data.len(){return None}
        let typ=&data[p+4..p+8];
        p+=12+len;
        if typ==b"IEND"{return Some(&data[start..p])}
    }
    None
}

fn tip_from_image(img:&DynamicImage)->TipMask{
    let rgba=img.to_rgba8();
    let has_alpha=rgba.pixels().any(|p|p[3]<250);
    let data=if has_alpha {
        rgba.pixels().map(|p|p[3]).collect()
    } else {
        img.to_luma8().pixels().map(|p|255u8.saturating_sub(p[0])).collect()
    };
    downsample_tip(TipMask{width:img.width(),height:img.height(),data})
}

fn downsample_tip(tip:TipMask)->TipMask{
    if tip.width<=MAX_TIP_SIDE&&tip.height<=MAX_TIP_SIDE{return tip}
    let Some(gray)=GrayImage::from_raw(tip.width,tip.height,tip.data) else{return round_tip(96,0.9)};
    let scale=MAX_TIP_SIDE as f32/tip.width.max(tip.height) as f32;
    let w=(tip.width as f32*scale).round().max(1.0) as u32;
    let h=(tip.height as f32*scale).round().max(1.0) as u32;
    let resized=image::imageops::resize(&gray,w,h,image::imageops::FilterType::Triangle);
    TipMask{width:w,height:h,data:resized.into_raw()}
}

fn icon_from_image(img:DynamicImage)->IconImage{
    let rgba=img.thumbnail(200,200).to_rgba8();
    IconImage{width:rgba.width(),height:rgba.height(),rgba:rgba.into_raw()}
}
fn load_icon(path:&Path)->Option<IconImage>{image::open(path).ok().map(icon_from_image)}
fn load_tip(path:&Path)->Option<TipMask>{
    let img=image::open(path).ok()?.to_luma8();
    Some(TipMask{width:img.width(),height:img.height(),data:img.into_raw()})
}

fn elliptical_tip(size:u32,hardness:f32,ratio:f32,angle_deg:f32)->TipMask{
    let size=size.max(8);let r=size as f32/2.0;let hard=hardness.clamp(0.0,0.999);
    let ratio=ratio.clamp(0.05,1.0);let a=angle_deg.to_radians();let (sin,cos)=a.sin_cos();
    let mut data=Vec::with_capacity((size*size) as usize);
    for y in 0..size {for x in 0..size {
        let dx=(x as f32+0.5-r)/r;let dy=(y as f32+0.5-r)/r;
        let xr=dx*cos+dy*sin;let yr=(-dx*sin+dy*cos)/ratio;
        let d=(xr*xr+yr*yr).sqrt();
        let alpha=if d<=hard{1.0}else{((1.0-d)/(1.0-hard)).clamp(0.0,1.0)};
        data.push((alpha*255.0).round() as u8);
    }}
    TipMask{width:size,height:size,data}
}

fn diamond_tip(size:u32,hardness:f32)->TipMask{
    let size=size.max(8);
    let hard=hardness.clamp(0.0,0.999);
    let mut data=Vec::with_capacity((size*size) as usize);
    for y in 0..size {for x in 0..size {
        let fx=((x as f32+0.5)/size as f32*2.0-1.0).abs();
        let fy=((y as f32+0.5)/size as f32*2.0-1.0).abs();
        let d=(fx+fy).clamp(0.0,2.0);
        let a=if d<=hard{1.0}else{((1.0-d)/(1.0-hard)).clamp(0.0,1.0)};
        data.push((a*255.0).round() as u8);
    }}
    TipMask{width:size,height:size,data}
}

fn square_tip(size:u32,hardness:f32)->TipMask{
    let size=size.max(8);
    let hard=hardness.clamp(0.0,0.999);
    let mut data=Vec::with_capacity((size*size) as usize);
    for y in 0..size { for x in 0..size {
        let fx=((x as f32+0.5)/size as f32*2.0-1.0).abs();
        let fy=((y as f32+0.5)/size as f32*2.0-1.0).abs();
        let d=fx.max(fy);
        let a=if d<=hard{1.0}else{((1.0-d)/(1.0-hard)).clamp(0.0,1.0)};
        data.push((a*255.0).round() as u8);
    }}
    TipMask{width:size,height:size,data}
}

fn blender_external_image(blend_path:&Path,raw:&str)->Option<(DynamicImage,PathBuf)>{
    let raw=raw.trim_matches(char::from(0)).trim();
    if raw.is_empty(){return None}
    let parent=blend_path.parent().unwrap_or(Path::new("."));
    let path=if let Some(relative)=raw.strip_prefix("//") {
        parent.join(relative.replace('\\',"/"))
    } else {
        let candidate=PathBuf::from(raw);
        if candidate.is_absolute(){candidate}else{parent.join(candidate)}
    };
    image::open(&path).ok().map(|image|(image,path))
}

fn round_tip(size:u32,hardness:f32)->TipMask{
    let size=size.max(8);let r=size as f32/2.0;let hard=hardness.clamp(0.0,0.999);
    let mut data=Vec::with_capacity((size*size) as usize);
    for y in 0..size {for x in 0..size {
        let dx=(x as f32+0.5-r)/r;let dy=(y as f32+0.5-r)/r;let d=(dx*dx+dy*dy).sqrt();
        let a=if d<=hard{1.0}else{((1.0-d)/(1.0-hard)).clamp(0.0,1.0)};
        data.push((a*255.0).round() as u8);
    }}
    TipMask{width:size,height:size,data}
}

fn save_tip_png(tip:&TipMask,path:&Path)->Result<(),String>{
    let gray=GrayImage::from_raw(tip.width,tip.height,tip.data.clone()).ok_or("Invalid tip image")?;
    gray.save_with_format(path,ImageFormat::Png).map_err(|e|e.to_string())
}
fn save_tip_preview_png(tip:&TipMask,path:&Path)->Result<(),String>{
    let mut rgba=Vec::with_capacity(tip.data.len()*4);
    for &a in &tip.data {rgba.extend_from_slice(&[20,20,20,a]);}
    let img=RgbaImage::from_raw(tip.width,tip.height,rgba).ok_or("Invalid tip preview")?;
    img.save_with_format(path,ImageFormat::Png).map_err(|e|e.to_string())
}
fn save_icon_png(icon:&IconImage,path:&Path)->Result<(),String>{
    let img=RgbaImage::from_raw(icon.width,icon.height,icon.rgba.clone()).ok_or("Invalid icon")?;
    img.save_with_format(path,ImageFormat::Png).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blender_asset_filter_policy_prefers_explicit_assets(){
        let keep=|has_assets:bool,is_asset:bool|!has_assets||is_asset;
        assert!(keep(false,false));
        assert!(keep(false,true));
        assert!(!keep(true,false));
        assert!(keep(true,true));
    }

    #[test]
    fn blender_pack_filter_prefers_custom_icons_when_asset_metadata_is_missing(){
        let keep=|has_assets:bool,is_asset:bool,has_custom_icons:bool,has_custom_icon:bool|{
            (!has_assets||is_asset) && (has_assets||!has_custom_icons||has_custom_icon)
        };
        assert!(keep(false,false,false,false));
        assert!(keep(false,false,true,true));
        assert!(!keep(false,false,true,false));
        assert!(keep(true,true,true,false));
        assert!(!keep(true,false,true,true));
    }

    #[test]
    fn supported_extensions_cover_requested_apps() {
        for ext in ["kpp","bundle","myb","gbr","gih","vbr","abr","sut","sutg","blend","brush","brushset","zip","afbrushes","brushpack"] {
            assert!(accepted_extensions().contains(&ext),"{ext}");
        }
    }

    fn temp_brush_path(ext: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("stick-import-{}-{stamp}.{ext}", std::process::id()))
    }

    fn archive_fixture(ext: &str, members: &[(&str, &[u8])]) -> PathBuf {
        use std::io::Write;
        let path = temp_brush_path(ext);
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for &(name, bytes) in members {
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn mixed_zip_preserves_abr_myb_and_named_image_tips() {
        // One legacy sampled ABR tip, 2 by 2 pixels at 8-bit depth.
        let mut record = vec![0; 4+2+1+8]; // misc, spacing, antialias and short bounds (v1)
        for bound in [0i32, 0, 2, 2] { record.extend(bound.to_be_bytes()); }
        record.extend(8u16.to_be_bytes());
        record.extend([0, 0, 64, 128, 255]); // raw compression and four mask pixels
        let mut abr = vec![0, 1, 0, 1, 0, 2]; // version, count, sampled entry
        abr.extend((record.len() as u32).to_be_bytes());
        abr.extend(record);
        let myb = br#"{"settings":{"radius_logarithmic":{"base_value":1.5}}}"#;
        let gray = b"P5\n2 1\n255\n\0\xff";
        let file = archive_fixture("zip", &[
            ("Presets/Ink.ABR", &abr), ("Presets/Soft.MYB", myb),
            ("Tips/Thin.PGM", gray), ("__MACOSX/Tips/._Thin.PGM", gray),
            ("Tips/Wide.PGM", gray), ("Presets/Bad.MYB", b"{}"),
        ]);
        let brushes = import_file(&file).unwrap();
        fs::remove_file(&file).unwrap();
        assert_eq!(brushes.len(), 4);
        assert!(brushes.iter().all(|b| b.source_path == file));
        assert!(brushes[0].source_format.contains(".abr"));
        assert!(!brushes[0].tips[0].data.is_empty());
        assert_eq!(brushes[1].suggested_name, "Soft");
        assert!((brushes[1].default_size - 2.0 * 1.5f32.exp()).abs() < 0.001);
        assert_eq!(brushes[2].suggested_name, "Thin");
        assert_eq!(brushes[2].tips[0].data, [255, 0]);
        assert_eq!(brushes[3].suggested_name, "Wide");
    }

    #[test]
    fn procreate_uses_the_shape_mask_without_importing_previews_or_grain() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_raw(3, 1, vec![0, 96, 255]).unwrap());
        let mut png = Cursor::new(Vec::new());
        img.write_to(&mut png, ImageFormat::Png).unwrap();
        let bytes = png.get_ref().as_slice();
        let file = archive_fixture("brushset", &[
            ("Ink/QuickLook/Thumbnail.png", bytes), ("Ink/Grain.png", bytes), ("Ink/Shape.png", bytes),
        ]);
        let brushes = import_file(&file).unwrap();
        fs::remove_file(file).unwrap();
        assert_eq!(brushes.len(), 1);
        assert_eq!(brushes[0].suggested_name, "Ink");
        assert_eq!(brushes[0].tips[0].data, [0, 96, 255]);
    }

    #[test]
    fn additional_image_types_work_as_files_and_packed_tips() {
        let pixels = image::RgbImage::from_raw(2, 1, vec![0,0,0, 255,255,255]).unwrap();
        for ext in ["tif", "tiff", "tga", "gif"] {
            assert!(accepted_extensions().contains(&ext));
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::ImageRgb8(pixels.clone()).write_to(&mut bytes, tip_image_format(ext).unwrap()).unwrap();
            let single = temp_brush_path(ext);
            fs::write(&single, bytes.get_ref()).unwrap();
            let packed = archive_fixture("zip", &[(&format!("Tips/Test.{ext}"), bytes.get_ref())]);
            for source in [single, packed] {
                let drafts = import_file(&source).unwrap();
                fs::remove_file(source).unwrap();
                assert_eq!(drafts[0].tips[0].data, [255,0], "{ext}");
            }
        }
    }

    #[test]
    fn a_krita_thumbnail_alone_is_never_installed_as_a_brush() {
        let path = temp_brush_path("kpp");
        DynamicImage::new_luma8(16, 16).save_with_format(&path, ImageFormat::Png).unwrap();
        let result = import_file(&path);
        fs::remove_file(path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn vbr_generated_circle_parses() {
        let text="GIMP-VBR\n1.0\nSoft Dot\n25\n16\n0.75\n1.0\n15\n";
        let draft=parse_vbr_text(text,Path::new("soft.vbr")).unwrap();
        assert_eq!(draft.suggested_name,"Soft Dot");
        assert!((draft.default_size-32.0).abs()<0.01);
        assert!((draft.spacing-0.25).abs()<0.001);
        assert_eq!(draft.tips.len(),1);
    }

    #[test]
    fn vbr_shaped_diamond_parses() {
        let text="GIMP-VBR\n1.5\nDiamond\nsquare\n20\n12\n2\n0.8\n1.0\n0\n";
        let draft=parse_vbr_text(text,Path::new("diamond.vbr")).unwrap();
        assert_eq!(draft.tips[0].width,128);
        assert!(draft.compatibility_note.contains("square"));
    }

    #[test]
    fn embedded_png_scanner_finds_multiple_images() {
        let a=DynamicImage::ImageRgba8(RgbaImage::from_pixel(12,12,image::Rgba([0,0,0,255])));
        let b=DynamicImage::ImageRgba8(RgbaImage::from_pixel(16,16,image::Rgba([255,255,255,255])));
        let mut pa=Cursor::new(Vec::new());
        let mut pb=Cursor::new(Vec::new());
        a.write_to(&mut pa,ImageFormat::Png).unwrap();
        b.write_to(&mut pb,ImageFormat::Png).unwrap();
        let mut blob=b"vendor-header".to_vec();
        blob.extend(pa.into_inner());
        blob.extend(b"middle");
        blob.extend(pb.into_inner());
        assert_eq!(find_png_images(&blob,8).len(),2);
    }

    #[test]
    fn mypaint_pressure_mappings_do_not_collapse_to_one_default() {
        let json=r#"{"settings":{
            "opaque":{"base_value":0.7,"inputs":{"pressure":[[0,0.2],[1,1.0]]}},
            "radius_logarithmic":{"base_value":2.0,"inputs":{}},
            "hardness":{"base_value":0.4},
            "dabs_per_basic_radius":{"base_value":2.0},
            "elliptical_dab_ratio":{"base_value":0.5},
            "elliptical_dab_angle":{"base_value":30.0},
            "offset_by_random":{"base_value":0.2}
        }}"#;
        let parsed:serde_json::Value=serde_json::from_str(json).unwrap();
        let settings=parsed.get("settings").unwrap();
        assert!(settings.get("opaque").unwrap().get("inputs").unwrap().get("pressure").is_some());
        assert!(settings.get("radius_logarithmic").unwrap().get("inputs").unwrap().get("pressure").is_none());
    }

    #[test]
    fn custom_id_prefix_is_project_safe() {
        let stroke=Stroke{
            points:vec![],color:[0,0,0,255],fill_color:None,base_width:10.0,is_vector:false,
            brush_preset:crate::project::KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:Some("custom:abc123".into()),tip_shape:crate::project::BrushTipShape::Round,
        };
        assert_eq!(stroke_custom_id(&stroke),Some("abc123"));
    }

    #[test]
    fn gbr_v2_gray_tip_decodes() {
        let name=b"test\0";
        let header=(28+name.len()) as u32;
        let mut data=Vec::new();
        for v in [header,2,2,1,1,u32::from_be_bytes(*b"GIMP"),25] {data.extend(v.to_be_bytes());}
        data.extend(name);data.extend([10,240]);
        let (n,tip,spacing,_)=parse_gbr(&data,0).unwrap();
        assert_eq!(n,"test");assert_eq!((tip.width,tip.height),(2,1));
        assert_eq!(tip.data,vec![10,240]);assert_eq!(spacing,25);
    }

    #[test]
    fn krita_kpp_uses_brush_definition_spacing_not_spacingvalue_multiplier() {
        let xml=r#"<Preset paintopid="paintbrush" name="GDquest_Ink_Sketch">
            <param name="PressureSize" type="internal">true</param>
            <param name="SizeSensor" type="string"><![CDATA[<!DOCTYPE params><params id="pressure"/>]]></param>
            <param name="PressureOpacity" type="internal">true</param>
            <param name="OpacitySensor" type="string"><![CDATA[<!DOCTYPE params><params id="pressure"/>]]></param>
            <param name="PressureScatter" type="internal">false</param>
            <param name="ScatterValue" type="internal">1</param>
            <param name="Scattering/Amount" type="internal">1</param>
            <param name="SpacingValue" type="internal">1</param>
            <param name="brush_definition" type="string"><![CDATA[
                <Brush type="auto_brush" spacing="0.1" angle="0" scale="1">
                    <MaskGenerator ratio="1" hfade="0.9" vfade="0.9" type="circle" diameter="11"/>
                </Brush>
            ]]></param>
        </Preset>"#;
        let draft=parse_krita_preset_xml(xml,None,Path::new("GDquest_Ink_Sketch.kpp"),|_|None).unwrap();
        assert!((draft.spacing-0.1).abs()<0.0001,"spacing={}",draft.spacing);
        assert_eq!(draft.scatter,0.0,"disabled Krita scatter must stay disabled");
        assert_eq!(draft.size_pressure,1.0);
        assert_eq!(draft.opacity_pressure,1.0);
        assert_eq!(draft.tips.len(),1);
    }

    #[test]
    fn krita_predefined_tip_is_not_silently_replaced_with_round_fallback() {
        let xml=r#"<Preset paintopid="paintbrush" name="Water_Waves">
            <param name="PressureSize" type="internal">true</param>
            <param name="SizeSensor" type="string"><![CDATA[<!DOCTYPE params><params id="pressure"/>]]></param>
            <param name="requiredBrushFile" type="string"><![CDATA[LJF_Water_Brush_03.gih]]></param>
            <param name="brush_definition" type="string"><![CDATA[
                <Brush type="gbr_brush" filename="LJF_Water_Brush_03.gih" spacing="0.1" scale="2.80406"/>
            ]]></param>
        </Preset>"#;
        let err=parse_krita_preset_xml(xml,None,Path::new("water.kpp"),|_|None).unwrap_err();
        assert!(err.contains("LJF_Water_Brush_03.gih"));
        assert!(err.contains("fake round brush"));
    }

    #[test]
    fn krita_predefined_gbr_resource_preserves_tip_and_scale() {
        let name=b"water\0";
        let header=(28+name.len()) as u32;
        let mut gbr=Vec::new();
        for v in [header,2,2,1,1,u32::from_be_bytes(*b"GIMP"),10] {
            gbr.extend(v.to_be_bytes());
        }
        gbr.extend(name);
        gbr.extend([12,240]);

        let xml=r#"<Preset paintopid="paintbrush" name="Resource_Brush">
            <param name="requiredBrushFile" type="string">water.gbr</param>
            <param name="brush_definition" type="string"><![CDATA[
                <Brush type="gbr_brush" filename="water.gbr" spacing="0.1" scale="2.5"/>
            ]]></param>
        </Preset>"#;
        let draft=parse_krita_preset_xml(xml,None,Path::new("resource.kpp"),|wanted|{
            (wanted=="water.gbr").then(||gbr.clone())
        }).unwrap();
        assert_eq!(draft.tips.len(),1);
        assert_eq!(draft.tips[0].data,vec![12,240]);
        assert!((draft.tip_scale-2.5).abs()<0.001);
        assert!((draft.spacing-0.1).abs()<0.001);
    }

    #[test]
    fn imported_krita_ten_percent_spacing_is_dense_not_beads() {
        let meta=CustomBrushMeta{
            id:"test".into(),name:"test".into(),source_format:"Krita .kpp".into(),
            source_path:String::new(),icon_path:String::new(),tip_paths:vec![],
            default_size:30.0,
            spacing:0.1,opacity:1.0,flow:1.0,size_pressure:0.0,opacity_pressure:0.0,
            angle_deg:0.0,follow_stroke:false,scatter:0.0,random_tip:false,tip_scale:1.0,
            render_mode:CustomBrushRenderMode::Continuous,
            engine_version:CUSTOM_BRUSH_ENGINE_VERSION,
            disabled_reason:String::new(),
            compatibility_note:String::new(),
        };
        let brush=RuntimeBrush{meta,tips:vec![round_tip(64,0.9)],icon:None};
        let stroke=Stroke{
            points:vec![
                crate::project::StrokePoint{x:0.0,y:0.0,pressure:1.0},
                crate::project::StrokePoint{x:100.0,y:0.0,pressure:1.0},
            ],
            color:[0,0,0,255],fill_color:None,base_width:20.0,is_vector:false,
            brush_preset:crate::project::KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:Some("custom:test".into()),tip_shape:crate::project::BrushTipShape::Round,
        };
        let mut centers=Vec::new();
        emit_stamps(&stroke,&brush,None,|stamp|centers.push(stamp.center[0]));
        assert!(centers.len()>40,"only {} stamps",centers.len());
        let max_gap=centers.windows(2).map(|w|w[1]-w[0]).fold(0.0_f32,f32::max);
        assert!(max_gap<=2.1,"visible bead gap: {max_gap}");
    }

    #[test]
    fn csp_effector_requires_actual_pressure_bit() {
        let mut blob=vec![0u8;44];
        blob[0..4].copy_from_slice(&(44u32.to_be_bytes()));
        // Non-empty blob with tilt only must NOT become pressure.
        blob[8..12].copy_from_slice(&(0x20i32.to_be_bytes()));
        let tilt=Value::Blob(blob.clone());
        assert!(!csp_effector_uses_pressure(Some(&tilt)));
        blob[8..12].copy_from_slice(&(0x10i32.to_be_bytes()));
        let pressure=Value::Blob(blob);
        assert!(csp_effector_uses_pressure(Some(&pressure)));
    }

    #[test]
    fn old_custom_brush_metadata_defaults_tip_scale_to_one() {
        let json=r#"{
          "id":"old","name":"old","source_format":"Krita .kpp","source_path":"",
          "icon_path":"","tip_paths":[],"spacing":0.1,"opacity":1.0,"flow":1.0,
          "size_pressure":1.0,"opacity_pressure":0.0,"angle_deg":0.0,
          "follow_stroke":false,"scatter":0.0,"random_tip":false
        }"#;
        let meta:CustomBrushMeta=serde_json::from_str(json).unwrap();
        assert_eq!(meta.tip_scale,1.0);
        assert_eq!(meta.default_size,30.0);
        assert_eq!(meta.render_mode,CustomBrushRenderMode::Stamp);
        assert_eq!(meta.engine_version,0);
        assert!(meta.disabled_reason.is_empty());
    }

    #[test]
    fn continuous_external_brush_preserves_vendor_spacing_at_large_sizes() {
        let meta=CustomBrushMeta{
            id:"continuous".into(),name:"continuous".into(),source_format:"Krita .kpp".into(),
            source_path:String::new(),icon_path:String::new(),tip_paths:vec![],
            default_size:30.0,
            spacing:0.10,opacity:1.0,flow:1.0,size_pressure:0.0,opacity_pressure:0.0,
            angle_deg:0.0,follow_stroke:false,scatter:0.0,random_tip:false,tip_scale:1.0,
            render_mode:CustomBrushRenderMode::Continuous,
            engine_version:CUSTOM_BRUSH_ENGINE_VERSION,disabled_reason:String::new(),
            compatibility_note:String::new(),
        };
        let brush=RuntimeBrush{meta,tips:vec![round_tip(64,0.9)],icon:None};
        let stroke=Stroke{
            points:vec![
                crate::project::StrokePoint{x:0.0,y:0.0,pressure:1.0},
                crate::project::StrokePoint{x:200.0,y:0.0,pressure:1.0},
            ],
            color:[0,0,0,255],fill_color:None,base_width:100.0,is_vector:false,
            brush_preset:crate::project::KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:Some("custom:continuous".into()),
            tip_shape:crate::project::BrushTipShape::Round,
        };
        let mut centers=Vec::new();
        emit_stamps(&stroke,&brush,None,|stamp|centers.push(stamp.center[0]));
        let max_gap=centers.windows(2).map(|w|w[1]-w[0]).fold(0.0_f32,f32::max);
        // A 10% vendor spacing at 100px diameter should remain about 10px.
        // That still gives ~90% overlap, but no longer explodes into redundant
        // 2.5-4% dabs that made imported brushes heavy and too similar.
        assert!(max_gap<=10.1,"vendor spacing mapping regressed: {max_gap}");
        assert!(max_gap>=8.0,"spacing collapsed back into oversampling: {max_gap}");
    }

    #[test]
    fn round_tip_is_bounded_and_visible() {
        let t=round_tip(64,0.8);
        assert_eq!(t.data.len(),64*64);
        assert!(t.data.iter().any(|&a|a==255));
        assert!(t.data.iter().any(|&a|a==0));
    }
}
