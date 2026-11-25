//! Interactive articulated pose reference for Drawing mode.
//! The guide is editor-only: it never becomes part of the exported artwork.

use eframe::egui;
use serde::{Deserialize,Serialize};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PoseJoint {
    Root, Chest, Head,
    LeftShoulder, RightShoulder,
    LeftElbow, RightElbow,
    LeftWrist, RightWrist,
    LeftHip, RightHip,
    LeftKnee, RightKnee,
    LeftAnkle, RightAnkle,
}

#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum BodyPreset {
    Classic,
    Slim,
    Standard,
    Athletic,
    Heavy,
    Tall,
    Short,
    Feminine,
    Cartoon,
    Custom,
}
impl Default for BodyPreset {
    fn default()->Self{Self::Classic}
}
pub const BODY_PRESETS:[BodyPreset;9]=[
    BodyPreset::Classic,BodyPreset::Slim,BodyPreset::Standard,
    BodyPreset::Athletic,BodyPreset::Heavy,BodyPreset::Tall,
    BodyPreset::Short,BodyPreset::Feminine,BodyPreset::Cartoon,
];
impl BodyPreset {
    pub fn labels(self)->(&'static str,&'static str){
        match self {
            Self::Classic=>("Classic","كلاسيك"),
            Self::Slim=>("Slim","نحيف"),
            Self::Standard=>("Standard","عادي"),
            Self::Athletic=>("Athletic","رياضي"),
            Self::Heavy=>("Heavy","ضخم"),
            Self::Tall=>("Tall","طويل"),
            Self::Short=>("Short","قصير"),
            Self::Feminine=>("Feminine","أنثوي"),
            Self::Cartoon=>("Cartoon","كرتوني"),
            Self::Custom=>("Custom","مخصص"),
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(default)]
pub struct PoseGuide {
    pub visible:bool,
    /// True draws the editor-only rig over artwork; false keeps it behind artwork.
    pub in_front:bool,
    pub x:f32,
    pub y:f32,
    pub scale:f32,
    pub opacity:f32,
    #[serde(default = "default_rig_color")]
    pub color:[u8;4],
    pub head_scale:f32,
    pub head_offset_x:f32,
    pub head_offset_y:f32,

    // Body proportions are independent from pose angles. They make the guide
    // useful for anatomy/blocking references instead of one fixed stick body.
    pub body_preset:BodyPreset,
    pub torso_length:f32,
    pub shoulder_width:f32,
    pub hip_width:f32,
    pub upper_arm_length:f32,
    pub forearm_length:f32,
    pub thigh_length:f32,
    pub shin_length:f32,

    pub torso_tilt_deg:f32,
    pub shoulder_deg:f32,
    pub hip_deg:f32,
    pub left_arm_deg:f32,
    pub right_arm_deg:f32,
    pub left_forearm_deg:f32,
    pub right_forearm_deg:f32,
    pub left_leg_deg:f32,
    pub right_leg_deg:f32,
    pub left_shin_deg:f32,
    pub right_shin_deg:f32,
}
fn default_rig_color()->[u8;4]{[70,190,255,255]}

impl Default for PoseGuide{
    fn default()->Self{Self{
        visible:false,in_front:true,x:0.5,y:0.55,scale:1.0,opacity:0.45,
        color:default_rig_color(),
        head_scale:1.0,head_offset_x:0.0,head_offset_y:0.0,
        body_preset:BodyPreset::Classic,
        torso_length:1.0,shoulder_width:1.0,hip_width:1.0,
        upper_arm_length:1.0,forearm_length:1.0,
        thigh_length:1.0,shin_length:1.0,
        torso_tilt_deg:0.0,shoulder_deg:0.0,hip_deg:0.0,
        left_arm_deg:125.0,right_arm_deg:55.0,
        left_forearm_deg:105.0,right_forearm_deg:75.0,left_leg_deg:110.0,
        right_leg_deg:70.0,left_shin_deg:95.0,right_shin_deg:85.0,
    }}
}
impl PoseGuide {
    pub fn apply_body_preset(&mut self,preset:BodyPreset){
        let (head,torso,shoulders,hips,upper,forearm,thigh,shin)=match preset {
            BodyPreset::Classic=>(1.00,1.00,1.00,1.00,1.00,1.00,1.00,1.00),
            BodyPreset::Slim=>(0.96,1.06,0.84,0.88,1.03,1.03,1.05,1.05),
            BodyPreset::Standard=>(0.98,1.02,1.04,1.00,1.02,1.02,1.03,1.03),
            BodyPreset::Athletic=>(0.92,1.04,1.26,0.94,1.06,1.06,1.07,1.07),
            BodyPreset::Heavy=>(1.04,0.98,1.28,1.26,0.99,0.99,0.98,0.98),
            BodyPreset::Tall=>(0.84,1.17,1.00,0.96,1.13,1.13,1.19,1.19),
            BodyPreset::Short=>(1.14,0.88,1.02,1.03,0.89,0.89,0.86,0.86),
            BodyPreset::Feminine=>(1.00,1.04,0.88,1.20,0.99,0.99,1.05,1.04),
            BodyPreset::Cartoon=>(1.38,0.82,1.08,1.02,0.91,0.91,0.90,0.90),
            BodyPreset::Custom=>{self.body_preset=BodyPreset::Custom;return},
        };
        self.body_preset=preset;
        self.head_scale=head;
        self.torso_length=torso;
        self.shoulder_width=shoulders;
        self.hip_width=hips;
        self.upper_arm_length=upper;
        self.forearm_length=forearm;
        self.thigh_length=thigh;
        self.shin_length=shin;
    }

    pub fn mark_custom(&mut self){self.body_preset=BodyPreset::Custom;}

    pub fn reset_pose_angles(&mut self){
        self.head_offset_x=0.0; self.head_offset_y=0.0;
        self.torso_tilt_deg=0.0; self.shoulder_deg=0.0; self.hip_deg=0.0;
        self.left_arm_deg=125.0; self.right_arm_deg=55.0;
        self.left_forearm_deg=105.0; self.right_forearm_deg=75.0;
        self.left_leg_deg=110.0; self.right_leg_deg=70.0;
        self.left_shin_deg=95.0; self.right_shin_deg=85.0;
    }
}

#[derive(Clone,Copy,Debug)]
struct PoseLayout {
    base:f32,
    root:egui::Pos2,
    chest:egui::Pos2,
    head:egui::Pos2,
    head_radius:f32,
    ls:egui::Pos2, rs:egui::Pos2,
    lh:egui::Pos2, rh:egui::Pos2,
    le:egui::Pos2, re:egui::Pos2,
    lw:egui::Pos2, rw:egui::Pos2,
    lk:egui::Pos2, rk:egui::Pos2,
    la:egui::Pos2, ra:egui::Pos2,
}
impl PoseLayout {
    fn joint(self,joint:PoseJoint)->egui::Pos2 {
        match joint {
            PoseJoint::Root=>self.root, PoseJoint::Chest=>self.chest, PoseJoint::Head=>self.head,
            PoseJoint::LeftShoulder=>self.ls, PoseJoint::RightShoulder=>self.rs,
            PoseJoint::LeftElbow=>self.le, PoseJoint::RightElbow=>self.re,
            PoseJoint::LeftWrist=>self.lw, PoseJoint::RightWrist=>self.rw,
            PoseJoint::LeftHip=>self.lh, PoseJoint::RightHip=>self.rh,
            PoseJoint::LeftKnee=>self.lk, PoseJoint::RightKnee=>self.rk,
            PoseJoint::LeftAnkle=>self.la, PoseJoint::RightAnkle=>self.ra,
        }
    }
}

const JOINTS:[PoseJoint;15]=[
    PoseJoint::Head,PoseJoint::LeftWrist,PoseJoint::RightWrist,
    PoseJoint::LeftAnkle,PoseJoint::RightAnkle,
    PoseJoint::LeftElbow,PoseJoint::RightElbow,
    PoseJoint::LeftKnee,PoseJoint::RightKnee,
    PoseJoint::LeftShoulder,PoseJoint::RightShoulder,
    PoseJoint::LeftHip,PoseJoint::RightHip,
    PoseJoint::Chest,PoseJoint::Root,
];

fn dir(angle:f32,len:f32)->egui::Vec2{
    let a=angle.to_radians();
    egui::vec2(a.cos()*len,a.sin()*len)
}
fn angle_deg(from:egui::Pos2,to:egui::Pos2)->f32{
    let d=to-from;
    d.y.atan2(d.x).to_degrees()
}
fn norm_angle(mut a:f32)->f32{
    while a>180.0{a-=360.0}
    while a<=-180.0{a+=360.0}
    a
}

fn solve_two_bone(
    anchor:egui::Pos2,target:egui::Pos2,l1:f32,l2:f32,current_upper_deg:f32
)->(f32,f32){
    let raw=target-anchor;
    let raw_len=raw.length();
    let direction=if raw_len>0.0001 {raw/raw_len}else{dir(current_upper_deg,1.0)};
    let min_d=(l1-l2).abs()+0.001;
    let max_d=(l1+l2-0.001).max(min_d);
    let d=raw_len.clamp(min_d,max_d);
    let target_angle=direction.y.atan2(direction.x);
    let cos_alpha=((d*d+l1*l1-l2*l2)/(2.0*d*l1.max(0.001))).clamp(-1.0,1.0);
    let alpha=cos_alpha.acos();
    let candidates=[target_angle+alpha,target_angle-alpha];
    let current=current_upper_deg.to_radians();
    let upper=candidates.into_iter().min_by(|a,b|{
        let da=norm_angle((*a-current).to_degrees()).abs();
        let db=norm_angle((*b-current).to_degrees()).abs();
        da.total_cmp(&db)
    }).unwrap_or(target_angle);
    let elbow=anchor+egui::vec2(upper.cos()*l1,upper.sin()*l1);
    let effective_target=anchor+direction*d;
    let lower=angle_deg(elbow,effective_target);
    (norm_angle(upper.to_degrees()),norm_angle(lower))
}

fn layout(guide:&PoseGuide,canvas:(f32,f32))->PoseLayout{
    let base=canvas.0.min(canvas.1)*0.22*guide.scale.clamp(0.25,3.0);
    let root=egui::pos2(canvas.0*guide.x.clamp(0.0,1.0),canvas.1*guide.y.clamp(0.0,1.0));
    let torso_len=base*0.55*guide.torso_length.clamp(0.55,1.65);
    let upper_len=base*0.34*guide.upper_arm_length.clamp(0.55,1.60);
    let fore_len=base*0.31*guide.forearm_length.clamp(0.55,1.60);
    let thigh_len=base*0.44*guide.thigh_length.clamp(0.55,1.70);
    let shin_len=base*0.42*guide.shin_length.clamp(0.55,1.70);
    let torso_dir=dir(-90.0+guide.torso_tilt_deg,torso_len);
    let chest=root+torso_dir;
    let head_anchor=chest+dir(-90.0+guide.torso_tilt_deg,base*0.22);
    let head=head_anchor+egui::vec2(
        guide.head_offset_x.clamp(-0.6,0.6)*base,
        guide.head_offset_y.clamp(-0.6,0.6)*base,
    );
    let shoulder_half=dir(guide.shoulder_deg,
        base*0.22*guide.shoulder_width.clamp(0.55,1.65));
    let hip_half=dir(guide.hip_deg,
        base*0.15*guide.hip_width.clamp(0.55,1.70));
    let ls=chest-shoulder_half; let rs=chest+shoulder_half;
    let lh=root-hip_half; let rh=root+hip_half;
    let le=ls+dir(guide.left_arm_deg,upper_len);
    let re=rs+dir(guide.right_arm_deg,upper_len);
    let lw=le+dir(guide.left_forearm_deg,fore_len);
    let rw=re+dir(guide.right_forearm_deg,fore_len);
    let lk=lh+dir(guide.left_leg_deg,thigh_len);
    let rk=rh+dir(guide.right_leg_deg,thigh_len);
    let la=lk+dir(guide.left_shin_deg,shin_len);
    let ra=rk+dir(guide.right_shin_deg,shin_len);
    PoseLayout{base,root,chest,head,head_radius:base*0.105*guide.head_scale.clamp(0.45,2.0),
        ls,rs,lh,rh,le,re,lw,rw,lk,rk,la,ra}
}

pub fn draw(
    painter:&egui::Painter,guide:&PoseGuide,canvas:(f32,f32),
    to_screen:impl Fn(egui::Pos2)->egui::Pos2
){
    draw_active(painter,guide,canvas,to_screen,None);
}

pub fn draw_active(
    painter:&egui::Painter,guide:&PoseGuide,canvas:(f32,f32),
    to_screen:impl Fn(egui::Pos2)->egui::Pos2,
    active:Option<PoseJoint>,
){
    if !guide.visible{return}
    let g=layout(guide,canvas);
    let alpha=(guide.opacity.clamp(0.05,1.0)*guide.color[3] as f32).round() as u8;
    let color=egui::Color32::from_rgba_unmultiplied(
        guide.color[0],guide.color[1],guide.color[2],alpha);
    let active_color=egui::Color32::from_rgba_unmultiplied(255,201,64,alpha.max(150));
    let stroke=egui::Stroke::new(2.0,color);
    let line=|a:egui::Pos2,b:egui::Pos2|painter.line_segment([to_screen(a),to_screen(b)],stroke);
    line(g.root,g.chest); line(g.ls,g.rs); line(g.lh,g.rh);
    line(g.ls,g.le);line(g.le,g.lw);line(g.rs,g.re);line(g.re,g.rw);
    line(g.lh,g.lk);line(g.lk,g.la);line(g.rh,g.rk);line(g.rk,g.ra);

    // Radius is stored in stage units and transformed through the exact same
    // stage->screen transform as the center, so zoom/pointer exit cannot resize it.
    let head_screen=to_screen(g.head);
    let head_edge=to_screen(g.head+egui::vec2(g.head_radius,0.0));
    let head_radius_px=head_screen.distance(head_edge).max(1.0);
    painter.circle_stroke(head_screen,head_radius_px,
        egui::Stroke::new(if active==Some(PoseJoint::Head){2.8}else{2.0},
            if active==Some(PoseJoint::Head){active_color}else{color}));

    for joint in JOINTS {
        if joint==PoseJoint::Head{continue}
        let p=to_screen(g.joint(joint));
        let is_active=active==Some(joint);
        painter.circle_filled(p,if is_active{5.0}else{3.4},
            if is_active{active_color}else{color});
        if is_active {
            painter.circle_stroke(p,8.0,egui::Stroke::new(1.3,egui::Color32::WHITE));
        }
    }
}

pub fn hit_test_joint(
    guide:&PoseGuide,canvas:(f32,f32),stage_pos:egui::Pos2,radius_stage:f32
)->Option<PoseJoint>{
    if !guide.visible{return None}
    let g=layout(guide,canvas);
    let r=radius_stage.max(1.0);
    JOINTS.into_iter()
        .filter_map(|joint|{
            let p=g.joint(joint);
            let d=p.distance(stage_pos);
            (d<=r).then_some((joint,d))
        })
        .min_by(|a,b|a.1.total_cmp(&b.1))
        .map(|x|x.0)
}

pub fn drag_joint(
    guide:&mut PoseGuide,joint:PoseJoint,canvas:(f32,f32),stage_pos:egui::Pos2
){
    if canvas.0<=0.0||canvas.1<=0.0{return}
    let g=layout(guide,canvas);
    let upper_len=g.base*0.34*guide.upper_arm_length.clamp(0.55,1.60);
    let fore_len=g.base*0.31*guide.forearm_length.clamp(0.55,1.60);
    let thigh_len=g.base*0.44*guide.thigh_length.clamp(0.55,1.70);
    let shin_len=g.base*0.42*guide.shin_length.clamp(0.55,1.70);
    match joint {
        PoseJoint::Root=>{
            guide.x=(stage_pos.x/canvas.0).clamp(0.0,1.0);
            guide.y=(stage_pos.y/canvas.1).clamp(0.0,1.0);
        }
        PoseJoint::Chest=>{
            guide.torso_tilt_deg=norm_angle(angle_deg(g.root,stage_pos)+90.0);
        }
        PoseJoint::Head=>{
            let anchor=g.chest+dir(-90.0+guide.torso_tilt_deg,g.base*0.22);
            let d=stage_pos-anchor;
            guide.head_offset_x=(d.x/g.base.max(1.0)).clamp(-0.6,0.6);
            guide.head_offset_y=(d.y/g.base.max(1.0)).clamp(-0.6,0.6);
        }
        PoseJoint::RightShoulder=>guide.shoulder_deg=norm_angle(angle_deg(g.chest,stage_pos)),
        PoseJoint::LeftShoulder=>guide.shoulder_deg=norm_angle(angle_deg(g.chest,stage_pos)-180.0),
        PoseJoint::RightHip=>guide.hip_deg=norm_angle(angle_deg(g.root,stage_pos)),
        PoseJoint::LeftHip=>guide.hip_deg=norm_angle(angle_deg(g.root,stage_pos)-180.0),
        PoseJoint::LeftElbow=>guide.left_arm_deg=norm_angle(angle_deg(g.ls,stage_pos)),
        PoseJoint::RightElbow=>guide.right_arm_deg=norm_angle(angle_deg(g.rs,stage_pos)),
        PoseJoint::LeftWrist=>{
            let (upper,lower)=solve_two_bone(
                g.ls,stage_pos,upper_len,fore_len,guide.left_arm_deg);
            guide.left_arm_deg=upper;guide.left_forearm_deg=lower;
        }
        PoseJoint::RightWrist=>{
            let (upper,lower)=solve_two_bone(
                g.rs,stage_pos,upper_len,fore_len,guide.right_arm_deg);
            guide.right_arm_deg=upper;guide.right_forearm_deg=lower;
        }
        PoseJoint::LeftKnee=>guide.left_leg_deg=norm_angle(angle_deg(g.lh,stage_pos)),
        PoseJoint::RightKnee=>guide.right_leg_deg=norm_angle(angle_deg(g.rh,stage_pos)),
        PoseJoint::LeftAnkle=>{
            let (upper,lower)=solve_two_bone(
                g.lh,stage_pos,thigh_len,shin_len,guide.left_leg_deg);
            guide.left_leg_deg=upper;guide.left_shin_deg=lower;
        }
        PoseJoint::RightAnkle=>{
            let (upper,lower)=solve_two_bone(
                g.rh,stage_pos,thigh_len,shin_len,guide.right_leg_deg);
            guide.right_leg_deg=upper;guide.right_shin_deg=lower;
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test] fn default_pose_is_bounded(){
        let p=PoseGuide::default();
        assert!((0.0..=1.0).contains(&p.x));
        assert!((0.0..=1.0).contains(&p.y));
        assert!(p.scale>0.0);
        assert_eq!(p.head_scale,1.0);
        assert_eq!(p.body_preset,BodyPreset::Classic);
    }

    #[test] fn old_saved_pose_gets_new_body_defaults(){
        let p:PoseGuide=serde_json::from_str(r#"{"visible":true,"x":0.5,"y":0.5,"scale":1.0,"opacity":0.5,"torso_tilt_deg":0.0,"shoulder_deg":0.0,"hip_deg":0.0,"left_arm_deg":125.0,"right_arm_deg":55.0,"left_forearm_deg":105.0,"right_forearm_deg":75.0,"left_leg_deg":110.0,"right_leg_deg":70.0,"left_shin_deg":95.0,"right_shin_deg":85.0}"#).unwrap();
        assert_eq!(p.head_scale,1.0);
        assert_eq!(p.torso_length,1.0);
        assert_eq!(p.shoulder_width,1.0);
    }

    #[test] fn presets_change_real_body_proportions(){
        let mut p=PoseGuide::default();
        p.apply_body_preset(BodyPreset::Tall);
        assert_eq!(p.body_preset,BodyPreset::Tall);
        assert!(p.torso_length>1.0);
        assert!(p.thigh_length>1.0);
        assert!(p.head_scale<1.0);
        p.apply_body_preset(BodyPreset::Cartoon);
        assert!(p.head_scale>1.2);
        assert!(p.torso_length<1.0);
    }

    #[test] fn wrist_ik_moves_both_arm_segments(){
        let mut p=PoseGuide::default();
        p.apply_body_preset(BodyPreset::Tall);
        let before_upper=p.left_arm_deg;
        let before_lower=p.left_forearm_deg;
        let g=layout(&p,(1000.0,800.0));
        drag_joint(&mut p,PoseJoint::LeftWrist,(1000.0,800.0),
            g.ls+egui::vec2(-60.0,90.0));
        assert_ne!(p.left_arm_deg,before_upper);
        assert_ne!(p.left_forearm_deg,before_lower);
    }

    #[test] fn dragging_left_elbow_changes_only_left_upper_arm(){
        let mut p=PoseGuide::default();
        let before=p.right_arm_deg;
        let g=layout(&p,(1000.0,800.0));
        drag_joint(&mut p,PoseJoint::LeftElbow,(1000.0,800.0),g.ls+egui::vec2(-80.0,0.0));
        assert_ne!(p.left_arm_deg,125.0);
        assert_eq!(p.right_arm_deg,before);
    }
}
