//! Small safe layer operations; called after UI events so egui rendering
//! never holds a mutable borrow of the same layer being manipulated.
use crate::project::AniProject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerAction {
    Delete,
    Duplicate,
    MoveUp,
    MoveDown,
}

pub fn rename_layer(
    project:&mut AniProject,index:usize,new_name:&str,
)->Result<(), &'static str> {
    let name=new_name.trim();
    if name.is_empty() { return Err("Layer name cannot be empty."); }
    if name.chars().count()>80 { return Err("Layer name is too long."); }
    let layer=project.layers.get_mut(index).ok_or("The selected layer no longer exists.")?;
    layer.name=name.to_owned();
    Ok(())
}

pub fn apply_layer_action(
    project: &mut AniProject,
    index: usize,
    action: LayerAction,
) -> Result<usize, &'static str> {
    if index >= project.layers.len() {
        return Err("The selected layer no longer exists.");
    }
    if project.layers[index].is_pose_rig {
        return Err("The pose rig layer is managed by the rig controls.");
    }
    match action {
        LayerAction::Delete => {
            if project.layers.iter().filter(|layer| !layer.is_pose_rig).count() <= 1 {
                return Err("A project must keep at least one layer.");
            }
            if project.layers[index].locked {
                return Err("Unlock the layer before deleting it.");
            }
            project.layers.remove(index);
            Ok(index.min(project.layers.len() - 1))
        }
        LayerAction::Duplicate => {
            let mut copied = project.layers[index].clone();
            copied.name.push_str(" Copy");
            copied.locked = false;
            copied.xp_first_paint_rewarded = false;
            project.layers.insert(index, copied);
            Ok(index)
        }
        LayerAction::MoveUp => {
            if index == 0 {
                return Err("This layer is already on top.");
            }
            project.layers.swap(index, index - 1);
            Ok(index - 1)
        }
        LayerAction::MoveDown => {
            if index + 1 >= project.layers.len() {
                return Err("This layer is already at the bottom.");
            }
            project.layers.swap(index, index + 1);
            Ok(index + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::AniProject;

    #[test]
    fn renaming_layer_preserves_media_and_keyframes_byte_for_byte() {
        let mut project=AniProject::default();
        let media=crate::project::MediaItem{
            path:"C:/art/test.png".into(),media_type:crate::project::MediaType::Image,
            x:5.0,y:7.0,width:320.0,height:180.0,rotation_deg:15.0,
            flip_h:false,flip_v:false
        };
        project.layers[0].keyframes.get_mut(&1).unwrap().media_items.push(media);
        let before=project.layers[0].keyframes.clone();
        rename_layer(&mut project,0,"Poster").unwrap();
        assert_eq!(project.layers[0].name,"Poster");
        assert_eq!(project.layers[0].keyframes,before);
        assert_eq!(project.layers[0].keyframes[&1].media_items[0].path,"C:/art/test.png");
    }

    #[test]
    fn deleting_only_layer_is_rejected() {
        let mut project = AniProject::default();
        assert!(apply_layer_action(&mut project, 0, LayerAction::Delete).is_err());
        assert_eq!(project.layers.len(), 1);
    }

    #[test]
    fn locked_layer_cannot_be_deleted() {
        let mut project = AniProject::default();
        apply_layer_action(&mut project, 0, LayerAction::Duplicate).unwrap();
        project.layers[0].locked = true;
        assert!(apply_layer_action(&mut project, 0, LayerAction::Delete).is_err());
        assert_eq!(project.layers.len(), 2);
    }

    #[test]
    fn duplicating_preserves_content_and_unlocks_copy() {
        let mut project = AniProject::default();
        project.layers[0].locked = true;
        project.layers[0].opacity = 0.35;
        apply_layer_action(&mut project, 0, LayerAction::Duplicate).unwrap();
        assert_eq!(project.layers.len(), 2);
        assert!(!project.layers[0].locked);
        assert!((project.layers[0].opacity - 0.35).abs() < f32::EPSILON);
        assert!(project.layers[0].name.ends_with(" Copy"));
        assert!(project.layers[1].locked);
    }

    #[test]
    fn moving_layers_swaps_order_without_renaming() {
        let mut project = AniProject::default();
        apply_layer_action(&mut project, 0, LayerAction::Duplicate).unwrap();
        project.layers[0].name = "Top".into();
        project.layers[1].name = "Bottom".into();
        let selected = apply_layer_action(&mut project, 0, LayerAction::MoveDown).unwrap();
        assert_eq!(selected, 1);
        assert_eq!(project.layers[0].name, "Bottom");
        assert_eq!(project.layers[1].name, "Top");
    }
}
