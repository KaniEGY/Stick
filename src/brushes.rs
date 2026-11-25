use crate::project::{BrushTipShape, KritaBrushPreset, Stroke};
use eframe::egui;

/// رسم أيقونة مخصصة لكل فرشة في لوحة الخصائص
pub fn draw_brush_preset_icon(painter: &egui::Painter, rect: egui::Rect, preset: KritaBrushPreset, active: bool) {
    let c = rect.center();
    let col = if active {
        egui::Color32::WHITE
    } else {
        match preset {
            KritaBrushPreset::GPenInk => egui::Color32::from_rgb(56, 189, 248),
            KritaBrushPreset::Watercolor => egui::Color32::from_rgb(34, 197, 94),
            KritaBrushPreset::ChiselMarker => egui::Color32::from_rgb(249, 115, 22),
            KritaBrushPreset::SoftAirbrush => egui::Color32::from_rgb(192, 132, 252),
            KritaBrushPreset::SketchPencil => egui::Color32::from_rgb(250, 204, 21),
            KritaBrushPreset::DryBrush => egui::Color32::from_rgb(225, 156, 102),
            KritaBrushPreset::Chalk => egui::Color32::from_rgb(196, 222, 226),
            KritaBrushPreset::Charcoal => egui::Color32::from_rgb(149, 165, 178),
            KritaBrushPreset::SoftPastel => egui::Color32::from_rgb(255, 171, 208),
            KritaBrushPreset::Fineliner => egui::Color32::from_rgb(131, 230, 250),
            KritaBrushPreset::SmoothInk => egui::Color32::from_rgb(79, 197, 241),
            KritaBrushPreset::TaperedPen => egui::Color32::from_rgb(240, 186, 101),
            KritaBrushPreset::Calligraphy => egui::Color32::from_rgb(232, 144, 226),
            KritaBrushPreset::SoftSketchPencil => egui::Color32::from_rgb(187, 200, 211),
            KritaBrushPreset::GraphitePencil => egui::Color32::from_rgb(176, 183, 191),
            KritaBrushPreset::SketchBrush => egui::Color32::from_rgb(239, 170, 92),
        }
    };

    match preset {
        KritaBrushPreset::GPenInk => {
            // سن قلم حبر معدني
            let pts = vec![
                c + egui::vec2(0.0, -9.0),
                c + egui::vec2(6.0, 2.0),
                c + egui::vec2(3.0, 8.0),
                c + egui::vec2(-3.0, 8.0),
                c + egui::vec2(-6.0, 2.0),
            ];
            painter.add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
        }
        KritaBrushPreset::Watercolor => {
            // قطرة ألوان مائية ناعمة
            painter.circle_filled(c + egui::vec2(0.0, 2.5), 6.0, col);
            let top = vec![
                c + egui::vec2(0.0, -8.5),
                c + egui::vec2(5.5, 1.5),
                c + egui::vec2(-5.5, 1.5),
            ];
            painter.add(egui::Shape::convex_polygon(top, col, egui::Stroke::NONE));
        }
        KritaBrushPreset::ChiselMarker => {
            // سن ماركر مشطوف عريض
            let pts = vec![
                c + egui::vec2(-6.0, 7.0),
                c + egui::vec2(2.0, -7.0),
                c + egui::vec2(7.0, -3.0),
                c + egui::vec2(-1.0, 9.0),
            ];
            painter.add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
        }
        KritaBrushPreset::SoftAirbrush => {
            // بخاخ رذاذ ناعم
            painter.circle_stroke(c, 7.0, egui::Stroke::new(1.5, col));
            painter.circle_filled(c, 3.0, col);
            painter.circle_filled(c + egui::vec2(-4.0, -3.0), 1.2, col);
            painter.circle_filled(c + egui::vec2(4.0, 3.0), 1.2, col);
        }
        KritaBrushPreset::SketchPencil => {
            // قلم رصاص خشبي
            let body = vec![
                c + egui::vec2(-6.0, 4.0),
                c + egui::vec2(4.0, -6.0),
                c + egui::vec2(7.0, -3.0),
                c + egui::vec2(-3.0, 7.0),
            ];
            painter.add(egui::Shape::convex_polygon(body, col, egui::Stroke::NONE));
            let tip = vec![
                c + egui::vec2(-6.0, 4.0),
                c + egui::vec2(-3.0, 7.0),
                c + egui::vec2(-8.0, 8.0),
            ];
            painter.add(egui::Shape::convex_polygon(tip, col, egui::Stroke::NONE));
        }
        KritaBrushPreset::DryBrush => {
            for i in 0..5 {
                let x = i as f32 * 2.5 - 5.0;
                painter.line_segment([c + egui::vec2(x-2.0, 7.0 - (i%2) as f32 * 3.0),
                    c + egui::vec2(x+2.0,-7.0)], egui::Stroke::new(1.6,col));
            }
        }
        KritaBrushPreset::Chalk => {
            painter.line_segment([c + egui::vec2(-4.0,5.0), c + egui::vec2(5.0,-6.0)],
                egui::Stroke::new(5.5,col));
            for (x,y) in [(-7.0,7.0),(-3.0,8.0)] {
                painter.circle_filled(c + egui::vec2(x,y),1.1,col);
            }
        }
        KritaBrushPreset::Charcoal => {
            for i in 0..7 {
                let d=i as f32 - 3.0;
                painter.line_segment([c+egui::vec2(d-4.0,8.0),c+egui::vec2(d+4.0,-8.0)],
                    egui::Stroke::new(if i%2==0 {1.6}else{2.5},col));
            }
        }
        KritaBrushPreset::SoftPastel => {
            painter.circle_filled(c,7.0,col);
            for i in 0..5 {
                let x = i as f32 * 3.0 - 6.0;
                painter.circle_filled(c+egui::vec2(x,5.0-(i%2) as f32*3.0),1.5,col);
            }
        }
        KritaBrushPreset::Fineliner => {
            painter.line_segment([c+egui::vec2(-7.,7.),c+egui::vec2(7.,-7.)],
                egui::Stroke::new(1.7,col));
            painter.circle_filled(c+egui::vec2(7.,-7.),1.8,col);
        }
        KritaBrushPreset::SmoothInk => {
            let pts=(0..20).map(|i| {
                let t=i as f32/19.;
                c+egui::vec2(-8.+t*16.,(t*std::f32::consts::TAU).sin()*4.)
            }).collect();
            painter.add(egui::Shape::line(pts,egui::Stroke::new(2.6,col)));
        }
        KritaBrushPreset::TaperedPen => {
            painter.add(egui::Shape::convex_polygon(vec![
                c+egui::vec2(-8.,8.),c+egui::vec2(-3.,-2.),
                c+egui::vec2(0.,-7.),c+egui::vec2(2.,-2.),
                c+egui::vec2(8.,-8.),c+egui::vec2(4.,4.)
            ],col,egui::Stroke::NONE));
        }
        KritaBrushPreset::Calligraphy => {
            painter.add(egui::Shape::convex_polygon(vec![
                c+egui::vec2(-8.,4.),c+egui::vec2(3.,-8.),
                c+egui::vec2(7.,-4.),c+egui::vec2(-4.,8.)
            ],col,egui::Stroke::NONE));
        }
        KritaBrushPreset::SoftSketchPencil => {
            for k in 0..3 {
                painter.line_segment([
                    c+egui::vec2(-8.+k as f32,7.),
                    c+egui::vec2(7.,-8.+k as f32)
                ],egui::Stroke::new(1.0,col));
            }
        }
        KritaBrushPreset::GraphitePencil => {
            painter.add(egui::Shape::convex_polygon(vec![
                c+egui::vec2(-7.,5.),c+egui::vec2(3.,-7.),
                c+egui::vec2(7.,-3.),c+egui::vec2(-3.,8.)
            ],col,egui::Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(vec![
                c+egui::vec2(-7.,5.),c+egui::vec2(-3.,8.),c+egui::vec2(-9.,9.)
            ],egui::Color32::from_gray(72),egui::Stroke::NONE));
            for (dx,dy) in [(-7.,-4.),(-2.,5.),(5.,5.)] {
                painter.circle_filled(c+egui::vec2(dx,dy),0.9,col);
            }
        }
        KritaBrushPreset::SketchBrush => {
            // Broad sketch brush: several close bristles, visibly different
            // from the narrow Sketch Pencil preset.
            for (offset,width) in [(-4.0,1.4),(-1.5,2.0),(1.5,1.6),(4.0,1.2)] {
                painter.line_segment([
                    c+egui::vec2(-8.0,5.5+offset*0.25),
                    c+egui::vec2(8.0,-5.5+offset*0.25)
                ],egui::Stroke::new(width,col));
            }
            painter.circle_filled(c+egui::vec2(-6.5,6.0),1.4,col);
        }
    }
}

/// Fast preview-only visibility check. Export is untouched. We must keep
/// a conservative margin for wide brush circles and anti-aliased edges.
/// Any invalid coordinate fails open to avoid hiding user artwork.
fn may_touch_clip<F: Fn(egui::Pos2) -> egui::Pos2>(
    stroke: &Stroke,
    ox: f32,
    oy: f32,
    scale: f32,
    clip: egui::Rect,
    to_screen: &F,
) -> bool {
    if stroke.points.is_empty() { return false; }
    if !scale.is_finite() || !clip.is_finite() { return true; }

    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in &stroke.points {
        if !p.x.is_finite() || !p.y.is_finite() { return true; }
        let q = to_screen(egui::pos2(p.x + ox, p.y + oy));
        if !q.x.is_finite() || !q.y.is_finite() { return true; }
        min.x = min.x.min(q.x);
        min.y = min.y.min(q.y);
        max.x = max.x.max(q.x);
        max.y = max.y.max(q.y);
    }
    let margin = (stroke.base_width.abs() * scale.abs() * 2.5 + 4.0).max(4.0);
    if !margin.is_finite() { return true; }
    let bounds = egui::Rect::from_min_max(
        min - egui::vec2(margin, margin),
        max + egui::vec2(margin, margin),
    );
    clip.intersects(bounds)
}

/// مصيّر الفرش الاحترافي:
/// - يرسم الألوان المائية والماركر والبخاخ كمسار واحد متصل (PathShape) بدون دوائر متداخلة أو نقاط متقطعة
/// - يزيل الذيل المدبب من G-Pen وقلم الفيكتور

#[derive(Clone)]
struct MarkerRasterCacheEntry {
    key:u64,
    texture:egui::TextureHandle,
    size:egui::Vec2,
}
#[derive(Clone,Default)]
struct MarkerRasterCache {
    entries:Vec<MarkerRasterCacheEntry>,
}

fn marker_has_hairpin(points:&[egui::Pos2],width:f32)->bool {
    if points.len()<3 || !width.is_finite() || width<=0.0 {return false;}
    let reach=(width*0.75).clamp(6.0,96.0);
    for i in 1..points.len()-1 {
        let center=points[i];
        let mut before=i-1;
        while before>0 && center.distance(points[before])<reach {before-=1;}
        let mut after=i+1;
        while after+1<points.len() && center.distance(points[after])<reach {after+=1;}
        let a=center-points[before];
        let b=points[after]-center;
        let la=a.length();
        let lb=b.length();
        if la>0.001 && lb>0.001 {
            let dot=(a/la).dot(b/lb);
            if dot < -0.42 {return true;}
        }
    }
    false
}

/// Detect loops/retracing without O(n^2) segment-pair scanning. Marker samples
/// that return within roughly half a nib width after travelling more than a nib
/// width are routed to the safe raster path. That covers circles, spirals,
/// retraced strokes and dense scribbles that cannot be represented by one
/// simple outline polygon.
fn marker_has_self_overlap(points:&[egui::Pos2],width:f32)->bool {
    if points.len()<6 || !width.is_finite() || width<=0.0 {return false;}
    use std::collections::HashMap;
    let cell=(width*0.55).clamp(4.0,96.0);
    let threshold=(width*0.48).clamp(2.5,80.0);
    let threshold_sq=threshold*threshold;
    let min_travel=(width*1.35).clamp(10.0,180.0);
    let mut grid:HashMap<(i32,i32),Vec<(egui::Pos2,f32)>>=HashMap::new();
    let mut travel=0.0f32;
    let mut previous:Option<egui::Pos2>=None;
    let sample_step=(width*0.10).clamp(1.5,6.0);
    let mut last_inserted=None::<egui::Pos2>;

    for &p in points {
        if !p.x.is_finite() || !p.y.is_finite(){continue;}
        if let Some(prev)=previous {
            let d=prev.distance(p);
            if d.is_finite(){travel+=d.max(0.0);}
        }
        previous=Some(p);
        if last_inserted.is_some_and(|q|q.distance(p)<sample_step){continue;}

        let gx=(p.x/cell).floor() as i32;
        let gy=(p.y/cell).floor() as i32;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(bucket)=grid.get(&(gx+dx,gy+dy)) {
                    if bucket.iter().any(|(q,old_travel)|
                        travel-*old_travel>min_travel && q.distance_sq(p)<threshold_sq)
                    {
                        return true;
                    }
                }
            }
        }
        grid.entry((gx,gy)).or_default().push((p,travel));
        last_inserted=Some(p);
    }
    false
}

fn marker_needs_safe_raster(points:&[egui::Pos2],width:f32)->bool {
    points.len()>4_000
        || marker_has_hairpin(points,width)
        || marker_has_self_overlap(points,width)
}

/// Cheap live marker preview. While the pointer is down we deliberately avoid
/// tessellating or rasterizing the entire growing stroke every frame. The final
/// committed stroke is rendered by the accurate marker pipeline after release.
pub fn draw_marker_live_preview<F:Fn(egui::Pos2)->egui::Pos2>(
    painter:&egui::Painter,
    stroke:&Stroke,
    scale:f32,
    to_screen:&F,
){
    if stroke.points.is_empty(){return;}
    let max_points=700usize;
    let stride=((stroke.points.len()+max_points-1)/max_points).max(1);
    let mut pts=Vec::with_capacity(stroke.points.len().min(max_points)+1);
    for (i,p) in stroke.points.iter().enumerate() {
        if i%stride==0 {
            pts.push(to_screen(egui::pos2(p.x,p.y)));
        }
    }
    if let Some(last)=stroke.points.last() {
        let p=to_screen(egui::pos2(last.x,last.y));
        if pts.last().is_none_or(|q|q.distance_sq(p)>0.01){pts.push(p);}
    }
    if pts.len()==1 {
        pts.push(pts[0]+egui::vec2(0.01,0.0));
    }
    let alpha=((stroke.color[3] as f32)*stroke.opacity.clamp(0.0,1.0)) as u8;
    if alpha==0{return;}
    let ink=egui::Color32::from_rgba_unmultiplied(
        stroke.color[0],stroke.color[1],stroke.color[2],alpha);
    let width=(stroke.base_width*0.68*scale).max(0.5);
    painter.add(egui::Shape::line(pts,egui::Stroke::new(width,ink)));
}

fn paint_marker_safe_raster(
    painter:&egui::Painter,
    points:&[egui::Pos2],
    width:f32,
    color:egui::Color32,
)->bool {
    if points.len()<2 || color.a()==0 || !width.is_finite() || width<=0.0 {return false;}

    let native_ppp=painter.ctx().pixels_per_point().clamp(1.0,2.0);
    let margin=(width*0.65+3.0).max(4.0);
    let mut min=egui::pos2(f32::INFINITY,f32::INFINITY);
    let mut max=egui::pos2(f32::NEG_INFINITY,f32::NEG_INFINITY);
    for &p in points {
        if !p.x.is_finite() || !p.y.is_finite() {continue;}
        min=min.min(p); max=max.max(p);
    }
    if !min.x.is_finite() || !max.x.is_finite() {return false;}
    let mut bounds=egui::Rect::from_min_max(
        min-egui::vec2(margin,margin),
        max+egui::vec2(margin,margin),
    );
    bounds=bounds.intersect(painter.clip_rect());
    if !bounds.is_positive(){return false;}

    // Hard memory budget: never let one marker preview allocate a giant
    // Pixmap + RGBA staging buffer + GPU texture. Downsample large strokes
    // rather than risking an out-of-memory process exit.
    const MAX_RASTER_PIXELS:f32=1_200_000.0;
    let point_area=(bounds.width().max(1.0)*bounds.height().max(1.0)).max(1.0);
    let budget_ppp=(MAX_RASTER_PIXELS/point_area).sqrt();
    let ppp=native_ppp.min(budget_ppp).clamp(0.22,native_ppp);
    let pw=(bounds.width()*ppp).ceil().max(1.0) as u32;
    let ph=(bounds.height()*ppp).ceil().max(1.0) as u32;
    if pw>3072 || ph>3072 || (pw as u64)*(ph as u64)>1_300_000 {return false;}

    use std::hash::{Hash,Hasher};
    let mut hasher=std::collections::hash_map::DefaultHasher::new();
    pw.hash(&mut hasher); ph.hash(&mut hasher);
    width.to_bits().hash(&mut hasher);
    ppp.to_bits().hash(&mut hasher);
    [color.r(),color.g(),color.b(),color.a()].hash(&mut hasher);
    for &p in points {
        let x=((p.x-bounds.min.x)*64.0).round() as i32;
        let y=((p.y-bounds.min.y)*64.0).round() as i32;
        x.hash(&mut hasher); y.hash(&mut hasher);
    }
    let key=hasher.finish();
    let cache_id=egui::Id::new("stick_marker_safe_raster_cache");
    let mut cache=painter.ctx().data(|d|
        d.get_temp::<MarkerRasterCache>(cache_id).unwrap_or_default());

    if let Some(entry)=cache.entries.iter().find(|e|e.key==key).cloned() {
        let rect=egui::Rect::from_min_size(bounds.min,entry.size);
        painter.image(
            entry.texture.id(),rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),
            egui::Color32::WHITE,
        );
        return true;
    }

    let Some(mut pixmap)=tiny_skia::Pixmap::new(pw,ph) else {return false;};
    let mut pb=tiny_skia::PathBuilder::new();
    let first=points[0];
    pb.move_to((first.x-bounds.min.x)*ppp,(first.y-bounds.min.y)*ppp);
    for &p in &points[1..] {
        pb.line_to((p.x-bounds.min.x)*ppp,(p.y-bounds.min.y)*ppp);
    }
    let Some(path)=pb.finish() else {return false;};

    // Rasterize coverage fully opaque first. Alpha is applied exactly once when
    // the resulting texture is composited onto the canvas, so retraced/hairpin
    // regions cannot become darker than the rest of the same marker stroke.
    let mut paint=tiny_skia::Paint::default();
    paint.anti_alias=true;
    paint.set_color_rgba8(255,255,255,255);
    let stroke=tiny_skia::Stroke{
        width:(width*ppp).max(0.5),
        line_cap:tiny_skia::LineCap::Butt,
        line_join:tiny_skia::LineJoin::Round,
        ..tiny_skia::Stroke::default()
    };
    pixmap.stroke_path(&path,&paint,&stroke,tiny_skia::Transform::identity(),None);

    let mut rgba=Vec::with_capacity((pw as usize)*(ph as usize)*4);
    // Color32 stores premultiplied RGB. The texture constructor expects straight
    // RGB, so recover it before applying the coverage alpha.
    let [r,g,b,a]=color.to_srgba_unmultiplied();
    for px in pixmap.data().chunks_exact(4) {
        let coverage=px[3] as u16;
        let alpha=((coverage*a as u16+127)/255) as u8;
        rgba.extend_from_slice(&[r,g,b,alpha]);
    }
    let image=egui::ColorImage::from_rgba_unmultiplied([pw as usize,ph as usize],&rgba);
    let texture=painter.ctx().load_texture(
        format!("stick_marker_safe_{key:016x}"),
        image,
        egui::TextureOptions::LINEAR,
    );
    let size=egui::vec2(pw as f32/ppp,ph as f32/ppp);
    let rect=egui::Rect::from_min_size(bounds.min,size);
    painter.image(
        texture.id(),rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),
        egui::Color32::WHITE,
    );

    cache.entries.push(MarkerRasterCacheEntry{key,texture,size});
    if cache.entries.len()>16 {
        let remove=cache.entries.len()-16;
        cache.entries.drain(0..remove);
    }
    painter.ctx().data_mut(|d|d.insert_temp(cache_id,cache));
    true
}

fn marker_strip_mesh(points: &[egui::Pos2], width: f32, color: egui::Color32) -> Option<egui::Mesh> {
    if points.len() < 2 || !width.is_finite() || width <= 0.0 || color.a() == 0 {
        return None;
    }

    // The previous marker renderer emitted one quad per centerline segment.
    // With a wide translucent nib, neighbouring quads overlap on curves because
    // the fitted centerline is sampled much more densely than the marker width.
    // Alpha blending then paints those overlap wedges several times, creating
    // the black triangles visible on the canvas.
    //
    // Build one *outline polygon* for the whole marker body and triangulate that
    // polygon once instead. Earcut handles concave bends without painting the
    // same join area repeatedly.
    let min_step = (width * 0.055).clamp(0.75, 6.0);
    let min_step_sq = min_step * min_step;
    let mut clean = Vec::with_capacity(points.len().min(10_000));
    for (index, &p) in points.iter().enumerate() {
        if !p.x.is_finite() || !p.y.is_finite() { continue; }
        let is_last = index + 1 == points.len();
        if clean.is_empty()
            || is_last
            || clean.last().is_none_or(|q: &egui::Pos2| q.distance_sq(p) >= min_step_sq)
        {
            if clean.last().is_none_or(|q: &egui::Pos2| q.distance_sq(p) > 0.01) {
                clean.push(p);
            }
        }
        // fill_render caps polygons at 20k vertices; the marker outline contains
        // two vertices per centerline point.
        if clean.len() >= 9_500 { break; }
    }
    if let Some(&last) = points.last() {
        if last.x.is_finite() && last.y.is_finite()
            && clean.last().is_none_or(|q| q.distance_sq(last) > 0.01)
            && clean.len() < 9_500
        {
            clean.push(last);
        }
    }
    if clean.len() < 2 { return None; }

    let radius = (width * 0.5).max(0.25);
    let dir = |a: egui::Pos2, b: egui::Pos2| {
        let d = b - a;
        let len = d.length();
        if len > 0.0001 { d / len } else { egui::vec2(1.0, 0.0) }
    };

    let mut left = Vec::with_capacity(clean.len());
    let mut right = Vec::with_capacity(clean.len());
    for i in 0..clean.len() {
        let prev = if i == 0 { dir(clean[0], clean[1]) } else { dir(clean[i - 1], clean[i]) };
        let next = if i + 1 == clean.len() { prev } else { dir(clean[i], clean[i + 1]) };
        let prev_n = egui::vec2(-prev.y, prev.x);
        let next_n = egui::vec2(-next.y, next.x);

        let (normal, miter) = if i == 0 {
            (next_n, 1.0)
        } else if i + 1 == clean.len() {
            (prev_n, 1.0)
        } else {
            let dot = prev.dot(next).clamp(-1.0, 1.0);
            if dot < -0.72 {
                // A near reversal cannot support a wide miter without folding.
                // Keep a bounded bevel-like cross section instead.
                (next_n, 1.0)
            } else {
                let sum = prev_n + next_n;
                let n = if sum.length_sq() > 0.0001 { sum.normalized() } else { next_n };
                let denom = n.dot(next_n).abs().max(0.20);
                (n, (1.0 / denom).min(1.30))
            }
        };

        let off = normal * (radius * miter);
        left.push(clean[i] + off);
        right.push(clean[i] - off);
    }

    let mut outline = Vec::with_capacity(clean.len() * 2);
    outline.extend(left);
    outline.extend(right.into_iter().rev());

    crate::fill_render::solid_polygon_mesh(&outline, color)
}

pub fn draw_stroke_pro<F: Fn(egui::Pos2) -> egui::Pos2>(
    painter: &egui::Painter,
    stroke: &Stroke,
    override_color: Option<[u8; 4]>,
    layer_opacity: f32,
    scale: f32,
    ox: f32,
    oy: f32,
    to_screen: &F,
) {
    if stroke.points.is_empty() {
        return;
    }

    // Large offscreen strokes used to allocate and paint thousands of dabs
    // on every UI frame. Cull conservatively before sampling/painting.
    if !may_touch_clip(stroke, ox, oy, scale, painter.clip_rect(), to_screen) {
        return;
    }

    let c = override_color.unwrap_or(stroke.color);
    let combined_opacity = stroke.opacity.clamp(0.0, 1.0) * layer_opacity.clamp(0.0, 1.0);
    let eff_alpha = ((c[3] as f32) * combined_opacity) as u8;

    // Draw any Bucket fill before the early-return G-Pen dab renderer.
    // The old order silently discarded fill colors on hand-drawn loops.
    if let Some(fc) = stroke.fill_color {
        let fill_c = override_color.unwrap_or(fc);
        let rings:Vec<Vec<egui::Pos2>>=crate::fill_tools::split_fill_rings(&stroke.points)
            .into_iter()
            .map(|ring|ring.iter()
                .map(|p|to_screen(egui::pos2(p.x+ox,p.y+oy))).collect())
            .collect();
        // Region fills may be concave and may contain nested closed contours.
        // Earcut receives those inner rings as holes so they stay independently
        // paintable instead of inheriting the outer Bucket color.
        if let Some(mesh) = crate::fill_render::solid_polygon_mesh_rings(
            &rings,
            egui::Color32::from_rgba_unmultiplied(
                fill_c[0], fill_c[1], fill_c[2],
                (fill_c[3] as f32 * combined_opacity) as u8,
            ),
        ) {
            painter.add(egui::Shape::mesh(mesh));
        }
    }

    // Optional fills are rendered above. The brush body must not create
    // an artificial halo when its own combined opacity is zero.
    if eff_alpha == 0 { return; }

    if crate::custom_brush::preview(
        painter,
        stroke,
        [c[0],c[1],c[2],eff_alpha],
        |x,y| to_screen(egui::pos2(x+ox,y+oy)),
    ) {
        return;
    }

    if let Some(texture) = crate::texture_brush::TextureBrush::for_stroke(stroke) {
        texture.preview(painter, stroke, [c[0],c[1],c[2],eff_alpha], |x,y|
            to_screen(egui::pos2(x+ox,y+oy)));
        return;
    }

    // Raster tip shapes are handled by the shared continuous-stroke engine.
    // Never stamp square/triangle polygons repeatedly along the path.

    use crate::brush_engine::{visit_round_marks, RoundMark};
    let screen = |x,y| to_screen(egui::pos2(x+ox,y+oy));
    let mut screen_pts: Option<Vec<egui::Pos2>> = None;
    visit_round_marks(stroke, |mark, flow| {
        let ink = egui::Color32::from_rgba_unmultiplied(c[0],c[1],c[2], (eff_alpha as f32 * flow) as u8);
        let width = match mark {
            RoundMark::Dot(dab) => {
                painter.circle_filled(screen(dab.x,dab.y), dab.radius*scale, ink);
                return;
            }
            RoundMark::Outline(outline) => {
                let path: Vec<_> = outline.into_iter().map(|(x,y)| screen(x,y)).collect();
                if let Some(mesh) = crate::fill_render::solid_polygon_mesh(&path, ink) {
                    painter.add(egui::Shape::mesh(mesh));
                    return;
                }
                stroke.base_width
            }
            RoundMark::Line { width } => width,
        };
        let project_points = || {
            if stroke.is_vector || stroke.brush_preset == KritaBrushPreset::Fineliner {
                let mut path = crate::vector_path::vector_centerline(stroke);
                if stroke.tip_shape == BrushTipShape::Square {
                    crate::vector_path::extend_square_caps(&mut path, width);
                }
                path.into_iter().map(|(x,y)| screen(x,y)).collect()
            } else if stroke.brush_preset == KritaBrushPreset::GPenInk {
                crate::brush_engine::smooth_krita_2d_points(&stroke.points,width)
                    .into_iter().map(|p|screen(p.x,p.y)).collect()
            } else if stroke.brush_preset == KritaBrushPreset::SmoothInk {
                crate::brush_engine::smooth_ink_points(&stroke.points, width)
                    .into_iter().map(|p| screen(p.x,p.y)).collect()
            } else {
                crate::brush_engine::smooth_freehand_points(&stroke.points, width)
                    .into_iter().map(|p| screen(p.x,p.y)).collect()
            }
        };
        let points = if flow == 1.0 {
            screen_pts.take().unwrap_or_else(project_points)
        } else {
            screen_pts.get_or_insert_with(project_points).clone()
        };
        if !stroke.is_vector && stroke.brush_preset == KritaBrushPreset::ChiselMarker {
            let marker_width=width*scale;
            let safe=marker_needs_safe_raster(&points,marker_width);
            // Loops, spirals, retracing and hairpins are never sent through
            // the outline triangulator: a self-intersecting polygon can bridge
            // interior areas and look exactly like Bucket Fill.
            let body_painted=if safe {
                if paint_marker_safe_raster(painter,&points,marker_width,ink) {
                    true
                } else {
                    // Allocation/budget fallback: an open polyline can darken at
                    // crossings but cannot fill an enclosed region or allocate
                    // a huge raster texture.
                    painter.add(egui::Shape::line(
                        points.clone(),egui::Stroke::new(marker_width,ink)));
                    true
                }
            } else {
                marker_strip_mesh(&points,marker_width,ink).is_some_and(|mesh|{
                    painter.add(egui::Shape::mesh(mesh));
                    true
                })
            };
            if body_painted {
                if points.len() >= 2 {
                    for (endpoint, interior) in [
                        (points[0], points[1]),
                        (points[points.len()-1], points[points.len()-2]),
                    ] {
                        let cap = crate::brush_engine::raster_tip_cap_polygon(
                            (endpoint.x, endpoint.y), (interior.x, interior.y),
                            width * scale, stroke.tip_shape,
                        );
                        if cap.len() >= 3 {
                            painter.add(egui::Shape::convex_polygon(
                                cap.into_iter().map(|(x,y)| egui::pos2(x,y)).collect(),
                                ink, egui::Stroke::NONE,
                            ));
                        }
                    }
                }
                return;
            }
        }

        painter.add(egui::Shape::line(points.clone(), egui::Stroke::new(width * scale, ink)));
        // Non-round raster tips use outside half-caps so endpoints do not get
        // a second translucent coat.
        if !stroke.is_vector && points.len() >= 2
            && stroke.tip_shape != BrushTipShape::Round
        {
            let pairs=[
                (points[0],points[1]),
                (points[points.len()-1],points[points.len()-2]),
            ];
            for (endpoint,interior) in pairs {
                let cap=crate::brush_engine::raster_tip_cap_polygon(
                    (endpoint.x,endpoint.y),(interior.x,interior.y),
                    width*scale,stroke.tip_shape);
                if cap.len()>=3 {
                    painter.add(egui::Shape::convex_polygon(
                        cap.into_iter().map(|(x,y)|egui::pos2(x,y)).collect(),
                        ink,egui::Stroke::NONE));
                }
            }
        } else if stroke.tip_shape == BrushTipShape::Round {
            if let (Some(first),Some(last))=(points.first(),points.last()) {
                let radius=(width*scale*0.5).max(0.25);
                painter.circle_filled(*first,radius,ink);
                painter.circle_filled(*last,radius,ink);
            }
        }
    });
}

#[cfg(test)]
mod viewport_tests {
    use super::*;
    use crate::project::StrokePoint;

    fn stroke(points: &[(f32, f32)]) -> Stroke {
        Stroke {
            points: points.iter().map(|&(x,y)| StrokePoint { x, y, pressure: 1.0 }).collect(),
            color: [0,0,0,255],
            fill_color: None,
            base_width: 8.0,
            is_vector: false,
            brush_preset: KritaBrushPreset::GPenInk,
            opacity: 1.0,
            brush_texture_id: None,
            tip_shape: BrushTipShape::Round,
        }
    }

    #[test]
    fn live_marker_draws_each_pass_with_a_clear_gap() {
        let ctx = egui::Context::default();
        let result = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            for points in [
                &[(10., 12.), (90., 12.)][..],
                &[(90., 24.), (10., 24.)],
                &[(10., 40.), (90., 40.), (90., 54.), (10., 54.)],
            ] {
                let mut s = stroke(points);
                s.brush_preset = KritaBrushPreset::ChiselMarker;
                s.base_width = 12.0;
                s.opacity = 0.4;
                draw_stroke_pro(&painter, &s, None, 1.0, 1.0, 0.0, 0.0, &|p| p);
            }
        });
        let shapes = ctx.tessellate(result.shapes, result.pixels_per_point);
        let covered = |p: egui::Pos2| shapes.iter().any(|shape| {
            let egui::epaint::Primitive::Mesh(mesh) = &shape.primitive else { return false };
            mesh.indices.chunks_exact(3).any(|ix| {
                let vertex = |i: usize| mesh.vertices[ix[i] as usize].pos;
                let (a,b,c) = (vertex(0), vertex(1), vertex(2));
                let side = |u: egui::Pos2, v: egui::Pos2| (v-u).rot90().dot(p-u);
                let signs = [side(a,b), side(b,c), side(c,a)];
                let area = (b-a).rot90().dot(c-a).abs();
                area > 0.0001 && (signs.iter().all(|&s| s >= 0.0) || signs.iter().all(|&s| s <= 0.0))
            })
        });
        for x in [19.5, 40.5, 62.5] {
            for y in [12., 24., 40., 54.] { assert!(covered(egui::pos2(x, y))); }
            for y in [18., 32., 47.] { assert!(!covered(egui::pos2(x, y))); }
        }
    }

    #[test]
    fn marker_body_is_triangulated_as_one_concave_surface() {
        let color = egui::Color32::from_rgba_unmultiplied(20,30,40,100);
        // Deliberately use a very wide marker over tightly spaced bends. The
        // old per-segment quad strip overlapped itself here and darkened joins.
        let points = [
            egui::pos2(0.0,0.0), egui::pos2(5.0,0.0),
            egui::pos2(5.0,5.0), egui::pos2(10.0,5.0),
            egui::pos2(15.0,9.0),
        ];
        let mesh = marker_strip_mesh(&points, 20.0, color).expect("marker body");
        assert!(mesh.vertices.len() >= 6);
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(mesh.vertices.iter().all(|v|
            v.pos.x.is_finite() && v.pos.y.is_finite() && v.color == color));
        assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len()));

        // Earcut triangulates one polygon. Interior triangles may share edges,
        // but their total area must equal the polygon area rather than the
        // inflated area produced by overlapping per-segment quads.
        let tri_area:f32=mesh.indices.chunks_exact(3).map(|ix|{
            let a=mesh.vertices[ix[0] as usize].pos;
            let b=mesh.vertices[ix[1] as usize].pos;
            let c=mesh.vertices[ix[2] as usize].pos;
            ((b-a).rot90().dot(c-a)).abs()*0.5
        }).sum();
        let mut poly_area=0.0f32;
        for i in 0..mesh.vertices.len() {
            let a=mesh.vertices[i].pos;
            let b=mesh.vertices[(i+1)%mesh.vertices.len()].pos;
            poly_area += a.x*b.y-a.y*b.x;
        }
        poly_area=poly_area.abs()*0.5;
        assert!((tri_area-poly_area).abs() < 0.1, "triangles must not overlap");
    }

    #[test]
    fn marker_detects_loop_retrace_for_safe_raster() {
        let mut pts=Vec::new();
        for lap in 0..2 {
            for i in 0..64 {
                let a=(i as f32/64.0)*std::f32::consts::TAU;
                let r=45.0+lap as f32*2.0;
                pts.push(egui::pos2(100.0+a.cos()*r,100.0+a.sin()*r));
            }
        }
        assert!(marker_has_self_overlap(&pts,24.0));
        assert!(marker_needs_safe_raster(&pts,24.0));
    }

    #[test]
    fn marker_simple_arc_stays_on_fast_path() {
        let pts=[
            egui::pos2(0.0,0.0),egui::pos2(20.0,2.0),
            egui::pos2(40.0,8.0),egui::pos2(60.0,18.0),
            egui::pos2(80.0,34.0),
        ];
        assert!(!marker_has_self_overlap(&pts,18.0));
        assert!(!marker_needs_safe_raster(&pts,18.0));
    }

    #[test]
    fn marker_detects_tight_return_hairpin() {
        let pts=[
            egui::pos2(0.0,120.0),
            egui::pos2(12.0,80.0),
            egui::pos2(22.0,35.0),
            egui::pos2(26.0,0.0),
            egui::pos2(38.0,4.0),
            egui::pos2(40.0,42.0),
            egui::pos2(40.0,90.0),
        ];
        assert!(marker_has_hairpin(&pts,28.0));
    }

    #[test]
    fn marker_does_not_flag_normal_curve_as_hairpin() {
        let pts=[
            egui::pos2(0.0,80.0),
            egui::pos2(20.0,55.0),
            egui::pos2(45.0,35.0),
            egui::pos2(75.0,25.0),
            egui::pos2(105.0,30.0),
        ];
        assert!(!marker_has_hairpin(&pts,28.0));
    }

    #[test]
    fn ordinary_gpen_round_tip_remains_round_cap_eligible() {
        let s = stroke(&[(0.0,0.0),(20.0,0.0)]);
        assert_eq!(s.brush_preset, KritaBrushPreset::GPenInk);
        assert_eq!(s.tip_shape, BrushTipShape::Round);
        let mut line_marks = 0;
        crate::brush_engine::visit_round_marks(&s, |mark, _| {
            if matches!(mark, crate::brush_engine::RoundMark::Line { .. }) {
                line_marks += 1;
            }
        });
        assert_eq!(line_marks, 1);
    }

    #[test]
    fn airbrush_alpha_follows_opacity_without_phantom_glow() {
        let mut airbrush = stroke(&[(0.0,0.0), (20.0,0.0)]);
        airbrush.brush_preset = KritaBrushPreset::SoftAirbrush;
        let mut alpha = Vec::new();
        crate::brush_engine::visit_round_marks(&airbrush, |_, flow| alpha.push((flow * 100.0) as u8));
        assert_eq!(alpha, [25,50,100]);
    }

    #[test]
    fn skips_completely_offscreen_strokes() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(100.0,100.0));
        let far = stroke(&[(1000.0,1000.0),(1200.0,1200.0)]);
        assert!(!may_touch_clip(&far,0.0,0.0,1.0,clip,&|p|p));
    }

    #[test]
    fn keeps_line_crossing_viewport_even_with_endpoints_outside() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(100.0,100.0));
        let crossing = stroke(&[(-200.0,50.0),(300.0,50.0)]);
        assert!(may_touch_clip(&crossing,0.0,0.0,1.0,clip,&|p|p));
    }

    #[test]
    fn wide_stroke_near_edge_is_not_lost() {
        let clip = egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(100.0,100.0));
        let near = stroke(&[(104.0,50.0)]);
        assert!(may_touch_clip(&near,0.0,0.0,1.0,clip,&|p|p));
    }
}
