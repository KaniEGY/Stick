//! Fast canvas radial menus and on-canvas adjustment feedback.
//! Kept separate from the editor state machine so popup painting/hit-testing
//! does not add more branches to the main drawing loop.
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickMenuKind {
    Shapes,
    Colors,
    VectorEraser,
    RasterEraser,
    CanvasView,
    Selection,
}

#[derive(Clone, Copy, Debug)]
pub struct QuickMenuState {
    pub kind: QuickMenuKind,
    pub center: egui::Pos2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuickMenuAction {
    Shape(usize),
    ToggleShapeVector,
    Color([u8; 4]),
    CustomColor,
    ToggleBucketVector,
    Close,
}

const MENU_SIZE: f32 = 236.0;
const RING_RADIUS: f32 = 78.0;

fn draw_shape_icon(painter:&egui::Painter, center:egui::Pos2, index:usize, color:egui::Color32) {
    let stroke=egui::Stroke::new(2.0,color);
    match index {
        0 => { painter.line_segment(
            [center+egui::vec2(-9.0,7.0),center+egui::vec2(9.0,-7.0)],stroke); }
        1 => { painter.circle_stroke(center,7.5,stroke); }
        2 => { painter.rect_stroke(
            egui::Rect::from_center_size(center,egui::vec2(14.0,14.0)),1.5,stroke); }
        3 => { painter.rect_stroke(
            egui::Rect::from_center_size(center,egui::vec2(17.0,11.0)),1.5,stroke); }
        4 => { painter.add(egui::Shape::closed_line(vec![
            center+egui::vec2(0.0,-8.0),
            center+egui::vec2(8.0,7.0),
            center+egui::vec2(-8.0,7.0),
        ],stroke)); }
        _ => {
            let pts=(0..5).map(|i|{
                let a=-std::f32::consts::FRAC_PI_2+i as f32*std::f32::consts::TAU/5.0;
                center+egui::vec2(a.cos(),a.sin())*8.0
            }).collect();
            painter.add(egui::Shape::closed_line(pts,stroke));
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    state: QuickMenuState,
    shape_labels: &[&str; 6],
    colors: &[[u8; 4]],
    selected_shape: usize,
    shape_vector: bool,
    selected_color: [u8; 4],
    custom_label: &str,
    bucket_mode: Option<bool>,
) -> Option<QuickMenuAction> {
    let origin = state.center - egui::vec2(MENU_SIZE * 0.5, MENU_SIZE * 0.5);
    let mut action = None;
    egui::Area::new(egui::Id::new("stick_canvas_quick_radial_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(origin)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(MENU_SIZE, MENU_SIZE),
                egui::Sense::hover(),
            );
            let center = rect.center();
            let center_rect=egui::Rect::from_center_size(center,egui::vec2(62.0,62.0));
            let center_response=ui.allocate_rect(center_rect,egui::Sense::click());
            let center_fill=match state.kind {
                QuickMenuKind::Colors=>egui::Color32::from_rgba_unmultiplied(
                    selected_color[0],selected_color[1],selected_color[2],selected_color[3]),
                QuickMenuKind::Shapes=>if shape_vector {
                    egui::Color32::from_rgba_unmultiplied(14,104,126,246)
                } else {
                    egui::Color32::from_rgba_unmultiplied(18,24,33,246)
                },
                QuickMenuKind::VectorEraser | QuickMenuKind::RasterEraser | QuickMenuKind::CanvasView | QuickMenuKind::Selection =>
                    egui::Color32::from_rgba_unmultiplied(18,24,33,246),
            };
            ui.painter().circle_filled(center,31.0,center_fill);
            ui.painter().circle_stroke(
                center,31.0,
                egui::Stroke::new(
                    if center_response.hovered(){2.2}else{1.4},
                    egui::Color32::from_rgb(66,205,242)));
            ui.painter().circle_stroke(
                center,104.0,
                egui::Stroke::new(1.0,egui::Color32::from_rgba_unmultiplied(66,205,242,72)));

            match state.kind {
                QuickMenuKind::Shapes => {
                    ui.painter().text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        if shape_vector {"VECTOR"} else {"RASTER"},
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(223, 232, 241),
                    );
                    if center_response.clicked() {
                        action=Some(QuickMenuAction::ToggleShapeVector);
                    }
                    for (index, _label) in shape_labels.iter().enumerate() {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + index as f32 * std::f32::consts::TAU / shape_labels.len() as f32;
                        let p = center + egui::vec2(angle.cos(), angle.sin()) * RING_RADIUS;
                        let r = egui::Rect::from_center_size(p, egui::vec2(43.0, 43.0));
                        let response = ui.allocate_rect(r, egui::Sense::click());
                        let active = index == selected_shape;
                        let fill = if active {
                            egui::Color32::from_rgb(14, 104, 126)
                        } else if response.hovered() {
                            egui::Color32::from_rgb(35, 58, 72)
                        } else {
                            egui::Color32::from_rgb(28, 34, 43)
                        };
                        ui.painter().circle_filled(p, 20.0, fill);
                        ui.painter().circle_stroke(
                            p,
                            20.0,
                            egui::Stroke::new(
                                if active { 2.0 } else { 1.1 },
                                if active {
                                    egui::Color32::from_rgb(255, 205, 63)
                                } else {
                                    egui::Color32::from_rgb(66, 205, 242)
                                },
                            ),
                        );
                        draw_shape_icon(ui.painter(),p,index,egui::Color32::WHITE);
                        if response.clicked() {
                            action = Some(QuickMenuAction::Shape(index));
                        }
                    }
                }
                QuickMenuKind::Colors => {
                    let luminance=(selected_color[0] as u16*299
                        +selected_color[1] as u16*587
                        +selected_color[2] as u16*114)/1000;
                    let label_color=if luminance>150 {
                        egui::Color32::BLACK
                    } else {egui::Color32::WHITE};
                    if let Some(vector)=bucket_mode {
                        let lower:Vec<_>=(0..=32).map(|i|{
                            let angle=i as f32*std::f32::consts::PI/32.;
                            center+egui::vec2(angle.cos(),angle.sin())*30.
                        }).collect();
                        ui.painter().add(egui::Shape::convex_polygon(lower,
                            egui::Color32::from_rgb(30,51,65),egui::Stroke::NONE));
                        ui.painter().line_segment([center+egui::vec2(-30.,0.),center+egui::vec2(30.,0.)],
                            egui::Stroke::new(1.,egui::Color32::from_rgb(66,205,242)));
                        ui.painter().text(center+egui::vec2(0.,-13.),egui::Align2::CENTER_CENTER,
                            custom_label,egui::FontId::proportional(10.),label_color);
                        ui.painter().text(center+egui::vec2(0.,13.),egui::Align2::CENTER_CENTER,
                            if vector {"VECTOR"} else {"RASTER"},egui::FontId::proportional(10.),egui::Color32::WHITE);
                        if center_response.clicked() {
                            if let Some(p)=center_response.interact_pointer_pos().filter(|p|p.distance(center)<=31.) {
                                action=Some(if p.y<center.y {QuickMenuAction::CustomColor} else {QuickMenuAction::ToggleBucketVector});
                            }
                        }
                    } else {
                    ui.painter().text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        custom_label,
                        egui::FontId::proportional(10.0),
                        label_color,
                    );
                    if center_response.clicked() {
                        action=Some(QuickMenuAction::CustomColor);
                    }
                    }
                    let count = colors.len().max(1);
                    for (index, color) in colors.iter().copied().enumerate() {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + index as f32 * std::f32::consts::TAU / count as f32;
                        let p = center + egui::vec2(angle.cos(), angle.sin()) * RING_RADIUS;
                        let r = egui::Rect::from_center_size(p, egui::vec2(32.0, 32.0));
                        let response = ui.allocate_rect(r, egui::Sense::click());
                        let fill = egui::Color32::from_rgba_unmultiplied(
                            color[0], color[1], color[2], color[3],
                        );
                        ui.painter().circle_filled(p, 13.0, fill);
                        ui.painter().circle_stroke(
                            p,
                            if color == selected_color { 16.0 } else { 14.0 },
                            egui::Stroke::new(
                                if color == selected_color { 2.2 } else { 1.2 },
                                if color == selected_color {
                                    egui::Color32::from_rgb(255, 205, 63)
                                } else {
                                    egui::Color32::from_rgb(225, 230, 235)
                                },
                            ),
                        );
                        if response.clicked() {
                            action = Some(QuickMenuAction::Color(color));
                        }
                    }
                }
                QuickMenuKind::VectorEraser | QuickMenuKind::RasterEraser | QuickMenuKind::CanvasView | QuickMenuKind::Selection => {}
            }
        });

    if action.is_none() && ctx.input(|i| i.pointer.primary_pressed()) {
        let outside = ctx.input(|i| i.pointer.interact_pos())
            .is_none_or(|p| p.distance(state.center) > 118.0);
        if outside {
            action = Some(QuickMenuAction::Close);
        }
    }
    action
}

pub fn show_mode_menu(
    ctx:&egui::Context,
    state:QuickMenuState,
    items:&[(&str,&str)],
    selected:Option<usize>,
)->Option<usize>{
    if items.is_empty(){return None}
    let origin=state.center-egui::vec2(MENU_SIZE*0.5,MENU_SIZE*0.5);
    let mut action=None;
    egui::Area::new(egui::Id::new(("stick_canvas_tool_mode_menu",state.kind as u8)))
        .order(egui::Order::Foreground)
        .fixed_pos(origin)
        .show(ctx,|ui|{
            let (rect,_)=ui.allocate_exact_size(egui::vec2(MENU_SIZE,MENU_SIZE),egui::Sense::hover());
            let center=rect.center();
            let center_hit=egui::Rect::from_center_size(center,egui::vec2(56.0,56.0));
            let center_response=ui.allocate_rect(center_hit,egui::Sense::click());
            ui.painter().circle_filled(center,27.0,
                if center_response.hovered(){egui::Color32::from_rgb(48,58,70)}
                else{egui::Color32::from_rgba_unmultiplied(18,24,33,246)});
            ui.painter().circle_stroke(center,27.0,egui::Stroke::new(
                if center_response.hovered(){2.0}else{1.4},
                egui::Color32::from_rgb(66,205,242)));
            ui.painter().text(center,egui::Align2::CENTER_CENTER,"×",
                egui::FontId::proportional(18.0),egui::Color32::from_rgb(220,228,236));
            if center_response.clicked(){action=Some(usize::MAX);}
            let count=items.len();
            for (index,(icon,label)) in items.iter().enumerate() {
                let angle=-std::f32::consts::FRAC_PI_2
                    +index as f32*std::f32::consts::TAU/count as f32;
                let p=center+egui::vec2(angle.cos(),angle.sin())*RING_RADIUS;
                let hit=egui::Rect::from_center_size(p,egui::vec2(62.0,62.0));
                let response=ui.allocate_rect(hit,egui::Sense::click());
                let active=selected==Some(index);
                let fill=if active {egui::Color32::from_rgb(113,88,25)}
                    else if response.hovered(){egui::Color32::from_rgb(35,58,72)}
                    else{egui::Color32::from_rgb(28,34,43)};
                ui.painter().circle_filled(p,28.0,fill);
                ui.painter().circle_stroke(p,28.0,egui::Stroke::new(
                    if active{2.2}else{1.2},
                    if active{egui::Color32::from_rgb(255,205,63)}
                    else{egui::Color32::from_rgb(66,205,242)}));
                ui.painter().text(p+egui::vec2(0.0,-5.0),egui::Align2::CENTER_CENTER,*icon,
                    egui::FontId::proportional(17.0),egui::Color32::WHITE);
                ui.painter().text(p+egui::vec2(0.0,13.0),egui::Align2::CENTER_CENTER,*label,
                    egui::FontId::proportional(9.0),egui::Color32::from_rgb(225,230,235));
                if response.clicked(){action=Some(index);}
            }
        });
    if action.is_none() && ctx.input(|i|i.pointer.primary_pressed()) {
        let outside=ctx.input(|i|i.pointer.interact_pos())
            .is_none_or(|p|p.distance(state.center)>118.0);
        if outside {return Some(usize::MAX)}
    }
    action
}

pub fn draw_size_feedback(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius_px: f32,
    color: egui::Color32,
    value: f32,
    percent: f32,
    label: &str,
) {
    let r = radius_px.clamp(3.0, 220.0);
    painter.circle_stroke(center, r + 2.0, egui::Stroke::new(2.4, egui::Color32::BLACK));
    painter.circle_stroke(center, r + 1.0, egui::Stroke::new(1.2, egui::Color32::WHITE));
    painter.circle_stroke(center, r, egui::Stroke::new(1.7, color));
    painter.text(
        center + egui::vec2(0.0, r + 9.0),
        egui::Align2::CENTER_TOP,
        format!("{} {:.0}px • {:.0}%", label, value, percent.clamp(0.0,100.0)),
        egui::FontId::monospace(11.0),
        egui::Color32::WHITE,
    );
}

pub fn draw_opacity_feedback(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: [u8; 4],
    opacity: f32,
) {
    let alpha = ((color[3] as f32) * opacity.clamp(0.0, 1.0)).round() as u8;
    let fill = egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], alpha);
    painter.circle_filled(center, 10.0, fill);
    painter.circle_stroke(center, 11.5, egui::Stroke::new(1.3, egui::Color32::WHITE));
    painter.circle_stroke(center, 13.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
    painter.text(
        center + egui::vec2(0.0, 18.0),
        egui::Align2::CENTER_TOP,
        format!("{}%", (opacity.clamp(0.0, 1.0) * 100.0).round() as u32),
        egui::FontId::monospace(11.0),
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_menu_state_is_copyable_and_kind_is_stable() {
        let state = QuickMenuState {
            kind: QuickMenuKind::Colors,
            center: egui::pos2(40.0, 50.0),
        };
        let copied = state;
        assert_eq!(copied.kind, QuickMenuKind::Colors);
        assert_eq!(copied.center, egui::pos2(40.0, 50.0));
    }
}
