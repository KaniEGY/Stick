//! Non-destructive vector interpolation shared by live canvas and raster/video export.
//! Control points in the project are never changed by render-time smoothing.
use crate::project::{Stroke, StrokePoint};

const MAX_CONTROL_POINTS: usize = 5_000;
const MAX_RENDER_SAMPLES: usize = 24_000;
const MAX_SUBDIVISIONS: usize = 24;
const CORNER_COS_THRESHOLD: f32 = 0.15;

type Point = (f32, f32);

fn distance(a: Point, b: Point) -> f32 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// Preserve deliberate corners instead of rounding every node as if it were
/// a smooth Bezier handle. A deflection of roughly 57 degrees is treated as a
/// corner. Coincident neighbors are handled by the duplicate-point filter.
fn sharp_corner(before: Point, at: Point, after: Point) -> bool {
    let incoming = (at.0 - before.0, at.1 - before.1);
    let outgoing = (after.0 - at.0, after.1 - at.1);
    let lengths = distance(before, at) * distance(at, after);
    lengths > 0.0001
        && (incoming.0 * outgoing.0 + incoming.1 * outgoing.1) / lengths
            < CORNER_COS_THRESHOLD
}

fn tangent(before: Point, at: Point, after: Point, segment: Point, segment_len: f32) -> Point {
    if sharp_corner(before, at, after) {
        // Each adjacent segment uses its own chord at a hard corner, so
        // the corner stays at the precise position of the editable node.
        return segment;
    }
    let total = distance(before, at) + distance(at, after);
    if total <= 0.0001 { return segment; }
    let scale = segment_len / total;
    ((after.0 - before.0) * scale, (after.1 - before.1) * scale)
}


/// Smooth freehand vector control nodes before simplification. Tiny hand
/// oscillations are averaged while deliberate ~90°+ corners remain anchored.
/// This mutates only Vector Pen input at commit time; geometric shapes and
/// imported SVG control points are not passed through this function.
pub fn smooth_freehand_nodes(points: &mut Vec<StrokePoint>, amount: f32) {
    if points.len() < 3 || points.len() > MAX_CONTROL_POINTS { return; }
    let amount=amount.clamp(0.0,0.75);
    if amount<=0.001 {return;}
    let closed=points.len()>3 && distance(
        (points[0].x,points[0].y),
        (points[points.len()-1].x,points[points.len()-1].y)
    )<=0.001;
    let unique=points.len()-usize::from(closed);
    if unique<3{return;}

    for _ in 0..2 {
        let before=points.clone();
        let range_start=if closed {0}else{1};
        let range_end=if closed {unique}else{unique-1};
        for i in range_start..range_end {
            let prev=if i==0 {unique-1}else{i-1};
            let next=if i+1==unique {0}else{i+1};
            let a=(before[prev].x,before[prev].y);
            let b=(before[i].x,before[i].y);
            let d=(before[next].x,before[next].y);
            if sharp_corner(a,b,d){continue;}
            // Weighted moving average resists shrinkage while removing the
            // high-frequency zig-zag visible in slow mouse/tablet input.
            let tx=(a.0+2.0*b.0+d.0)*0.25;
            let ty=(a.1+2.0*b.1+d.1)*0.25;
            points[i].x=b.0+(tx-b.0)*amount;
            points[i].y=b.1+(ty-b.1)*amount;
        }
        if closed {
            points[unique]=points[0].clone();
        }
    }
}

/// Interpolate curved vector segments with an adaptive sample count. This
/// avoids obvious flat facets on long curves, without multiplying tiny
/// segments into excessive geometry or modifying any saved vector nodes.
///
/// Closed strokes use cyclic tangents so the seam does not develop a cusp.
/// Straight segments and intentional hard corners remain straight.
pub fn smoothed_path(points: &[StrokePoint]) -> Vec<Point> {
    if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Vec::new();
    }
    if points.len() < 3 || points.len() > MAX_CONTROL_POINTS {
        return points.iter().map(|p| (p.x, p.y)).collect();
    }

    let closed = points.len() > 3
        && distance(
            (points[0].x, points[0].y),
            (points[points.len() - 1].x, points[points.len() - 1].y),
        ) <= 0.001;
    let end = points.len() - usize::from(closed);
    let mut nodes: Vec<Point> = Vec::with_capacity(end);
    for p in &points[..end] {
        let pos = (p.x, p.y);
        if nodes.last().is_none_or(|&last| distance(last, pos) > 0.001) {
            nodes.push(pos);
        }
    }
    let n = nodes.len();
    if n < 3 {
        let mut path = nodes;
        if closed && !path.is_empty() { path.push(path[0]); }
        return path;
    }

    let edges = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(1 + edges * 6);
    out.push(nodes[0]);
    for i in 0..edges {
        let a = if i > 0 { nodes[i - 1] } else if closed { nodes[n - 1] } else { nodes[0] };
        let b = nodes[i];
        let c = nodes[(i + 1) % n];
        let d = if i + 2 < n { nodes[i + 2] } else if closed {
            nodes[(i + 2) % n]
        } else { nodes[n - 1] };

        let length = distance(b, c);
        if !length.is_finite() || length <= 0.001 {
            out.push(c);
            continue;
        }
        // At a hard corner the outgoing/incoming tangent follows this
        // segment's own chord. Gentle bends use chord-length-weighted Hermite
        // tangents to reduce uneven input-point spacing artifacts.
        let chord = (c.0 - b.0, c.1 - b.1);
        let m0 = tangent(a, b, c, chord, length);
        let m1 = tangent(b, c, d, chord, length);
        let steps = ((length / 6.0).ceil() as usize).clamp(3, MAX_SUBDIVISIONS);

        for j in 1..=steps {
            if j == steps {
                // Keep the original control node exactly, even after float
                // calculations; this also guarantees exact loop closure.
                out.push(c);
                continue;
            }
            let t = j as f32 / steps as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let h0 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h1 = t3 - 2.0 * t2 + t;
            let h2 = -2.0 * t3 + 3.0 * t2;
            let h3 = t3 - t2;
            let x = h0 * b.0 + h1 * m0.0 + h2 * c.0 + h3 * m1.0;
            let y = h0 * b.1 + h1 * m0.1 + h2 * c.1 + h3 * m1.1;
            // Safeguard against loops and badly spaced nodes without the
            // axis-by-axis per-segment clamp that flattened smooth bends.
            let margin = length * 0.1;
            let min_x = a.0.min(b.0).min(c.0).min(d.0) - margin;
            let max_x = a.0.max(b.0).max(c.0).max(d.0) + margin;
            let min_y = a.1.min(b.1).min(c.1).min(d.1) - margin;
            let max_y = a.1.max(b.1).max(c.1).max(d.1) + margin;
            out.push((x.clamp(min_x, max_x), y.clamp(min_y, max_y)));
        }
    }
    out
}


/// Smooth render-only centerline for the freehand Vector Ink pen.
///
/// Unlike `smoothed_path`, this intentionally rounds sharp freehand turns.
/// Geometric shapes keep using `smoothed_path`, so squares/triangles still
/// have exact corners. The saved editable nodes are never changed.
pub fn smoothed_ink_path(points: &[StrokePoint]) -> Vec<Point> {
    if points.is_empty()
        || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Vec::new();
    }
    // Preserve the complete saved centerline when subdivision would be too expensive.
    if points.len() > MAX_CONTROL_POINTS {
        return points.iter().map(|p| (p.x, p.y)).collect();
    }
    let closed=points.len()>3 && distance(
        (points[0].x,points[0].y),
        (points[points.len()-1].x,points[points.len()-1].y)
    )<=0.001;
    let end=points.len()-usize::from(closed);
    let mut path:Vec<Point>=Vec::with_capacity(end);
    for p in &points[..end] {
        let q=(p.x,p.y);
        if path.last().is_none_or(|&last|distance(last,q)>0.001){path.push(q);}
    }
    if path.len()<2 {return path;}

    // Two Chaikin passes remove the visible V-shaped/miter corner produced by
    // freehand samples, while staying bounded (<= ~4x control-node count).
    for _ in 0..2 {
        if path.len()*2+2>MAX_RENDER_SAMPLES {break;}
        let source=path;
        let n=source.len();
        let mut next=Vec::with_capacity(n*2+2);
        if closed {
            for i in 0..n {
                let a=source[i];
                let b=source[(i+1)%n];
                next.push((a.0*0.75+b.0*0.25,a.1*0.75+b.1*0.25));
                next.push((a.0*0.25+b.0*0.75,a.1*0.25+b.1*0.75));
            }
        } else {
            next.push(source[0]);
            for pair in source.windows(2) {
                let a=pair[0]; let b=pair[1];
                next.push((a.0*0.75+b.0*0.25,a.1*0.75+b.1*0.25));
                next.push((a.0*0.25+b.0*0.75,a.1*0.25+b.1*0.75));
            }
            next.push(source[n-1]);
        }
        path=next;
    }

    // Densify long spans to keep thick vector ink visually smooth at high DPI.
    let mut dense=Vec::with_capacity(path.len()*2);
    for (i,p) in path.iter().copied().enumerate() {
        if i==0 {dense.push(p);continue;}
        let a=*dense.last().unwrap();
        let len=distance(a,p);
        let remaining = path.len() - i - 1 + usize::from(closed);
        let available = MAX_RENDER_SAMPLES - dense.len() - remaining;
        let steps = ((len / 3.0).ceil() as usize).clamp(1, 16).min(available);
        for j in 1..=steps {
            let t=j as f32/steps as f32;
            dense.push(if j == steps { p } else { (a.0+(p.0-a.0)*t,a.1+(p.1-a.1)*t) });
        }
    }
    if closed && !dense.is_empty() && dense.last()!=dense.first() {
        dense.push(dense[0]);
    }
    dense
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickNodeEdit {
    Inserted { stroke: usize, point: usize },
    Removed { stroke: usize, point: usize },
}

fn segment_projection(a: &StrokePoint, b: &StrokePoint, x: f32, y: f32)
    -> Option<(f32, f32, f32, f32)>
{
    let dx=b.x-a.x;
    let dy=b.y-a.y;
    let len2=dx*dx+dy*dy;
    if !len2.is_finite() || len2<=0.000001 { return None; }
    let t=(((x-a.x)*dx+(y-a.y)*dy)/len2).clamp(0.0,1.0);
    let px=a.x+dx*t;
    let py=a.y+dy*t;
    Some((px,py,t,(px-x).hypot(py-y)))
}

/// Right-click vector node helper. Existing nodes win over segment insertion.
/// Closed contours keep their duplicated closing endpoint intact.
pub fn quick_edit_node(
    strokes: &mut [Stroke], x: f32, y: f32, tolerance: f32,
) -> Option<QuickNodeEdit> {
    quick_edit_node_in_selection(strokes,&[],x,y,tolerance)
}

pub fn quick_edit_node_prefer_selection(
    strokes: &mut [Stroke], selected_strokes: &[usize],
    x: f32, y: f32, tolerance: f32,
) -> Option<QuickNodeEdit> {
    if selected_strokes.is_empty() {
        return quick_edit_node(strokes,x,y,tolerance);
    }
    // Selected strokes get first priority, but a stale selection must never
    // make right-click look broken on another visible vector path.
    if let Some(edit)=quick_edit_node_in_selection(
        strokes,selected_strokes,x,y,tolerance
    ) {
        return Some(edit);
    }
    quick_edit_node(strokes,x,y,tolerance)
}

pub fn quick_edit_node_in_selection(
    strokes: &mut [Stroke], selected_strokes: &[usize],
    x: f32, y: f32, tolerance: f32,
) -> Option<QuickNodeEdit> {
    if !x.is_finite() || !y.is_finite() || !tolerance.is_finite() || tolerance<=0.0 {
        return None;
    }
    let mut nearest_node: Option<(usize,usize,f32)> = None;
    for (si,stroke) in strokes.iter().enumerate()
        .filter(|(si,s)|s.is_vector && (selected_strokes.is_empty() || selected_strokes.contains(si))) {
        for (pi,p) in stroke.points.iter().enumerate() {
            let d=(p.x-x).hypot(p.y-y);
            if d<=tolerance && nearest_node.is_none_or(|(_,_,best)|d<best) {
                nearest_node=Some((si,pi,d));
            }
        }
    }
    if let Some((si,pi,_))=nearest_node {
        let stroke=&mut strokes[si];
        let closed=stroke.points.len()>3 && stroke.points.first().zip(stroke.points.last())
            .is_some_and(|(a,b)|(a.x-b.x).hypot(a.y-b.y)<=0.001);
        if closed {
            let unique=stroke.points.len().saturating_sub(1);
            if unique<=3 { return None; }
            if pi==0 || pi==stroke.points.len()-1 {
                stroke.points.pop();
                stroke.points.remove(0);
            } else {
                stroke.points.remove(pi);
                stroke.points.pop();
            }
            if let Some(first)=stroke.points.first().cloned() {stroke.points.push(first);}
        } else {
            if stroke.points.len()<=2 { return None; }
            stroke.points.remove(pi);
        }
        return Some(QuickNodeEdit::Removed{stroke:si,point:pi});
    }

    // Hit-test the same smoothed curve the user actually sees. The previous
    // code tested only the straight control-point chords, so a right click on
    // a visibly curved section could miss even though the cursor was on ink.
    let mut nearest_segment: Option<(usize,usize,f32,f32,f32,f32)> = None;
    for (si,stroke) in strokes.iter().enumerate()
        .filter(|(si,s)|s.is_vector && (selected_strokes.is_empty() || selected_strokes.contains(si))) {
        let rendered=smoothed_path(&stroke.points);
        let mut nearest_visible:Option<(f32,f32,f32)>=None;
        for pair in rendered.windows(2) {
            let a=StrokePoint{x:pair[0].0,y:pair[0].1,pressure:1.0};
            let b=StrokePoint{x:pair[1].0,y:pair[1].1,pressure:1.0};
            let Some((px,py,_,d))=segment_projection(&a,&b,x,y) else {continue};
            if nearest_visible.is_none_or(|(_,_,best)|d<best) {
                nearest_visible=Some((px,py,d));
            }
        }
        let Some((vx,vy,visible_distance))=nearest_visible else {continue};
        if visible_distance>tolerance {continue;}
        // Map the visible hit back to its nearest editable control segment.
        // The new node is placed exactly on the rendered curve at the click,
        // while pressure is interpolated from that owning control segment.
        let mut raw_owner:Option<(usize,f32,f32)>=None;
        for (segment,pair) in stroke.points.windows(2).enumerate() {
            let Some((_,_,t,d))=segment_projection(&pair[0],&pair[1],vx,vy) else {continue};
            if raw_owner.is_none_or(|(_,_,best)|d<best) {
                raw_owner=Some((segment,t,d));
            }
        }
        let Some((segment,t,_))=raw_owner else {continue};
        if nearest_segment.is_none_or(|(_,_,_,_,_,best)|visible_distance<best) {
            nearest_segment=Some((si,segment,vx,vy,t,visible_distance));
        }
    }
    let (si,segment,px,py,t,_)=nearest_segment?;
    let pressure={
        let a=&strokes[si].points[segment];
        let b=&strokes[si].points[segment+1];
        a.pressure+(b.pressure-a.pressure)*t
    };
    let point=segment+1;
    strokes[si].points.insert(point,StrokePoint{x:px,y:py,pressure});
    Some(QuickNodeEdit::Inserted{stroke:si,point})
}

/// Extend an open polyline by half its stroke width for square line caps.
/// Closed paths have no endpoints and must never gain artificial extensions.
pub fn extend_square_caps(path: &mut [Point], width: f32) {
    if path.len() < 2 || !width.is_finite() || width <= 0.0
        || path.iter().any(|&(x, y)| !x.is_finite() || !y.is_finite())
    { return; }
    let first = path[0];
    let last_idx = path.len() - 1;
    let last = path[last_idx];
    if path.len() > 2 && distance(first, last) <= 0.001 { return; }

    let first_direction = path.iter().skip(1)
        .find(|&&p| distance(first, p) > 0.001).copied();
    let last_direction = path.iter().rev().skip(1)
        .find(|&&p| distance(last, p) > 0.001).copied();
    let radius = width * 0.5;
    if let Some(next) = first_direction {
        let len = distance(first, next);
        path[0] = (first.0 - (next.0 - first.0) / len * radius,
                   first.1 - (next.1 - first.1) / len * radius);
    }
    if let Some(before) = last_direction {
        let len = distance(before, last);
        path[last_idx] = (last.0 + (last.0 - before.0) / len * radius,
                          last.1 + (last.1 - before.1) / len * radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f32, y: f32) -> StrokePoint { StrokePoint { x, y, pressure: 1.0 } }

    fn vector(points:&[(f32,f32)]) -> Stroke {
        Stroke {
            points:points.iter().map(|&(x,y)|p(x,y)).collect(),
            color:[0,0,0,255],fill_color:None,base_width:4.0,is_vector:true,
            brush_preset:crate::project::KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:None,tip_shape:crate::project::BrushTipShape::Round,
        }
    }

    #[test]
    fn quick_node_edit_respects_selected_vector_stroke() {
        let mut strokes=vec![
            vector(&[(0.,0.),(100.,0.),(100.,100.)]),
            vector(&[(0.,4.),(100.,4.),(100.,104.)])
        ];
        let edit=quick_edit_node_in_selection(&mut strokes,&[1],50.,1.,8.).unwrap();
        assert!(matches!(edit,QuickNodeEdit::Inserted{stroke:1,..}));
    }

    #[test]
    fn quick_node_edit_inserts_on_segment_and_removes_existing_node() {
        let mut strokes=vec![vector(&[(0.,0.),(100.,0.),(100.,100.)])];
        let edit=quick_edit_node(&mut strokes,50.,4.,10.).unwrap();
        assert!(matches!(edit,QuickNodeEdit::Inserted{stroke:0,point:1}));
        assert!((strokes[0].points[1].x-50.).abs()<0.01);
        assert!((strokes[0].points[1].y).abs()<0.01);
        let edit=quick_edit_node(&mut strokes,50.,0.,5.).unwrap();
        assert!(matches!(edit,QuickNodeEdit::Removed{stroke:0,point:1}));
        assert_eq!(strokes[0].points.len(),3);
    }

    #[test]
    fn quick_node_edit_hits_the_visible_smoothed_curve_not_only_raw_chords() {
        let mut strokes=vec![vector(&[(0.,0.),(70.,0.),(120.,45.),(190.,45.)])];
        let rendered=smoothed_path(&strokes[0].points);
        let mut chosen=None;
        for &(x,y) in rendered.iter().skip(2).take(rendered.len().saturating_sub(4)) {
            let raw_distance=strokes[0].points.windows(2)
                .filter_map(|pair|segment_projection(&pair[0],&pair[1],x,y).map(|v|v.3))
                .fold(f32::INFINITY,f32::min);
            if raw_distance>1.0 {chosen=Some((x,y,raw_distance));break;}
        }
        let (x,y,raw_distance)=chosen.expect("curve must deviate from at least one raw chord");
        let tolerance=(raw_distance*0.6).max(0.4);
        assert!(strokes[0].points.windows(2).all(|pair|
            segment_projection(&pair[0],&pair[1],x,y).is_none_or(|v|v.3>tolerance)));
        let before=strokes[0].points.len();
        assert!(matches!(quick_edit_node(&mut strokes,x,y,tolerance),
            Some(QuickNodeEdit::Inserted{..})));
        assert_eq!(strokes[0].points.len(),before+1);
    }

    #[test]
    fn quick_node_edit_falls_back_to_visible_unselected_vector() {
        let mut strokes=vec![
            vector(&[(0.,0.),(80.,0.)]),
            vector(&[(0.,40.),(80.,40.)]),
        ];
        let before=strokes[1].points.len();
        let edit=quick_edit_node_prefer_selection(&mut strokes,&[0],40.,40.,8.)
            .expect("right-click should edit the visible second path");
        assert!(matches!(edit,QuickNodeEdit::Inserted{stroke:1,..}));
        assert_eq!(strokes[1].points.len(),before+1);
    }

    #[test]
    fn quick_node_edit_preserves_closed_contour_closure() {
        let mut strokes=vec![vector(&[
            (0.,0.),(100.,0.),(100.,100.),(0.,100.),(0.,0.)
        ])];
        assert!(matches!(quick_edit_node(&mut strokes,100.,0.,3.),
            Some(QuickNodeEdit::Removed{..})));
        assert_eq!(strokes[0].points.first(),strokes[0].points.last());
        assert_eq!(strokes[0].points.len(),4);
    }

    #[test]
    fn smooth_path_preserves_ends_and_editable_control_nodes() {
        let nodes = vec![p(0., 0.), p(20., 10.), p(35., 10.), p(45., 30.)];
        let before = nodes.clone();
        let result = smoothed_path(&nodes);
        assert_eq!(nodes, before);
        assert_eq!(result.first(), Some(&(0., 0.)));
        assert_eq!(result.last(), Some(&(45., 30.)));
        for node in nodes {
            assert!(result.contains(&(node.x, node.y)));
        }
        assert!(result.windows(2).all(|w|
            w.iter().all(|&(x, y)| x.is_finite() && y.is_finite())));
    }

    #[test]
    fn freehand_smoothing_reduces_small_zigzag_but_keeps_right_angle_corner() {
        let mut jitter=vec![p(0.,0.),p(10.,1.8),p(20.,-1.6),p(30.,1.4),p(40.,0.)];
        let before=jitter.iter().map(|p|p.y.abs()).sum::<f32>();
        smooth_freehand_nodes(&mut jitter,0.6);
        let after=jitter.iter().map(|p|p.y.abs()).sum::<f32>();
        assert!(after<before);

        let mut corner=vec![p(0.,0.),p(30.,0.),p(30.,30.)];
        smooth_freehand_nodes(&mut corner,0.7);
        assert!((corner[1].x-30.).abs()<0.001 && corner[1].y.abs()<0.001);
    }

    #[test]
    fn a_straight_line_remains_collinear_even_with_many_nodes() {
        let path = smoothed_path(&[p(0., 1.), p(10., 1.), p(100., 1.)]);
        assert!(path.len() > 10);
        assert!(path.iter().all(|&(x, y)| (y - 1.).abs() < 0.0001 && x >= -0.001 && x <= 100.001));
        assert!(path.windows(2).all(|w| w[1].0 + 0.001 >= w[0].0));
    }

    #[test]
    fn fast_corner_is_not_rounded_away_or_overshot() {
        let path = smoothed_path(&[p(0., 0.), p(40., 0.), p(40., 40.)]);
        assert!(path.contains(&(40., 0.)));
        assert!(path.iter().all(|&(x, y)| x >= -0.001 && x <= 40.001 && y >= -0.001 && y <= 40.001));
        assert!(path.iter().all(|&(x, y)| (x - 40.).abs() < 0.001 || y.abs() < 0.001));
    }

    #[test]
    fn freehand_ink_rounds_a_right_angle_without_mutating_nodes() {
        let nodes=vec![p(0.,0.),p(40.,0.),p(40.,40.)];
        let before=nodes.clone();
        let path=smoothed_ink_path(&nodes);
        assert_eq!(nodes,before);
        assert!(path.len()>nodes.len());
        assert!(!path.contains(&(40.,0.)), "freehand render should round the hard vertex");
        assert_eq!(path.first(),Some(&(0.,0.)));
        assert_eq!(path.last(),Some(&(40.,40.)));
    }

    #[test]
    fn a_closed_circle_like_path_has_a_smooth_seam() {
        let points = [p(0., -30.), p(30., 0.), p(0., 30.), p(-30., 0.), p(0., -30.)];
        let path = smoothed_path(&points);
        assert_eq!(path.first(), path.last());
        assert!(path.len() > points.len());
        let mut capped = path.clone();
        extend_square_caps(&mut capped, 8.);
        assert_eq!(capped, path);
    }

    #[test]
    fn vector_ink_keeps_the_last_node_on_long_strokes() {
        for count in [MAX_CONTROL_POINTS, MAX_CONTROL_POINTS + 1] {
            let points: Vec<_> = (0..count).map(|i| p(i as f32 * 64.0, 1.0)).collect();
            let path = smoothed_ink_path(&points);
            assert_eq!(path.first(), Some(&(0.0, 1.0)));
            assert_eq!(path.last(), Some(&(points[count - 1].x, 1.0)));
            assert!(path.len() <= MAX_RENDER_SAMPLES);
        }
    }

    #[test]
    fn duplicate_and_invalid_samples_are_bounded() {
        let path = smoothed_path(&[p(0., 0.), p(0., 0.), p(10., 5.), p(20., 5.)]);
        assert_eq!(path.first(), Some(&(0., 0.)));
        assert_eq!(path.last(), Some(&(20., 5.)));
        assert!(path.len() < 100);
        assert!(smoothed_path(&[p(0., 0.), p(f32::NAN, 1.), p(5., 5.)]).is_empty());
    }

    #[test]
    fn square_caps_extend_both_ends_and_ignore_duplicate_neighbors() {
        let mut path = vec![(0., 0.), (0., 0.), (10., 0.), (10., 0.)];
        extend_square_caps(&mut path, 4.);
        assert_eq!(path[0], (-2., 0.));
        assert_eq!(path[3], (12., 0.));
    }

    #[test]
    fn long_curves_get_more_render_samples_than_short_ones() {
        let short = smoothed_path(&[p(0., 0.), p(10., 5.), p(20., 0.)]);
        let long = smoothed_path(&[p(0., 0.), p(100., 50.), p(200., 0.)]);
        assert!(long.len() > short.len());
        assert!(long.len() <= 1 + 2 * MAX_SUBDIVISIONS);
    }
}
