//! Pixel-selection core for the smart/manual selection and raster painting tools.
//! Pixels passed to this module use premultiplied RGBA, as in tiny-skia.
//! Independent of egui and the project schema, so the selection logic can be
//! tested without a window or mutating existing documents.

const MAX_PIXELS: usize = 16 * 1024 * 1024;
const MAX_POLYGON_POINTS: usize = 4096;
const MAX_SMART_EDGES: usize = 1_048_576;

// A mixed boundary pixel belongs partly to the target color and partly to
// a different color (for example, the blue outline around a red circle).
// Preserve the non-target contribution when applying new opaque paint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SmartEdge {
    index: usize,
    original: [u8;4],
    reference: [u8;4],
    coverage: u8,
    correct_composite: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    Replace,
    Add,
    Subtract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelSelection {
    width: u32,
    height: u32,
    selected: Vec<bool>,
    smart_edges: Vec<SmartEdge>,
}

impl PixelSelection {
    pub fn empty(width: u32, height: u32) -> Option<Self> {
        let count = (width as usize).checked_mul(height as usize)?;
        if count == 0 || count > MAX_PIXELS { return None; }
        Some(Self { width, height, selected: vec![false; count], smart_edges: Vec::new() })
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height { return None; }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn valid_pixels(&self, rgba: &[u8]) -> bool {
        rgba.len() == self.selected.len() * 4
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.index(x, y).is_some_and(|i| self.selected[i])
    }

    pub fn count(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    pub fn clear(&mut self) {
        self.selected.fill(false);
        self.smart_edges.clear();
    }

    /// Generate a bounded RGBA overlay for the editor. Edges get a stronger
    /// alpha, while the interior stays transparent enough to see the artwork.
    /// Large canvases downsample only the preview, never the actual mask.
    pub fn preview_rgba(&self, rgb: [u8;3]) -> (usize,usize,Vec<u8>) {
        let step = self.width.max(self.height).div_ceil(2048).max(1);
        let width = self.width.div_ceil(step);
        let height = self.height.div_ceil(step);
        let mut rgba = vec![0; width as usize * height as usize * 4];
        for y in 0..height {
            for x in 0..width {
                let sx = x*step;
                let sy = y*step;
                let any = (sy..(sy+step).min(self.height)).any(|yy|
                    (sx..(sx+step).min(self.width)).any(|xx| self.contains(xx,yy)));
                if !any { continue; }
                let edge = sx == 0 || sy == 0 || sx+step >= self.width || sy+step >= self.height
                    || !self.contains(sx.saturating_sub(1),sy)
                    || !self.contains(sx,sy.saturating_sub(1))
                    || !self.contains((sx+step).min(self.width-1),sy)
                    || !self.contains(sx,(sy+step).min(self.height-1));
                let i=(y as usize * width as usize+x as usize)*4;
                rgba[i..i+4].copy_from_slice(&[rgb[0],rgb[1],rgb[2],if edge {205} else {35}]);
            }
        }
        (width as usize,height as usize,rgba)
    }

    /// Magic-wand selection of all pixels of the sampled color, including
    /// disconnected islands. Compare straight RGBA so changing opacity does
    /// not inadvertently turn bright colors into dark matching colors.
    pub fn select_color(&mut self, rgba: &[u8], seed: (u32,u32), tolerance: u8,
                        mode: EditMode) -> bool {
        if !self.valid_pixels(rgba) { return false; }
        let Some(index) = self.index(seed.0, seed.1) else { return false; };
        let reference = straight_rgba(&rgba[index*4..index*4+4]);
        self.smart_edges.clear();
        for (selected, pixel) in self.selected.iter_mut().zip(rgba.chunks_exact(4)) {
            let matches = if reference[3] == 0 {
                pixel[3] == 0
            } else if pixel[3] == 0 {
                false
            } else {
                let candidate = straight_rgba(pixel);
                candidate.iter().zip(reference).all(|(&a,b)| a.abs_diff(b) <= tolerance)
            };
            *selected = match mode {
                EditMode::Replace => matches,
                EditMode::Add => *selected || matches,
                EditMode::Subtract => *selected && !matches,
            };
        }
        true
    }

    /// Global magic wand with edge-aware antialias recovery. Do not simply
    /// dilate the mask: a solid contrasting outline, even one pixel wide,
    /// must stay protected. Only recover adjacent pixels explainable as
    /// mixtures of the selected color and a local, unselected color.
    pub fn select_smart_color(&mut self, rgba:&[u8], seed:(u32,u32),
        tolerance:u8, correct_composite:bool) -> bool {
        if !self.valid_pixels(rgba) {return false;}
        let Some(start)=self.index(seed.0,seed.1) else {return false;};
        let reference=straight_rgba(&rgba[start*4..start*4+4]);
        if !self.select_color(rgba,seed,tolerance,EditMode::Replace) {return false;}
        let w=self.width as usize; let h=self.height as usize;
        let core=self.selected.clone();
        let mut edges=Vec::new();
        // Inspect one native-pixel band, including diagonal neighbors.
        // This bounds the refinement and prevents gradient chaining.
        for i in 0..core.len() {
            if core[i] {continue;}
            let x=i%w; let y=i/w;
            let x0=x.saturating_sub(1); let x1=(x+1).min(w-1);
            let y0=y.saturating_sub(1); let y1=(y+1).min(h-1);
            if !(y0..=y1).any(|yy|(x0..=x1).any(|xx|core[yy*w+xx])) {continue;}
            let original=straight_rgba(&rgba[i*4..i*4+4]);
            let mut best=None::<(f32,u8)>;
            if reference[3]==255 && original[3]==255 {
                // Find a genuine neighboring non-target endpoint. Prefer the
                // strongest compatible contrast instead of another fringe pixel.
                let mut strength=0.0_f32;
                for yy in y0..=y1 {
                    for xx in x0..=x1 {
                        let j=yy*w+xx;
                        if j==i || core[j] {continue;}
                        let outside=straight_rgba(&rgba[j*4..j*4+4]);
                        if outside[3]!=255 {continue;}
                        if let Some((coverage,contrast,error))=mixed_color(reference,original,outside) {
                            if best.is_none() || contrast>strength && error<=best.unwrap().0+25.
                                || error+25.<best.unwrap().0 {
                                best=Some((error,coverage)); strength=contrast;
                            }
                        }
                    }
                }
            } else if reference[3]>0 && original[3]>0 && original[3]<reference[3]
                && original[..3].iter().zip(&reference[..3]).all(|(&a,&b)|a.abs_diff(b)<=tolerance.max(16)) {
                // Transparent antialiasing with the same pigment.
                best=Some((0.,((original[3] as u16*255)/reference[3] as u16) as u8));
            }
            if let Some((_,coverage))=best {
                if edges.len()>=MAX_SMART_EDGES {break;}
                edges.push(SmartEdge{index:i,original,reference,coverage,
                    correct_composite:correct_composite && original[3]==255 && reference[3]==255});
            }
        }
        for edge in &edges {self.selected[edge.index]=true;}
        self.smart_edges=edges;
        true
    }

    /// Four-connected color flood, unlike the global magic wand.
    pub fn select_connected_color(&mut self, rgba:&[u8], seed:(u32,u32), tolerance:u8)->bool {
        if !self.valid_pixels(rgba) {return false;}
        let Some(start)=self.index(seed.0,seed.1) else {return false;};
        let reference=straight_rgba(&rgba[start*4..start*4+4]);
        self.clear();
        let mut visited=vec![false;self.selected.len()];
        let mut queue=std::collections::VecDeque::new();
        queue.push_back(start);
        visited[start]=true;
        let w=self.width as usize; let h=self.height as usize;
        while let Some(i)=queue.pop_front() {
            let candidate=straight_rgba(&rgba[i*4..i*4+4]);
            if reference[3]==0 {
                if candidate[3]!=0 {continue;}
            } else if candidate[3]==0 || !candidate.iter().zip(reference).all(|(&a,b)|a.abs_diff(b)<=tolerance) {
                continue;
            }
            self.selected[i]=true;
            let x=i%w; let y=i/w;
            for next in [(x>0).then(||i-1),(x+1<w).then(||i+1),(y>0).then(||i-w),(y+1<h).then(||i+w)].into_iter().flatten() {
                if !visited[next] {visited[next]=true;queue.push_back(next);}
            }
        }
        true
    }

    pub fn intersect(&mut self, other:&Self) -> bool {
        if self.width!=other.width || self.height!=other.height {return false;}
        for (a,&b) in self.selected.iter_mut().zip(&other.selected) {*a &= b;}
        self.smart_edges.retain(|e|self.selected[e.index]);
        if self.smart_edges.is_empty() {
            self.smart_edges.extend(other.smart_edges.iter().copied().filter(|e|self.selected[e.index]));
        }
        true
    }

    /// Manual polygon selection using even-odd scan conversion of pixel
    /// centers. Supports concave and self-crossing polygons without filling
    /// their bounding rectangles. Add and Subtract refine the existing mask.
    pub fn edit_polygon(&mut self, points: &[(f32,f32)], mode: EditMode) -> bool {
        if points.len() < 3 || points.len() > MAX_POLYGON_POINTS
            || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return false;
        }
        let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min)
            .floor().clamp(0.0, self.height as f32) as u32;
        let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max)
            .ceil().clamp(0.0, self.height as f32) as u32;
        // Limit edge/scanline work before modifying the existing selection.
        if u64::from(max_y-min_y) * points.len() as u64 > 32_000_000 { return false; }
        if mode == EditMode::Replace { self.clear(); }
        let mut intersections = Vec::with_capacity(points.len());
        for y in min_y..max_y {
            let py = y as f64 + 0.5;
            intersections.clear();
            for i in 0..points.len() {
                let (ax,ay) = points[i];
                let (bx,by) = points[(i+1)%points.len()];
                let (ax,ay,bx,by) = (ax as f64,ay as f64,bx as f64,by as f64);
                if (ay > py) != (by > py) {
                    intersections.push(ax + (py-ay)*(bx-ax)/(by-ay));
                }
            }
            intersections.sort_unstable_by(f64::total_cmp);
            for pair in intersections.chunks_exact(2) {
                let start = (pair[0]-0.5).ceil().clamp(0.0,self.width as f64) as usize;
                let end = (pair[1]-0.5).ceil().clamp(0.0,self.width as f64) as usize;
                let row = y as usize * self.width as usize;
                self.selected[row+start..row+end].fill(mode != EditMode::Subtract);
            }
        }
        self.smart_edges.retain(|e|self.selected[e.index]);
        true
    }

    /// Clip a cropped premultiplied render buffer using canvas coordinates.
    pub fn clip_crop(&self, rgba: &mut [u8], origin: (u32,u32), size: (u32,u32), invert: bool) -> bool {
        let Some(expected) = (size.0 as usize).checked_mul(size.1 as usize).and_then(|n|n.checked_mul(4)) else { return false; };
        if size.0 == 0 || rgba.len() != expected { return false; }
        for (i,pixel) in rgba.chunks_exact_mut(4).enumerate() {
            let x=origin.0.saturating_add((i%size.0 as usize) as u32);
            let y=origin.1.saturating_add((i/size.0 as usize) as u32);
            if self.contains(x,y)==invert { pixel.fill(0); }
        }
        true
    }

    /// Brush clipping differs from object movement. At an opaque mixed edge,
    /// reconstruct the correct new composite color instead of leaving red
    /// hairlines or painting the entire blue outline pixel purple.
    pub fn clip_brush_crop(&self, rgba:&mut [u8], origin:(u32,u32), size:(u32,u32))->bool {
        if !self.clip_crop(rgba,origin,size,false) {return false;}
        for edge in &self.smart_edges {
            let (x,y)=((edge.index%self.width as usize) as u32,(edge.index/self.width as usize) as u32);
            if x<origin.0 || y<origin.1 || x-origin.0>=size.0 || y-origin.1>=size.1 {continue;}
            let i=((y-origin.1) as usize*size.0 as usize+(x-origin.0) as usize)*4;
            refine_paint_pixel(&mut rgba[i..i+4],edge);
        }
        true
    }

    pub fn translate_mask(&mut self, dx:i32, dy:i32) {
        if dx==0 && dy==0 {return;}
        let mut next=vec![false;self.selected.len()];
        for (i,&selected) in self.selected.iter().enumerate() {
            if !selected {continue;}
            let x=(i%self.width as usize) as i64+i64::from(dx);
            let y=(i/self.width as usize) as i64+i64::from(dy);
            if x>=0 && y>=0 && x<i64::from(self.width) && y<i64::from(self.height) {
                next[y as usize*self.width as usize+x as usize]=true;
            }
        }
        self.selected=next;
        // The reference composite no longer describes the translated edge.
        self.smart_edges.clear();
    }

    /// Zero out all pixels outside the mask. Intended for a temporary brush
    /// render buffer, not for the existing layer's original artwork.
    pub fn clip_paint(&self, rgba: &mut [u8]) -> bool {
        if !self.valid_pixels(rgba) { return false; }
        for (&selected, pixel) in self.selected.iter().zip(rgba.chunks_exact_mut(4)) {
            if !selected { pixel.fill(0); }
        }
        true
    }

    /// Apply a straight RGBA color to the selected pixels with source-over
    /// blending. This is a genuine raster fill on a premultiplied pixel layer.
    pub fn paint_color(&self, rgba: &mut [u8], color: [u8;4]) -> bool {
        if !self.valid_pixels(rgba) { return false; }
        let a = color[3];
        let premultiplied = [
            mul_alpha(color[0], a), mul_alpha(color[1], a), mul_alpha(color[2], a), a
        ];
        let mut edges=self.smart_edges.iter().peekable();
        for (i,(&selected, pixel)) in self.selected.iter().zip(rgba.chunks_exact_mut(4)).enumerate() {
            if !selected {continue;}
            let mut paint=premultiplied;
            if edges.peek().is_some_and(|e|e.index==i) {
                if let Some(edge)=edges.next() {refine_paint_pixel(&mut paint,edge);}
            }
            source_over(pixel,&paint);
        }
        true
    }

    /// Move only selected pixels and move the selection with them. The source
    /// is copied before any destination write so overlapping translations are
    /// deterministic. Pixels outside the destination canvas are clipped.
    pub fn translate(&mut self, rgba: &mut [u8], dx: i32, dy: i32) -> bool {
        if !self.valid_pixels(rgba) { return false; }
        if dx == 0 && dy == 0 { return true; }
        let old = rgba.to_vec();
        for (&selected, pixel) in self.selected.iter().zip(rgba.chunks_exact_mut(4)) {
            if selected { pixel.fill(0); }
        }
        let mut next = vec![false; self.selected.len()];
        for (i, &selected) in self.selected.iter().enumerate() {
            if !selected { continue; }
            let x = (i % self.width as usize) as i64 + i64::from(dx);
            let y = (i / self.width as usize) as i64 + i64::from(dy);
            if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
                continue;
            }
            let j = y as usize * self.width as usize + x as usize;
            source_over(&mut rgba[j*4..j*4+4], &old[i*4..i*4+4]);
            next[j] = true;
        }
        self.selected = next;
        self.smart_edges.clear();
        true
    }
}

/// Return target coverage for a pixel lying on the color line between
/// target and neighboring non-target pigment. Reject solid non-target pixels
/// and unrelated colors; tolerate small 8-bit rasterization rounding errors.
fn mixed_color(target:[u8;4],pixel:[u8;4],outside:[u8;4])->Option<(u8,f32,f32)> {
    let mut contrast=0.; let mut dot=0.;
    for c in 0..3 {
        let d=target[c] as f32-outside[c] as f32;
        contrast+=d*d; dot+=(pixel[c] as f32-outside[c] as f32)*d;
    }
    if contrast<32.*32. {return None;}
    let t=dot/contrast;
    if !(0.035..0.985).contains(&t) {return None;}
    let mut error=0.;
    for c in 0..3 {
        let expected=outside[c] as f32+(target[c] as f32-outside[c] as f32)*t;
        error+=(pixel[c] as f32-expected).powi(2);
    }
    if error>3.*10.*10. {return None;}
    Some(((t*255.).round() as u8,contrast,error))
}

fn refine_paint_pixel(p:&mut [u8],edge:&SmartEdge) {
    if p[3]==0 {return;}
    if edge.correct_composite {
        let a=p[3];
        let paint=straight_rgba(p);
        for c in 0..3 {
            let delta=paint[c] as f32-edge.reference[c] as f32;
            let corrected=(edge.original[c] as f32+delta*edge.coverage as f32/255.).round().clamp(0.,255.) as u8;
            p[c]=mul_alpha(corrected,a);
        }
    } else {
        for c in p { *c=mul_alpha(*c,edge.coverage); }
    }
}

fn mul_alpha(value: u8, alpha: u8) -> u8 {
    ((u16::from(value) * u16::from(alpha) + 127) / 255) as u8
}

fn straight_rgba(pixel: &[u8]) -> [u8;4] {
    let a = u16::from(pixel[3]);
    if a == 0 { return [0;4]; }
    let un = |c| ((u16::from(c)*255 + a/2)/a).min(255) as u8;
    [un(pixel[0]), un(pixel[1]), un(pixel[2]), pixel[3]]
}

fn source_over(dst: &mut [u8], src: &[u8]) {
    let inverse = 255 - src[3];
    for i in 0..4 {
        dst[i] = src[i].saturating_add(mul_alpha(dst[i], inverse));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_color_selects_disconnected_islands_and_not_other_colors() {
        let rgba = [255,0,0,255, 0,0,255,255, 255,0,0,255, 0,0,0,0];
        let mut s = PixelSelection::empty(4,1).unwrap();
        assert!(s.select_color(&rgba,(0,0),0,EditMode::Replace));
        assert_eq!(s.count(),2);
        assert!(s.contains(0,0) && s.contains(2,0));
        assert!(!s.contains(1,0) && !s.contains(3,0));
    }

    #[test]
    fn smart_selection_recovers_antialiased_red_blue_edge_without_covering_outline() {
        let rgba=[255,0,0,255, 255,0,0,255, 128,0,127,255, 0,0,255,255, 0,0,255,255];
        let mut s=PixelSelection::empty(5,1).unwrap();
        assert!(s.select_smart_color(&rgba,(0,0),8,true));
        assert_eq!(s.count(),3);
        assert!(s.contains(2,0)); assert!(!s.contains(3,0));
        let mut paint=[0u8;20];s.paint_color(&mut paint,[0,255,0,255]);
        assert_eq!(&paint[0..4],&[0,255,0,255]);
        assert_eq!(&paint[8..12],&[0,128,127,255]);
        assert_eq!(&paint[12..16],&[0,0,0,0]);
        let mut brush=[0,255,0,255].repeat(5);
        assert!(s.clip_brush_crop(&mut brush,(0,0),(5,1)));
        assert_eq!(brush,paint);
    }

    #[test]
    fn smart_selection_does_not_bridge_a_solid_one_pixel_outline() {
        let rgba=[255,0,0,255, 0,0,255,255, 255,0,0,255, 0,255,0,255];
        let mut s=PixelSelection::empty(4,1).unwrap();
        s.select_smart_color(&rgba,(0,0),8,true);
        assert_eq!(s.count(),2);
        assert!(!s.contains(1,0) && !s.contains(3,0));
    }

    #[test]
    fn smart_selection_recovers_diagonal_edges_and_keeps_manual_subtract() {
        let mut rgba=[0,0,255,255].repeat(9);
        rgba[0..4].copy_from_slice(&[255,0,0,255]);
        rgba[16..20].copy_from_slice(&[64,0,191,255]);
        let mut s=PixelSelection::empty(3,3).unwrap();
        s.select_smart_color(&rgba,(0,0),8,true);
        assert!(s.contains(1,1));
        s.edit_polygon(&[(1.,1.),(2.,1.),(2.,2.),(1.,2.)],EditMode::Subtract);
        assert!(!s.contains(1,1)); assert!(s.smart_edges.is_empty());
    }

    #[test]
    fn raster_flood_selects_only_the_connected_island() {
        let rgba=[255,0,0,255,0,0,255,255,255,0,0,255];
        let mut s=PixelSelection::empty(3,1).unwrap();
        assert!(s.select_connected_color(&rgba,(0,0),0));
        assert_eq!(s.count(),1);
        assert!(!s.contains(2,0));
    }

    #[test]
    fn transparent_pixels_are_distinct_from_opaque_black() {
        let rgba = [0,0,0,0, 0,0,0,255];
        let mut s = PixelSelection::empty(2,1).unwrap();
        assert!(s.select_color(&rgba,(0,0),255,EditMode::Replace));
        assert_eq!(s.count(),1);
    }

    #[test]
    fn manual_polygon_can_add_and_subtract_without_erasing_other_islands() {
        let mut s = PixelSelection::empty(6,6).unwrap();
        let square = |x:f32,y:f32| [(x,y),(x+2.,y),(x+2.,y+2.),(x,y+2.)];
        assert!(s.edit_polygon(&square(0.,0.),EditMode::Replace));
        assert!(s.edit_polygon(&square(4.,4.),EditMode::Add));
        assert_eq!(s.count(),8);
        assert!(s.edit_polygon(&square(0.,0.),EditMode::Subtract));
        assert_eq!(s.count(),4);
        assert!(s.contains(5,5));
    }

    #[test]
    fn concave_polygon_does_not_select_empty_notch() {
        let mut s = PixelSelection::empty(5,5).unwrap();
        assert!(s.edit_polygon(&[(0.,0.),(5.,0.),(5.,2.),(2.,2.),(2.,5.),(0.,5.)],EditMode::Replace));
        assert_eq!(s.count(),16);
        assert!(!s.contains(4,4));
    }

    #[test]
    fn painting_only_changes_selected_pixels() {
        let mut s = PixelSelection::empty(2,1).unwrap();
        s.edit_polygon(&[(0.,0.),(1.,0.),(1.,1.),(0.,1.)],EditMode::Replace);
        let mut rgba = [0u8;8];
        assert!(s.paint_color(&mut rgba,[255,0,0,128]));
        assert_eq!(rgba,[128,0,0,128,0,0,0,0]);
        let mut brush = [255u8;8];
        assert!(s.clip_paint(&mut brush));
        assert_eq!(brush,[255,255,255,255,0,0,0,0]);
    }

    #[test]
    fn moving_overlapping_region_preserves_source_and_selection() {
        let mut s = PixelSelection::empty(4,1).unwrap();
        s.edit_polygon(&[(0.,0.),(2.,0.),(2.,1.),(0.,1.)],EditMode::Replace);
        let mut rgba = [255,0,0,255, 0,255,0,255, 0,0,255,255, 0,0,0,0];
        assert!(s.translate(&mut rgba,1,0));
        assert_eq!(rgba,[0,0,0,0, 255,0,0,255, 0,255,0,255, 0,0,0,0]);
        assert!(s.contains(1,0) && s.contains(2,0) && !s.contains(0,0));
    }

    #[test]
    fn cropped_brush_mask_clips_at_canvas_offset_and_can_invert() {
        let mut s=PixelSelection::empty(4,4).unwrap();
        s.edit_polygon(&[(2.,2.),(3.,2.),(3.,3.),(2.,3.)],EditMode::Replace);
        let mut paint=[255u8;16];let mut rest=paint;
        assert!(s.clip_crop(&mut paint,(1,2),(2,2),false));
        assert_eq!(paint,[0,0,0,0,255,255,255,255,0,0,0,0,0,0,0,0]);
        assert!(s.clip_crop(&mut rest,(1,2),(2,2),true));
        assert_eq!(rest,[255,255,255,255,0,0,0,0,255,255,255,255,255,255,255,255]);
        s.translate_mask(-1,1);
        assert!(s.contains(1,3)); assert_eq!(s.count(),1);
    }

    #[test]
    fn invalid_sizes_and_buffers_leave_existing_selection_unchanged() {
        assert!(PixelSelection::empty(u32::MAX,u32::MAX).is_none());
        let mut s = PixelSelection::empty(2,2).unwrap();
        s.edit_polygon(&[(0.,0.),(1.,0.),(1.,1.),(0.,1.)],EditMode::Replace);
        let before = s.clone();
        assert!(!s.select_color(&[0;4],(0,0),0,EditMode::Replace));
        assert_eq!(s,before);
        assert!(!s.edit_polygon(&[(f32::NAN,0.),(1.,0.),(0.,1.)],EditMode::Replace));
        assert_eq!(s,before);
    }
}
