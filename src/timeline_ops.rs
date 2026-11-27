//! Timeline editing logic shared by both timeline styles and keyboard shortcuts.
//! Frame numbers are 1-based. A missing key holds the preceding exposure (or
//! evaluates its tween). A blank key explicitly stops the preceding exposure.
use crate::project::{AniProject, FrameData, FrameMarkerKind, TweenMode};

pub const MAX_TIMELINE_FRAMES: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameAction {
    Copy,
    Cut,
    Paste,
    Clear,
    DeleteKeyframe,
    DeleteFrame,
    Keyframe,
    BlankKeyframe,
    Duplicate,
    ExtendHold,
    InsertEmpty,
}

/// Remove one unit of time across all layers. A key can be removed only if
/// its exposure ends here; otherwise retain the pose needed by the next cel.
fn delete_time(project: &mut AniProject, frame: usize) {
    for index in 0..project.layers.len() {
        let layer = &project.layers[index];
        let continued = (frame < project.total_frames
            && layer.keyframes.contains_key(&frame)
            && !layer.keyframes.contains_key(&(frame + 1)))
        .then(|| project.evaluate_layer_at_frame(layer, frame + 1));

        let keys = &mut project.layers[index].keyframes;
        *keys = std::mem::take(keys)
            .into_iter()
            .filter(|(key, _)| *key != frame)
            .map(|(key, data)| (if key > frame { key - 1 } else { key }, data))
            .collect();
        if let Some(data) = continued {
            keys.insert(frame, data);
        }
        keys.entry(1).or_default();
    }
    for track in &mut project.audio_tracks {
        if track.start_frame > frame
            || (track.start_frame == frame && frame == project.total_frames)
        {
            track.start_frame -= 1;
        }
        if track.end_frame >= frame {
            track.end_frame = track.end_frame.saturating_sub(1).max(track.start_frame);
        }
    }
    project.xp_credited_frames.retain_mut(|key| {
        if *key == frame {
            return false;
        }
        if *key > frame {
            *key -= 1;
        }
        true
    });
    project.total_frames -= 1;
}

/// Duplicate inserts after the selected cel. InsertEmpty inserts before it.
/// Only the selected layer's keys move; audio and other layers stay in sync
/// with their original absolute frame numbers.
fn insert_cel(project: &mut AniProject, index: usize, frame: usize, action: FrameAction) {
    let duplicate = action == FrameAction::Duplicate;
    let at = if duplicate { frame + 1 } else { frame };
    let layer = &project.layers[index];
    let inserted = if duplicate {
        let mut data = project.evaluate_layer_at_frame(layer, frame);
        data.timeline_marker = FrameMarkerKind::CopiedPose;
        data
    } else {
        FrameData { timeline_marker: FrameMarkerKind::InsertedEmpty, ..Default::default() }
    };
    // A blank cel in a hold must occupy exactly one frame. If there is an
    // explicit key here already it will move intact and resume the exposure.
    let resume = (!duplicate && !layer.keyframes.contains_key(&at))
        .then(|| project.evaluate_layer_at_frame(layer, frame));

    // Keep the preceding visible tween pose when the inserted key becomes the
    // new endpoint. Authored keys remain editable and the tween is subdivided.
    let prior = at - 1;
    let tween_at_prior = layer
        .keyframes
        .iter()
        .filter(|(key, _)| **key <= prior)
        .max_by_key(|(key, _)| **key)
        .is_some_and(|(&key, data)| {
            key < prior && (data.motion_tween || data.tween_mode != TweenMode::None)
        });
    let previous_pose = (tween_at_prior && layer.keyframes.keys().any(|&key| key > prior))
        .then(|| project.evaluate_layer_at_frame(layer, prior));

    let keys = &mut project.layers[index].keyframes;
    *keys = std::mem::take(keys)
        .into_iter()
        .map(|(key, data)| (if key >= at { key + 1 } else { key }, data))
        .collect();
    keys.insert(at, inserted);
    if let Some(data) = resume {
        keys.insert(at + 1, data);
    }
    if let Some(data) = previous_pose {
        keys.insert(prior, data);
    }
    project.total_frames += 1;
}

pub fn apply_frame_edit(
    project: &mut AniProject,
    layer_index: usize,
    frame: usize,
    action: FrameAction,
) -> Result<(), &'static str> {
    if frame == 0 || (frame > project.total_frames && action != FrameAction::ExtendHold) {
        return Err("Frame is outside the timeline.");
    }
    let layer = project.layers.get(layer_index).ok_or("Layer is missing.")?;
    if action == FrameAction::DeleteFrame {
        if project.total_frames <= 1 {
            return Err("Cannot delete the last frame.");
        }
        if project.layers.iter().any(|layer| layer.locked) {
            return Err("Unlock all layers before deleting time across the timeline.");
        }
        delete_time(project, frame);
        return Ok(());
    }
    if layer.locked {
        return Err("This layer is locked.");
    }
    match action {
        FrameAction::ExtendHold => {
            // A hold is time, not a cloned FrameData. The preceding authored key
            // remains the only source for all newly exposed cells.
            if project.total_frames >= MAX_TIMELINE_FRAMES
                || frame >= MAX_TIMELINE_FRAMES
                || layer.keyframes.keys().any(|&key| key >= MAX_TIMELINE_FRAMES)
            {
                return Err("The timeline has reached its frame limit.");
            }
            let keys = &mut project.layers[layer_index].keyframes;
            *keys = std::mem::take(keys)
                .into_iter()
                .map(|(key, data)| (if key > frame { key + 1 } else { key }, data))
                .collect();
            project.total_frames = project.total_frames.max(frame) + 1;
        }
        FrameAction::Duplicate | FrameAction::InsertEmpty => {
            if project.total_frames >= MAX_TIMELINE_FRAMES
                || layer
                    .keyframes
                    .keys()
                    .any(|&key| key >= MAX_TIMELINE_FRAMES)
            {
                return Err("The timeline has reached its frame limit.");
            }
            insert_cel(project, layer_index, frame, action);
        }
        FrameAction::Clear | FrameAction::BlankKeyframe => {
            project.layers[layer_index]
                .keyframes
                .insert(frame, FrameData { timeline_marker: FrameMarkerKind::Blank, ..Default::default() });
        }
        FrameAction::Keyframe => {
            let mut data = project.evaluate_layer_at_frame(layer, frame);
            data.timeline_marker = FrameMarkerKind::CopiedPose;
            project.layers[layer_index].keyframes.insert(frame, data);
        }
        FrameAction::DeleteKeyframe => {
            let keys = &mut project.layers[layer_index].keyframes;
            if keys.remove(&frame).is_none() {
                return Err("There is no keyframe to delete here.");
            }
            keys.entry(1).or_default();
        }
        FrameAction::DeleteFrame => unreachable!(),
        FrameAction::Copy | FrameAction::Cut | FrameAction::Paste => {
            return Err("Clipboard operations need the editor clipboard.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{KritaBrushPreset, Stroke, StrokePoint};

    #[test]
    fn timeline_insertions_round_trip_through_editor_history() {
        let mut app = crate::StickApp::new(&egui::Context::default());
        app.config.sound_effects_enabled = false;
        app.project.total_frames = 3;
        app.timeline_length = 3;
        app.current_frame = 2;
        app.project.layers[0].keyframes.get_mut(&1).unwrap().label = "held".into();
        for action in [FrameAction::Duplicate, FrameAction::InsertEmpty, FrameAction::ExtendHold] {
            let before = serde_json::to_value(&app.project).unwrap();
            assert!(app.edit_timeline_frame(0, 2, action));
            let after = serde_json::to_value(&app.project).unwrap();
            let inserted_frame = app.current_frame;
            assert_eq!(app.timeline_length, 4);
            assert_eq!(app.undo_stack.len(), 1);
            app.undo();
            assert_eq!(serde_json::to_value(&app.project).unwrap(), before);
            assert_eq!((app.current_frame, app.timeline_length), (2, 3));
            assert!(!app.edit_timeline_frame(0, 0, action));
            app.redo();
            assert_eq!(serde_json::to_value(&app.project).unwrap(), after);
            assert_eq!(
                (app.current_frame, app.timeline_length),
                (inserted_frame, 4)
            );
            app.undo();
        }
    }

    #[test]
    fn clipboard_paste_respects_locks_limits_and_commits_preview_duration() {
        let mut app = crate::StickApp::new(&egui::Context::default());
        app.config.sound_effects_enabled = false;
        app.project.layers[0].keyframes.get_mut(&1).unwrap().label = "copied".into();
        app.copy_frame(0, 1);
        app.project.layers[0].keyframes.get_mut(&1).unwrap().label = "destination".into();
        app.project.layers[0].locked = true;
        app.paste_frame(0, 1);
        app.project.layers[0].locked = false;
        app.paste_frame(0, MAX_TIMELINE_FRAMES + 1);
        assert_eq!(app.project.layers[0].keyframes[&1].label, "destination");
        assert_eq!(app.project.layers[0].keyframes.len(), 1);
        assert!(app.undo_stack.is_empty());

        app.paste_frame(0, 11);
        assert_eq!(app.project.total_frames, 11);
        assert_eq!(app.project.layers[0].keyframes[&11].label, "copied");
        assert_eq!(app.project.layers[0].keyframes.len(), 2);
        app.undo();
        assert_eq!(app.project.total_frames, 1);
        assert_eq!(app.project.layers[0].keyframes.len(), 1);
        app.paste_frame(0, 1);
        assert_eq!(app.project.layers[0].keyframes[&1].label, "copied");
        app.undo();
        assert_eq!(app.project.layers[0].keyframes[&1].label, "destination");
    }

    fn dab() -> Stroke {
        Stroke {
            points: vec![StrokePoint {
                x: 15.0,
                y: 20.0,
                pressure: 1.0,
            }],
            color: [0, 0, 0, 255],
            fill_color: None,
            base_width: 5.0,
            is_vector: false,
            brush_preset: KritaBrushPreset::GPenInk,
            opacity: 1.0,
            brush_texture_id: None,
            tip_shape: crate::project::BrushTipShape::Round,
        }
    }

    #[test]
    fn keyframe_copies_current_visible_content_without_removing_original() {
        let mut p = AniProject::default();
        p.total_frames = 12;
        p.layers[0]
            .keyframes
            .get_mut(&1)
            .unwrap()
            .strokes
            .push(dab());
        apply_frame_edit(&mut p, 0, 6, FrameAction::Keyframe).unwrap();
        assert_eq!(p.layers[0].keyframes[&1].strokes.len(), 1);
        assert_eq!(p.layers[0].keyframes[&6].strokes.len(), 1);
        assert_eq!(p.layers[0].keyframes[&6].timeline_marker, FrameMarkerKind::CopiedPose);
        apply_frame_edit(&mut p, 0, 7, FrameAction::BlankKeyframe).unwrap();
        assert!(p.layers[0].keyframes[&7].strokes.is_empty());
        assert_eq!(p.layers[0].keyframes[&7].timeline_marker, FrameMarkerKind::Blank);
    }

    #[test]
    fn clearing_a_frame_creates_an_empty_keyframe_here_only() {
        let mut p = AniProject::default();
        p.total_frames = 10;
        p.layers[0]
            .keyframes
            .get_mut(&1)
            .unwrap()
            .strokes
            .push(dab());
        apply_frame_edit(&mut p, 0, 5, FrameAction::Clear).unwrap();
        assert_eq!(p.layers[0].keyframes[&1].strokes.len(), 1);
        assert!(p.layers[0].keyframes[&5].strokes.is_empty());
    }

    #[test]
    fn deleting_time_shifts_all_layers_and_preserves_alignment() {
        let mut p = AniProject::default();
        p.total_frames = 10;
        let mut another = p.layers[0].clone();
        another.keyframes.clear();
        another.keyframes.insert(5, FrameData::default());
        p.layers[0].keyframes.insert(5, FrameData::default());
        p.layers.push(another);
        apply_frame_edit(&mut p, 0, 3, FrameAction::DeleteFrame).unwrap();
        assert_eq!(p.total_frames, 9);
        assert!(p.layers.iter().all(|l| l.keyframes.contains_key(&4)));
        assert!(p.layers.iter().all(|l| !l.keyframes.contains_key(&5)));
    }

    #[test]
    fn deleting_first_frame_preserves_the_second_frames_held_drawing() {
        let mut p = AniProject::default();
        p.total_frames = 4;
        p.layers[0]
            .keyframes
            .get_mut(&1)
            .unwrap()
            .strokes
            .push(dab());
        apply_frame_edit(&mut p, 0, 1, FrameAction::DeleteFrame).unwrap();
        assert_eq!(p.total_frames, 3);
        assert_eq!(
            p.layers[0].keyframes[&1].strokes.len(),
            1,
            "deleting frame 1 should not erase artwork held on old frame 2"
        );
    }

    #[test]
    fn locked_layer_rejects_changes_and_global_deletion() {
        let mut p = AniProject::default();
        p.total_frames = 10;
        p.layers[0].locked = true;
        assert!(apply_frame_edit(&mut p, 0, 5, FrameAction::BlankKeyframe).is_err());
        assert!(apply_frame_edit(&mut p, 0, 5, FrameAction::DeleteFrame).is_err());
        assert_eq!(p.total_frames, 10);
        assert_eq!(p.layers[0].keyframes.len(), 1);
    }

    #[test]
    fn a_keyframe_can_be_deleted_without_removing_timeline_time() {
        let mut p = AniProject::default();
        p.total_frames = 10;
        p.layers[0].keyframes.insert(5, FrameData::default());
        apply_frame_edit(&mut p, 0, 5, FrameAction::DeleteKeyframe).unwrap();
        assert_eq!(p.total_frames, 10);
        assert!(!p.layers[0].keyframes.contains_key(&5));
    }

    #[test]
    fn duplicate_held_cel_preserves_all_content_and_moves_the_next_key() {
        let mut p = AniProject::default();
        p.total_frames = 4;
        let mut content: FrameData = serde_json::from_value(serde_json::json!({
            "text_items": [{"content":"held"}], "media_items": [{"path":"sample.png"}],
            "symbol_instances": [{"symbol_name":"bird"}], "label":"original"
        }))
        .unwrap();
        content.strokes.push(dab());
        p.layers[0].keyframes.insert(1, content.clone());
        p.layers[0].keyframes.insert(
            3,
            FrameData {
                label: "next".into(),
                ..Default::default()
            },
        );
        let mut background = p.layers[0].clone();
        background.locked = true;
        p.layers.push(background);
        let unchanged = serde_json::to_value(&p.layers[1]).unwrap();
        apply_frame_edit(&mut p, 0, 2, FrameAction::Duplicate).unwrap();
        assert_eq!(p.total_frames, 5);
        let copied = &p.layers[0].keyframes[&3];
        let mut expected = content.clone();
        expected.timeline_marker = FrameMarkerKind::CopiedPose;
        assert_eq!(copied, &expected);
        assert_eq!(p.layers[0].keyframes[&4].label, "next");
        p.layers[0].keyframes.get_mut(&3).unwrap().strokes.clear();
        assert_eq!(p.layers[0].keyframes[&1].strokes.len(), 1);
        assert_eq!(serde_json::to_value(&p.layers[1]).unwrap(), unchanged);
    }

    #[test]
    fn extend_hold_does_not_clone_and_preserves_other_layers() {
        let mut p = AniProject::default();
        p.total_frames = 4;
        p.layers[0].keyframes.get_mut(&1).unwrap().strokes.push(dab());
        p.layers[0].keyframes.insert(3, FrameData { label: "next".into(), ..Default::default() });
        p.layers.push(p.layers[0].clone());
        let other = serde_json::to_value(&p.layers[1]).unwrap();
        for at in [1, 2, 3] {
            apply_frame_edit(&mut p, 0, at, FrameAction::ExtendHold).unwrap();
        }
        assert_eq!(p.total_frames, 7);
        assert_eq!(p.layers[0].keyframes.len(), 2);
        assert_eq!(p.layers[0].keyframes[&6].label, "next");
        p.layers[0].keyframes.get_mut(&1).unwrap().strokes.clear();
        for at in 1..6 {
            assert!(p.evaluate_layer_at_frame(&p.layers[0], at).strokes.is_empty());
            assert_eq!(p.layers[0].keyframes.contains_key(&at), at == 1);
        }
        assert_eq!(serde_json::to_value(&p.layers[1]).unwrap(), other);
    }

    #[test]
    fn extend_hold_from_preview_and_limits_are_safe() {
        let mut p = AniProject::default();
        apply_frame_edit(&mut p, 0, 11, FrameAction::ExtendHold).unwrap();
        assert_eq!(p.total_frames, 12);
        assert_eq!(p.layers[0].keyframes.len(), 1);
        let before = serde_json::to_value(&p).unwrap();
        assert!(apply_frame_edit(&mut p, 0, 0, FrameAction::ExtendHold).is_err());
        assert!(apply_frame_edit(&mut p, 0, MAX_TIMELINE_FRAMES, FrameAction::ExtendHold).is_err());
        assert_eq!(serde_json::to_value(&p).unwrap(), before);
        p.layers[0].locked = true;
        assert!(apply_frame_edit(&mut p, 0, 1, FrameAction::ExtendHold).is_err());
        assert_eq!(p.total_frames, 12);
    }

    #[test]
    fn inserted_blank_interrupts_a_hold_for_exactly_one_frame() {
        for at in [1, 3, 5] {
            let mut p = AniProject::default();
            p.total_frames = 5;
            p.layers[0]
                .keyframes
                .get_mut(&1)
                .unwrap()
                .strokes
                .push(dab());
            apply_frame_edit(&mut p, 0, at, FrameAction::InsertEmpty).unwrap();
            assert_eq!(p.total_frames, 6);
            for frame in 1..=6 {
                let visible = p.evaluate_layer_at_frame(&p.layers[0], frame);
                assert_eq!(visible.strokes.len(), usize::from(frame != at));
            }
        }
    }

    #[test]
    fn inserted_empty_and_duplicate_have_distinct_persistent_markers() {
        let mut p = AniProject::default();
        p.total_frames = 3;
        apply_frame_edit(&mut p, 0, 2, FrameAction::InsertEmpty).unwrap();
        assert_eq!(p.layers[0].keyframes[&2].timeline_marker, FrameMarkerKind::InsertedEmpty);
        apply_frame_edit(&mut p, 0, 2, FrameAction::Duplicate).unwrap();
        assert_eq!(p.layers[0].keyframes[&3].timeline_marker, FrameMarkerKind::CopiedPose);
    }
    #[test]
    fn duplicate_can_append_and_insertion_failures_are_atomic() {
        let mut p = AniProject::default();
        apply_frame_edit(&mut p, 0, 1, FrameAction::Duplicate).unwrap();
        assert_eq!(p.total_frames, 2);
        assert!(p.layers[0].keyframes.contains_key(&2));
        for action in [FrameAction::Duplicate, FrameAction::InsertEmpty] {
            assert!(apply_frame_edit(&mut p, 0, 0, action).is_err());
            assert!(apply_frame_edit(&mut p, 0, 3, action).is_err());
            p.layers[0].locked = true;
            assert!(apply_frame_edit(&mut p, 0, 1, action).is_err());
            p.layers[0].locked = false;
            p.total_frames = MAX_TIMELINE_FRAMES;
            assert!(apply_frame_edit(&mut p, 0, 1, action).is_err());
            assert_eq!(p.total_frames, MAX_TIMELINE_FRAMES);
            assert_eq!(p.layers[0].keyframes.len(), 2);
            p.total_frames = 2;
        }
    }

    #[test]
    fn duplicate_uses_the_visible_tween_pose() {
        let mut p = AniProject::default();
        p.total_frames = 5;
        for (key, x) in [(1, 0.0), (5, 20.0)] {
            let data = serde_json::from_value(serde_json::json!({
                "symbol_instances":[{"symbol_name":"bird","x":x}],
                "tween_mode":"Classic", "easing":"Linear"
            }))
            .unwrap();
            p.layers[0].keyframes.insert(key, data);
        }
        apply_frame_edit(&mut p, 0, 3, FrameAction::Duplicate).unwrap();
        for (key, x) in [(2, 5.0), (3, 10.0), (4, 10.0), (5, 15.0), (6, 20.0)] {
            assert_eq!(
                p.evaluate_layer_at_frame(&p.layers[0], key)
                    .symbol_instances[0]
                    .x,
                x
            );
        }
    }

    #[test]
    fn delete_time_keeps_mid_span_artwork_and_shifts_global_markers() {
        let mut p = AniProject::default();
        p.total_frames = 7;
        p.layers[0].keyframes.insert(
            3,
            FrameData {
                strokes: vec![dab()],
                ..Default::default()
            },
        );
        p.layers[0].keyframes.insert(7, FrameData::default());
        p.audio_tracks.push(
            serde_json::from_value(serde_json::json!({
                "start_frame": 4, "end_frame": 7
            }))
            .unwrap(),
        );
        p.xp_credited_frames = vec![1, 3, 7];
        apply_frame_edit(&mut p, 0, 3, FrameAction::DeleteFrame).unwrap();
        assert_eq!(p.evaluate_layer_at_frame(&p.layers[0], 3).strokes.len(), 1);
        assert!(p.layers[0].keyframes[&6].strokes.is_empty());
        assert_eq!(p.audio_tracks[0].start_frame, 3);
        assert_eq!(p.audio_tracks[0].end_frame, 6);
        assert_eq!(p.xp_credited_frames, vec![1, 6]);
    }
}
