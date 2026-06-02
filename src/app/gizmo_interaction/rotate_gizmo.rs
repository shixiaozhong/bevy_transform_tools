use bevy::prelude::*;

use super::{GizmoDragMode, GizmoDragState, distance_to_segment};
use crate::app::{
    coordinates::print_axis_to_world,
    gizmo_layout::{rotation_handle_layout, rotation_handle_radial},
};

pub(super) fn pick_rotate_gizmo_axis(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    radius: f32,
    start_transforms: Vec<(u32, Transform)>,
) -> Option<GizmoDragState> {
    let mut best = None::<(f32, GizmoDragState)>;

    for print_axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let Some(distance) = distance_to_projected_rotation_handle(
            cursor,
            camera,
            camera_transform,
            origin,
            radius,
            print_axis,
        ) else {
            continue;
        };
        let axis = print_axis_to_world(print_axis);
        let start_vector = print_axis_to_world(rotation_handle_radial(print_axis));

        let drag = GizmoDragState {
            mode: GizmoDragMode::Rotate {
                axis,
                origin,
                start_vector,
                start_transforms: start_transforms.clone(),
                angle_delta: 0.0,
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

pub(super) fn hovered_rotate_gizmo_handle_axis(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    radius: f32,
) -> Option<(Vec3, f32)> {
    let mut best = None::<(f32, Vec3, f32)>;

    for print_axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let Some(distance) = distance_to_projected_rotation_handle(
            cursor,
            camera,
            camera_transform,
            origin,
            radius,
            print_axis,
        ) else {
            continue;
        };

        let axis = print_axis_to_world(print_axis);
        let angle = cursor_rotation_vector(cursor, camera, camera_transform, origin, axis)
            .map(|current_vector| {
                signed_rotation_angle(
                    print_axis_to_world(rotation_handle_radial(print_axis)),
                    current_vector,
                    axis,
                )
            })
            .unwrap_or(0.0);

        if best
            .map(|(best_distance, _, _)| distance < best_distance)
            .unwrap_or(true)
        {
            best = Some((distance, axis, angle));
        }
    }

    best.map(|(_, axis, angle)| (axis, angle))
}

fn distance_to_projected_rotation_handle(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    radius: f32,
    print_axis: Vec3,
) -> Option<f32> {
    let layout = rotation_handle_layout(origin, radius, print_axis);
    let start = camera
        .world_to_viewport(camera_transform, layout.start)
        .ok()?;
    let end = camera
        .world_to_viewport(camera_transform, layout.end)
        .ok()?;
    let center = camera
        .world_to_viewport(camera_transform, layout.center)
        .ok()?;
    let distance = distance_to_segment(cursor, start, end).min(cursor.distance(center));

    if distance <= 18.0 {
        Some(distance)
    } else {
        None
    }
}

pub(super) fn cursor_rotation_vector(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    axis: Vec3,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let point = ray.plane_intersection_point(origin, InfinitePlane3d::new(axis))?;
    (point - origin).try_normalize()
}

pub(super) fn signed_rotation_angle(start: Vec3, current: Vec3, axis: Vec3) -> f32 {
    axis.dot(start.cross(current)).atan2(start.dot(current))
}
