//! Procedural brush tips in stroke coordinates. Preview and raster exports
//! use the same alpha mask, orientation, pressure and spacing.
use crate::brush_engine::{dab_polygon, visit_pressure_dabs_spaced, BrushDab};
use crate::project::{BrushTipShape, KritaBrushPreset, Stroke};
use eframe::egui;
use std::sync::OnceLock;
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform};

const SIZE: usize = 32;
const AREA: usize = SIZE * SIZE;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextureBrush {
    Watercolor,
    Marker,
    Airbrush,
    Sketch,
    Graphite,
    Dry,
    Chalk,
    Charcoal,
    Pastel,
    Calligraphy,
    SoftSketch,
    SketchBrush,
}

impl TextureBrush {
    pub fn for_stroke(stroke: &Stroke) -> Option<Self> {
        if stroke.is_vector {
            return None;
        }
        match stroke.brush_preset {
            KritaBrushPreset::Watercolor => Some(Self::Watercolor),
            // Marker is rendered as one continuous translucent stroke in the
            // shared line engine. Repeated rectangular texture stamps caused
            // the visible square/block pattern reported by artists.
            KritaBrushPreset::ChiselMarker => None,
            KritaBrushPreset::SoftAirbrush => Some(Self::Airbrush),
            KritaBrushPreset::SketchPencil => Some(Self::Sketch),
            KritaBrushPreset::GraphitePencil => Some(Self::Graphite),
            KritaBrushPreset::DryBrush => Some(Self::Dry),
            KritaBrushPreset::Chalk => Some(Self::Chalk),
            KritaBrushPreset::Charcoal => Some(Self::Charcoal),
            KritaBrushPreset::SoftPastel => Some(Self::Pastel),
            KritaBrushPreset::Calligraphy => Some(Self::Calligraphy),
            KritaBrushPreset::SoftSketchPencil => Some(Self::SoftSketch),
            KritaBrushPreset::SketchBrush => Some(Self::SketchBrush),
            _ => None,
        }
    }

    fn mask(self, tip: BrushTipShape) -> &'static [u8; AREA] {
        static MASKS: [OnceLock<[u8; AREA]>; 72] = [const { OnceLock::new() }; 72];
        MASKS[self as usize * 6 + tip as usize].get_or_init(|| self.make_mask(tip))
    }

    fn make_mask(self, tip: BrushTipShape) -> [u8; AREA] {
        // Rectangular aspect ratios are applied by the stamp transform.
        let footprint = match tip {
            BrushTipShape::TallRectangle | BrushTipShape::WideRectangle => BrushTipShape::Square,
            other => other,
        };
        let polygon = dab_polygon(
            footprint,
            BrushDab {
                x: 0.0,
                y: 0.0,
                radius: 1.0,
            },
        );
        std::array::from_fn(|index| {
            let (x, y) = (index % SIZE, index / SIZE);
            let u = (2.0 * x as f32 + 1.0) / SIZE as f32 - 1.0;
            let v = (2.0 * y as f32 + 1.0) / SIZE as f32 - 1.0;
            // Clockwise vertices in canvas coordinates; signed distance to the
            // nearest side also gives polygon tips a naturally broken edge.
            // Marker and calligraphy deliberately own a chisel footprint so
            // they cannot look like the ordinary round 2D pen.
            let inside = if matches!(self, Self::Calligraphy | Self::Marker) {
                let angle:f32=if self==Self::Calligraphy {0.50}else{0.34};
                let (sin,cos)=angle.sin_cos();
                let dx=u*cos+v*sin;
                let dy=-u*sin+v*cos;
                let half_h=if self==Self::Calligraphy {0.34}else{0.52};
                (1.0-dx.abs()).min(half_h-dy.abs())
            } else if polygon.is_empty() {
                1.0 - u.hypot(v)
            } else {
                polygon
                    .iter()
                    .zip(polygon.iter().cycle().skip(1))
                    .map(|(&(ax, ay), &(bx, by))| {
                        let (dx, dy) = (bx - ax, by - ay);
                        (dx * (v - ay) - dy * (u - ax)) / dx.hypot(dy)
                    })
                    .fold(f32::INFINITY, f32::min)
            };
            let grain = noise(index as u32);
            let radial=(1.0-u.hypot(v)).clamp(0.0,1.0);
            let edge = match self {
                Self::Airbrush => radial.powf(1.9),
                Self::Watercolor => {
                    let wobble=noise((x*7+y*17+27_001) as u32);
                    ((inside+0.10)*3.2-wobble*0.26).clamp(0.0,1.0)
                }
                Self::Marker | Self::Calligraphy =>
                    ((inside-0.01)*18.0).clamp(0.0,1.0),
                Self::Sketch | Self::Graphite | Self::SoftSketch | Self::SketchBrush =>
                    ((inside-0.02)*7.0-grain*0.28).clamp(0.0,1.0),
                _ => ((inside-0.04)*8.0-grain*0.65).clamp(0.0,1.0),
            };
            let ink = match self {
                Self::Watercolor => {
                    let paper=noise((x*19+y*23+10_007) as u32);
                    let bloom=noise((x/3+y/3*SIZE+55_001) as u32);
                    (0.18+0.35*paper+0.17*bloom)*radial.max(0.45)
                }
                Self::Marker => {
                    let fiber=noise((x*3+y*29+12_343) as u32);
                    0.82+0.15*fiber
                }
                Self::Airbrush => {
                    let spray=noise((x*41+y*67+80_003) as u32);
                    (0.26+0.18*spray)*radial
                }
                Self::Sketch => {
                    let graphite=noise((x*13+y*47+41_011) as u32);
                    let tooth=noise((x*31+y*5+91_019) as u32);
                    if graphite<0.18 {0.0}
                    else {(0.16+0.42*graphite)*(0.62+0.38*tooth)}
                }
                Self::Graphite => {
                    let graphite=noise((x*71+y*17+151_007) as u32);
                    let paper=noise((x*11+y*83+190_027) as u32);
                    let streak=noise((x/2+y*SIZE+220_009) as u32);
                    if graphite<0.24 || paper<0.10 {0.0}
                    else {(0.10+0.52*graphite)*(0.48+0.38*paper+0.14*streak)}
                }
                Self::Dry => {
                    let bristle = noise(y as u32 + 1024);
                    let split = noise((x / 5 + y * SIZE + AREA) as u32);
                    if bristle < 0.23 || split < 0.13 {
                        0.0
                    } else {
                        (0.30 + 0.55 * bristle) * (0.70 + 0.30 * grain)
                    }
                }
                Self::Chalk => {
                    if grain < 0.35 {
                        0.0
                    } else {
                        0.18 + 0.58 * grain
                    }
                }
                Self::Charcoal => {
                    let scratch = noise((x * 17 + y * 9 + 40_003) as u32);
                    if scratch < 0.28 || grain < 0.13 { 0.0 }
                    else { (0.28 + 0.72 * grain) * (0.65 + 0.35 * scratch) }
                }
                Self::Pastel => {
                    let soft = (1.0 - u.hypot(v) * 0.38).clamp(0.20, 1.0);
                    if grain < 0.17 { 0.0 }
                    else { (0.34 + 0.48 * grain) * soft }
                }
                Self::Calligraphy => 1.0,
                Self::SoftSketch => {
                    let pencil = noise((x * 13 + y * 71 + 91_001) as u32);
                    if grain < 0.22 { 0.0 }
                    else { (0.10 + 0.30 * grain) * (0.65 + 0.35 * pencil) }
                }
                Self::SketchBrush => {
                    let bristle=noise((x*37+y*11+271_003) as u32);
                    let paper=noise((x*7+y*61+311_021) as u32);
                    let streak=noise((x/3+y*SIZE+351_017) as u32);
                    if bristle < 0.12 { 0.0 }
                    else {
                        (0.22+0.46*bristle)*(0.70+0.30*paper)*(0.85+0.15*streak)
                    }
                }
            };
            // Keep taps visible even when the tip is smaller than a pixel.
            let ink = if u.abs() < 0.07 && v.abs() < 0.07 {
                let minimum=match self {
                    Self::Airbrush=>0.12,
                    Self::Watercolor=>0.22,
                    Self::Sketch|Self::Graphite|Self::SoftSketch=>0.18,
                    Self::SketchBrush=>0.24,
                    _=>0.45,
                };
                ink.max(minimum)
            } else {
                ink
            };
            (255.0 * ink * edge).round() as u8
        })
    }

    fn spacing_radius_ratio(self) -> f32 {
        // Mature paint engines typically space dabs at roughly 8-15% of the
        // brush diameter for smooth media. Our old universal 24%-diameter
        // spacing exposed individual stamps on pencils/chalk/dry media.
        match self {
            Self::Airbrush => 0.12,
            Self::Watercolor => 0.16,
            Self::Sketch => 0.13,
            Self::Graphite => 0.11,
            Self::Dry => 0.14,
            Self::Chalk => 0.14,
            Self::Charcoal => 0.12,
            Self::Pastel => 0.13,
            Self::Calligraphy => 0.10,
            Self::SoftSketch => 0.12,
            Self::SketchBrush => 0.11,
            Self::Marker => 0.10,
        }
    }

    fn direction_memory(self) -> f32 {
        match self {
            Self::Sketch | Self::Graphite | Self::SoftSketch => 0.58,
            Self::SketchBrush => 0.54,
            Self::Dry | Self::Chalk | Self::Charcoal | Self::Pastel => 0.48,
            Self::Watercolor => 0.35,
            _ => 0.0,
        }
    }

    fn stamps(self, stroke: &Stroke, mut emit: impl FnMut(Stamp)) {
        let (tip_wide,tip_tall)=match stroke.tip_shape {
            BrushTipShape::TallRectangle=>(0.5,1.5),
            BrushTipShape::WideRectangle=>(1.5,0.5),
            _=>(1.0,1.0),
        };
        let (preset_wide,preset_tall)=match self {
            Self::Watercolor=>(1.18,1.10),
            Self::Marker=>(1.28,0.62),
            Self::Airbrush=>(1.55,1.55),
            Self::Sketch=>(0.82,0.72),
            Self::Graphite=>(0.72,0.62),
            Self::Calligraphy=>(1.20,0.62),
            Self::SoftSketch=>(0.95,0.82),
            Self::SketchBrush=>(1.08,0.90),
            _=>(1.0,1.0),
        };
        let (wide,tall)=(tip_wide*preset_wide,tip_tall*preset_tall);
        let mut ordinal = 0u32;
        let mut remembered_dir: Option<[f32;2]> = None;
        // Texture is still raster, but its stamp centers must follow the same
        // fitted freehand centerline as normal ink. Sampling raw pointer chords
        // made elongated tips look like little horizontal/vertical bars on a curve.
        let smoothed=crate::brush_engine::smooth_freehand_points(
            &stroke.points,stroke.base_width);
        let centerline: &[crate::project::StrokePoint]=if smoothed.len()>=2 {
            &smoothed
        } else {
            &stroke.points
        };
        visit_pressure_dabs_spaced(
            centerline,
            stroke.base_width,
            self.spacing_radius_ratio(),
            |dab, [dx, dy]| {
            let pressure = (2.0 * dab.radius / stroke.base_width).clamp(0.0, 1.0);
            let variation = noise(ordinal.wrapping_add(3201));
            // Grain brushes rotate slightly from stamp to stamp; marker follows
            // travel while calligraphy keeps a fixed nib angle.
            let jitter=match self {
                Self::Chalk|Self::Charcoal|Self::Pastel=>0.80,
                Self::Sketch|Self::Graphite|Self::SoftSketch=>0.28,
                Self::SketchBrush=>0.18,
                Self::Watercolor=>0.16,
                _=>0.0,
            };
            let memory=self.direction_memory();
            let [mut sx,mut sy]=[dx,dy];
            if memory>0.0 {
                if let Some([px,py])=remembered_dir {
                    sx=px*memory+dx*(1.0-memory);
                    sy=py*memory+dy*(1.0-memory);
                    let len=sx.hypot(sy).max(1e-5);
                    sx/=len;
                    sy/=len;
                }
                remembered_dir=Some([sx,sy]);
            }
            let (sin, cos) = ((variation - 0.5) * jitter).sin_cos();
            let (tx, ty) = if self == Self::Calligraphy {
                (1.0,0.0)
            } else {(sx * cos - sy * sin, sx * sin + sy * cos)};
            let radius_scale=match self {
                Self::Watercolor=>1.45,
                Self::Airbrush=>1.55,
                Self::Graphite=>0.88,
                _=>1.0,
            };
            let r = dab.radius*radius_scale;
            let flow=match self {
                Self::Watercolor=>(0.13+0.24*pressure)*(0.82+0.18*variation),
                Self::Marker=>(0.42+0.24*pressure)*(0.92+0.08*variation),
                Self::Airbrush=>(0.05+0.16*pressure)*(0.82+0.18*variation),
                Self::Sketch=>(0.15+0.40*pressure)*(0.72+0.28*variation),
                Self::Graphite=>(0.08+0.62*pressure.powf(1.45))*(0.72+0.28*variation),
                Self::Dry=>(0.28+0.54*pressure)*(0.70+0.30*variation),
                Self::Chalk=>(0.18+0.44*pressure)*(0.68+0.32*variation),
                Self::Charcoal=>(0.24+0.58*pressure)*(0.70+0.30*variation),
                Self::Pastel=>(0.20+0.42*pressure)*(0.76+0.24*variation),
                Self::Calligraphy=>0.90,
                Self::SoftSketch=>(0.10+0.34*pressure)*(0.74+0.26*variation),
                Self::SketchBrush=>(0.18+0.50*pressure)*(0.78+0.22*variation),
            };
            emit(Stamp {
                center: [dab.x, dab.y],
                along: [tx * r * wide, ty * r * wide],
                across: [-ty * r * tall, tx * r * tall],
                flow,
            });
            ordinal = ordinal.wrapping_add(1);
        });
    }

    pub fn preview(
        self,
        painter: &egui::Painter,
        stroke: &Stroke,
        rgba: [u8; 4],
        to_screen: impl Fn(f32, f32) -> egui::Pos2,
    ) {
        let ctx = painter.ctx();
        // Bake the selected RGB into the cached alpha-mask texture itself.
        // This avoids relying on vertex tinting for textured brushes and keeps
        // watercolor, spray and graphite hues identical in preview and export.
        let key = egui::Id::new((
            "stick_brush_mask", self as u8, stroke.tip_shape as u8,
            rgba[0], rgba[1], rgba[2],
        ));
        let cached = ctx.data(|data| data.get_temp::<egui::TextureHandle>(key));
        let texture = cached.unwrap_or_else(|| {
            let pixels = self
                .mask(stroke.tip_shape)
                .iter()
                .map(|&a| egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], a))
                .collect();
            let texture = ctx.load_texture(
                "brush tip",
                egui::ColorImage {
                    size: [SIZE, SIZE],
                    pixels,
                },
                egui::TextureOptions::LINEAR,
            );
            ctx.data_mut(|data| data.insert_temp(key, texture.clone()));
            texture
        });
        let mut mesh = egui::Mesh::with_texture(texture.id());
        self.stamps(stroke, |stamp| {
            let start = mesh.vertices.len() as u32;
            // The texture already contains the hue; vertex color only scales
            // the flow/opacity for this stamp.
            let color = egui::Color32::from_white_alpha(
                (rgba[3] as f32 * stamp.flow).round() as u8,
            );
            for (u, v) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
                let [x, y] = stamp.corner(u, v);
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: to_screen(x, y),
                    uv: egui::pos2(u, v),
                    color,
                });
            }
            mesh.indices
                .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        });
        painter.add(egui::Shape::mesh(mesh));
    }

    pub fn export(self, target: &mut Pixmap, stroke: &Stroke, alpha: u8, transform: Transform) {
        let mut tip = Pixmap::new(SIZE as u32, SIZE as u32).unwrap();
        for (px, &a) in tip
            .data_mut()
            .chunks_exact_mut(4)
            .zip(self.mask(stroke.tip_shape))
        {
            for (out, &c) in px[..3].iter_mut().zip(&stroke.color[..3]) {
                *out = ((c as u16 * a as u16 + 127) / 255) as u8;
            }
            px[3] = a;
        }
        self.stamps(stroke, |stamp| {
            let origin = stamp.corner(0., 0.);
            let k = 2.0 / SIZE as f32;
            let position = Transform::from_row(
                stamp.along[0] * k,
                stamp.along[1] * k,
                stamp.across[0] * k,
                stamp.across[1] * k,
                origin[0],
                origin[1],
            )
            .post_concat(transform);
            let paint = PixmapPaint {
                opacity: (alpha as f32 * stamp.flow).round() / 255.0,
                quality: FilterQuality::Bilinear,
                ..PixmapPaint::default()
            };
            target.draw_pixmap(0, 0, tip.as_ref(), &paint, position, None);
        });
    }
}

#[derive(Clone, Copy)]
struct Stamp {
    center: [f32; 2],
    along: [f32; 2],
    across: [f32; 2],
    flow: f32,
}

impl Stamp {
    fn corner(&self, u: f32, v: f32) -> [f32; 2] {
        let (u, v) = (2.0 * u - 1.0, 2.0 * v - 1.0);
        [
            self.center[0] + self.along[0] * u + self.across[0] * v,
            self.center[1] + self.along[1] * u + self.across[1] * v,
        ]
    }
}

// Integer avalanche hash. Fixed seeds reproduce the same paper and bristles
// after undo, zoom, export, and reopening a project.
fn noise(mut seed: u32) -> f32 {
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x7feb352d);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x846ca68b);
    ((seed ^ (seed >> 16)) >> 8) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_textured_brush_families_have_distinct_real_masks() {
        let variants=[
            TextureBrush::Watercolor,TextureBrush::Marker,TextureBrush::Airbrush,
            TextureBrush::Sketch,TextureBrush::Graphite,TextureBrush::Dry,
            TextureBrush::Chalk,TextureBrush::Charcoal,TextureBrush::Pastel,
            TextureBrush::Calligraphy,TextureBrush::SoftSketch,
            TextureBrush::SketchBrush,
        ];
        let masks:Vec<_>=variants.iter()
            .map(|brush|brush.make_mask(BrushTipShape::Round)).collect();
        for (i,a) in masks.iter().enumerate() {
            assert!(a.iter().any(|&px|px>0));
            for b in masks.iter().skip(i+1) {
                assert_ne!(a,b,"each brush family must keep a visibly distinct texture");
            }
        }
    }

    #[test]
    fn colored_brush_masks_are_tinted_by_the_selected_color() {
        for (brush, preset) in [
            (TextureBrush::Watercolor, KritaBrushPreset::Watercolor),
            (TextureBrush::Airbrush, KritaBrushPreset::SoftAirbrush),
            (TextureBrush::Sketch, KritaBrushPreset::SketchPencil),
            (TextureBrush::SoftSketch, KritaBrushPreset::SoftSketchPencil),
            (TextureBrush::Graphite, KritaBrushPreset::GraphitePencil),
        ] {
            let stroke = Stroke {
                points: vec![
                    crate::project::StrokePoint { x: 12.0, y: 24.0, pressure: 1.0 },
                    crate::project::StrokePoint { x: 116.0, y: 24.0, pressure: 1.0 },
                ],
                color: [28, 154, 232, 255],
                fill_color: None,
                base_width: 18.0,
                is_vector: false,
                brush_preset: preset,
                opacity: 1.0,
                brush_texture_id: None,
                tip_shape: BrushTipShape::Round,
            };
            let mut target = Pixmap::new(128, 48).unwrap();
            brush.export(&mut target, &stroke, 255, Transform::identity());
            assert!(target.pixels().any(|pixel| {
                let color=pixel.demultiply();
                color.alpha() > 20
                    && (i16::from(color.red()) - 28).abs() <= 2
                    && (i16::from(color.green()) - 154).abs() <= 2
                    && (i16::from(color.blue()) - 232).abs() <= 2
            }), "{preset:?} texture ignored its stroke color");
        }
    }

    fn marker_routes_to_continuous_shared_stroke_engine() {
        let stroke=Stroke {
            points:Vec::new(),color:[0,0,0,255],fill_color:None,base_width:20.0,
            is_vector:false,brush_preset:KritaBrushPreset::ChiselMarker,
            opacity:0.42,brush_texture_id:None,tip_shape:BrushTipShape::Round,
        };
        assert_eq!(TextureBrush::for_stroke(&stroke),None);
    }

    #[test]
    fn textured_media_use_dense_spacing() {
        for brush in [
            TextureBrush::Watercolor,TextureBrush::Airbrush,TextureBrush::Sketch,
            TextureBrush::Graphite,TextureBrush::Dry,TextureBrush::Chalk,
            TextureBrush::Charcoal,TextureBrush::Pastel,TextureBrush::Calligraphy,
            TextureBrush::SoftSketch,
        ] {
            assert!(brush.spacing_radius_ratio() <= 0.16);
        }
    }

    #[test]
    fn textured_curve_sampling_uses_fitted_raster_centerline(){
        let stroke=Stroke{
            points:vec![
                crate::project::StrokePoint{x:0.,y:0.,pressure:1.},
                crate::project::StrokePoint{x:30.,y:2.,pressure:1.},
                crate::project::StrokePoint{x:55.,y:28.,pressure:1.},
                crate::project::StrokePoint{x:60.,y:70.,pressure:1.},
            ],
            color:[0,0,0,255],fill_color:None,base_width:12.,
            is_vector:false,brush_preset:KritaBrushPreset::GraphitePencil,
            opacity:1.,brush_texture_id:None,tip_shape:BrushTipShape::Round,
        };
        let mut count=0usize;
        TextureBrush::Graphite.stamps(&stroke,|_|count+=1);
        assert!(count>30,"curve should receive dense texture stamps, got {count}");
    }

    #[test]
    fn graphite_mask_has_paper_tooth_and_lower_density_than_marker() {
        let graphite=TextureBrush::Graphite.make_mask(BrushTipShape::Round);
        let marker=TextureBrush::Marker.make_mask(BrushTipShape::Round);
        let mut graphite_gaps=0usize;
        let mut marker_gaps=0usize;
        let mut graphite_sum=0u32;
        let mut marker_sum=0u32;
        let mut samples=0u32;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let u=(2.0*x as f32+1.0)/SIZE as f32-1.0;
                let v=(2.0*y as f32+1.0)/SIZE as f32-1.0;
                if u*u+v*v<=0.25 {
                    let index=y*SIZE+x;
                    graphite_gaps+=usize::from(graphite[index]==0);
                    marker_gaps+=usize::from(marker[index]==0);
                    graphite_sum+=u32::from(graphite[index]);
                    marker_sum+=u32::from(marker[index]);
                    samples+=1;
                }
            }
        }
        assert!(graphite_gaps>marker_gaps,
            "graphite should expose more paper tooth inside the nib");
        assert!(graphite_sum/samples<marker_sum/samples,
            "graphite should be lighter than marker in the same central footprint");
    }

    #[test]
    fn graphite_pressure_builds_from_light_to_dark() {
        let stroke=Stroke {
            points:vec![
                crate::project::StrokePoint{x:0.0,y:0.0,pressure:0.15},
                crate::project::StrokePoint{x:80.0,y:0.0,pressure:1.0},
            ],
            color:[0,0,0,255],fill_color:None,base_width:10.0,is_vector:false,
            brush_preset:KritaBrushPreset::GraphitePencil,opacity:0.62,
            brush_texture_id:None,tip_shape:BrushTipShape::Round,
        };
        let mut flows=Vec::new();
        TextureBrush::Graphite.stamps(&stroke,|stamp|flows.push(stamp.flow));
        assert!(flows.len()>3);
        assert!(flows.last().copied().unwrap()>flows.first().copied().unwrap()*1.8,
            "hard pressure must build visibly darker graphite");
    }

    #[test]
    fn new_graphite_preset_routes_through_texture_engine() {
        let stroke=Stroke {
            points:Vec::new(),color:[0,0,0,255],fill_color:None,base_width:7.0,
            is_vector:false,brush_preset:KritaBrushPreset::GraphitePencil,
            opacity:0.62,brush_texture_id:None,tip_shape:BrushTipShape::Round,
        };
        assert_eq!(TextureBrush::for_stroke(&stroke),Some(TextureBrush::Graphite));
    }
}
