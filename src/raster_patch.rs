//! Embedded raster paint patches: no external temporary PNG paths.
use crate::project::{Stroke,StrokePoint,BrushTipShape,KritaBrushPreset};
use base64::{Engine as _,engine::general_purpose::STANDARD};
use eframe::egui;
use tiny_skia::{Pixmap,IntSize,Transform,PixmapPaint,FilterQuality};

const PREFIX:&str="stick-raster-png-v1:";
const MAX_PIXELS:u64=16*1024*1024;
const MAX_ENCODED:usize=96*1024*1024;

pub fn is_patch(s:&Stroke)->bool {
    s.brush_texture_id.as_deref().is_some_and(|id|id.starts_with(PREFIX))
}

fn decode(s:&Stroke)->Option<Pixmap> {
    let data=s.brush_texture_id.as_deref()?.strip_prefix(PREFIX)?;
    if data.len()>MAX_ENCODED {return None;}
    let bytes=STANDARD.decode(data).ok()?;
    if bytes.len()<24 || &bytes[..8]!=b"\x89PNG\r\n\x1a\n" || &bytes[12..16]!=b"IHDR" {return None;}
    let w=u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h=u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    if w==0 || h==0 || u64::from(w)*u64::from(h)>MAX_PIXELS {return None;}
    Pixmap::decode_png(&bytes).ok()
}

/// Store only nontransparent pixels' bounding rectangle. Four editable corners
/// give the patch normal translation/scale/rotation behavior without changing
/// old strokes or requiring external files.
pub fn from_pixmap(p:&Pixmap,origin:(u32,u32))->Option<Stroke> {
    let (w,h)=(p.width(),p.height());
    let (mut x0,mut y0,mut x1,mut y1)=(w,h,0,0);
    for (i,px) in p.data().chunks_exact(4).enumerate() {
        if px[3]==0 {continue;}
        let x=(i%w as usize) as u32;
        let y=(i/w as usize) as u32;
        x0=x0.min(x); y0=y0.min(y); x1=x1.max(x+1); y1=y1.max(y+1);
    }
    if x0>=x1 || y0>=y1 {return None;}
    let (cw,ch)=(x1-x0,y1-y0);
    let mut data=Vec::with_capacity(cw as usize*ch as usize*4);
    for y in y0..y1 {
        let start=(y as usize*w as usize+x0 as usize)*4;
        data.extend_from_slice(&p.data()[start..start+cw as usize*4]);
    }
    let crop=Pixmap::from_vec(data,IntSize::from_wh(cw,ch)?)?;
    let png=STANDARD.encode(crop.encode_png().ok()?);
    if png.len()>MAX_ENCODED {return None;}
    let (x,y)=((origin.0+x0) as f32,(origin.1+y0) as f32);
    let (r,b)=(x+cw as f32,y+ch as f32);
    let points=[(x,y),(r,y),(r,b),(x,b)].into_iter().map(|(x,y)|StrokePoint{x,y,pressure:1.}).collect();
    Some(Stroke{points,color:[0,0,0,0],fill_color:None,base_width:1.,
        is_vector:false,brush_preset:KritaBrushPreset::GPenInk,opacity:1.,
        brush_texture_id:Some(format!("{PREFIX}{png}")),tip_shape:BrushTipShape::Round})
}

pub fn render(p:&mut Pixmap,s:&Stroke,ts:Transform) {
    let Some(img)=decode(s) else {return;};
    let [a,b,_,d,..]=s.points.as_slice() else {return;};
    let w=img.width() as f32; let h=img.height() as f32;
    let transform=Transform::from_row((b.x-a.x)/w,(b.y-a.y)/w,(d.x-a.x)/h,(d.y-a.y)/h,a.x,a.y).post_concat(ts);
    let paint=PixmapPaint{opacity:s.opacity.clamp(0.,1.),quality:FilterQuality::Nearest,..Default::default()};
    p.draw_pixmap(0,0,img.as_ref(),&paint,transform,None);
}

pub fn preview<F:Fn(egui::Pos2)->egui::Pos2>(painter:&egui::Painter,s:&Stroke,opacity:f32,ox:f32,oy:f32,to_screen:&F) {
    let Some(id)=s.brush_texture_id.as_deref() else {return;};
    let [a,b,c,d,..]=s.points.as_slice() else {return;};
    let ctx=painter.ctx();
    let key=egui::Id::new(("stick_embedded_raster",id));
    let cached=ctx.data(|data|data.get_temp::<egui::TextureHandle>(key));
    let texture=if let Some(t)=cached {t} else {
        let Some(pix)=decode(s) else {return;};
        let mut rgba=Vec::with_capacity(pix.data().len());
        for p in pix.pixels() {
            let p=p.demultiply(); rgba.extend_from_slice(&[p.red(),p.green(),p.blue(),p.alpha()]);
        }
        let tex=ctx.load_texture("embedded raster",egui::ColorImage::from_rgba_unmultiplied(
            [pix.width() as usize,pix.height() as usize],&rgba),egui::TextureOptions::NEAREST);
        ctx.data_mut(|data|data.insert_temp(key,tex.clone())); tex
    };
    let mut mesh=egui::Mesh::with_texture(texture.id());
    let color=egui::Color32::from_white_alpha((opacity.clamp(0.,1.)*255.).round() as u8);
    for (pt,u,v) in [(a,0.,0.),(b,1.,0.),(c,1.,1.),(d,0.,1.)] {
        mesh.vertices.push(egui::epaint::Vertex{pos:to_screen(egui::pos2(pt.x+ox,pt.y+oy)),uv:egui::pos2(u,v),color});
    }
    mesh.indices.extend_from_slice(&[0,1,2,0,2,3]);
    painter.add(egui::Shape::mesh(mesh));
}

/// Render a stroke into a bounded crop, retaining the exact brush footprint.
pub fn rasterize(s:&Stroke,w:u32,h:u32)->Option<(Pixmap,(u32,u32))> {
    if s.points.is_empty() || w==0 || h==0 {return None;}
    let margin=if is_patch(s) {2.} else {s.base_width.abs()*4.+4.};
    if !margin.is_finite() {return None;}
    let (mut x0,mut y0,mut x1,mut y1)=(f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY);
    for p in &s.points {
        if !p.x.is_finite() || !p.y.is_finite() {return None;}
        x0=x0.min(p.x); y0=y0.min(p.y); x1=x1.max(p.x); y1=y1.max(p.y);
    }
    let x=(x0-margin).floor().clamp(0.,w as f32) as u32;
    let y=(y0-margin).floor().clamp(0.,h as f32) as u32;
    let r=(x1+margin).ceil().clamp(0.,w as f32) as u32;
    let b=(y1+margin).ceil().clamp(0.,h as f32) as u32;
    let (cw,ch)=(r.saturating_sub(x),b.saturating_sub(y));
    if cw==0 || ch==0 || u64::from(cw)*u64::from(ch)>MAX_PIXELS {return None;}
    let mut pix=Pixmap::new(cw,ch)?;
    crate::exporter::draw_skia_stroke(&mut pix,s,Transform::from_translate(-(x as f32),-(y as f32)));
    Some((pix,(x,y)))
}

pub fn clipped(s:&Stroke,w:u32,h:u32,mask:&crate::pixel_selection::PixelSelection)->Option<Stroke> {
    let (mut pix,origin)=rasterize(s,w,h)?;
    let size=(pix.width(),pix.height());
    if !mask.clip_crop(pix.data_mut(),origin,size,false) {return None;}
    from_pixmap(&pix,origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_patch_round_trips_and_translates() {
        let mut src=Pixmap::new(4,4).unwrap();
        src.data_mut()[20..24].copy_from_slice(&[100,0,0,255]);
        let mut stroke=from_pixmap(&src,(2,3)).unwrap();
        assert!(is_patch(&stroke)); assert_eq!(stroke.points[0].x,3.);
        for p in &mut stroke.points {p.x+=2.;}
        let mut dst=Pixmap::new(10,10).unwrap();
        render(&mut dst,&stroke,Transform::identity());
        assert_eq!(&dst.data()[(4*10+5)*4..(4*10+5)*4+4],&[100,0,0,255]);
    }
}
