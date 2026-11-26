//! Accurate eraser geometry for both complete-stroke and partial erasers.
//! The previous implementation tested only stored vertices, allowing a long
//! segment to pass straight through the eraser with no effect.
use crate::project::{Stroke, StrokePoint};
use crate::fill_tools::contour_contains;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EraserMode {
    Partial,
    WholeStroke,
}
impl Default for EraserMode {
    fn default()->Self{Self::Partial}
}


fn distance_sq_segment(a:&StrokePoint,b:&StrokePoint,c:(f32,f32)) -> f32 {
    let dx=b.x-a.x;
    let dy=b.y-a.y;
    let length_sq=dx*dx+dy*dy;
    let t=if length_sq>1e-8 {
        ((c.0-a.x)*dx+(c.1-a.y)*dy)/length_sq
    }else{0.0}.clamp(0.0,1.0);
    ((a.x+dx*t)-c.0).powi(2)+((a.y+dy*t)-c.1).powi(2)
}
fn lerp(a:&StrokePoint,b:&StrokePoint,t:f32)->StrokePoint {
    StrokePoint{x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t,
        pressure:a.pressure+(b.pressure-a.pressure)*t}
}
fn outside(p:&StrokePoint,c:(f32,f32),radius_sq:f32)->bool {
    (p.x-c.0).powi(2)+(p.y-c.1).powi(2)>radius_sq
}
fn stroke_touched(s:&Stroke,c:(f32,f32),radius:f32)->bool {
    if crate::raster_patch::is_patch(s) {return crate::raster_patch::touches_circle(s,c,radius);}
    if s.points.is_empty() { return false; }
    if s.fill_color.is_some() && contour_contains(s,c.0,c.1) {return true;}
    let hit=radius.max(0.0)+(s.base_width.abs()*0.5);
    let hit_sq=hit*hit;
    if s.points.len()==1 {
        return !outside(&s.points[0],c,hit_sq);
    }
    s.points.windows(2).any(|w| distance_sq_segment(&w[0],&w[1],c)<=hit_sq)
}

/// Full eraser: remove an entire object when any part of its stroke passes
/// through the circular hit area.
pub fn erase_whole(strokes:&mut Vec<Stroke>,center:(f32,f32),radius:f32)->bool {
    if !center.0.is_finite() || !center.1.is_finite() || !radius.is_finite() || radius<=0.0{return false;}
    let old=strokes.len();
    strokes.retain(|s|!stroke_touched(s,center,radius));
    strokes.len()!=old
}

fn split_stroke(mut s:Stroke,c:(f32,f32),radius:f32)->Vec<Stroke> {
    // Callers already tested the visible footprint, including the width of a dab.
    if s.points.len()<2 { return Vec::new(); }
    // The fill-only element is a region, not a drawable path. Removing its
    // polygon corners would leave invalid fill geometry; erase it as a region.
    if s.fill_color.is_some() {
        if stroke_touched(&s,c,radius) {return Vec::new();}
        return vec![s];
    }
    let effective=(radius+s.base_width.abs()*0.5).max(0.01);
    let points = std::mem::take(&mut s.points);
    let radius_sq=effective*effective;
    let mut output=Vec::new();
    let mut part:Vec<StrokePoint>=Vec::new();
    let finish=|part:&mut Vec<StrokePoint>,output:&mut Vec<Stroke>|{
        if !part.is_empty() {
            let mut piece=s.clone();
            piece.points=std::mem::take(part);
            piece.fill_color=None;
            output.push(piece);
        }
    };
    for pair in points.windows(2) {
        let (a,b)=(&pair[0],&pair[1]);
        let dx=b.x-a.x;
        let dy=b.y-a.y;
        let len_sq=dx*dx+dy*dy;
        let mut cuts=vec![0.0_f32,1.0_f32];
        if len_sq>1e-10 {
            let fx=a.x-c.0;
            let fy=a.y-c.1;
            let dot=fx*dx+fy*dy;
            let disc=dot*dot-len_sq*(fx*fx+fy*fy-radius_sq);
            if disc>=0.0 && disc.is_finite() {
                let root=disc.sqrt();
                for t in [(-dot-root)/len_sq,(-dot+root)/len_sq] {
                    if t>0.00001 && t<0.99999 {cuts.push(t);}
                }
            }
        }
        cuts.sort_by(f32::total_cmp);
        cuts.dedup_by(|a,b|(*a-*b).abs()<0.00001);
        for window in cuts.windows(2) {
            let (t0,t1)=(window[0],window[1]);
            if t1-t0<=0.00001 {continue;}
            let mid=lerp(a,b,(t0+t1)*0.5);
            if outside(&mid,c,radius_sq) {
                let start=lerp(a,b,t0);
                let end=lerp(a,b,t1);
                if part.last().is_some_and(|p|
                    (p.x-start.x).hypot(p.y-start.y)>0.001) {
                    finish(&mut part,&mut output);
                }
                if part.is_empty(){part.push(start);}
                if part.last().is_none_or(|p|
                    (p.x-end.x).hypot(p.y-end.y)>0.001) {
                    part.push(end);
                }
            } else {
                finish(&mut part,&mut output);
            }
        }
    }
    finish(&mut part,&mut output);
    output
}

/// Partial eraser: clip segments against the eraser disc, even when no sampled
/// vertex is near the cursor. Retain legitimate one-point dabs.
pub fn erase_partial(strokes:&mut Vec<Stroke>,center:(f32,f32),radius:f32)->bool {
    if !center.0.is_finite()||!center.1.is_finite()||!radius.is_finite()||radius<=0.0{return false;}
    let mut changed=false;
    let mut output=Vec::with_capacity(strokes.len());
    for s in strokes.drain(..) {
        if crate::raster_patch::is_patch(&s) {
            let (piece,edited)=crate::raster_patch::erase_circle(s,center,radius);
            changed|=edited;
            if let Some(piece)=piece {output.push(piece);}
        } else if stroke_touched(&s,center,radius) {
            changed=true;
            output.extend(split_stroke(s,center,radius));
        } else {
            output.push(s);
        }
    }
    *strokes=output;
    changed
}

pub fn erase_with_mode(
    strokes:&mut Vec<Stroke>,center:(f32,f32),radius:f32,mode:EraserMode
)->bool{
    match mode {
        EraserMode::Partial=>erase_partial(strokes,center,radius),
        EraserMode::WholeStroke=>erase_whole(strokes,center,radius),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{BrushTipShape,KritaBrushPreset};
    fn line(points:&[(f32,f32)])->Stroke{
        Stroke{points:points.iter().map(|&(x,y)|StrokePoint{x,y,pressure:1.0}).collect(),
            color:[0,0,0,255],fill_color:None,base_width:4.0,is_vector:true,
            brush_preset:KritaBrushPreset::GPenInk,opacity:1.0,
            brush_texture_id:None,tip_shape:BrushTipShape::Round}
    }
    #[test]
    fn complete_eraser_hits_long_segment_without_nearby_vertices() {
        let mut strokes=vec![line(&[(0.,50.),(100.,50.)])];
        assert!(erase_whole(&mut strokes,(50.,50.),8.));
        assert!(strokes.is_empty());
    }
    #[test]
    fn partial_eraser_splits_line_at_disc_boundaries() {
        let mut strokes=vec![line(&[(0.,50.),(100.,50.)])];
        assert!(erase_partial(&mut strokes,(50.,50.),8.));
        assert_eq!(strokes.len(),2);
        assert!((strokes[0].points.last().unwrap().x-40.).abs()<0.1);
        assert!((strokes[1].points.first().unwrap().x-60.).abs()<0.1);
    }
    #[test]
    fn erasers_leave_distant_stroke_unchanged() {
        let mut strokes=vec![line(&[(0.,50.),(100.,50.)])];
        assert!(!erase_partial(&mut strokes,(50.,200.),8.));
        assert!(!erase_whole(&mut strokes,(50.,200.),8.));
        assert_eq!(strokes.len(),1);
    }
    #[test]
    fn single_dab_is_erased_without_creating_invalid_fragments() {
        let mut strokes=vec![line(&[(12.,12.)])];
        // Brush and eraser rims overlap while the dab's center is outside.
        assert!(erase_partial(&mut strokes,(18.,12.),5.));
        assert!(strokes.is_empty());
    }
}
