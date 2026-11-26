//! Lightweight stage-space hit testing for marquee/lasso and text selection.
//! No extra image buffers; works with existing .stick vector project data.
use crate::project::{Stroke, TextItem};
use eframe::egui;

fn text_unrotated_rect(text:&TextItem)->egui::Rect {
    let lines: Vec<&str> = text.content.lines().collect();
    let max_chars = lines.iter().map(|line| line.chars().count()).max().unwrap_or(1).max(1);
    let height = (lines.len().max(1) as f32 * text.font_size * 1.32).max(12.0);
    let width = (max_chars as f32 * text.font_size * 0.60).max(14.0);
    egui::Rect::from_min_size(egui::pos2(text.x,text.y),egui::vec2(width,height))
}

pub fn text_corners(text:&TextItem)->[egui::Pos2;4] {
    let rect=text_unrotated_rect(text);
    let origin=egui::pos2(text.x,text.y);
    let angle=if text.rotation_deg.is_finite(){text.rotation_deg.to_radians()}else{0.0};
    let (sn,cs)=angle.sin_cos();
    let rotate=|p:egui::Pos2| {
        let d=p-origin;
        origin+egui::vec2(d.x*cs-d.y*sn,d.x*sn+d.y*cs)
    };
    [rotate(rect.left_top()),rotate(rect.right_top()),rotate(rect.right_bottom()),rotate(rect.left_bottom())]
}

pub fn text_bounds(text:&TextItem)->egui::Rect {
    let corners=text_corners(text);
    let mut min=corners[0];
    let mut max=corners[0];
    for p in corners.into_iter().skip(1){min=min.min(p);max=max.max(p);}
    egui::Rect::from_min_max(min,max)
}

pub fn text_hit_test(text:&TextItem,cursor:egui::Pos2)->bool {
    if !cursor.x.is_finite()||!cursor.y.is_finite(){return false;}
    let origin=egui::pos2(text.x,text.y);
    let angle=if text.rotation_deg.is_finite(){-text.rotation_deg.to_radians()}else{0.0};
    let (sn,cs)=angle.sin_cos();
    let d=cursor-origin;
    let local=origin+egui::vec2(d.x*cs-d.y*sn,d.x*sn+d.y*cs);
    text_unrotated_rect(text).contains(local)
}

pub fn stroke_intersects_marquee(stroke: &Stroke, rect: egui::Rect) -> bool {
    if stroke.points.is_empty() { return false; }
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in &stroke.points {
        if !p.x.is_finite() || !p.y.is_finite() { return false; }
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    egui::Rect::from_min_max(min, max).expand(stroke.base_width.max(1.0) * 0.5).intersects(rect)
}

pub fn point_in_polygon(point: egui::Pos2, polygon: &[egui::Pos2]) -> bool {
    if polygon.len() < 3 { return false; }
    let mut inside = false;
    let mut prev = polygon[polygon.len() - 1];
    for &p in polygon {
        if (p.y > point.y) != (prev.y > point.y) {
            let intersection = (prev.x - p.x) * (point.y - p.y) / (prev.y - p.y) + p.x;
            if point.x < intersection { inside = !inside; }
        }
        prev = p;
    }
    inside
}

/// Select geometry crossing the lasso even when all vertices are outside.
fn segments_intersect(a: egui::Pos2, b: egui::Pos2, c: egui::Pos2, d: egui::Pos2) -> bool {
    fn orient(a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }
    fn on_segment(a: egui::Pos2, b: egui::Pos2, p: egui::Pos2) -> bool {
        p.x >= a.x.min(b.x) - 1e-4 && p.x <= a.x.max(b.x) + 1e-4 &&
        p.y >= a.y.min(b.y) - 1e-4 && p.y <= a.y.max(b.y) + 1e-4
    }
    let (ab_c, ab_d, cd_a, cd_b) = (orient(a,b,c),orient(a,b,d),orient(c,d,a),orient(c,d,b));
    if (ab_c > 0.0 && ab_d < 0.0 || ab_c < 0.0 && ab_d > 0.0) &&
       (cd_a > 0.0 && cd_b < 0.0 || cd_a < 0.0 && cd_b > 0.0) { return true; }
    (ab_c.abs() < 1e-4 && on_segment(a,b,c)) ||
    (ab_d.abs() < 1e-4 && on_segment(a,b,d)) ||
    (cd_a.abs() < 1e-4 && on_segment(c,d,a)) ||
    (cd_b.abs() < 1e-4 && on_segment(c,d,b))
}

pub fn stroke_inside_lasso(stroke: &Stroke, polygon: &[egui::Pos2]) -> bool {
    if polygon.len() < 3 || stroke.points.is_empty() ||
        stroke.points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) { return false; }
    if stroke.points.iter().any(|p| point_in_polygon(egui::pos2(p.x, p.y), polygon)) { return true; }
    for segment in stroke.points.windows(2) {
        let (a, b) = (egui::pos2(segment[0].x, segment[0].y),
                      egui::pos2(segment[1].x, segment[1].y));
        for i in 0..polygon.len() {
            if segments_intersect(a, b, polygon[i], polygon[(i+1)%polygon.len()]) { return true; }
        }
    }
    stroke.fill_color.is_some() &&
        crate::fill_tools::contour_contains(stroke, polygon[0].x, polygon[0].y)
}

pub fn rotate_points_around(stroke: &mut Stroke, pivot: egui::Pos2, angle_rad: f32) {
    if !angle_rad.is_finite() { return; }
    let (sn, cs) = angle_rad.sin_cos();
    for point in &mut stroke.points {
        let dx = point.x - pivot.x;
        let dy = point.y - pivot.y;
        point.x = pivot.x + dx * cs - dy * sn;
        point.y = pivot.y + dx * sn + dy * cs;
    }
}

/// Exact stroke/region hit-testing instead of using a whole stroke's
/// bounding rectangle. Especially important for diagonal lines and curves
/// whose large bounding boxes contain lots of blank canvas.
pub fn stroke_hit_test(stroke: &Stroke, cursor: egui::Pos2, slop: f32) -> bool {
    if crate::raster_patch::is_patch(stroke) {
        return crate::raster_patch::color_at(stroke,cursor.x,cursor.y).is_some()
            || crate::raster_patch::touches_circle(stroke,(cursor.x,cursor.y),slop.max(0.5));
    }
    outline_hit_test(stroke, cursor, slop) || (stroke.fill_color.is_some()
        && crate::fill_tools::contour_contains(stroke, cursor.x, cursor.y))
}

fn outline_hit_test(stroke: &Stroke, cursor: egui::Pos2, slop: f32) -> bool {
    if !cursor.x.is_finite() || !cursor.y.is_finite()
        || stroke.points.is_empty()
        || stroke.points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return false;
    }
    let radius = (stroke.base_width.abs() * 0.5 + slop.max(0.0)).max(0.5);
    let r_sq = radius * radius;
    let pt = &stroke.points[0];
    if stroke.points.len() == 1 {
        return (pt.x - cursor.x).powi(2) + (pt.y - cursor.y).powi(2) <= r_sq;
    }
    for segment in stroke.points.windows(2) {
        let (a,b)=(&segment[0],&segment[1]);
        let dx=b.x-a.x;
        let dy=b.y-a.y;
        let len_sq=dx*dx+dy*dy;
        let t=if len_sq>1e-8 {
            ((cursor.x-a.x)*dx+(cursor.y-a.y)*dy)/len_sq
        } else {0.0}.clamp(0.0,1.0);
        if (cursor.x - (a.x+t*dx)).powi(2)
          + (cursor.y - (a.y+t*dy)).powi(2) <= r_sq {
            return true;
        }
    }
    false
}

/// Pick the visually relevant outline or fill color from the nearest topmost
/// touched stroke. Fill-only regions have transparent outlines; these should
/// return their fill color instead of transparent black.
pub fn pick_color(strokes: &[Stroke], cursor: egui::Pos2, slop: f32) -> Option<[u8;4]> {
    for stroke in strokes.iter().rev() {
        if stroke.opacity <= 0.0 { continue; }
        if crate::raster_patch::is_patch(stroke) {
            if let Some(color)=crate::raster_patch::color_at(stroke,cursor.x,cursor.y) {return Some(color);}
            continue;
        }
        if stroke.color[3] > 0 && outline_hit_test(stroke, cursor, slop) {
            return Some(stroke.color);
        }
        if let Some(fill) = stroke.fill_color {
            if fill[3] > 0 && crate::fill_tools::contour_contains(stroke, cursor.x, cursor.y) {
                return Some(fill);
            }
        }
    }
    None
}

pub fn pick_layer_color(project: &crate::project::AniProject, frame: usize, cursor: egui::Pos2, slop: f32) -> Option<[u8; 4]> {
    // The painter traverses layers in reverse, so index zero is frontmost.
    project.layers.iter().filter(|layer| layer.visible && layer.opacity > 0.0 && !layer.is_guide)
        .find_map(|layer| pick_color(&project.evaluate_layer_at_frame(layer, frame).strokes, cursor, slop))
}

/// Return the nearest editable vertex on any vector stroke within a
/// stage-space radius; brush raster paths are intentionally not node-edited.
pub fn nearest_vector_node(
    strokes: &[Stroke],
    cursor: egui::Pos2,
    radius: f32,
) -> Option<(usize, usize)> {
    let mut nearest = None;
    let mut best = radius.max(0.0).powi(2);
    for (si, stroke) in strokes.iter().enumerate() {
        if !stroke.is_vector { continue; }
        for (pi, point) in stroke.points.iter().enumerate() {
            let distance = (point.x - cursor.x).powi(2) + (point.y - cursor.y).powi(2);
            if distance.is_finite() && distance <= best {
                nearest = Some((si, pi));
                best = distance;
            }
        }
    }
    nearest
}

/// Adjust the view offset so the document point under the mouse does not move
/// when the user zooms. Coordinates are screen-space points.
pub fn zoom_pan_around_pointer(
    pan: egui::Vec2, viewport_center: egui::Pos2, pointer: egui::Pos2,
    old_zoom: f32, new_zoom: f32,
) -> egui::Vec2 {
    if !old_zoom.is_finite() || old_zoom <= 0.0 || !new_zoom.is_finite() || new_zoom <= 0.0 {
        return pan;
    }
    pan + (pointer - (viewport_center + pan)) * (1.0 - new_zoom / old_zoom)
}

/// Geometry-only scaling to keep interactive transform independent of Zoom.
pub fn scale_points_around(stroke: &mut Stroke, anchor: egui::Pos2, factor: f32) {
    if !factor.is_finite() || factor <= 0.0 { return; }
    for point in &mut stroke.points {
        point.x = anchor.x + (point.x - anchor.x) * factor;
        point.y = anchor.y + (point.y - anchor.y) * factor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{KritaBrushPreset, StrokePoint};
    fn stroke(x1: f32, y1: f32, x2: f32, y2: f32) -> Stroke {
        Stroke {
            points: vec![
                StrokePoint { x: x1, y: y1, pressure: 1.0 },
                StrokePoint { x: x2, y: y2, pressure: 1.0 }
            ],
            color: [0,0,0,255], fill_color: None,
            base_width: 2.0, is_vector: true,
            brush_preset: KritaBrushPreset::GPenInk,
            opacity: 1.0, brush_texture_id: None, tip_shape: crate::project::BrushTipShape::Round,
        }
    }
    #[test]
    fn select_click_rejects_diagonal_line_bounding_box_false_positive() {
        let s=stroke(0.,0.,100.,100.);
        assert!(!stroke_hit_test(&s,egui::pos2(50.,85.),3.));
        assert!(stroke_hit_test(&s,egui::pos2(50.,51.),3.));
    }

    #[test]
    fn eyedropper_uses_segments_and_opaque_fill_colors() {
        let mut s=stroke(0.,0.,100.,100.);
        s.color=[22,44,66,255];
        assert_eq!(pick_color(&[s],egui::pos2(70.,71.),3.),Some([22,44,66,255]));
        let mut filled=stroke(0.,0.,100.,0.);
        filled.points=vec![
            crate::project::StrokePoint{x:10.,y:10.,pressure:1.},
            crate::project::StrokePoint{x:100.,y:10.,pressure:1.},
            crate::project::StrokePoint{x:100.,y:100.,pressure:1.},
            crate::project::StrokePoint{x:10.,y:100.,pressure:1.},
            crate::project::StrokePoint{x:10.,y:10.,pressure:1.}
        ];
        filled.color=[0,0,0,255];
        filled.fill_color=Some([240,70,10,255]);
        assert!(stroke_inside_lasso(&filled, &[egui::pos2(30.,30.), egui::pos2(40.,30.), egui::pos2(35.,40.)]));
        assert_eq!(pick_color(std::slice::from_ref(&filled),egui::pos2(10.,45.),3.),Some(filled.color));
        assert_eq!(pick_color(&[filled],egui::pos2(40.,45.),3.),Some([240,70,10,255]));
    }

    #[test]
    fn eyedropper_checks_overlapping_layers_from_the_front() {
        let mut project = crate::project::AniProject::default();
        let mut back = project.layers[0].clone();
        let mut mark = stroke(0., 0., 100., 0.);
        back.keyframes.entry(1).or_default().strokes.push(mark.clone());
        mark.color = [255, 0, 0, 255];
        project.layers[0].keyframes.entry(1).or_default().strokes.push(mark);
        project.layers.push(back);
        let position = egui::pos2(10., 0.);
        assert_eq!(pick_layer_color(&project, 1, position, 1.), Some([255, 0, 0, 255]));
        project.layers[0].visible = false;
        assert_eq!(pick_layer_color(&project, 1, position, 1.), Some([0, 0, 0, 255]));
    }

    #[test]
    fn marquee_selects_crossing_strokes_with_outside_endpoints() {
        let rect = egui::Rect::from_min_max(egui::pos2(5.0,5.0), egui::pos2(20.0,20.0));
        assert!(stroke_intersects_marquee(&stroke(-10.0, 10.0, 50.0, 10.0), rect));
        assert!(!stroke_intersects_marquee(&stroke(40.0, 40.0, 60.0, 60.0), rect));
    }
    #[test]
    fn lasso_point_containment_distinguishes_inside_outside() {
        let poly = [egui::pos2(0.0,0.0), egui::pos2(30.0,0.0), egui::pos2(15.0,30.0)];
        assert!(point_in_polygon(egui::pos2(15.0,10.0), &poly));
        assert!(!point_in_polygon(egui::pos2(40.0,40.0), &poly));
        assert!(stroke_inside_lasso(&stroke(15.0,10.0,40.0,40.0), &poly));
    }
    #[test]
    fn vector_node_hit_test_selects_nearest_actual_vertex() {
        let vector = stroke(10.0, 10.0, 80.0, 80.0);
        let mut raster = stroke(11.0, 11.0, 70.0, 70.0);
        raster.is_vector = false;
        assert_eq!(nearest_vector_node(&[vector, raster],
            egui::pos2(12.0, 12.0), 7.0), Some((0, 0)));
        assert_eq!(nearest_vector_node(&[stroke(10.0, 10.0, 80.0, 80.0)],
            egui::pos2(40.0, 40.0), 7.0), None);
    }
    #[test]
    fn transform_scale_applies_to_all_vertices_without_pixel_drifts() {
        let mut shape = stroke(10.0, 20.0, 20.0, 40.0);
        scale_points_around(&mut shape, egui::pos2(10.0, 20.0), 2.0);
        assert_eq!((shape.points[0].x, shape.points[0].y), (10.0, 20.0));
        assert_eq!((shape.points[1].x, shape.points[1].y), (30.0, 60.0));
    }

    #[test]
    fn lasso_crossing_line_without_contained_vertices_is_selected() {
        let tri = [egui::pos2(5.0, 5.0), egui::pos2(25.0, 5.0), egui::pos2(15.0, 25.0)];
        assert!(stroke_inside_lasso(&stroke(-20.0, 12.0, 50.0, 12.0), &tri));
        assert!(!stroke_inside_lasso(&stroke(-20.0, 40.0, 50.0, 40.0), &tri));
    }

    #[test]
    fn rotation_preserves_distances_and_moves_vertices() {
        let mut line = stroke(20.0, 10.0, 10.0, 20.0);
        rotate_points_around(&mut line, egui::pos2(10.0, 10.0), std::f32::consts::FRAC_PI_2);
        assert!((line.points[0].x - 10.0).abs() < 0.01);
        assert!((line.points[0].y - 20.0).abs() < 0.01);
    }

    #[test]
    fn cursor_anchor_is_unchanged_on_zoom_in_and_out() {
        let center = egui::pos2(400.0, 300.0);
        let mouse = egui::pos2(540.0, 450.0);
        let pan = egui::vec2(10.0, -20.0);
        let after = zoom_pan_around_pointer(pan, center, mouse, 1.0, 2.0);
        let old_doc = (mouse - (center + pan)) / 1.0;
        let new_doc = (mouse - (center + after)) / 2.0;
        assert!((old_doc - new_doc).length() < 0.001);
        let back = zoom_pan_around_pointer(after, center, mouse, 2.0, 1.0);
        assert!((back - pan).length() < 0.001);
    }

    #[test]
    fn rotated_text_hit_test_tracks_visual_rotation() {
        let text=TextItem{content:"ABCDEFG".into(),x:100.,y:100.,font_size:30.,
            color:[0,0,0,255],rotation_deg:90.};
        assert!(text_hit_test(&text,egui::pos2(90.,115.)));
        assert!(!text_hit_test(&text,egui::pos2(125.,115.)));
        let bounds=text_bounds(&text);
        assert!(bounds.height()>bounds.width());
    }

    #[test]
    fn text_hit_bounds_track_content_size() {
        let text = TextItem {
            content: "Hello".into(), x: 10.0, y: 20.0,
            font_size: 30.0, color: [0,0,0,255], rotation_deg: 0.0,
        };
        let b = text_bounds(&text);
        assert!(b.contains(egui::pos2(15.0,25.0)));
        assert!(b.width() >= 85.0);
    }
}
