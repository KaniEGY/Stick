//! Continuous, pressure-aware stroke geometry for the canvas and exporters.
//! A single filled outline prevents translucent dab overlap from making
//! unwanted dark joints. The saved raw input points remain untouched.
use crate::project::StrokePoint;

const MAX_INPUT_POINTS: usize = 12_000;
const MAX_RENDER_SAMPLES: usize = 24_000;
const ROUND_CAP_STEPS: usize = 12;

/// Speed-aware pointer stabilizer for live freehand input. Slow, tiny motion
/// gets the requested smoothing, while fast travel progressively catches up to
/// the real pointer so higher stabilizer values do not feel like a long rubber
/// band. This only filters input coordinates; pressure is handled separately.
pub fn adaptive_stabilized_xy(
    previous_filtered: (f32, f32),
    previous_raw: Option<(f32, f32)>,
    raw: (f32, f32),
    strength: f32,
) -> (f32, f32) {
    if !previous_filtered.0.is_finite() || !previous_filtered.1.is_finite()
        || !raw.0.is_finite() || !raw.1.is_finite() {
        return raw;
    }
    let strength = strength.clamp(0.0, 0.85);
    if strength <= 0.001 { return raw; }

    let lag = (raw.0 - previous_filtered.0).hypot(raw.1 - previous_filtered.1);
    let travel = previous_raw
        .filter(|p| p.0.is_finite() && p.1.is_finite())
        .map(|p| (raw.0 - p.0).hypot(raw.1 - p.1))
        .unwrap_or(lag);

    let base_follow = (1.0 - strength * 0.95).clamp(0.18, 1.0);
    let motion = (travel / 5.0).clamp(0.0, 1.0);
    let separation = (lag / 12.0).clamp(0.0, 1.0);
    let catch_up = motion.max(separation);
    let follow = base_follow + (1.0 - base_follow) * catch_up;

    (
        previous_filtered.0 + (raw.0 - previous_filtered.0) * follow,
        previous_filtered.1 + (raw.1 - previous_filtered.1) * follow,
    )
}

fn length(a: &StrokePoint, b: &StrokePoint) -> f32 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// De-jitter gentle segments, preserve genuine turns, then resample by
/// travelled distance. The sample budget bounds memory and rendering cost.
///
/// This is render-time geometry only. It does not simplify project strokes,
/// move editable vector nodes or fabricate stylus pressure.
fn prepared_points(points: &[StrokePoint], width: f32) -> Vec<StrokePoint> {
    if points.len() > MAX_INPUT_POINTS || !width.is_finite() || width <= 0.0 {
        return Vec::new();
    }
    let mut clean: Vec<StrokePoint> = Vec::with_capacity(points.len());
    for p in points {
        if !p.x.is_finite() || !p.y.is_finite()
            || !p.pressure.is_finite() || p.pressure <= 0.0 {
            continue;
        }
        if clean.last().is_none_or(|last| length(last, p) > 0.3) {
            clean.push(p.clone());
        }
    }
    if clean.len() < 2 { return Vec::new(); }

    // Only smooth small cross-track deviations where the stroke direction
    // stays broadly the same. Do not round intentional 60–180 degree corners.
    let original = clean.clone();
    let tolerance = (width * 0.17).clamp(0.35, 2.25);
    for i in 1..clean.len() - 1 {
        let (a, b, c) = (&original[i - 1], &original[i], &original[i + 1]);
        let l1 = length(a, b);
        let l2 = length(b, c);
        let chord = length(a, c);
        if l1 < 0.001 || l2 < 0.001 || chord < 0.001 { continue; }
        let cos = ((b.x - a.x) * (c.x - b.x) +
            (b.y - a.y) * (c.y - b.y)) / (l1 * l2);
        let cross_track = ((b.x - a.x) * (c.y - a.y)
            - (b.y - a.y) * (c.x - a.x)).abs() / chord;
        if cos > 0.62 && cross_track <= tolerance {
            clean[i].x = 0.20 * a.x + 0.60 * b.x + 0.20 * c.x;
            clean[i].y = 0.20 * a.y + 0.60 * b.y + 0.20 * c.y;
        }
    }

    let total: f32 = clean.windows(2).map(|pair| length(&pair[0], &pair[1])).sum();
    let budget = MAX_RENDER_SAMPLES.saturating_sub(clean.len()).max(1);
    let spacing = (width * 0.15).clamp(1.0, 3.5)
        .max(total / budget as f32);
    let mut samples = Vec::with_capacity(clean.len().min(MAX_RENDER_SAMPLES));
    samples.push(clean[0].clone());
    for pair in clean.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let segment = length(a, b);
        if segment <= 0.001 { continue; }
        // ceil(segment/spacing)-1 < segment/spacing, so the total extra
        // samples cannot exceed the global memory budget.
        let steps = ((segment / spacing).ceil() as usize).clamp(1, MAX_RENDER_SAMPLES);
        for j in 1..=steps {
            let t = j as f32 / steps as f32;
            if j == steps {
                samples.push(b.clone());
            } else {
                samples.push(StrokePoint {
                    x: a.x + (b.x - a.x) * t,
                    y: a.y + (b.y - a.y) * t,
                    pressure: a.pressure + (b.pressure - a.pressure) * t,
                });
            }
        }
    }
    samples
}

fn normalized(dx: f32, dy: f32) -> (f32, f32) {
    let magnitude = dx.hypot(dy);
    if magnitude > 0.0001 { (dx / magnitude, dy / magnitude) }
    else { (1.0, 0.0) }
}

/// A single outline with round ends and pressure-varying width. A bounded
/// miter join prevents long spikes at acute angles. Both the canvas and
/// tiny-skia exporter fill exactly this polygon.
pub fn pressure_outline(points: &[StrokePoint], width: f32) -> Vec<(f32, f32)> {
    let samples = prepared_points(points, width);
    if samples.len() < 2 { return Vec::new(); }
    let n = samples.len();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    let mut radii = Vec::with_capacity(n);

    for i in 0..n {
        let center = &samples[i];
        let before = &samples[i.saturating_sub(1)];
        let after = &samples[(i + 1).min(n - 1)];
        let prev_dir = normalized(center.x - before.x, center.y - before.y);
        let next_dir = normalized(after.x - center.x, after.y - center.y);
        let (nx, ny, miter) = if i == 0 {
            (-next_dir.1, next_dir.0, 1.0)
        } else if i == n - 1 {
            (-prev_dir.1, prev_dir.0, 1.0)
        } else {
            let (n0x, n0y) = (-prev_dir.1, prev_dir.0);
            let (n1x, n1y) = (-next_dir.1, next_dir.0);
            let sum = normalized(n0x + n1x, n0y + n1y);
            let denom = (sum.0 * n1x + sum.1 * n1y).abs();
            (sum.0, sum.1, (1.0 / denom.max(0.01)).min(1.5))
        };
        let pressure = center.pressure.clamp(0.05, 2.0);
        let radius = (width * 0.5 * pressure).clamp(0.25, 512.0);
        radii.push(radius);
        let offset = radius * miter;
        left.push((center.x + nx * offset, center.y + ny * offset));
        right.push((center.x - nx * offset, center.y - ny * offset));
    }

    let mut outline = Vec::with_capacity(n * 2 + ROUND_CAP_STEPS * 2);
    outline.extend(left);

    // End cap continues the left boundary towards the right boundary.
    let end = &samples[n - 1];
    let before = &samples[n - 2];
    let (tx, ty) = normalized(end.x - before.x, end.y - before.y);
    let angle = ty.atan2(tx) + std::f32::consts::FRAC_PI_2;
    for j in 1..=ROUND_CAP_STEPS {
        let a = angle - std::f32::consts::PI * j as f32 / ROUND_CAP_STEPS as f32;
        outline.push((end.x + a.cos() * radii[n - 1],
                      end.y + a.sin() * radii[n - 1]));
    }

    outline.extend(right.into_iter().rev().skip(1));

    // Start cap returns the right boundary to the first left vertex.
    let start = &samples[0];
    let next = &samples[1];
    let (tx, ty) = normalized(next.x - start.x, next.y - start.y);
    let angle = ty.atan2(tx) - std::f32::consts::FRAC_PI_2;
    for j in 1..ROUND_CAP_STEPS {
        let a = angle - std::f32::consts::PI * j as f32 / ROUND_CAP_STEPS as f32;
        outline.push((start.x + a.cos() * radii[0],
                      start.y + a.sin() * radii[0]));
    }
    outline
}


#[cfg(test)]
mod tests {
    use super::*;
    fn p(x:f32,y:f32,pressure:f32)->StrokePoint { StrokePoint{x,y,pressure} }

    #[test]
    fn pressure_changes_width_without_dark_overlapping_dabs() {
        let nodes = [p(0.,0.,1.),p(50.,0.,0.5),p(100.,0.,1.)];
        let result=pressure_outline(&nodes,20.);
        let start_y=result.first().unwrap().1;
        let middle_y=result.iter().filter(|&&(x,_)|(x-50.).abs()<0.1)
            .map(|&(_,y)|y).next().unwrap();
        assert!((start_y-middle_y).abs()>3.0);
        assert!(result.iter().all(|&(x,y)| x.is_finite() && y.is_finite()));
    }

    #[test]
    fn straight_and_fast_strokes_get_reasonable_spacing() {
        let nodes=[p(0.,0.,1.),p(500.,0.,1.)];
        let samples=prepared_points(&nodes,8.);
        assert!(samples.len()>50);
        assert!(samples.windows(2).all(|w|w[0].x<=w[1].x && w[1].x-w[0].x<=4.));
        assert!(samples.iter().all(|p|p.y==0.));
    }

    #[test]
    fn jitter_is_reduced_without_moving_hard_corners() {
        let jitter = [p(0.,0.,1.),p(10.,1.,1.),p(20.,0.,1.)];
        let points=prepared_points(&jitter,12.);
        let center=points.iter().find(|p|(p.x-10.).abs()<0.01).unwrap();
        assert!(center.y<1.);
        let corners=[p(0.,0.,1.),p(20.,0.,1.),p(20.,20.,1.)];
        let points=prepared_points(&corners,12.);
        assert!(points.iter().any(|p|p.x==20.&&p.y==0.));
    }

    #[test]
    fn curve_and_circle_are_finite_and_bounded() {
        let nodes=[p(0.,-30.,1.),p(30.,0.,1.),p(0.,30.,1.),
            p(-30.,0.,1.),p(0.,-30.,1.)];
        let outline=pressure_outline(&nodes,6.);
        assert!(!outline.is_empty());
        assert!(outline.iter().all(|&(x,y)|x.is_finite()&&y.is_finite()));
        assert!(outline.len()<=MAX_RENDER_SAMPLES*2+ROUND_CAP_STEPS*2);
    }

    #[test]
    fn duplicate_or_invalid_points_are_safe() {
        assert!(pressure_outline(&[p(1.,1.,1.),p(1.,1.,1.)],10.).is_empty());
        assert!(pressure_outline(&[p(1.,1.,1.),p(f32::NAN,2.,1.)],10.).is_empty());
        assert!(pressure_outline(&[],5.).is_empty());
        assert!(pressure_outline(&[p(1.,1.,1.),p(2.,2.,1.)],f32::NAN).is_empty());
    }

    #[test]
    fn adaptive_stabilizer_filters_jitter_but_catches_fast_motion() {
        let slow=adaptive_stabilized_xy((0.,0.),Some((0.,0.)),(1.,0.),0.70);
        assert!(slow.0>0.18 && slow.0<0.60, "slow={slow:?}");
        let fast=adaptive_stabilized_xy((0.,0.),Some((0.,0.)),(20.,0.),0.70);
        assert!(fast.0>18.0, "fast={fast:?}");
        assert_eq!(adaptive_stabilized_xy((2.,3.),Some((2.,3.)),(9.,11.),0.0),(9.,11.));
    }

    #[test]
    fn adaptive_stabilizer_catches_moderate_motion_without_tail_lag() {
        let next = adaptive_stabilized_xy((0.,0.), Some((0.,0.)), (5.,0.), 0.75);
        assert_eq!(next, (5.,0.));
        // Even if individual events are tiny, accumulated separation must catch up.
        let next = adaptive_stabilized_xy((0.,0.), Some((11.,0.)), (12.,0.), 0.75);
        assert_eq!(next, (12.,0.));
    }

    #[test]
    fn adaptive_stabilizer_never_produces_non_finite_output() {
        let result=adaptive_stabilized_xy((f32::NAN,0.),None,(4.,5.),0.7);
        assert_eq!(result,(4.,5.));
    }

    #[test]
    fn render_sample_budget_is_never_exceeded() {
        let nodes:Vec<_>=(0..10_000).map(|i|p(i as f32*10.,i as f32%7.,1.)).collect();
        let sampled=prepared_points(&nodes,2.);
        assert!(sampled.len()<=MAX_RENDER_SAMPLES);
        assert_eq!(sampled.first().unwrap().x,0.);
        assert_eq!(sampled.last().unwrap().x,99_990.);
    }
}
