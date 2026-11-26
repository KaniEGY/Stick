//! High-value drawing helpers shared by Stick's Drawing workspace.
//! The algorithms here are deliberately independent from UI state so they can
//! be unit-tested and reused by live canvas, undoable actions and exporters.

use crate::project::{BrushTipShape, KritaBrushPreset, Stroke, StrokePoint};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerspectiveSnap {
    pub mode: u8,
    pub width: f32,
    pub height: f32,
    pub vp_x: f32,
    pub vp_y: f32,
    pub left_vp_x: f32,
    pub right_vp_x: f32,
    pub two_point_y: f32,
    pub parallel_angle_deg: f32,
}

fn project_to_line(anchor:(f32,f32), target:(f32,f32), dir:(f32,f32))->(f32,f32) {
    let len2=dir.0*dir.0+dir.1*dir.1;
    if !len2.is_finite() || len2<=1e-6 {return target}
    let dx=target.0-anchor.0;
    let dy=target.1-anchor.1;
    let t=(dx*dir.0+dy*dir.1)/len2;
    (anchor.0+dir.0*t,anchor.1+dir.1*t)
}

fn dist2(a:(f32,f32),b:(f32,f32))->f32 {
    let dx=a.0-b.0; let dy=a.1-b.1; dx*dx+dy*dy
}

/// Snap a live pointer sample to the active perspective ruler. The anchor is
/// normally the previous committed stroke point, matching drawing-app ruler
/// behavior without destructively rewriting old points.
pub fn snap_to_perspective(
    anchor:(f32,f32), target:(f32,f32), cfg:PerspectiveSnap
)->(f32,f32) {
    if cfg.mode==0 || !anchor.0.is_finite() || !anchor.1.is_finite()
        || !target.0.is_finite() || !target.1.is_finite()
        || cfg.width<=0.0 || cfg.height<=0.0
    {
        return target;
    }
    match cfg.mode {
        1 => {
            let horizontal=(target.0,anchor.1);
            let vertical=(anchor.0,target.1);
            if dist2(horizontal,target)<=dist2(vertical,target){horizontal}else{vertical}
        }
        2 => {
            let a=cfg.parallel_angle_deg.to_radians();
            project_to_line(anchor,target,(a.cos(),a.sin()))
        }
        3 => {
            let vp=(cfg.width*cfg.vp_x.clamp(0.0,1.0),
                    cfg.height*cfg.vp_y.clamp(0.0,1.0));
            project_to_line(anchor,target,(vp.0-anchor.0,vp.1-anchor.1))
        }
        4 => {
            let y=cfg.height*cfg.two_point_y.clamp(0.0,1.0);
            let left=(cfg.width*cfg.left_vp_x.clamp(-1.0,2.0),y);
            let right=(cfg.width*cfg.right_vp_x.clamp(-1.0,2.0),y);
            let a=project_to_line(anchor,target,(left.0-anchor.0,left.1-anchor.1));
            let b=project_to_line(anchor,target,(right.0-anchor.0,right.1-anchor.1));
            if dist2(a,target)<=dist2(b,target){a}else{b}
        }
        _=>target,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartShapeKind { Line, Ellipse, Rectangle, Triangle }

#[derive(Clone, Debug)]
pub struct SmartShape {
    pub kind: SmartShapeKind,
    pub points: Vec<StrokePoint>,
    pub confidence: f32,
}

fn path_length(points:&[StrokePoint])->f32 {
    points.windows(2).map(|p|(p[1].x-p[0].x).hypot(p[1].y-p[0].y)).sum()
}

fn bbox(points:&[StrokePoint])->Option<(f32,f32,f32,f32)> {
    let mut minx=f32::INFINITY; let mut miny=f32::INFINITY;
    let mut maxx=f32::NEG_INFINITY; let mut maxy=f32::NEG_INFINITY;
    for p in points {
        if !p.x.is_finite()||!p.y.is_finite(){return None}
        minx=minx.min(p.x); miny=miny.min(p.y);
        maxx=maxx.max(p.x); maxy=maxy.max(p.y);
    }
    (minx<maxx && miny<maxy).then_some((minx,miny,maxx,maxy))
}

fn make_line(a:&StrokePoint,b:&StrokePoint)->Vec<StrokePoint>{
    vec![a.clone(),b.clone()]
}
fn make_rect(minx:f32,miny:f32,maxx:f32,maxy:f32,pressure:f32)->Vec<StrokePoint>{
    vec![
        StrokePoint{x:minx,y:miny,pressure},
        StrokePoint{x:maxx,y:miny,pressure},
        StrokePoint{x:maxx,y:maxy,pressure},
        StrokePoint{x:minx,y:maxy,pressure},
        StrokePoint{x:minx,y:miny,pressure},
    ]
}
fn make_ellipse(minx:f32,miny:f32,maxx:f32,maxy:f32,pressure:f32)->Vec<StrokePoint>{
    let cx=(minx+maxx)*0.5; let cy=(miny+maxy)*0.5;
    let rx=(maxx-minx)*0.5; let ry=(maxy-miny)*0.5;
    let mut out=Vec::with_capacity(49);
    for i in 0..=48 {
        let a=std::f32::consts::TAU*i as f32/48.0;
        out.push(StrokePoint{x:cx+rx*a.cos(),y:cy+ry*a.sin(),pressure});
    }
    out
}

fn point_segment_distance(p:(f32,f32),a:(f32,f32),b:(f32,f32))->f32 {
    let vx=b.0-a.0; let vy=b.1-a.1;
    let len2=vx*vx+vy*vy;
    if len2<=1e-8{return (p.0-a.0).hypot(p.1-a.1)}
    let t=(((p.0-a.0)*vx+(p.1-a.1)*vy)/len2).clamp(0.0,1.0);
    (p.0-(a.0+vx*t)).hypot(p.1-(a.1+vy*t))
}
fn rdp(points:&[(f32,f32)],epsilon:f32)->Vec<(f32,f32)>{
    if points.len()<3{return points.to_vec()}
    let a=points[0]; let b=points[points.len()-1];
    let mut best=(0usize,0.0f32);
    for (i,p) in points.iter().enumerate().skip(1).take(points.len()-2) {
        let d=point_segment_distance(*p,a,b);
        if d>best.1{best=(i,d);}
    }
    if best.1>epsilon {
        let mut left=rdp(&points[..=best.0],epsilon);
        let right=rdp(&points[best.0..],epsilon);
        left.pop(); left.extend(right); left
    } else {vec![a,b]}
}

/// Recognise common hand-drawn primitives while preserving the caller's brush
/// style. The caller replaces only the centerline points; color/texture/width
/// remain untouched.
pub fn recognize_smart_shape(points:&[StrokePoint])->Option<SmartShape>{
    if points.len()<4 || points.len()>20_000{return None}
    let (minx,miny,maxx,maxy)=bbox(points)?;
    let w=maxx-minx; let h=maxy-miny;
    let diag=w.hypot(h).max(1.0);
    let path=path_length(points);
    let first=points.first()?; let last=points.last()?;
    let chord=(last.x-first.x).hypot(last.y-first.y);
    let avg_pressure=points.iter().map(|p|p.pressure).sum::<f32>()/points.len() as f32;

    if chord/diag>0.75 && path>0.0 && chord/path>0.965 {
        return Some(SmartShape{kind:SmartShapeKind::Line,
            points:make_line(first,last),confidence:(chord/path).clamp(0.0,1.0)});
    }

    let closed=(first.x-last.x).hypot(first.y-last.y)<=diag*0.22;
    if !closed || w<4.0 || h<4.0{return None}

    let cx=(minx+maxx)*0.5; let cy=(miny+maxy)*0.5;
    let rx=(w*0.5).max(1.0); let ry=(h*0.5).max(1.0);
    let mut radial_err=0.0;
    for p in points {
        let r=(((p.x-cx)/rx).powi(2)+((p.y-cy)/ry).powi(2)).sqrt();
        radial_err+=(r-1.0).abs();
    }
    radial_err/=points.len() as f32;
    if radial_err<0.16 {
        return Some(SmartShape{kind:SmartShapeKind::Ellipse,
            points:make_ellipse(minx,miny,maxx,maxy,avg_pressure),
            confidence:(1.0-radial_err/0.16).clamp(0.0,1.0)});
    }

    let edge_tol=(diag*0.045).clamp(2.0,18.0);
    let edge_hits=points.iter().filter(|p|{
        (p.x-minx).abs()<edge_tol||(p.x-maxx).abs()<edge_tol
            ||(p.y-miny).abs()<edge_tol||(p.y-maxy).abs()<edge_tol
    }).count();
    let edge_ratio=edge_hits as f32/points.len() as f32;
    if edge_ratio>0.76 {
        return Some(SmartShape{kind:SmartShapeKind::Rectangle,
            points:make_rect(minx,miny,maxx,maxy,avg_pressure),
            confidence:edge_ratio.clamp(0.0,1.0)});
    }

    let mut raw:Vec<(f32,f32)>=points.iter().map(|p|(p.x,p.y)).collect();
    if raw.first().zip(raw.last()).is_some_and(|(a,b)|(a.0-b.0).hypot(a.1-b.1)<diag*0.22) {
        raw.pop();
    }
    if raw.len()>=3 {
        raw.push(raw[0]);
        let simple=rdp(&raw,diag*0.075);
        let mut unique=simple;
        if unique.len()>1 && unique.first()==unique.last(){unique.pop();}
        if unique.len()==3 {
            let mut out:Vec<StrokePoint>=unique.into_iter()
                .map(|(x,y)|StrokePoint{x,y,pressure:avg_pressure}).collect();
            out.push(out[0].clone());
            return Some(SmartShape{kind:SmartShapeKind::Triangle,points:out,confidence:0.80});
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorEraseMode { WholeStroke, Segment, ToIntersection }

fn segment_intersection(a:(f32,f32),b:(f32,f32),c:(f32,f32),d:(f32,f32))
    ->Option<(f32,f32,f32,f32)>
{
    let r=(b.0-a.0,b.1-a.1); let s=(d.0-c.0,d.1-c.1);
    let cross=r.0*s.1-r.1*s.0;
    if cross.abs()<1e-6{return None}
    let q=(c.0-a.0,c.1-a.1);
    let t=(q.0*s.1-q.1*s.0)/cross;
    let u=(q.0*r.1-q.1*r.0)/cross;
    if (0.0..=1.0).contains(&t)&&(0.0..=1.0).contains(&u) {
        Some((a.0+r.0*t,a.1+r.1*t,t,u))
    } else {None}
}
fn nearest_segment(stroke:&Stroke,pos:(f32,f32),radius:f32)->Option<(usize,f32,f32)>{
    let mut best=None;
    for (i,pair) in stroke.points.windows(2).enumerate(){
        let a=(pair[0].x,pair[0].y); let b=(pair[1].x,pair[1].y);
        let vx=b.0-a.0; let vy=b.1-a.1; let l2=vx*vx+vy*vy;
        if l2<=1e-8{continue}
        let t=(((pos.0-a.0)*vx+(pos.1-a.1)*vy)/l2).clamp(0.0,1.0);
        let q=(a.0+vx*t,a.1+vy*t);
        let d=(q.0-pos.0).hypot(q.1-pos.1);
        if d<=radius && best.is_none_or(|(_,_,bd)|d<bd){best=Some((i,t,d));}
    }
    best
}
fn cumulative(stroke:&Stroke)->Vec<f32>{
    let mut out=Vec::with_capacity(stroke.points.len());
    out.push(0.0);
    for pair in stroke.points.windows(2){
        let d=(pair[1].x-pair[0].x).hypot(pair[1].y-pair[0].y);
        out.push(out.last().copied().unwrap_or(0.0)+d);
    }
    out
}
fn sample_at_distance(stroke:&Stroke,cum:&[f32],d:f32)->StrokePoint{
    if d<=0.0{return stroke.points[0].clone()}
    let total=*cum.last().unwrap_or(&0.0);
    if d>=total{return stroke.points.last().cloned().unwrap_or(StrokePoint{x:0.,y:0.,pressure:1.})}
    let idx=cum.windows(2).position(|w|w[0]<=d&&d<=w[1]).unwrap_or(0);
    let span=(cum[idx+1]-cum[idx]).max(1e-6);
    let t=(d-cum[idx])/span;
    let a=&stroke.points[idx]; let b=&stroke.points[idx+1];
    StrokePoint{x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t,
        pressure:a.pressure+(b.pressure-a.pressure)*t}
}
fn slice_distance(stroke:&Stroke,cum:&[f32],from:f32,to:f32)->Vec<StrokePoint>{
    if to-from<=1e-3{return Vec::new()}
    let mut out=vec![sample_at_distance(stroke,cum,from)];
    for (i,p) in stroke.points.iter().enumerate(){
        if cum[i]>from+1e-4 && cum[i]<to-1e-4{out.push(p.clone());}
    }
    out.push(sample_at_distance(stroke,cum,to));
    out.dedup_by(|a,b|(a.x-b.x).hypot(a.y-b.y)<0.001);
    out
}

/// CSP-style vector eraser. ToIntersection removes the section bounded by
/// the nearest intersections; with one side open it erases to that endpoint.
pub fn vector_erase_at(strokes:&mut Vec<Stroke>,pos:(f32,f32),radius:f32,mode:VectorEraseMode)->bool{
    if radius<=0.0 || !radius.is_finite(){return false}
    let mut hit=None;
    for (i,s) in strokes.iter().enumerate().rev(){
        if !s.is_vector{continue}
        if let Some((seg,t,d))=nearest_segment(s,pos,radius){
            hit=Some((i,seg,t,d));break;
        }
    }
    let Some((index,seg,t,_))=hit else{return false};
    if mode==VectorEraseMode::WholeStroke {strokes.remove(index);return true;}
    let stroke=strokes[index].clone();
    if stroke.points.len()<2{return false}
    let cum=cumulative(&stroke);
    let seg_len=cum[seg+1]-cum[seg];
    let cursor_d=cum[seg]+seg_len*t;
    let total=*cum.last().unwrap_or(&0.0);

    let (erase_from,erase_to)=if mode==VectorEraseMode::Segment {
        let local=(radius*0.85).max(stroke.base_width*0.8);
        ((cursor_d-local).max(0.0),(cursor_d+local).min(total))
    } else {
        let mut intersections=Vec::<f32>::new();
        for (j,pair) in stroke.points.windows(2).enumerate(){
            let a=(pair[0].x,pair[0].y); let b=(pair[1].x,pair[1].y);
            let own_len=(cum[j+1]-cum[j]).max(1e-6);
            for (oi,other) in strokes.iter().enumerate(){
                if oi==index || !other.is_vector{continue}
                for opair in other.points.windows(2){
                    if let Some((_,_,tt,_))=segment_intersection(
                        a,b,(opair[0].x,opair[0].y),(opair[1].x,opair[1].y))
                    {
                        intersections.push(cum[j]+own_len*tt);
                    }
                }
            }
        }
        intersections.sort_by(f32::total_cmp);
        intersections.dedup_by(|a,b|(*a-*b).abs()<1.0);
        let before=intersections.iter().copied().filter(|d|*d<cursor_d-0.5).last().unwrap_or(0.0);
        let after=intersections.iter().copied().find(|d|*d>cursor_d+0.5).unwrap_or(total);
        (before,after)
    };

    let before=slice_distance(&stroke,&cum,0.0,erase_from);
    let after=slice_distance(&stroke,&cum,erase_to,total);
    strokes.remove(index);
    let mut insert=index;
    for points in [before,after] {
        if points.len()>=2 {
            let mut part=stroke.clone();
            part.points=points;
            part.fill_color=None;
            strokes.insert(insert,part);
            insert+=1;
        }
    }
    true
}

pub fn change_vector_width(strokes:&mut [Stroke],selected:&[usize],factor:f32)->usize{
    if !factor.is_finite(){return 0}
    let f=factor.clamp(0.1,10.0); let mut n=0;
    for &i in selected {
        if let Some(s)=strokes.get_mut(i).filter(|s|s.is_vector){
            s.base_width=(s.base_width*f).clamp(0.25,512.0);n+=1;
        }
    }
    n
}

/// Pin-style warp for selected strokes. Points near the pin follow the drag;
/// influence fades smoothly to zero at radius.
pub fn puppet_warp(
    strokes:&mut [Stroke],selected:&[usize],pin:(f32,f32),delta:(f32,f32),radius:f32,strength:f32
)->usize{
    if radius<=0.0 || !radius.is_finite() || !delta.0.is_finite() || !delta.1.is_finite(){return 0}
    let strength=strength.clamp(0.0,1.0); let mut changed=0;
    for &i in selected {
        let Some(s)=strokes.get_mut(i) else{continue};
        if crate::raster_patch::is_patch(s) {continue;}
        let mut local=false;
        for p in &mut s.points {
            let d=(p.x-pin.0).hypot(p.y-pin.1);
            if d>=radius{continue}
            let x=1.0-d/radius;
            let w=x*x*(3.0-2.0*x)*strength;
            p.x+=delta.0*w; p.y+=delta.1*w; local=true;
        }
        if local{changed+=1;}
    }
    changed
}

pub fn selection_fill_stroke(points:&[(f32,f32)],color:[u8;4])->Option<Stroke>{
    if points.len()<3 || points.len()>10_000{return None}
    let mut out:Vec<StrokePoint>=points.iter().copied()
        .filter(|(x,y)|x.is_finite()&&y.is_finite())
        .map(|(x,y)|StrokePoint{x,y,pressure:1.0}).collect();
    if out.len()<3{return None}
    if out.first().zip(out.last()).is_none_or(|(a,b)|(a.x-b.x).hypot(a.y-b.y)>0.001){
        out.push(out[0].clone());
    }
    Some(Stroke{
        points:out,color:[0,0,0,0],fill_color:Some(color),base_width:1.0,
        is_vector:true,brush_preset:KritaBrushPreset::GPenInk,opacity:1.0,
        brush_texture_id:None,tip_shape:BrushTipShape::Round,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x:f32,y:f32)->StrokePoint{StrokePoint{x,y,pressure:1.0}}
    fn stroke(points:Vec<StrokePoint>)->Stroke{Stroke{
        points,color:[0,0,0,255],fill_color:None,base_width:4.0,is_vector:true,
        brush_preset:KritaBrushPreset::SmoothInk,opacity:1.0,brush_texture_id:None,
        tip_shape:BrushTipShape::Round,
    }}

    #[test]
    fn perspective_one_point_projects_to_vanishing_ray(){
        let q=snap_to_perspective((100.,500.),(300.,420.),PerspectiveSnap{
            mode:3,width:1000.,height:800.,vp_x:0.5,vp_y:0.25,left_vp_x:0.1,
            right_vp_x:0.9,two_point_y:0.3,parallel_angle_deg:0.0});
        let a=(500.0_f32-100.0,200.0_f32-500.0);
        let cross=(q.0-100.0)*a.1-(q.1-500.0)*a.0;
        let perpendicular_error=cross.abs()/(a.0*a.0+a.1*a.1).sqrt();
        assert!(perpendicular_error<0.001,"perpendicular error: {perpendicular_error}");
    }

    #[test]
    fn smart_shape_recognizes_ellipse(){
        let mut pts=Vec::new();
        for i in 0..=30 {
            let a=std::f32::consts::TAU*i as f32/30.0;
            pts.push(p(100.+50.*a.cos(),100.+30.*a.sin()));
        }
        let shape=recognize_smart_shape(&pts).unwrap();
        assert_eq!(shape.kind,SmartShapeKind::Ellipse);
        assert!(shape.points.len()>20);
    }

    #[test]
    fn vector_eraser_to_intersection_splits_center(){
        let mut strokes=vec![
            stroke(vec![p(0.,50.),p(100.,50.)]),
            stroke(vec![p(25.,0.),p(25.,100.)]),
            stroke(vec![p(75.,0.),p(75.,100.)]),
        ];
        assert!(vector_erase_at(&mut strokes,(50.,50.),8.,VectorEraseMode::ToIntersection));
        assert_eq!(strokes.len(),4);
        let horiz:Vec<_>=strokes.iter().filter(|s|
            s.points.first().zip(s.points.last()).is_some_and(|(a,b)|(a.y-b.y).abs()<0.01)
        ).collect();
        assert_eq!(horiz.len(),2);
    }

    #[test]
    fn puppet_warp_has_radial_falloff(){
        let mut strokes=vec![stroke(vec![p(0.,0.),p(50.,0.),p(100.,0.)])];
        assert_eq!(puppet_warp(&mut strokes,&[0],(50.,0.),(0.,20.),60.,1.0),1);
        assert!(strokes[0].points[1].y>19.0);
        assert!(strokes[0].points[0].y<2.0);
    }
}
