//! Non-destructive stage chrome. Everything in this module is a screen-space
//! decoration: none of these colors, rounded corners, borders or shadows are
//! stored in .stick projects or passed to PNG/video exporters.
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CanvasAppearance {
    pub border_visible: bool,
    pub border_color: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub shadow_enabled: bool,
    pub workspace_color: [u8; 4],
    pub paper_override: bool,
    pub paper_color: [u8; 4],
    pub paper_checkerboard: bool,
}

impl Default for CanvasAppearance {
    fn default() -> Self {
        Self {
            border_visible: false,
            border_color: [255, 202, 66, 255],
            border_width: 2.0,
            corner_radius: 0.0,
            shadow_enabled: false,
            workspace_color: [30, 31, 36, 255],
            paper_override: false,
            paper_color: [255, 255, 255, 255],
            paper_checkerboard: false,
        }
    }
}

impl CanvasAppearance {
    pub fn width(&self) -> f32 {
        if self.border_width.is_finite() {
            self.border_width.clamp(0.5, 12.0)
        } else { 2.0 }
    }

    fn radius(&self, width: f32, height: f32) -> f32 {
        if !self.corner_radius.is_finite() { return 0.0; }
        self.corner_radius.clamp(0.0, 64.0).min(width * 0.5).min(height * 0.5)
    }

    pub fn workspace_paint(&self) -> egui::Color32 {
        let c = self.workspace_color;
        egui::Color32::from_rgb(c[0], c[1], c[2])
    }
}

/// Paint the optional drop shadow before the paper/background is rendered.
pub fn draw_shadow(
    painter: &egui::Painter,
    width: f32,
    height: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
    appearance: &CanvasAppearance,
) {
    if !appearance.shadow_enabled || width <= 0.0 || height <= 0.0 { return; }
    let corners = [
        egui::pos2(0.0, 0.0), egui::pos2(width, 0.0),
        egui::pos2(width, height), egui::pos2(0.0, height),
    ];
    // Five screen-pixel layers provide a subtle, bounded approximation
    // without a large blurred texture or project-sized allocation.
    for layer in (1..=5).rev() {
        let offset = egui::vec2(layer as f32 * 1.6, layer as f32 * 2.0);
        let verts = corners.iter().map(|&p| to_screen(p) + offset).collect();
        painter.add(egui::Shape::convex_polygon(
            verts, egui::Color32::from_black_alpha((18 - layer * 2) as u8),
            egui::Stroke::NONE,
        ));
    }
}

fn corner_arc(center: egui::Pos2, radius: f32, start: f32, count: usize) -> Vec<egui::Pos2> {
    (0..=count).map(|index| {
        let angle = start + index as f32 * std::f32::consts::FRAC_PI_2 / count as f32;
        center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
    }).collect()
}

/// Draw only on the editor screen. Rounded corner masks use the workspace
/// color to hide stage corner pixels, without destructively trimming artwork.
pub fn draw_chrome(
    painter: &egui::Painter,
    width: f32,
    height: f32,
    scale: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
    appearance: &CanvasAppearance,
) {
    if width <= 0.0 || height <= 0.0 { return; }
    let radius = appearance.radius(width, height);
    let corners = [
        (egui::pos2(0.0, 0.0), egui::pos2(radius, radius), std::f32::consts::PI),
        (egui::pos2(width, 0.0), egui::pos2(width - radius, radius), std::f32::consts::FRAC_PI_2 * 3.0),
        (egui::pos2(width, height), egui::pos2(width - radius, height - radius), 0.0),
        (egui::pos2(0.0, height), egui::pos2(radius, height - radius), std::f32::consts::FRAC_PI_2),
    ];
    let mut outline = Vec::with_capacity(52);
    if radius >= 1.0 {
        for (corner, center, start) in corners {
            let arc = corner_arc(center, radius, start, 12);
            // Each outside-corner wedge is a tiny triangle, so no concave
            // mesh or large texture allocations are required.
            for pair in arc.windows(2) {
                painter.add(egui::Shape::convex_polygon(
                    vec![to_screen(corner), to_screen(pair[0]), to_screen(pair[1])],
                    appearance.workspace_paint(), egui::Stroke::NONE,
                ));
            }
            outline.extend(arc.into_iter().map(to_screen));
        }
    } else {
        outline.extend([
            egui::pos2(0.0, 0.0), egui::pos2(width, 0.0),
            egui::pos2(width, height), egui::pos2(0.0, height)
        ].into_iter().map(to_screen));
    }
    if appearance.border_visible {
        let c = appearance.border_color;
        painter.add(egui::Shape::closed_line(outline, egui::Stroke::new(
            appearance.width() * scale.clamp(0.25, 4.0),
            egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_nondestructive_and_legacy_settings_are_accepted() {
        let parsed: CanvasAppearance = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed, CanvasAppearance::default());
        assert!(!parsed.border_visible);
        assert!(!parsed.shadow_enabled);
        assert_eq!(parsed.corner_radius, 0.);
    }
    #[test]
    fn geometry_values_are_bounded_even_for_corrupt_settings() {
        let mut a = CanvasAppearance::default();
        a.corner_radius = f32::INFINITY;
        a.border_width = f32::NAN;
        assert_eq!(a.radius(1920., 1080.), 0.);
        assert_eq!(a.width(), 2.);
        a.corner_radius = 120.;
        assert_eq!(a.radius(60., 40.), 20.);
    }
    #[test]
    fn every_quarter_arc_includes_both_exact_endpoints() {
        let arc = corner_arc(egui::pos2(10., 10.), 10.,
            std::f32::consts::PI, 12);
        assert_eq!(arc.len(), 13);
        assert!((arc[0].x - 0.).abs() < 0.001);
        assert!((arc.last().unwrap().y - 0.).abs() < 0.001);
    }
}
