use bevy::prelude::*;

use super::{GizmoDragMode, GizmoDragState};
use crate::app::gizmo_layout::scale_handle_layout;

pub(super) fn pick_scale_gizmo_handle(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    bounds: (Vec3, Vec3),
    start_transforms: Vec<(u32, Transform)>,
) -> Option<GizmoDragState> {
    let mut best = None::<(f32, GizmoDragState)>;

    for handle in scale_handle_layout(bounds) {
        let position = handle.position;
        let projected = camera.world_to_viewport(camera_transform, position).ok()?;
        let distance = cursor.distance(projected);
        if distance > 18.0 {
            continue;
        }

        let axis_end = camera
            .world_to_viewport(camera_transform, position + handle.drag_axis)
            .ok()?;
        let mut screen_axis = axis_end - projected;
        if screen_axis.length_squared() < 16.0 {
            screen_axis = Vec2::Y;
        } else {
            screen_axis = screen_axis.normalize();
        }

        let drag = GizmoDragState {
            mode: GizmoDragMode::Scale {
                mode: handle.mode,
                screen_axis,
                start_cursor: cursor,
                start_transforms: start_transforms.clone(),
                origin: (bounds.0 + bounds.1) * 0.5,
            },
        };

        if best
            .as_ref()
            .map(|(best_distance, _)| distance < *best_distance)
            .unwrap_or(true)
        {
            best = Some((distance, drag));
        }
    }

    best.map(|(_, drag)| drag)
}
