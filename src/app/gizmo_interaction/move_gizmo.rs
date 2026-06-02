use bevy::prelude::*;

use super::{GizmoDragMode, GizmoDragState, distance_to_segment};
use crate::app::coordinates::print_axis_to_world;

pub(super) fn pick_move_gizmo_axis(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    length: f32,
    start_translations: Vec<(u32, Vec3)>,
) -> Option<GizmoDragState> {
    let axes = [
        print_axis_to_world(Vec3::X),
        print_axis_to_world(Vec3::Y),
        print_axis_to_world(Vec3::Z),
    ];
    let mut best = None::<(f32, GizmoDragState)>;

    for axis in axes {
        let start = camera.world_to_viewport(camera_transform, origin).ok()?;
        let end = camera
            .world_to_viewport(camera_transform, origin + axis * length)
            .ok()?;
        let screen_vector = end - start;
        let screen_length = screen_vector.length();
        if screen_length < 8.0 {
            continue;
        }

        let distance = distance_to_segment(cursor, start, end);
        if distance > 14.0 {
            continue;
        }

        let drag = GizmoDragState {
            mode: GizmoDragMode::Move {
                axis,
                screen_axis: screen_vector / screen_length,
                start_cursor: cursor,
                start_translations: start_translations.clone(),
                units_per_pixel: length / screen_length,
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
