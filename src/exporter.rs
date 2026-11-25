use crate::project::{AniProject, BrushTipShape};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, RgbaImage};
use rayon::prelude::*;
use std::fs;
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::process::{Command, Stdio};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VideoExportFormat {
    Mp4,
    Mov,
    Avi,
    Mkv,
    Webm,
    GifInstant,
}

impl VideoExportFormat {
    pub fn ext(&self) -> &'static str {
        match self {
            VideoExportFormat::Mp4 => "mp4",
            VideoExportFormat::Mov => "mov",
            VideoExportFormat::Avi => "avi",
            VideoExportFormat::Mkv => "mkv",
            VideoExportFormat::Webm => "webm",
            VideoExportFormat::GifInstant => "gif",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VideoExportFormat::Mp4 => "MP4 (H.264 - الأكثر انتشاراً)",
            VideoExportFormat::Mov => "MOV (QuickTime)",
            VideoExportFormat::Avi => "AVI (Windows Video)",
            VideoExportFormat::Mkv => "MKV (Matroska)",
            VideoExportFormat::Webm => "WebM (VP9 - شفافية/ويب)",
            VideoExportFormat::GifInstant => "GIF متحرك (فوري - بدون الحاجة لـ FFmpeg!)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageExportFormat {
    PngTransparent,
    PngSolid,
    Jpg,
    Webp,
    Bmp,
    Tiff,
}

impl ImageExportFormat {
    pub fn ext(&self) -> &'static str {
        match self {
            ImageExportFormat::PngTransparent | ImageExportFormat::PngSolid => "png",
            ImageExportFormat::Jpg => "jpg",
            ImageExportFormat::Webp => "webp",
            ImageExportFormat::Bmp => "bmp",
            ImageExportFormat::Tiff => "tiff",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ImageExportFormat::PngTransparent => "PNG (خلفية شفافة - Transparent)",
            ImageExportFormat::PngSolid => "PNG (بخلفية المسرح - High Quality)",
            ImageExportFormat::Jpg => "JPG / JPEG (صورة مضغوطة)",
            ImageExportFormat::Webp => "WebP (صيغة الويب الحديثة)",
            ImageExportFormat::Bmp => "BMP (Bitmap غير مضغوط)",
            ImageExportFormat::Tiff => "TIFF (جودة طباعة عالية)",
        }
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, ImageExportFormat::PngTransparent | ImageExportFormat::Webp)
    }
}

pub fn find_ffmpeg_executable() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let local_ffmpeg = exe.with_file_name("ffmpeg.exe");
        if local_ffmpeg.exists() {
            return Some(local_ffmpeg);
        }
    }
    if Command::new("ffmpeg").arg("-version").output().is_ok() {
        return Some(PathBuf::from("ffmpeg"));
    }
    let common_paths = [
        r"C:\ffmpeg\bin\ffmpeg.exe",
        r"C:\Program Files\ffmpeg\bin\ffmpeg.exe",
        r"C:\Program Files (x86)\ffmpeg\bin\ffmpeg.exe",
    ];
    for p in common_paths {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    None
}

pub fn ensure_ffmpeg_installed(target_dir: &Path) -> Result<PathBuf, String> {
    if let Some(existing) = find_ffmpeg_executable() {
        return Ok(existing);
    }
    let target_exe = target_dir.join("ffmpeg.exe");
    if target_exe.exists() {
        return Ok(target_exe);
    }

    let url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
    let resp = ureq::get(url)
        .set("User-Agent", "Stick-Studio")
        .call()
        .map_err(|e| format!("فشل الاتصال لتحميل FFmpeg: {}", e))?;

    let mut zip_bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut zip_bytes)
        .map_err(|e| format!("خطأ أثناء تحميل FFmpeg: {}", e))?;

    let cursor = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("فشل فك ضغط FFmpeg: {}", e))?;

    fs::create_dir_all(target_dir).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        if let Ok(mut file) = archive.by_index(i) {
            if file.name().ends_with("bin/ffmpeg.exe") || file.name().ends_with("bin\\ffmpeg.exe") {
                let mut out_file = fs::File::create(&target_exe).map_err(|e| format!("فشل حفظ ffmpeg.exe: {}", e))?;
                std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
                return Ok(target_exe);
            }
        }
    }

    Err("لم يتم العثور على ffmpeg.exe داخل الحزمة المحملة.".to_string())
}

pub fn render_frame_pixmap(project: &AniProject, frame_idx: usize, transparent_bg: bool, bg_rgba: [u8; 4]) -> Pixmap {
    let w = project.width.max(1);
    let h = project.height.max(1);
    let mut pixmap = Pixmap::new(w, h).unwrap();

    if !transparent_bg {
        pixmap.fill(Color::from_rgba8(bg_rgba[0], bg_rgba[1], bg_rgba[2], 255));
    }

    // Composite every layer only once with its opacity. This ensures that
    // overlapping strokes in a layer are not individually dimmed again.
    for layer in project.layers.iter().rev() {
        if !layer.visible || layer.is_guide {
            continue;
        }
        let layer_alpha = if layer.opacity.is_finite() {
            layer.opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        if layer_alpha <= 0.0 {
            continue;
        }

        let fd = project.evaluate_layer_at_frame(layer, frame_idx);
        if fd.media_items.is_empty() && fd.strokes.is_empty()
            && fd.text_items.is_empty() && fd.symbol_instances.is_empty() {
            continue;
        }
        // Fully opaque layers can be drawn directly onto the destination.
        // Only partially transparent layers need a separate full-canvas buffer.
        let mut layer_buffer = (layer_alpha < 1.0)
            .then(|| Pixmap::new(w, h).expect("Canvas dimensions already checked"));
        let layer_pixmap = layer_buffer.as_mut().unwrap_or(&mut pixmap);

        render_layer_contents(project, layer, &fd, frame_idx, layer_pixmap);

        if let Some(layer_pixmap) = layer_buffer {
            pixmap.draw_pixmap(
                0,
                0,
                layer_pixmap.as_ref(),
                &tiny_skia::PixmapPaint {
                    opacity: layer_alpha,
                    ..tiny_skia::PixmapPaint::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    pixmap
}

// Shared layer rasterization for export and the editor's group-opacity preview.
fn render_layer_contents(
    project: &AniProject,
    layer: &crate::project::Layer,
    fd: &crate::project::FrameData,
    frame_idx: usize,
    layer_pixmap: &mut Pixmap,
) {
    for m in &fd.media_items {
        if let Ok(img) = image::open(&m.path) {
            let mut resized = img
                .resize_exact(
                    m.width.max(1.0) as u32,
                    m.height.max(1.0) as u32,
                    image::imageops::FilterType::Triangle,
                )
                .to_rgba8();
            if layer.effect != crate::project::LayerEffectPreset::None {
                for pixel in resized.pixels_mut() {
                    pixel.0=layer.effect.apply_rgba(pixel.0);
                }
            }
            if let Some(mut sub_pix) = Pixmap::new(resized.width(), resized.height()) {
                copy_straight_rgba_to_pixmap(&resized, &mut sub_pix);
                let w = resized.width() as f32;
                let h = resized.height() as f32;
                let x_scale = if m.flip_h { -1.0 } else { 1.0 };
                let y_scale = if m.flip_v { -1.0 } else { 1.0 };
                let transform = Transform::from_translate(-w * 0.5, -h * 0.5)
                    .post_scale(x_scale, y_scale)
                    .post_rotate(m.rotation_deg)
                    .post_translate(m.x + m.width * 0.5, m.y + m.height * 0.5);
                layer_pixmap.draw_pixmap(
                    0,
                    0,
                    sub_pix.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    transform,
                    None,
                );
            }
        }
    }

    for s in &fd.strokes {
        if layer.effect == crate::project::LayerEffectPreset::None {
            draw_skia_stroke(layer_pixmap, s, Transform::identity());
        } else {
            let mut effected = s.clone();
            effected.color = layer.effect.apply_rgba(effected.color);
            effected.fill_color = effected.fill_color.map(|c| layer.effect.apply_rgba(c));
            draw_skia_stroke(layer_pixmap, &effected, Transform::identity());
        }
    }

    // Text must be present in every export, not only in the editor preview.
    for item in &fd.text_items {
        if layer.effect == crate::project::LayerEffectPreset::None {
            draw_export_text(layer_pixmap, item);
        } else {
            let mut effected = item.clone();
            effected.color = layer.effect.apply_rgba(effected.color);
            draw_export_text(layer_pixmap, &effected);
        }
    }

    for inst in &fd.symbol_instances {
        if let Some(sym) = project.library_symbols.get(&inst.symbol_name) {
            let sym_strokes = project.evaluate_symbol_strokes(sym, frame_idx);
            let ts = Transform::from_translate(-inst.anchor_offset_x, -inst.anchor_offset_y)
                .post_scale(inst.scale, inst.scale)
                .post_rotate(inst.rotation_deg)
                .post_translate(inst.x + inst.anchor_offset_x, inst.y + inst.anchor_offset_y);
            for s in sym_strokes {
                if layer.effect == crate::project::LayerEffectPreset::None {
                    draw_skia_stroke(layer_pixmap, s, ts);
                } else {
                    let mut effected = s.clone();
                    effected.color = layer.effect.apply_rgba(effected.color);
                    effected.fill_color = effected.fill_color.map(|c| layer.effect.apply_rgba(c));
                    draw_skia_stroke(layer_pixmap, &effected, ts);
                }
            }
        }
    }

}

pub(crate) fn render_layer_rgba(
    project: &AniProject,
    layer: &crate::project::Layer,
    fd: &crate::project::FrameData,
    frame_idx: usize,
) -> Option<RgbaImage> {
    let mut pixmap = Pixmap::new(project.width.max(1), project.height.max(1))?;
    render_layer_contents(project, layer, fd, frame_idx, &mut pixmap);
    Some(pixmap_to_rgba_image(&pixmap))
}

/// Load one local system font that can draw both Arabic presentation forms
/// and Latin letters. The font file is not bundled or redistributed.
fn export_text_font() -> Option<&'static fontdue::Font> {
    static FONT: OnceLock<Option<fontdue::Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        for path in [
            r"C:\Windows\Fonts\tahoma.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\segoeui.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    return Some(font);
                }
            }
        }
        None
    }).as_ref()
}

fn draw_export_text_unrotated(pixmap: &mut Pixmap, item: &crate::project::TextItem) {
    let Some(font) = export_text_font() else { return; };
    if item.content.trim().is_empty() || !item.font_size.is_finite() { return; }
    use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
    let shaped = crate::fix_ar(&item.content);
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: item.x, y: item.y,
        ..LayoutSettings::default()
    });
    layout.append(&[font], &TextStyle::new(&shaped, item.font_size.clamp(4.0, 512.0), 0));
    let (width, height) = (pixmap.width() as i32, pixmap.height() as i32);
    let bytes = pixmap.data_mut();
    for glyph in layout.glyphs() {
        let (metrics, bitmap) = font.rasterize_config(glyph.key);
        for y in 0..metrics.height {
            let py = glyph.y.floor() as i32 + y as i32;
            if py < 0 || py >= height { continue; }
            for x in 0..metrics.width {
                let px = glyph.x.floor() as i32 + x as i32;
                if px < 0 || px >= width { continue; }
                let coverage = bitmap[y * metrics.width + x] as f32 / 255.0;
                let src_alpha = coverage * (item.color[3] as f32 / 255.0);
                if src_alpha <= 0.0 { continue; }
                let target = (py as usize * width as usize + px as usize) * 4;
                let retain = 1.0 - src_alpha;
                for channel in 0..3 {
                    bytes[target + channel] = (item.color[channel] as f32 * src_alpha
                        + bytes[target + channel] as f32 * retain).round().clamp(0.0, 255.0) as u8;
                }
                bytes[target + 3] = (255.0 * src_alpha
                    + bytes[target + 3] as f32 * retain).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn draw_export_text(pixmap: &mut Pixmap, item: &crate::project::TextItem) {
    let angle = if item.rotation_deg.is_finite() { item.rotation_deg } else { 0.0 };
    if angle.abs() < 0.001 {
        draw_export_text_unrotated(pixmap, item);
        return;
    }
    let Some(mut temp) = Pixmap::new(pixmap.width(), pixmap.height()) else { return };
    draw_export_text_unrotated(&mut temp, item);
    let transform = Transform::from_translate(-item.x, -item.y)
        .post_rotate(angle)
        .post_translate(item.x, item.y);
    pixmap.draw_pixmap(0, 0, temp.as_ref(), &tiny_skia::PixmapPaint::default(), transform, None);
}

fn fill_skia_polygon(
    pixmap: &mut Pixmap, mut points: impl Iterator<Item=(f32,f32)>, paint: &Paint, ts: Transform,
) {
    let Some((x,y)) = points.next() else { return };
    let mut path = PathBuilder::new();
    path.move_to(x,y);
    for (x,y) in points { path.line_to(x,y); }
    path.close();
    if let Some(path) = path.finish() {
        pixmap.fill_path(&path,paint,tiny_skia::FillRule::Winding,ts,None);
    }
}

fn fill_skia_stroke_region(
    pixmap:&mut Pixmap, s:&crate::project::Stroke, paint:&Paint, ts:Transform,
) {
    let rings=crate::fill_tools::split_fill_rings(&s.points);
    if rings.is_empty() {return;}
    let mut path=PathBuilder::new();
    for ring in rings {
        let mut iter=ring.iter();
        let Some(first)=iter.next() else {continue};
        path.move_to(first.x,first.y);
        for p in iter {path.line_to(p.x,p.y);}
        path.close();
    }
    if let Some(path)=path.finish() {
        pixmap.fill_path(&path,paint,tiny_skia::FillRule::EvenOdd,ts,None);
    }
}

fn draw_skia_stroke(pixmap: &mut Pixmap, s: &crate::project::Stroke, ts: Transform) {
    use crate::brush_engine::{visit_round_marks, RoundMark};
    let Some(first) = s.points.first() else { return };
    let alpha = (s.color[3] as f32 * s.opacity.clamp(0.0,1.0)) as u8;
    let mut paint = Paint::default();
    paint.anti_alias = true;
    if let Some(fc) = s.fill_color.filter(|_| s.points.len() >= 3) {
        paint.set_color_rgba8(fc[0], fc[1], fc[2], (fc[3] as f32 * s.opacity.clamp(0.0,1.0)) as u8);
        fill_skia_stroke_region(pixmap,s,&paint,ts);
    }
    if alpha == 0 { return; }
    if crate::custom_brush::export(pixmap,s,alpha,ts) {
        return;
    }
    if let Some(texture) = crate::texture_brush::TextureBrush::for_stroke(s) {
        texture.export(pixmap, s, alpha, ts);
        return;
    }
    // Non-round raster tips now flow through the shared continuous-stroke engine.
    // This keeps canvas preview and PNG/video export on the same geometry path.
    let mut centerline = None;
    visit_round_marks(s, |mark, flow| {
        paint.set_color_rgba8(s.color[0], s.color[1], s.color[2], (alpha as f32 * flow) as u8);
        match mark {
            RoundMark::Outline(contour) => fill_skia_polygon(pixmap, contour.into_iter(), &paint, ts),
            RoundMark::Dot(dab) => {
                let mut pb = PathBuilder::new();
                pb.push_circle(dab.x,dab.y,dab.radius);
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path,&paint,tiny_skia::FillRule::Winding,ts,None);
                }
            }
            RoundMark::Line { width } => {
                let path = centerline.get_or_insert_with(|| {
                    let mut pb = PathBuilder::new();
                    if s.is_vector || s.brush_preset == crate::project::KritaBrushPreset::Fineliner {
                        let path = if s.is_vector
                            && s.brush_preset == crate::project::KritaBrushPreset::SmoothInk
                        {
                            crate::vector_path::smoothed_ink_path(&s.points)
                        } else {
                            crate::vector_path::smoothed_path(&s.points)
                        };
                        if let Some(&(x,y)) = path.first() {
                            pb.move_to(x,y);
                            for &(x,y) in path.iter().skip(1) { pb.line_to(x,y); }
                        }
                    } else if s.brush_preset == crate::project::KritaBrushPreset::GPenInk {
                        let path=crate::brush_engine::smooth_krita_2d_points(&s.points,width);
                        if let Some(first)=path.first() {
                            pb.move_to(first.x,first.y);
                            for p in path.iter().skip(1) {pb.line_to(p.x,p.y);}
                        }
                    } else if s.brush_preset == crate::project::KritaBrushPreset::SmoothInk {
                        let path = crate::brush_engine::smooth_ink_points(&s.points, width);
                        if let Some(first) = path.first() {
                            pb.move_to(first.x,first.y);
                            for p in path.iter().skip(1) { pb.line_to(p.x,p.y); }
                        }
                    } else {
                        let path = crate::brush_engine::smooth_freehand_points(&s.points, width);
                        if let Some(first) = path.first() {
                            pb.move_to(first.x,first.y);
                            for p in path.iter().skip(1) { pb.line_to(p.x,p.y); }
                        }
                    }
                    pb.finish()
                });
                if let Some(path) = path {
                    let brush = Stroke {
                        width,
                        line_cap: if s.is_vector && s.tip_shape == BrushTipShape::Square {
                            tiny_skia::LineCap::Square
                        } else if !s.is_vector && s.tip_shape != BrushTipShape::Round {
                            tiny_skia::LineCap::Butt
                        } else {
                            tiny_skia::LineCap::Round
                        },
                        line_join: tiny_skia::LineJoin::Round, ..Stroke::default()
                    };
                    pixmap.stroke_path(path,&paint,&brush,ts,None);

                    if !s.is_vector && s.tip_shape != BrushTipShape::Round && s.points.len() >= 2 {
                        let first=&s.points[0];
                        let second=&s.points[1];
                        let last=&s.points[s.points.len()-1];
                        let before_last=&s.points[s.points.len()-2];
                        for cap in [
                            crate::brush_engine::raster_tip_cap_polygon(
                                (first.x,first.y),(second.x,second.y),width,s.tip_shape),
                            crate::brush_engine::raster_tip_cap_polygon(
                                (last.x,last.y),(before_last.x,before_last.y),width,s.tip_shape),
                        ] {
                            if cap.len()>=3 {
                                fill_skia_polygon(pixmap,cap.into_iter(),&paint,ts);
                            }
                        }
                    }
                }
            }
        }
    });
}

// tiny-skia accepts premultiplied pixels; decoded PNG/media pixels are straight RGBA.
fn copy_straight_rgba_to_pixmap(image: &RgbaImage, pixmap: &mut Pixmap) {
    for (src, dst) in image.as_raw().chunks_exact(4).zip(pixmap.data_mut().chunks_exact_mut(4)) {
        let alpha = u16::from(src[3]);
        for c in 0..3 {
            dst[c] = ((u16::from(src[c]) * alpha + 127) / 255) as u8;
        }
        dst[3] = src[3];
    }
}

fn pixmap_to_rgba_image(pixmap: &Pixmap) -> RgbaImage {
    let w = pixmap.width();
    let h = pixmap.height();
    let mut raw_rgba = Vec::with_capacity((w * h * 4) as usize);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        raw_rgba.push(c.red());
        raw_rgba.push(c.green());
        raw_rgba.push(c.blue());
        raw_rgba.push(c.alpha());
    }
    RgbaImage::from_raw(w, h, raw_rgba).unwrap_or_else(|| RgbaImage::new(w, h))
}

pub fn export_single_image_format(
    project: &AniProject,
    frame_idx: usize,
    out_path: &Path,
    format: ImageExportFormat,
    bg_rgba: [u8; 4],
) -> Result<(), String> {
    let pixmap = render_frame_pixmap(project, frame_idx, format.is_transparent(), bg_rgba);
    if matches!(format, ImageExportFormat::PngTransparent | ImageExportFormat::PngSolid) {
        // tiny-skia writes PNG using straight RGBA (including transparent pixels).
        return pixmap.save_png(out_path).map_err(|e| e.to_string());
    }
    let img_buf = pixmap_to_rgba_image(&pixmap);
    let dyn_img = image::DynamicImage::ImageRgba8(img_buf);

    match format {
        ImageExportFormat::PngTransparent | ImageExportFormat::PngSolid => unreachable!(),
        ImageExportFormat::Jpg => {
            image::DynamicImage::ImageRgb8(dyn_img.to_rgb8())
                .save_with_format(out_path, ImageFormat::Jpeg)
                .map_err(|e| e.to_string())
        }
        ImageExportFormat::Webp => {
            dyn_img.save_with_format(out_path, ImageFormat::WebP).map_err(|e| e.to_string())
        }
        ImageExportFormat::Bmp => {
            image::DynamicImage::ImageRgb8(dyn_img.to_rgb8())
                .save_with_format(out_path, ImageFormat::Bmp)
                .map_err(|e| e.to_string())
        }
        ImageExportFormat::Tiff => {
            dyn_img.save_with_format(out_path, ImageFormat::Tiff).map_err(|e| e.to_string())
        }
    }
}

pub fn export_transparent_png_sequence(project: &AniProject, folder: &Path) -> Result<usize, String> {
    fs::create_dir_all(folder).map_err(|e| format!("Failed to create folder: {}", e))?;
    let total = project.total_frames.max(1);
    let frames: Vec<usize> = (1..=total).collect();

    frames.par_iter().try_for_each(|&f| {
        let pixmap = render_frame_pixmap(project, f, true, [255, 255, 255, 255]);
        let file_path = folder.join(format!("stick_frame_{:04}.png", f));
        pixmap.save_png(&file_path).map_err(|e| format!("PNG save error: {}", e))
    })?;

    Ok(total)
}

pub fn export_sprite_sheet(project: &AniProject, out_path: &Path) -> Result<usize, String> {
    let total = project.total_frames.max(1);
    let fw = project.width.max(1);
    let fh = project.height.max(1);

    let frames: Vec<RgbaImage> = (1..=total)
        .into_par_iter()
        .map(|f| {
            let pix = render_frame_pixmap(project, f, project.background_transparent, project.background_color);
            pixmap_to_rgba_image(&pix)
        })
        .collect();

    let mut sheet = RgbaImage::new(fw * (total as u32), fh);
    for (idx, frame_img) in frames.iter().enumerate() {
        image::imageops::overlay(&mut sheet, frame_img, (idx as i64) * (fw as i64), 0);
    }
    sheet.save_with_format(out_path, ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(total)
}

pub fn export_lottie_json(project: &AniProject, out_path: &Path) -> Result<(), String> {
    let total = project.total_frames.max(1);
    let fps = project.fps.max(1);
    let mut layers_json = Vec::new();

    for (l_idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible || layer.is_guide {
            continue;
        }
        let fd = project.evaluate_layer_at_frame(layer, 1);
        let mut shapes = Vec::new();
        for s in &fd.strokes {
            if s.points.len() < 2 {
                continue;
            }
            let vertices: Vec<[f32; 2]> = s.points.iter().map(|p| [p.x, p.y]).collect();
            let in_tangents: Vec<[f32; 2]> = vec![[0.0, 0.0]; vertices.len()];
            let out_tangents: Vec<[f32; 2]> = vec![[0.0, 0.0]; vertices.len()];
            shapes.push(serde_json::json!({
                "ty": "gr",
                "it": [
                    {
                        "ty": "sh",
                        "ks": {
                            "a": 0,
                            "k": {
                                "i": in_tangents,
                                "o": out_tangents,
                                "v": vertices,
                                "c": false
                            }
                        }
                    },
                    {
                        "ty": "st",
                        "c": { "a": 0, "k": [s.color[0] as f32 / 255.0, s.color[1] as f32 / 255.0, s.color[2] as f32 / 255.0, 1.0] },
                        "o": { "a": 0, "k": 100 },
                        "w": { "a": 0, "k": s.base_width },
                        "lc": 2,
                        "lj": 2
                    }
                ]
            }));
        }

        layers_json.push(serde_json::json!({
            "ddd": 0,
            "ind": l_idx + 1,
            "ty": 4,
            "nm": layer.name,
            "sr": 1,
            "ks": {
                "o": { "a": 0, "k": 100 },
                "r": { "a": 0, "k": 0 },
                "p": { "a": 0, "k": [0, 0, 0] },
                "a": { "a": 0, "k": [0, 0, 0] },
                "s": { "a": 0, "k": [100, 100, 100] }
            },
            "shapes": shapes,
            "ip": 0,
            "op": total,
            "st": 0
        }));
    }

    let lottie = serde_json::json!({
        "v": "5.7.4",
        "fr": fps,
        "ip": 0,
        "op": total,
        "w": project.width,
        "h": project.height,
        "nm": "Stick_Animation",
        "ddd": 0,
        "assets": [],
        "layers": layers_json
    });

    let serialized = serde_json::to_string_pretty(&lottie).map_err(|e| e.to_string())?;
    fs::write(out_path, serialized).map_err(|e| e.to_string())
}

// A held drawing only needs to be rendered once. Be conservative for projects
// with motion/shape tweens or nested animated symbols: render each frame.
fn export_frame_runs(project: &AniProject) -> Vec<(usize, usize)> {
    let total = project.total_frames.max(1);
    let dynamic = project.library_symbols.values().any(|s| !s.nested_keyframes.is_empty())
        || project.layers.iter().filter(|l| l.visible && !l.is_guide).any(|l| {
            l.keyframes.values().any(|f| f.motion_tween || f.tween_mode != crate::project::TweenMode::None)
        });
    if dynamic {
        return (1..=total).map(|frame| (frame, 1)).collect();
    }
    let mut starts = std::collections::BTreeSet::new();
    starts.insert(1);
    for layer in project.layers.iter().filter(|l| l.visible && !l.is_guide) {
        starts.extend(layer.keyframes.keys().copied().filter(|&f| f >= 1 && f <= total));
    }
    let starts: Vec<usize> = starts.into_iter().collect();
    starts.iter().enumerate().map(|(i, &start)| {
        let end = starts.get(i + 1).copied().unwrap_or(total.saturating_add(1));
        (start, end.saturating_sub(start).max(1))
    }).collect()
}

// Keep parallel rendering bounded by memory rather than retaining an entire
// animation in RAM. Budget includes the frame plus temporary compositing space.
fn export_batch_size(project: &AniProject) -> usize {
    let bytes = u64::from(project.width.max(1)) * u64::from(project.height.max(1)) * 8;
    let budget = 64 * 1024 * 1024u64;
    ((budget / bytes.max(1)).max(1) as usize)
        .min(rayon::current_num_threads().max(1))
        .min(8)
}

pub fn export_instant_gif(project: &AniProject, out_path: &Path, bg_rgba: [u8; 4]) -> Result<(), String> {
    let fps = project.fps.max(1);
    let delay_ms = (1000 / fps).max(15);
    let file = BufWriter::new(fs::File::create(out_path).map_err(|e| e.to_string())?);
    let mut encoder = GifEncoder::new_with_speed(file, 10);
    encoder.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;
    let runs = export_frame_runs(project);
    for batch in runs.chunks(export_batch_size(project)) {
        let images: Vec<RgbaImage> = batch.par_iter().map(|&(frame, _)| {
            pixmap_to_rgba_image(&render_frame_pixmap(project, frame, false, bg_rgba))
        }).collect();
        for (img, &(_, repeats)) in images.into_iter().zip(batch) {
            for _ in 0..repeats.saturating_sub(1) {
                let frame = Frame::from_parts(img.clone(), 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
                encoder.encode_frame(frame).map_err(|e| e.to_string())?;
            }
            let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            encoder.encode_frame(frame).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn export_video_multi_format(
    project: &AniProject,
    output_path: &Path,
    format: VideoExportFormat,
    bg_rgba: [u8; 4],
) -> Result<(), String> {
    if format == VideoExportFormat::GifInstant {
        return export_instant_gif(project, output_path, bg_rgba);
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let ffmpeg_bin = ensure_ffmpeg_installed(&exe_dir)?;
    let fps = project.fps.max(1).to_string();
    let size = format!("{}x{}", project.width.max(1), project.height.max(1));
    let transparent = format == VideoExportFormat::Webm && project.background_transparent;

    // Stream raw frames directly to the encoder: no intermediate PNG
    // compression/decompression, no temporary frame directory.
    let mut cmd = Command::new(&ffmpeg_bin);
    cmd.args(["-y", "-hide_banner", "-loglevel", "error", "-f", "rawvideo",
        "-pixel_format", "rgba", "-video_size", &size, "-framerate", &fps,
        "-i", "pipe:0", "-an"]);
    // Make the RGB -> YUV matrix and output metadata agree. Keep 4:2:0
    // compatibility for common video players; PNG remains the exact RGB output.
    let even_bt709 = "pad=ceil(iw/2)*2:ceil(ih/2)*2,scale=out_color_matrix=bt709:out_range=tv";
    let bt709 = "scale=out_color_matrix=bt709:out_range=tv";
    match format {
        VideoExportFormat::Mp4 | VideoExportFormat::Mov | VideoExportFormat::Mkv => {
            cmd.args(["-vf", even_bt709, "-c:v", "libx264", "-preset", "ultrafast",
                "-crf", "18", "-pix_fmt", "yuv420p", "-colorspace", "bt709",
                "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv"]);
            if format == VideoExportFormat::Mov { cmd.args(["-f", "mov"]); }
        }
        VideoExportFormat::Avi => {
            // AVI players often ignore color metadata and assume BT.601.
            // Use that matrix for this legacy container to avoid a color shift.
            cmd.args(["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2,scale=out_color_matrix=bt601:out_range=tv",
                "-c:v", "mpeg4", "-q:v", "3", "-pix_fmt", "yuv420p",
                "-colorspace", "smpte170m", "-color_primaries", "smpte170m",
                "-color_trc", "smpte170m", "-color_range", "tv"]);
        }
        VideoExportFormat::Webm => {
            cmd.args(["-vf", bt709, "-c:v", "libvpx-vp9", "-deadline", "realtime",
                "-pix_fmt", "yuva420p", "-colorspace", "bt709", "-color_primaries", "bt709",
                "-color_trc", "bt709", "-color_range", "tv"]);
        }
        VideoExportFormat::GifInstant => unreachable!(),
    }
    cmd.arg(output_path).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("FFmpeg launch error: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("FFmpeg stdin unavailable")?;
    let mut stderr = child.stderr.take().ok_or("FFmpeg stderr unavailable")?;
    // Drain stderr concurrently so an encoder error can never block the pipe.
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let write_result: Result<(), String> = (|| {
        let runs = export_frame_runs(project);
        for batch in runs.chunks(export_batch_size(project)) {
            let rendered: Vec<Pixmap> = batch.par_iter().map(|&(frame, _)| {
                render_frame_pixmap(project, frame, transparent, bg_rgba)
            }).collect();
            for (pixmap, &(_, repeats)) in rendered.into_iter().zip(batch) {
                // An opaque Pixmap already contains straight RGB because alpha=255.
                // Transparent WebM needs explicit unpremultiplication.
                let straight = transparent.then(|| pixmap_to_rgba_image(&pixmap));
                let bytes = straight.as_ref().map(|img| img.as_raw().as_slice())
                    .unwrap_or_else(|| pixmap.data());
                for _ in 0..repeats {
                    stdin.write_all(bytes).map_err(|e| format!("FFmpeg frame write error: {e}"))?;
                }
            }
        }
        Ok(())
    })();
    drop(stdin);
    if write_result.is_err() { let _ = child.kill(); }
    let status = child.wait().map_err(|e| format!("FFmpeg wait error: {e}"))?;
    let stderr = stderr_reader.join().unwrap_or_default();
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr);
        if !detail.trim().is_empty() {
            return Err(format!("فشل التصدير: {}", detail.trim()));
        }
        return write_result.and(Err(format!("FFmpeg exited with {status}")));
    }
    write_result
}

pub fn extract_video_frames(video_path: &Path, fps: u32) -> Result<Vec<PathBuf>, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let ffmpeg_bin = ensure_ffmpeg_installed(&exe_dir)?;

    let out_dir = std::env::temp_dir().join(format!("stick_vid_{}", std::process::id()));
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let pattern = out_dir.join("vid_%04d.png");
    let status = Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-i",
            video_path.to_str().unwrap_or(""),
            "-vf",
            &format!("fps={}", fps.max(1)),
            pattern.to_str().unwrap_or(""),
        ])
        .status()
        .map_err(|_| "Failed to execute FFmpeg.".to_string())?;

    if !status.success() {
        return Err("Failed to extract video frames.".to_string());
    }

    let mut files: Vec<PathBuf> = fs::read_dir(&out_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod layer_opacity_tests {
    use super::*;
    use crate::project::{KritaBrushPreset, Stroke as ProjectStroke, StrokePoint};

    fn project_with_solid_dab(opacity: f32) -> AniProject {
        let mut project = AniProject::default();
        project.width = 48;
        project.height = 48;
        project.layers[0].opacity = opacity;
        project.layers[0]
            .keyframes
            .get_mut(&1)
            .unwrap()
            .strokes
            .push(ProjectStroke {
                points: vec![StrokePoint { x: 24.0, y: 24.0, pressure: 1.0 }],
                color: [240, 30, 20, 255],
                fill_color: None,
                base_width: 16.0,
                is_vector: false,
                brush_preset: KritaBrushPreset::GPenInk,
                opacity: 1.0,
                brush_texture_id: None,
            tip_shape: BrushTipShape::Round,
            });
        project
    }

    fn center_alpha(project: &AniProject) -> u8 {
        render_frame_pixmap(project, 1, true, [0, 0, 0, 0])
            .pixel(24, 24)
            .expect("center pixel")
            .alpha()
    }

    #[test]
    fn marker_preserves_clear_pixels_between_parallel_and_return_passes() {
        let mut project = project_with_solid_dab(1.0);
        project.width = 104;
        project.height = 68;
        let strokes = &mut project.layers[0].keyframes.get_mut(&1).unwrap().strokes;
        let mut marker = strokes.pop().unwrap();
        marker.brush_preset = KritaBrushPreset::ChiselMarker;
        marker.base_width = 12.0;
        marker.opacity = 0.4;
        for path in [
            &[(12.,12.), (92.,12.)][..],
            &[(92.,25.), (12.,25.)],
            &[(12.,42.), (92.,42.), (92.,57.), (12.,57.)],
        ] {
            marker.points = path.iter().map(|&(x,y)| StrokePoint { x, y, pressure: 1.0 }).collect();
            strokes.push(marker.clone());
        }
        let image = render_frame_pixmap(&project, 1, true, [0;4]);
        for x in 18..80 {
            for y in [12,25,42,57] {
                assert!((98..=106).contains(&image.pixel(x,y).unwrap().alpha()));
            }
            for y in [18,33,49] {
                assert_eq!(image.pixel(x,y).unwrap().alpha(), 0, "gap at {x},{y}");
            }
        }
    }

    #[test]
    fn textured_brushes_keep_fast_strokes_and_survive_project_roundtrip() {
        use crate::persistence::{atomic_save, migration, sniff};
        let mut previous_pixels = Vec::new();
        for preset in [KritaBrushPreset::DryBrush, KritaBrushPreset::Chalk,
            KritaBrushPreset::Charcoal, KritaBrushPreset::SoftPastel] {
            let mut project = project_with_solid_dab(1.0);
            project.width = 220;
            let stroke = &mut project.layers[0].keyframes.get_mut(&1).unwrap().strokes[0];
            stroke.brush_preset = preset;
            stroke.points[0].x = 8.0;
            stroke.points[0].pressure = 0.35;
            stroke.points.push(StrokePoint { x: 210.0, y: 24.0, pressure: 1.0 });
            let image = render_frame_pixmap(&project, 1, true, [0;4]);
            for x in 8..210 {
                assert!((12..36).any(|y| image.pixel(x,y).unwrap().alpha() > 0),
                    "{preset:?} left column {x} empty");
            }
            let ink_at = |x| (10..38).map(|y| u32::from(image.pixel(x,y).unwrap().alpha())).sum::<u32>();
            assert!(ink_at(185) > ink_at(24));
            let bytes = atomic_save::serialize_and_compress_project(&project).unwrap();
            let (loaded, _) = migration::decode_and_migrate(sniff::sniff_project_bytes(&bytes).unwrap()).unwrap();
            let reloaded = render_frame_pixmap(&loaded, 1, true, [0;4]);
            assert_eq!(image.data(), reloaded.data());
            assert_ne!(image.data(), previous_pixels.as_slice());
            previous_pixels = image.data().to_owned();
        }
    }

    #[test]
    fn textured_brushes_preserve_selected_non_black_color_in_export() {
        for preset in [
            KritaBrushPreset::Watercolor,
            KritaBrushPreset::SoftAirbrush,
            KritaBrushPreset::SketchPencil,
            KritaBrushPreset::SoftSketchPencil,
            KritaBrushPreset::GraphitePencil,
        ] {
            let mut project = project_with_solid_dab(1.0);
            project.width = 128;
            project.height = 72;
            let stroke = &mut project.layers[0].keyframes.get_mut(&1).unwrap().strokes[0];
            stroke.color = [28, 154, 232, 255];
            stroke.opacity = 1.0;
            stroke.base_width = 18.0;
            stroke.brush_preset = preset;
            stroke.points[0].x = 10.0;
            stroke.points[0].y = 36.0;
            stroke.points.push(StrokePoint { x: 118.0, y: 36.0, pressure: 1.0 });

            let rendered = render_frame_pixmap(&project, 1, true, [0; 4]);
            let rgba = pixmap_to_rgba_image(&rendered);
            let colored_pixels: Vec<_> = rgba.pixels()
                .filter(|p| p.0[3] > 20)
                .collect();
            assert!(!colored_pixels.is_empty(), "{preset:?} produced no colored pixels");
            assert!(
                colored_pixels.iter().any(|p|
                    (i16::from(p.0[0]) - 28).abs() <= 2
                    && (i16::from(p.0[1]) - 154).abs() <= 2
                    && (i16::from(p.0[2]) - 232).abs() <= 2),
                "{preset:?} lost the selected RGB color"
            );
        }
    }

    #[test]
    fn watercolor_and_airbrush_export_the_translucent_outer_wash() {
        for preset in [KritaBrushPreset::Watercolor, KritaBrushPreset::SoftAirbrush] {
            let mut project = project_with_solid_dab(1.0);
            let stroke = &mut project.layers[0].keyframes.get_mut(&1).unwrap().strokes[0];
            stroke.brush_preset = preset;
            stroke.points[0].x = 6.0;
            stroke.points.push(StrokePoint { x: 42.0, y: 24.0, pressure: 1.0 });
            let rendered = render_frame_pixmap(&project, 1, true, [0;4]);
            let halo = rendered.pixel(24,33).unwrap().alpha();
            assert!(halo > 0 && halo < rendered.pixel(24,24).unwrap().alpha());
        }
    }

    #[test]
    fn project_background_color_and_transparency_render_correctly() {
        let mut project = AniProject::default();
        project.width = 12;
        project.height = 12;
        project.background_color = [24, 112, 208, 255];
        let solid = render_frame_pixmap(&project, 1, false, project.background_color);
        assert_eq!(solid.pixel(6, 6).unwrap().demultiply().red(), 24);
        assert_eq!(solid.pixel(6, 6).unwrap().demultiply().green(), 112);
        assert_eq!(solid.pixel(6, 6).unwrap().demultiply().blue(), 208);

        project.background_transparent = true;
        let transparent = render_frame_pixmap(&project, 1, true, project.background_color);
        assert_eq!(transparent.pixel(6, 6).unwrap().alpha(), 0);
    }

    fn straight_media_rgba_round_trips_without_darkening() {
        let image = RgbaImage::from_raw(2, 1, vec![240, 80, 40, 128, 10, 200, 150, 255]).unwrap();
        let mut pixmap = Pixmap::new(2, 1).unwrap();
        copy_straight_rgba_to_pixmap(&image, &mut pixmap);
        assert_eq!(&pixmap.data()[..4], &[120, 40, 20, 128]);
        let decoded = pixmap_to_rgba_image(&pixmap);
        for (a,b) in decoded.as_raw().iter().zip(image.as_raw()) {
            assert!((i16::from(*a) - i16::from(*b)).abs() <= 1);
        }
    }

    #[test]
    fn held_frames_reuse_render_without_changing_duration() {
        let mut project = project_with_solid_dab(1.0);
        project.total_frames = 12;
        project.layers[0].keyframes.insert(5, crate::project::FrameData::default());
        assert_eq!(export_frame_runs(&project), vec![(1, 4), (5, 8)]);
        project.layers[0].keyframes.get_mut(&1).unwrap().motion_tween = true;
        assert_eq!(export_frame_runs(&project), (1..=12).map(|f| (f, 1)).collect::<Vec<_>>());
    }

    #[test]
    fn opaque_and_transparent_png_keep_solid_brush_rgb() {
        let project = project_with_solid_dab(1.0);
        for transparent in [false, true] {
            let pixmap = render_frame_pixmap(&project, 1, transparent, [255; 4]);
            let png = pixmap.encode_png().unwrap();
            let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
            assert_eq!(decoded.get_pixel(24,24).0, [240,30,20,255]);
        }
    }

    #[test]
    fn layer_opacity_changes_export_alpha() {
        assert_eq!(center_alpha(&project_with_solid_dab(0.0)), 0);
        assert_eq!(center_alpha(&project_with_solid_dab(1.0)), 255);
        let half = center_alpha(&project_with_solid_dab(0.5));
        assert!((125..=130).contains(&half), "expected about 128, got {half}");
    }

    #[test]
    fn latin_and_arabic_text_export_uses_the_local_font_when_present() {
        if export_text_font().is_none() { return; }
        let mut project = AniProject::default();
        project.width = 200;
        project.height = 120;
        project.layers[0].keyframes.get_mut(&1).unwrap().text_items.push(
            crate::project::TextItem {
                content: "Hello أهلاً".to_owned(),
                x: 12.0, y: 16.0,
                font_size: 32.0, color: [0, 0, 0, 255], rotation_deg: 0.0,
            }
        );
        let rendered = render_frame_pixmap(&project, 1, true, [0,0,0,0]);
        assert!(rendered.data().chunks_exact(4).any(|p| p[3] != 0));
    }

    #[test]
    fn rotated_text_is_present_in_export() {
        if export_text_font().is_none() { return; }
        let mut project = AniProject::default();
        project.width = 180;
        project.height = 180;
        project.layers[0].keyframes.get_mut(&1).unwrap().text_items.push(
            crate::project::TextItem {
                content: "Rotate".into(), x: 70.0, y: 50.0,
                font_size: 28.0, color: [0,0,0,255], rotation_deg: 90.0,
            }
        );
        let rendered = render_frame_pixmap(&project, 1, true, [0,0,0,0]);
        assert!(rendered.data().chunks_exact(4).any(|p| p[3] != 0));
    }

    #[test]
    fn overlapping_strokes_do_not_square_layer_opacity() {
        let mut project = project_with_solid_dab(0.5);
        let duplicate = project.layers[0].keyframes[&1].strokes[0].clone();
        project.layers[0].keyframes.get_mut(&1).unwrap().strokes.push(duplicate);
        let alpha = center_alpha(&project);
        assert!((125..=130).contains(&alpha), "unexpected accumulated layer alpha: {alpha}");
    }

    #[test]
    fn hidden_and_guide_layers_do_not_export() {
        let mut project = project_with_solid_dab(1.0);
        project.layers[0].visible = false;
        assert_eq!(center_alpha(&project), 0);
        project.layers[0].visible = true;
        project.layers[0].is_guide = true;
        assert_eq!(center_alpha(&project), 0);
    }
}
