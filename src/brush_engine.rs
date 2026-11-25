//! Pressure-aware stroke geometry shared by the live canvas and raster exports.

use crate::project::{BrushTipShape, KritaBrushPreset, Stroke, StrokePoint};

/// Each layer is painted once so a translucent wash can build up between
/// strokes without darkening at input event boundaries.
pub enum RoundMark {
    Dot(BrushDab),
    Outline(Vec<(f32, f32)>),
    Line { width: f32 },
}

pub fn visit_round_marks(stroke: &Stroke, mut paint: impl FnMut(RoundMark, f32)) {
    let Some(first) = stroke.points.first() else {
        return;
    };
    let layers: &[(f32, f32)] = match (stroke.is_vector, stroke.brush_preset) {
        (false, KritaBrushPreset::Watercolor) => &[(1.25, 1.0 / 3.0), (0.90, 1.0)],
        (false, KritaBrushPreset::SoftAirbrush) => &[(1.55, 0.25), (1.05, 0.5), (0.65, 1.0)],
        _ => &[(1.0, 1.0)],
    };
    let pressure = if stroke.is_vector
        || matches!(stroke.brush_preset, KritaBrushPreset::Fineliner | KritaBrushPreset::ChiselMarker) {
        // Marker/highlighter width should stay stable like dedicated marker
        // tools in mature painting apps; pressure may still affect opacity at
        // the stroke level, but it must not turn the nib into blocky boxes.
        1.0
    } else {
        first.pressure.clamp(0.02, 2.0)
    };
    // Mouse input (and some tablets at rest) is effectively constant pressure.
    // Building and triangulating a filled pressure outline for that case is both
    // expensive and unsafe at very tight reversals where the outline can
    // self-intersect. Only switch to the pressure polygon when the pressure
    // range is visually meaningful; otherwise use the renderer's native
    // round-joined line path.
    let (min_pressure, max_pressure) = if stroke.is_vector
        || matches!(stroke.brush_preset, KritaBrushPreset::Fineliner | KritaBrushPreset::ChiselMarker) {
        (pressure, pressure)
    } else {
        stroke.points.iter().fold((pressure, pressure), |(min_p, max_p), p| {
            let p = p.pressure.clamp(0.02, 2.0);
            (min_p.min(p), max_p.max(p))
        })
    };
    let varying = !stroke.is_vector
        && !matches!(stroke.brush_preset, KritaBrushPreset::Fineliner | KritaBrushPreset::ChiselMarker)
        && max_pressure - min_pressure > 0.03;
    for &(size, flow) in layers {
        let width = stroke.base_width * size;
        if stroke.points.len() == 1 {
            let mut dab = dab_for(first, width);
            if stroke.is_vector || stroke.brush_preset == KritaBrushPreset::Fineliner {
                dab.radius = (0.5 * width).max(0.25);
            } else if !stroke.is_vector && stroke.brush_preset == KritaBrushPreset::ChiselMarker {
                dab.radius = (0.5 * width * 0.68).max(0.25);
            }
            if (!stroke.is_vector && stroke.tip_shape != BrushTipShape::Round)
                || (stroke.is_vector && stroke.tip_shape == BrushTipShape::Square)
            {
                paint(RoundMark::Outline(dab_polygon(stroke.tip_shape, dab)), flow);
            } else {
                paint(RoundMark::Dot(dab), flow);
            }
        } else {
            // Raster geometric tips are a *continuous stroke*, not a chain of
            // visible square/triangle stamps. Textured brushes are routed before
            // this engine, and the Vector Pen is intentionally excluded.
            if !stroke.is_vector && stroke.tip_shape != BrushTipShape::Round {
                let preset_scale=if stroke.brush_preset==KritaBrushPreset::ChiselMarker {
                    0.68
                } else {
                    1.0
                };
                let shaped_width=(width*preset_scale
                    *raster_tip_cross_scale(stroke.tip_shape)).max(0.5);
                let smoothed=smooth_freehand_points(&stroke.points,shaped_width);
                if varying && !has_risky_turn(&smoothed) {
                    let outline=crate::stroke_geometry::pressure_outline(
                        &smoothed,shaped_width);
                    if !outline.is_empty() {
                        paint(RoundMark::Outline(outline),flow);
                        continue;
                    }
                }
                paint(RoundMark::Line {
                    width:(shaped_width*pressure).max(0.5),
                },flow);
                continue;
            }
            if !stroke.is_vector && stroke.brush_preset==KritaBrushPreset::GPenInk && varying {
                let smoothed=smooth_krita_2d_points(&stroke.points,width);
                if !has_risky_turn(&smoothed) {
                    let outline=crate::stroke_geometry::pressure_outline(&smoothed,width);
                    if !outline.is_empty() {
                        paint(RoundMark::Outline(outline),flow);
                        continue;
                    }
                }
                // Hairpins are the only fallback that still needs round dabs.
                visit_pressure_dabs_spaced(&smoothed,width,0.28,
                    |dab,_|paint(RoundMark::Dot(dab),flow));
                continue;
            }
            if !stroke.is_vector && stroke.brush_preset == KritaBrushPreset::TaperedPen {
                let smoothed=smooth_freehand_points(&stroke.points,width);
                let tapered = taper_pressure_profile(&smoothed);
                if has_risky_turn(&tapered) {
                    // Filled pressure polygons can fold over themselves on hairpins.
                    // Fall back to pressure dabs only for that risky geometry so the
                    // tapered beginning/end remain visible instead of producing a black blob.
                    visit_pressure_dabs(&tapered, width, |dab, _| paint(RoundMark::Dot(dab), flow));
                    continue;
                }
                let outline = crate::stroke_geometry::pressure_outline(&tapered, width);
                if !outline.is_empty() {
                    paint(RoundMark::Outline(outline), flow);
                    continue;
                }
            }
            if !stroke.is_vector && stroke.brush_preset == KritaBrushPreset::SmoothInk {
                let smoothed = smooth_ink_points(&stroke.points, width);
                if varying {
                    if has_risky_turn(&smoothed) {
                        visit_pressure_dabs(&smoothed, width, |dab, _| paint(RoundMark::Dot(dab), flow));
                        continue;
                    }
                    let outline = crate::stroke_geometry::pressure_outline(&smoothed, width);
                    if !outline.is_empty() {
                        paint(RoundMark::Outline(outline), flow);
                        continue;
                    }
                }
                // Constant-pressure Smooth Ink uses the fast round-joined line path.
                // Canvas/export renderers substitute this preset's smoothed centerline.
            }
            if !stroke.is_vector && stroke.brush_preset == KritaBrushPreset::ChiselMarker {
                // Both renderers stroke this open centerline. Triangulating a
                // closed outline can span the gap between adjacent return passes.
                paint(RoundMark::Line { width: (width * 0.68).max(0.5) }, flow);
                continue;
            }
            if varying {
                let smoothed=smooth_freehand_points(&stroke.points,width);
                if has_risky_turn(&smoothed) {
                    // Ordinary translucent/soft brushes must never explode into
                    // a folded pressure polygon on a tight reversal. Preserve a
                    // continuous stroke for that rare geometry instead.
                    paint(
                        RoundMark::Line {
                            width:(width*pressure).max(0.5),
                        },
                        flow,
                    );
                    continue;
                }
                let outline = crate::stroke_geometry::pressure_outline(&smoothed, width);
                if !outline.is_empty() {
                    paint(RoundMark::Outline(outline), flow);
                    continue;
                }
            }
            paint(
                RoundMark::Line {
                    width: (width * pressure).max(0.5),
                },
                flow,
            );
        }
    }
}

/// Corner-preserving smoothing shared by raster line brushes. It runs only on
/// shallow local deviations, keeps endpoints/pressure untouched and never
/// smooths across a real corner or reversal. Multiple light passes remove the
/// mouse/tablet micro-zigzag without the overshoot of Catmull-Rom subdivision.
fn corner_preserving_smooth(
    points: &[StrokePoint],
    width: f32,
    weight: f32,
    passes: usize,
    tolerance_scale: f32,
) -> Vec<StrokePoint> {
    if points.len() < 3 || !width.is_finite() || width <= 0.0 {
        return points.to_vec();
    }
    let mut result = points.to_vec();
    let weight = weight.clamp(0.0, 0.24);
    let tolerance = (width * tolerance_scale).clamp(0.30, 2.8);
    for _ in 0..passes.clamp(1, 3) {
        let source = result.clone();
        for i in 1..source.len() - 1 {
            let a = &source[i - 1];
            let b = &source[i];
            let c = &source[i + 1];
            let prev = (b.x - a.x, b.y - a.y);
            let next = (c.x - b.x, c.y - b.y);
            let l1 = prev.0.hypot(prev.1);
            let l2 = next.0.hypot(next.1);
            let chord = (c.x - a.x).hypot(c.y - a.y);
            if l1 <= 0.001 || l2 <= 0.001 || chord <= 0.001 {
                continue;
            }
            let cos = (prev.0 * next.0 + prev.1 * next.1) / (l1 * l2);
            let offset = ((b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x)).abs()/chord;
            if cos > 0.62 && offset <= tolerance {
                result[i].x = weight * a.x + (1.0 - 2.0 * weight) * b.x + weight * c.x;
                result[i].y = weight * a.y + (1.0 - 2.0 * weight) * b.y + weight * c.y;
            }
        }
    }
    result
}

fn raster_curve_fit(
    points:&[StrokePoint],
    width:f32,
    weight:f32,
    passes:usize,
    tolerance_scale:f32,
)->Vec<StrokePoint>{
    const MAX_SAMPLES:usize=24_000;
    if points.len()<3 || points.len()>12_000 || !width.is_finite() || width<=0.0 {
        return points.to_vec();
    }
    let nodes=corner_preserving_smooth(points,width,weight,passes,tolerance_scale);
    if nodes.len()<3 {return nodes;}

    let distance=|a:&StrokePoint,b:&StrokePoint|(b.x-a.x).hypot(b.y-a.y);
    let hard_corner=|a:&StrokePoint,b:&StrokePoint,d:&StrokePoint|{
        let u=(b.x-a.x,b.y-a.y);
        let v=(d.x-b.x,d.y-b.y);
        let denom=u.0.hypot(u.1)*v.0.hypot(v.1);
        denom>0.0001 && (u.0*v.0+u.1*v.1)/denom<0.15
    };
    let tangent=|a:&StrokePoint,b:&StrokePoint,d:&StrokePoint,
                 segment:(f32,f32),segment_len:f32|{
        if hard_corner(a,b,d){return segment}
        let total=distance(a,b)+distance(b,d);
        if total<=0.0001{return segment}
        let scale=segment_len/total;
        ((d.x-a.x)*scale,(d.y-a.y)*scale)
    };

    // Similar to the vector renderer's Hermite fitting, but pressure stays a
    // raster attribute and is interpolated independently. This gives raster ink
    // a vector-like centerline without turning the saved stroke into vectors.
    let spacing=(width*0.12).clamp(0.75,2.5);
    let mut out=Vec::with_capacity((nodes.len()*3).min(MAX_SAMPLES));
    out.push(nodes[0].clone());
    for i in 0..nodes.len()-1 {
        let a=if i>0 {&nodes[i-1]} else {&nodes[i]};
        let b=&nodes[i];
        let d=&nodes[i+1];
        let e=if i+2<nodes.len(){&nodes[i+2]}else{d};
        let len=distance(b,d);
        if !len.is_finite() || len<=0.001 {continue}
        let chord=(d.x-b.x,d.y-b.y);
        let m0=tangent(a,b,d,chord,len);
        let m1=tangent(b,d,e,chord,len);
        let steps=((len/spacing).ceil() as usize).clamp(1,24);
        for j in 1..=steps {
            if out.len()>=MAX_SAMPLES {break}
            if j==steps {
                out.push(d.clone());
                continue;
            }
            let t=j as f32/steps as f32;
            let t2=t*t; let t3=t2*t;
            let h0=2.0*t3-3.0*t2+1.0;
            let h1=t3-2.0*t2+t;
            let h2=-2.0*t3+3.0*t2;
            let h3=t3-t2;
            let margin=len*0.10;
            let min_x=a.x.min(b.x).min(d.x).min(e.x)-margin;
            let max_x=a.x.max(b.x).max(d.x).max(e.x)+margin;
            let min_y=a.y.min(b.y).min(d.y).min(e.y)-margin;
            let max_y=a.y.max(b.y).max(d.y).max(e.y)+margin;
            out.push(StrokePoint{
                x:(h0*b.x+h1*m0.0+h2*d.x+h3*m1.0).clamp(min_x,max_x),
                y:(h0*b.y+h1*m0.1+h2*d.y+h3*m1.1).clamp(min_y,max_y),
                pressure:b.pressure+(d.pressure-b.pressure)*t,
            });
        }
        if out.len()>=MAX_SAMPLES {break}
    }
    if let Some(last)=nodes.last() {
        if out.last().is_none_or(|p|(p.x-last.x).hypot(p.y-last.y)>0.01)
            && out.len()<MAX_SAMPLES
        {out.push(last.clone());}
    }
    out
}

/// General raster freehand centerline. The old implementation only averaged
/// existing pointer samples, so long curved spans still rendered as visible
/// straight facets. This one curve-fits and distance-resamples the centerline.
pub(crate) fn smooth_freehand_points(points: &[StrokePoint], width: f32) -> Vec<StrokePoint> {
    raster_curve_fit(points,width,0.16,2,0.17)
}

/// Basic opaque 2D ink gets a little more cleanup while preserving deliberate
/// corners and real stylus pressure.
pub(crate) fn smooth_krita_2d_points(points: &[StrokePoint], width: f32) -> Vec<StrokePoint> {
    raster_curve_fit(points,width,0.18,2,0.19)
}

/// Smooth Ink deliberately uses the strongest safe raster curve fitting.
pub(crate) fn smooth_ink_points(points: &[StrokePoint], width: f32) -> Vec<StrokePoint> {
    raster_curve_fit(points,width,0.21,3,0.22)
}

/// Detect turns likely to make a filled pressure outline self-intersect.
/// Normal curves keep the single clean polygon; only sharp reversals use dabs.
fn has_risky_turn(points: &[StrokePoint]) -> bool {
    points.windows(3).any(|w| {
        let a=&w[0]; let b=&w[1]; let c=&w[2];
        let u=(b.x-a.x,b.y-a.y);
        let v=(c.x-b.x,c.y-b.y);
        let lu=u.0.hypot(u.1); let lv=v.0.hypot(v.1);
        if lu <= 0.001 || lv <= 0.001 { return false; }
        let cos=(u.0*v.0+u.1*v.1)/(lu*lv);
        cos < -0.10
    })
}
/// Natural tapered nib: reduce width progressively over the first and last
/// 16% of travel while retaining real input pressure through the middle.
fn taper_pressure_profile(points: &[StrokePoint]) -> Vec<StrokePoint> {
    let mut result=points.to_vec();
    if result.len()<2 { return result; }
    let mut traveled=Vec::with_capacity(result.len());
    traveled.push(0.0f32);
    for pair in points.windows(2) {
        let distance=(pair[1].x-pair[0].x).hypot(pair[1].y-pair[0].y);
        traveled.push(traveled.last().copied().unwrap_or(0.0)+
            if distance.is_finite() {distance.max(0.)} else {0.});
    }
    let total=traveled.last().copied().unwrap_or(0.0);
    if total<=0.001 {return result;}
    for (p, traveled) in result.iter_mut().zip(traveled) {
        let t=(traveled/total).clamp(0.,1.);
        let edge=(t.min(1.-t)/0.16).clamp(0.,1.);
        // Smoothstep avoids an abrupt shoulder as the taper reaches full size.
        let eased=edge*edge*(3.-2.*edge);
        p.pressure *= 0.16+0.84*eased;
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushDab {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

const MAX_DABS_PER_STROKE: usize = 20_000;

/// The same polygon geometry is used for the canvas and exported images.
pub fn dab_polygon(tip: BrushTipShape, dab: BrushDab) -> Vec<(f32, f32)> {
    let r = dab.radius.max(0.25);
    let (w, h) = match tip {
        BrushTipShape::TallRectangle => (r * 0.5, r * 1.5),
        BrushTipShape::WideRectangle => (r * 1.5, r * 0.5),
        _ => (r, r),
    };
    match tip {
        BrushTipShape::Round => Vec::new(),
        BrushTipShape::Square | BrushTipShape::TallRectangle | BrushTipShape::WideRectangle => {
            vec![
                (dab.x - w, dab.y - h),
                (dab.x + w, dab.y - h),
                (dab.x + w, dab.y + h),
                (dab.x - w, dab.y + h),
            ]
        }
        BrushTipShape::Triangle | BrushTipShape::Pentagon => {
            let count = if tip == BrushTipShape::Triangle { 3 } else { 5 };
            (0..count)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * i as f32 / count as f32;
                    (dab.x + r * a.cos(), dab.y + r * a.sin())
                })
                .collect()
        }
    }
}

/// Cross-stroke width multiplier for continuous raster tip shapes.
pub fn raster_tip_cross_scale(tip: BrushTipShape) -> f32 {
    match tip {
        BrushTipShape::TallRectangle => 1.45,
        BrushTipShape::WideRectangle => 0.62,
        BrushTipShape::Triangle => 0.92,
        _ => 1.0,
    }
}

/// Build only the *outside* half-cap of a continuous raster stroke.
/// Because this polygon starts at the endpoint cross-section and extends
/// outward, translucent marker caps do not repaint/darken the line body.
pub fn raster_tip_cap_polygon(
    endpoint: (f32,f32),
    interior: (f32,f32),
    body_width: f32,
    tip: BrushTipShape,
) -> Vec<(f32,f32)> {
    if !body_width.is_finite() || body_width <= 0.0
        || !endpoint.0.is_finite() || !endpoint.1.is_finite()
        || !interior.0.is_finite() || !interior.1.is_finite()
    {
        return Vec::new();
    }
    let dx=endpoint.0-interior.0;
    let dy=endpoint.1-interior.1;
    let len=dx.hypot(dy);
    if len <= 0.0001 { return Vec::new(); }
    let out=(dx/len,dy/len);
    let normal=(-out.1,out.0);
    let r=(body_width*0.5).max(0.25);
    let add=|a:f32,b:f32| (
        endpoint.0+out.0*a+normal.0*b,
        endpoint.1+out.1*a+normal.1*b
    );
    match tip {
        BrushTipShape::Round => {
            let angle=out.1.atan2(out.0);
            let steps=10;
            (0..=steps).map(|i| {
                let a=angle+std::f32::consts::FRAC_PI_2
                    -std::f32::consts::PI*i as f32/steps as f32;
                (endpoint.0+a.cos()*r,endpoint.1+a.sin()*r)
            }).collect()
        }
        BrushTipShape::Square => vec![add(0.0,r),add(r,r),add(r,-r),add(0.0,-r)],
        BrushTipShape::Triangle => vec![add(0.0,r),add(r*1.28,0.0),add(0.0,-r)],
        BrushTipShape::Pentagon => vec![
            add(0.0,r),add(r*0.70,r*0.72),add(r*1.18,0.0),
            add(r*0.70,-r*0.72),add(0.0,-r),
        ],
        BrushTipShape::TallRectangle => vec![
            add(0.0,r),add(r*0.45,r),add(r*0.45,-r),add(0.0,-r)
        ],
        BrushTipShape::WideRectangle => vec![
            add(0.0,r),add(r*1.65,r),add(r*1.65,-r),add(0.0,-r)
        ],
    }
}

fn dab_for(p: &StrokePoint, width: f32) -> BrushDab {
    BrushDab {
        x: p.x,
        y: p.y,
        radius: (0.5 * width * p.pressure.clamp(0.02, 2.0)).max(0.25),
    }
}

/// Visit dabs at distances measured along the whole path, independent of input
/// event density. The budget stretches spacing across extreme paths so their
/// entire trajectory is represented.
pub fn visit_pressure_dabs(
    points: &[StrokePoint],
    width: f32,
    paint: impl FnMut(BrushDab, [f32; 2]),
) {
    visit_pressure_dabs_spaced(points, width, 0.48, paint);
}

/// Same path-distance sampler with caller-controlled spacing, expressed as a
/// fraction of dab radius. Textured media need denser stamps than the basic
/// pressure-dab fallback so fast strokes do not reveal individual rectangles.
pub fn visit_pressure_dabs_spaced(
    points: &[StrokePoint],
    width: f32,
    spacing_radius_ratio: f32,
    paint: impl FnMut(BrushDab, [f32; 2]),
) {
    visit_pressure_dabs_spaced_impl(
        points,width,spacing_radius_ratio,0.12,MAX_DABS_PER_STROKE,paint);
}

/// Extra-dense sampler for imported continuous brush engines. It keeps the
/// normal sampler's safety budget, but allows spacing below 12% of the radius
/// so large imported auto-brushes cannot visibly break into circular beads.
pub fn visit_pressure_dabs_spaced_dense(
    points: &[StrokePoint],
    width: f32,
    spacing_radius_ratio: f32,
    paint: impl FnMut(BrushDab, [f32; 2]),
) {
    visit_pressure_dabs_spaced_impl(
        points,width,spacing_radius_ratio,0.02,MAX_DABS_PER_STROKE,paint);
}

/// Live imported-brush preview can use a smaller geometry budget than export.
/// The sampler increases spacing evenly across very long strokes, so the
/// pointer remains responsive without chopping off the stroke tail.
pub fn visit_pressure_dabs_spaced_dense_bounded(
    points: &[StrokePoint],
    width: f32,
    spacing_radius_ratio: f32,
    max_dabs: usize,
    paint: impl FnMut(BrushDab, [f32; 2]),
) {
    visit_pressure_dabs_spaced_impl(
        points,width,spacing_radius_ratio,0.02,
        max_dabs.clamp(2,MAX_DABS_PER_STROKE),paint);
}

fn visit_pressure_dabs_spaced_impl(
    points: &[StrokePoint],
    width: f32,
    spacing_radius_ratio: f32,
    minimum_ratio: f32,
    max_dabs: usize,
    mut paint: impl FnMut(BrushDab, [f32; 2]),
) {
    if !width.is_finite() || width <= 0.0 {
        return;
    }
    let valid = points
        .iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite() && p.pressure.is_finite());
    let mut path = valid.clone();
    let Some(first) = path.next() else { return };
    let width = width.clamp(0.5, 1024.0);

    // Use f64 for travel distances: a very large coordinate must still
    // advance when adding one spacing interval.
    let length = |a: &StrokePoint, b: &StrokePoint| {
        (f64::from(b.x) - f64::from(a.x)).hypot(f64::from(b.y) - f64::from(a.y))
    };
    let total: f64 = valid.zip(path.clone()).map(|(a, b)| length(a, b)).sum();
    let max_dabs=max_dabs.clamp(2,MAX_DABS_PER_STROKE);
    let minimum_spacing = total / (max_dabs - 1) as f64;
    let spacing_ratio = spacing_radius_ratio.clamp(minimum_ratio, 1.5) as f64;
    let spacing = |d: BrushDab| (f64::from(d.radius) * spacing_ratio).max(0.2).max(minimum_spacing);
    let mut previous = first;
    let mut last_dab = dab_for(first, width);
    let mut direction = [1.0, 0.0];
    let mut started = false;
    let mut remaining = spacing(last_dab);
    let mut count = 0;
    for next in path {
        let len = length(previous, next);
        if len > 0.0 {
            let dx = f64::from(next.x) - f64::from(previous.x);
            let dy = f64::from(next.y) - f64::from(previous.y);
            direction = [(dx / len) as f32, (dy / len) as f32];
            if !started {
                paint(last_dab, direction);
                started = true;
                count += 1;
            }
            while remaining <= len && count < max_dabs - 1 {
                let t = remaining / len;
                let sample = StrokePoint {
                    x: (f64::from(previous.x) + dx * t) as f32,
                    y: (f64::from(previous.y) + dy * t) as f32,
                    pressure: previous.pressure + (next.pressure - previous.pressure) * t as f32,
                };
                last_dab = dab_for(&sample, width);
                paint(last_dab, direction);
                count += 1;
                remaining += spacing(last_dab);
            }
            remaining -= len;
        }
        previous = next;
    }
    if !started || (last_dab.x - previous.x).hypot(last_dab.y - previous.y) > 0.01 {
        paint(dab_for(previous, width), direction);
    }
}

#[cfg(test)]
fn sample_pressure_dabs(points: &[StrokePoint], width: f32) -> Vec<BrushDab> {
    let mut dabs = Vec::new();
    visit_pressure_dabs(points, width, |dab, _| dabs.push(dab));
    dabs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32, pressure: f32) -> StrokePoint {
        StrokePoint { x, y, pressure }
    }

    #[test]
    fn six_tip_shapes_have_correct_vertex_counts() {
        let d = BrushDab {
            x: 10.0,
            y: 15.0,
            radius: 5.0,
        };
        assert_eq!(dab_polygon(BrushTipShape::Round, d).len(), 0);
        assert_eq!(dab_polygon(BrushTipShape::Square, d).len(), 4);
        assert_eq!(dab_polygon(BrushTipShape::Triangle, d).len(), 3);
        assert_eq!(dab_polygon(BrushTipShape::Pentagon, d).len(), 5);
        assert_eq!(dab_polygon(BrushTipShape::TallRectangle, d).len(), 4);
        assert_eq!(dab_polygon(BrushTipShape::WideRectangle, d).len(), 4);
    }

    #[test]
    fn gpen_single_tap_generates_a_visible_dab() {
        let dabs = sample_pressure_dabs(&[point(4.0, 8.0, 0.5)], 10.0);
        assert_eq!(dabs.len(), 1);
        assert!((dabs[0].radius - 2.5).abs() < 0.01);
    }

    #[test]
    fn interpolation_preserves_start_end_and_varies_width() {
        let dabs = sample_pressure_dabs(&[point(0.0, 0.0, 0.2), point(40.0, 0.0, 1.0)], 10.0);
        assert!(dabs.len() > 5);
        assert!((dabs.first().unwrap().x - 0.0).abs() < 0.01);
        assert!((dabs.last().unwrap().x - 40.0).abs() < 0.01);
        assert!(dabs.first().unwrap().radius < dabs.last().unwrap().radius);
        assert!(dabs.iter().all(|d| d.radius.is_finite() && d.radius > 0.0));
    }

    #[test]
    fn marker_renders_one_open_constant_width_path() {
        let s=Stroke {
            points:vec![point(0.,0.,0.2),point(20.,2.,1.0),point(40.,0.,0.4)],
            color:[0,0,0,255],fill_color:None,base_width:20.,is_vector:false,
            brush_preset:KritaBrushPreset::ChiselMarker,opacity:0.42,
            brush_texture_id:None,tip_shape:BrushTipShape::Round,
        };
        let mut widths = Vec::new();
        visit_round_marks(&s, |mark, _| match mark {
            RoundMark::Line { width } => widths.push(width),
            _ => panic!("marker must stroke the centerline"),
        });
        assert_eq!(widths, vec![20.0 * 0.68]);
    }

    #[test]
    fn polygonal_raster_tips_route_as_one_continuous_stroke() {
        for tip in [
            BrushTipShape::Square,BrushTipShape::Triangle,BrushTipShape::Pentagon,
            BrushTipShape::TallRectangle,BrushTipShape::WideRectangle,
        ] {
            let s=Stroke {
                points:vec![point(0.,0.,1.),point(20.,4.,1.),point(40.,0.,1.)],
                color:[0,0,0,255],fill_color:None,base_width:18.,is_vector:false,
                brush_preset:KritaBrushPreset::ChiselMarker,opacity:0.42,
                brush_texture_id:None,tip_shape:tip,
            };
            let mut lines=0;
            let mut dots=0;
            visit_round_marks(&s,|mark,_|match mark {
                RoundMark::Line{..}=>lines+=1,
                RoundMark::Dot(_)=>dots+=1,
                RoundMark::Outline(_)=>{},
            });
            assert_eq!(lines,1,"{tip:?} must be one continuous line");
            assert_eq!(dots,0,"{tip:?} must not fall back to repeated dabs");
        }
    }

    #[test]
    fn marker_half_cap_stays_outside_the_stroke_body() {
        let cap=raster_tip_cap_polygon((0.,0.),(10.,0.),20.,BrushTipShape::Round);
        assert!(cap.len()>=8);
        // Start cap points outward (negative X here), never into the body.
        assert!(cap.iter().all(|(x,_)|*x<=0.001));
    }

    #[test]
    fn dense_texture_spacing_emits_more_samples_than_default_spacing() {
        let pts=[point(0.,0.,1.),point(100.,0.,1.)];
        let mut normal=0;
        let mut dense=0;
        visit_pressure_dabs(&pts,20.,|_,_|normal+=1);
        visit_pressure_dabs_spaced(&pts,20.,0.24,|_,_|dense+=1);
        assert!(dense>normal);
    }

    #[test]
    fn fineliner_ignores_pressure_and_keeps_constant_width() {
        let s=Stroke {
            points:vec![point(0.,0.,0.2),point(30.,0.,1.0)],
            color:[0,0,0,255],fill_color:None,base_width:8.,is_vector:false,
            brush_preset:KritaBrushPreset::Fineliner,opacity:1.,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut widths=Vec::new();
        visit_round_marks(&s,|mark,_|if let RoundMark::Line{width}=mark {widths.push(width)});
        assert_eq!(widths,vec![8.]);
    }
    #[test]
    fn vector_ink_tap_matches_constant_width_line() {
        let stroke = Stroke {
            points: vec![point(10., 10., 0.1)], base_width: 8., is_vector: true,
            color: [0,0,0,255], fill_color: None, opacity: 1., brush_texture_id: None,
            brush_preset: KritaBrushPreset::SmoothInk, tip_shape: BrushTipShape::Round,
        };
        let mut radii = Vec::new();
        visit_round_marks(&stroke, |mark, _| {
            if let RoundMark::Dot(dab) = mark { radii.push(dab.radius); }
        });
        assert_eq!(radii, vec![4.]);
    }

    #[test]
    fn variable_pressure_gpen_prefers_one_continuous_fitted_outline() {
        let s=Stroke {
            points:vec![point(0.,0.,0.25),point(20.,1.,0.8),point(40.,0.,1.0)],
            color:[0,0,0,255],fill_color:None,base_width:10.,is_vector:false,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut dots=0;
        let mut outlines=0;
        visit_round_marks(&s,|mark,_|match mark {
            RoundMark::Dot(_)=>dots+=1,
            RoundMark::Outline(_)=>outlines+=1,
            RoundMark::Line{..}=>{}
        });
        assert_eq!(outlines,1);
        assert_eq!(dots,0);
    }

    #[test]
    fn krita_2d_smoothing_reduces_micro_jitter_but_keeps_corner() {
        let jitter=vec![point(0.,0.,1.),point(10.,1.2,1.),point(20.,0.,1.)];
        let smooth=smooth_krita_2d_points(&jitter,10.);
        let center=smooth.iter().min_by(|a,b|
            (a.x-10.).abs().total_cmp(&(b.x-10.).abs())).unwrap();
        assert!(center.y<jitter[1].y);
        let corner=vec![point(0.,0.,1.),point(20.,0.,1.),point(20.,20.,1.)];
        let smooth=smooth_krita_2d_points(&corner,10.);
        assert!(smooth.iter().any(|p|(p.x-20.).abs()<0.001 && p.y.abs()<0.001));
        assert_eq!((smooth.first().unwrap().x,smooth.first().unwrap().y),(0.,0.));
        assert_eq!((smooth.last().unwrap().x,smooth.last().unwrap().y),(20.,20.));
    }

    #[test]
    fn ordinary_brush_hairpin_never_uses_folded_pressure_outline() {
        let s=Stroke {
            points:vec![point(0.,0.,0.3),point(20.,0.,1.0),point(1.,1.,0.5)],
            color:[0,0,0,255],fill_color:None,base_width:14.,is_vector:false,
            brush_preset:KritaBrushPreset::Watercolor,opacity:0.7,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut lines=0;
        let mut outlines=0;
        visit_round_marks(&s,|mark,_|match mark {
            RoundMark::Line{..}=>lines+=1,
            RoundMark::Outline(_)=>outlines+=1,
            RoundMark::Dot(_)=>{}
        });
        assert!(lines>0);
        assert_eq!(outlines,0);
    }

    #[test]
    fn constant_pressure_gpen_uses_fast_line_not_filled_outline() {
        let s=Stroke {
            points:vec![point(0.,0.,1.0),point(10.,0.,1.0),point(10.,2.,1.0),
                point(2.,2.,1.0),point(2.,4.,1.0)],
            color:[0,0,0,255],fill_color:None,base_width:7.,is_vector:false,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut lines=0;
        let mut outlines=0;
        visit_round_marks(&s,|mark,_|match mark {
            RoundMark::Line{..}=>lines+=1,
            RoundMark::Outline(_)=>outlines+=1,
            RoundMark::Dot(_)=>{}
        });
        assert_eq!(lines,1);
        assert_eq!(outlines,0);
    }

    #[test]
    fn meaningful_pressure_gpen_keeps_curve_as_single_coverage_region() {
        let s=Stroke {
            points:vec![point(0.,0.,0.25),point(20.,0.,0.7),point(40.,5.,1.0)],
            color:[0,0,0,255],fill_color:None,base_width:8.,is_vector:false,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut outlines=0;
        visit_round_marks(&s,|mark,_|if matches!(mark,RoundMark::Outline(_)){outlines+=1});
        assert_eq!(outlines,1);
    }

    #[test]
    fn raster_curve_fit_densifies_a_curve_without_moving_endpoints() {
        let raw=[
            point(0.,0.,1.),point(35.,8.,0.8),point(70.,35.,0.6),point(95.,80.,1.0)
        ];
        let smooth=smooth_freehand_points(&raw,10.);
        assert!(smooth.len()>raw.len());
        assert_eq!((smooth.first().unwrap().x,smooth.first().unwrap().y),(0.,0.));
        assert_eq!((smooth.last().unwrap().x,smooth.last().unwrap().y),(95.,80.));
        assert!(smooth.windows(2).all(|w|(w[1].x-w[0].x).hypot(w[1].y-w[0].y)<=4.0));
    }

    #[test]
    fn tiny_pressure_noise_does_not_force_polygon_geometry() {
        let s=Stroke {
            points:vec![point(0.,0.,0.99),point(10.,0.,1.0),point(20.,0.,1.01)],
            color:[0,0,0,255],fill_color:None,base_width:8.,is_vector:false,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.,brush_texture_id:None,
            tip_shape:BrushTipShape::Round,
        };
        let mut lines=0;
        visit_round_marks(&s,|mark,_|if matches!(mark,RoundMark::Line{..}){lines+=1});
        assert_eq!(lines,1);
    }

    #[test]
    fn risky_hairpins_are_detected_for_safe_brush_fallback() {
        let hairpin=[point(0.,0.,1.),point(20.,0.,1.),point(2.,2.,1.)];
        let gentle=[point(0.,0.,1.),point(20.,1.,1.),point(40.,4.,1.)];
        assert!(has_risky_turn(&hairpin));
        assert!(!has_risky_turn(&gentle));
    }
    #[test]
    fn tapered_pen_reduces_end_pressure_and_preserves_midstroke() {
        let points=[point(0.,0.,1.),point(10.,0.,1.),point(20.,0.,1.),point(30.,0.,1.)];
        let profile=taper_pressure_profile(&points);
        assert!(profile[0].pressure<profile[1].pressure);
        assert!(profile[3].pressure<profile[2].pressure);
        assert!(profile[1].pressure>0.95);
    }
    #[test]
    fn smooth_ink_keeps_intentional_sharp_corners() {
        let points=[point(0.,0.,1.),point(10.,0.,1.),point(10.,10.,1.)];
        let sample=smooth_ink_points(&points,8.);
        assert!(sample.len()>points.len());
        assert!(sample.iter().any(|p|(p.x-10.).abs()<0.001 && p.y.abs()<0.001));
        assert_eq!((sample.first().unwrap().x,sample.first().unwrap().y),(0.,0.));
        assert_eq!((sample.last().unwrap().x,sample.last().unwrap().y),(10.,10.));
        let j=[point(0.,0.,1.),point(10.,1.,1.),point(20.,0.,1.)];
        let smooth=smooth_ink_points(&j,12.);
        let center=smooth.iter().min_by(|a,b|
            (a.x-10.).abs().total_cmp(&(b.x-10.).abs())).unwrap();
        assert!(center.y<j[1].y);
    }
    #[test]
    fn general_freehand_smoothing_reduces_micro_jitter_without_rounding_corner() {
        let jitter=[point(0.,0.,1.),point(8.,0.9,1.),point(16.,0.,1.),point(24.,0.8,1.),point(32.,0.,1.)];
        let smooth=smooth_freehand_points(&jitter,10.);
        let near8=smooth.iter().min_by(|a,b|
            (a.x-8.).abs().total_cmp(&(b.x-8.).abs())).unwrap();
        let near24=smooth.iter().min_by(|a,b|
            (a.x-24.).abs().total_cmp(&(b.x-24.).abs())).unwrap();
        assert!(near8.y<jitter[1].y);
        assert!(near24.y<jitter[3].y);
        assert_eq!(smooth.first(),jitter.first());
        assert_eq!(smooth.last(),jitter.last());
        let corner=[point(0.,0.,1.),point(20.,0.,1.),point(20.,20.,1.)];
        let fitted=smooth_freehand_points(&corner,12.);
        assert!(fitted.iter().any(|p|(p.x-20.).abs()<0.001 && p.y.abs()<0.001));
        assert_eq!((fitted.first().unwrap().x,fitted.first().unwrap().y),(0.,0.));
        assert_eq!((fitted.last().unwrap().x,fitted.last().unwrap().y),(20.,20.));
    }

    #[test]
    fn invalid_or_empty_input_is_safe() {
        assert!(sample_pressure_dabs(&[], 7.0).is_empty());
        assert!(sample_pressure_dabs(&[point(1.0, 1.0, 1.0)], f32::NAN).is_empty());
        assert!(sample_pressure_dabs(&[point(f32::NAN, 0.0, 1.0)], 7.0).is_empty());
    }

    #[test]
    fn dab_count_stays_bounded_on_long_strokes() {
        let dabs = sample_pressure_dabs(&[point(0.0, 0.0, 1.0), point(200_000.0, 0.0, 1.0)], 6.0);
        assert!(dabs.len() <= MAX_DABS_PER_STROKE);
        assert!((dabs.last().unwrap().x - 200_000.0).abs() < 0.01);
        assert!(dabs.windows(2).all(|p| p[1].x - p[0].x <= 11.0));
    }

    #[test]
    fn bounded_dense_sampler_preserves_endpoint() {
        let points=[point(0.,0.,1.),point(50_000.,0.,1.)];
        let mut dabs=Vec::new();
        visit_pressure_dabs_spaced_dense_bounded(&points,4.,0.02,256,
            |dab,_|dabs.push(dab));
        assert!(dabs.len()<=256);
        assert!((dabs.last().unwrap().x-50_000.).abs()<0.01);
    }

    #[test]
    fn pressure_spacing_is_independent_of_input_event_density() {
        let sparse = [point(0., 0., 0.25), point(100., 0., 1.0)];
        let dense: Vec<_> = (0..=100)
            .map(|i| point(i as f32, 0., 0.25 + 0.75 * i as f32 / 100.))
            .collect();
        let samples = sample_pressure_dabs(&sparse, 8.);
        let other = sample_pressure_dabs(&dense, 8.);
        assert_eq!(samples.len(), other.len());
        assert!(samples
            .iter()
            .zip(other)
            .all(|(a, b)| (a.x - b.x).abs() < 0.001 && (a.radius - b.radius).abs() < 0.001));
    }
}
