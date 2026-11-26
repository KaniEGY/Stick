//! Finger/sculpt brush for editing vector-backed and freehand strokes.
//!
//! Performance rule: topology preparation happens once at the start of a
//! sculpt gesture. Pointer-move events only displace existing points. This
//! prevents the old progressive point-growth/Vec rebuild that caused lag after
//! using Sculpt repeatedly.
use crate::project::{Stroke,StrokePoint};

const MAX_SCULPT_POINTS_PER_STROKE: usize = 6_000;

fn stroke_near(stroke:&Stroke,at:(f32,f32),pad:f32)->bool {
    if stroke.points.is_empty(){return false;}
    let mut min_x=f32::INFINITY; let mut max_x=f32::NEG_INFINITY;
    let mut min_y=f32::INFINITY; let mut max_y=f32::NEG_INFINITY;
    for p in &stroke.points {
        min_x=min_x.min(p.x); max_x=max_x.max(p.x);
        min_y=min_y.min(p.y); max_y=max_y.max(p.y);
    }
    !(at.0 < min_x-pad || at.0 > max_x+pad || at.1 < min_y-pad || at.1 > max_y+pad)
}

/// Prepare sparse vector/fill contours once when the mouse button goes down.
/// Repeated pointer movement must never call this function.
pub fn prepare_sculpt(strokes:&mut [Stroke],at:(f32,f32),radius:f32) {
    if !at.0.is_finite()||!at.1.is_finite()||!radius.is_finite(){return;}
    let radius=radius.clamp(2.0,500.0);
    let step=(radius*0.24).clamp(5.0,30.0);
    for stroke in strokes {
        if stroke.points.len()<2 || stroke.points.len()>=MAX_SCULPT_POINTS_PER_STROKE {continue;}
        let pad=radius+stroke.base_width.max(1.0)*0.5;
        if !stroke_near(stroke,at,pad){continue;}

        let was_closed=stroke.points.len()>2
            && stroke.points.first().zip(stroke.points.last())
                .is_some_and(|(a,b)|(a.x-b.x).hypot(a.y-b.y)<=0.001);

        let needs_refine=stroke.points.windows(2).any(|pair|{
            let a=&pair[0]; let b=&pair[1];
            let length=(b.x-a.x).hypot(b.y-a.y);
            if length<=step*1.8{return false;}
            let mx=(a.x+b.x)*0.5; let my=(a.y+b.y)*0.5;
            (mx-at.0).hypot(my-at.1)<=radius+length*0.5
        });
        if !needs_refine {continue;}

        let mut dense=Vec::with_capacity((stroke.points.len()*2)
            .min(MAX_SCULPT_POINTS_PER_STROKE));
        dense.push(stroke.points[0].clone());
        for pair in stroke.points.windows(2) {
            let a=&pair[0]; let b=&pair[1];
            let length=(b.x-a.x).hypot(b.y-a.y);
            let mx=(a.x+b.x)*0.5; let my=(a.y+b.y)*0.5;
            let close=(mx-at.0).hypot(my-at.1)<=radius+length*0.5;
            let subdivisions=if close {
                (length/step).ceil().clamp(1.0,6.0) as usize
            } else {1};
            for k in 1..=subdivisions {
                let t=k as f32/subdivisions as f32;
                dense.push(StrokePoint{
                    x:a.x+(b.x-a.x)*t,
                    y:a.y+(b.y-a.y)*t,
                    pressure:a.pressure+(b.pressure-a.pressure)*t,
                });
                if dense.len()>=MAX_SCULPT_POINTS_PER_STROKE {break;}
            }
            if dense.len()>=MAX_SCULPT_POINTS_PER_STROKE {break;}
        }
        if dense.len()<MAX_SCULPT_POINTS_PER_STROKE {
            if was_closed && dense.len()>2 {
                let first=dense[0].clone();
                let last=dense.len()-1;
                dense[last]=first;
            }
            stroke.points=dense;
        }
    }
}

pub fn sculpt(strokes:&mut [Stroke],at:(f32,f32),motion:(f32,f32),radius:f32,strength:f32)->bool {
    if !at.0.is_finite()||!at.1.is_finite()
        ||!motion.0.is_finite()||!motion.1.is_finite()
        ||!radius.is_finite()||radius<2.0
        ||!strength.is_finite()||strength<=0.0{return false;}
    let radius=radius.clamp(2.0,500.0);
    let strength=strength.clamp(0.0,1.0);
    let mut changed=false;

    for stroke in strokes {
        if stroke.points.is_empty() || crate::raster_patch::is_patch(stroke) {continue;}
        let pad=radius+stroke.base_width.max(1.0)*0.5;
        if !stroke_near(stroke,at,pad){continue;}

        let was_closed=stroke.points.len()>2
            && stroke.points.first().zip(stroke.points.last())
                .is_some_and(|(a,b)|(a.x-b.x).hypot(a.y-b.y)<=0.001);

        // Hot path: no allocations, no point insertion and no Vec rebuild.
        for point in &mut stroke.points {
            let d=(point.x-at.0).hypot(point.y-at.1);
            if d>=radius{continue;}
            let t=(1.0-d/radius).clamp(0.0,1.0);
            let w=t*t*(3.0-2.0*t)*strength;
            point.x+=motion.0*w;
            point.y+=motion.1*w;
            changed=true;
        }

        if was_closed && stroke.points.len()>2 {
            let first=stroke.points[0].clone();
            let last=stroke.points.len()-1;
            stroke.points[last]=first;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{BrushTipShape,KritaBrushPreset};

    fn stroke(points:Vec<StrokePoint>,fill:Option<[u8;4]>)->Stroke {
        Stroke{points,color:[0,0,0,255],fill_color:fill,base_width:4.,is_vector:true,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:None,tip_shape:BrushTipShape::Round}
    }

    #[test]
    fn sculpt_deforms_filled_closed_contours_and_keeps_the_seam_closed(){
        let mut s=stroke(vec![
            StrokePoint{x:0.,y:0.,pressure:1.},StrokePoint{x:100.,y:0.,pressure:1.},
            StrokePoint{x:100.,y:100.,pressure:1.},StrokePoint{x:0.,y:100.,pressure:1.},
            StrokePoint{x:0.,y:0.,pressure:1.}],Some([20,40,80,255]));
        prepare_sculpt(std::slice::from_mut(&mut s),(50.,0.),35.);
        assert!(sculpt(std::slice::from_mut(&mut s),(50.,0.),(0.,20.),35.,1.0));
        assert_eq!(s.points.first(),s.points.last());
        assert!(s.points.iter().any(|p|p.y>0.));
    }

    #[test]
    fn finger_moves_local_midsegment_but_not_far_points(){
        let mut s=stroke(vec![
            StrokePoint{x:0.,y:10.,pressure:1.},
            StrokePoint{x:200.,y:10.,pressure:1.}],None);
        prepare_sculpt(std::slice::from_mut(&mut s),(100.,10.),40.);
        assert!(s.points.len()>2);
        assert!(sculpt(std::slice::from_mut(&mut s),(100.,10.),(0.,30.),40.,0.8));
        assert!(s.points.iter().any(|p|p.y>10.));
        assert_eq!(s.points.first().unwrap().y,10.);
        assert_eq!(s.points.last().unwrap().y,10.);
    }

    #[test]
    fn pointer_moves_never_grow_topology_after_prepare(){
        let mut s=stroke(vec![
            StrokePoint{x:0.,y:0.,pressure:1.},
            StrokePoint{x:240.,y:0.,pressure:1.}],None);
        prepare_sculpt(std::slice::from_mut(&mut s),(120.,0.),45.);
        let prepared=s.points.len();
        for x in 80..160 {
            let _=sculpt(std::slice::from_mut(&mut s),(x as f32,0.),(1.,0.4),45.,0.8);
        }
        assert_eq!(s.points.len(),prepared);
    }
}
