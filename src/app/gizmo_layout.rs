use bevy::prelude::*;

use super::coordinates::{axis_x_color, axis_y_color, axis_z_color, print_axis_to_world};

#[derive(Clone, Copy)]
pub(super) enum ScaleDragMode {
    Uniform,
    Axis { component: usize },
}

#[derive(Clone, Copy)]
pub(super) struct ScaleHandleLayout {
    pub(super) position: Vec3,
    pub(super) color: Color,
    pub(super) mode: ScaleDragMode,
    pub(super) drag_axis: Vec3,
}

#[derive(Clone, Copy)]
pub(super) struct RotationHandleLayout {
    pub(super) radial: Vec3,
    pub(super) tangent: Vec3,
    pub(super) center: Vec3,
    pub(super) start: Vec3,
    pub(super) end: Vec3,
}

pub(super) fn move_gizmo_length(size: Vec3) -> f32 {
    size.max_element().abs().max(2.2) * 0.7
}

pub(super) fn rotate_gizmo_radius(size: Vec3) -> f32 {
    move_gizmo_length(size) * 0.86
}

pub(super) fn scale_handle_layout(bounds: (Vec3, Vec3)) -> [ScaleHandleLayout; 9] {
    let (min, max) = bounds;
    let center = (min + max) * 0.5;
    let bottom_y = min.y;
    let top_y = max.y;
    let red = axis_x_color();
    let green = axis_y_color();
    let blue = axis_z_color();
    let cyan = Color::srgb(0.0, 0.78, 0.82);

    [
        ScaleHandleLayout {
            position: Vec3::new(min.x, bottom_y, min.z),
            color: cyan,
            mode: ScaleDragMode::Uniform,
            drag_axis: (Vec3::new(min.x, bottom_y, min.z) - center).normalize_or_zero(),
        },
        ScaleHandleLayout {
            position: Vec3::new(max.x, bottom_y, min.z),
            color: cyan,
            mode: ScaleDragMode::Uniform,
            drag_axis: (Vec3::new(max.x, bottom_y, min.z) - center).normalize_or_zero(),
        },
        ScaleHandleLayout {
            position: Vec3::new(max.x, bottom_y, max.z),
            color: cyan,
            mode: ScaleDragMode::Uniform,
            drag_axis: (Vec3::new(max.x, bottom_y, max.z) - center).normalize_or_zero(),
        },
        ScaleHandleLayout {
            position: Vec3::new(min.x, bottom_y, max.z),
            color: cyan,
            mode: ScaleDragMode::Uniform,
            drag_axis: (Vec3::new(min.x, bottom_y, max.z) - center).normalize_or_zero(),
        },
        ScaleHandleLayout {
            position: Vec3::new(min.x, bottom_y, center.z),
            color: red,
            mode: ScaleDragMode::Axis { component: 0 },
            drag_axis: Vec3::NEG_X,
        },
        ScaleHandleLayout {
            position: Vec3::new(max.x, bottom_y, center.z),
            color: red,
            mode: ScaleDragMode::Axis { component: 0 },
            drag_axis: Vec3::X,
        },
        ScaleHandleLayout {
            position: Vec3::new(center.x, bottom_y, min.z),
            color: green,
            mode: ScaleDragMode::Axis { component: 2 },
            drag_axis: Vec3::NEG_Z,
        },
        ScaleHandleLayout {
            position: Vec3::new(center.x, bottom_y, max.z),
            color: green,
            mode: ScaleDragMode::Axis { component: 2 },
            drag_axis: Vec3::Z,
        },
        ScaleHandleLayout {
            position: Vec3::new(center.x, top_y, center.z),
            color: blue,
            mode: ScaleDragMode::Axis { component: 1 },
            drag_axis: Vec3::Y,
        },
    ]
}

pub(super) fn rotation_handle_layout(
    origin: Vec3,
    radius: f32,
    print_axis: Vec3,
) -> RotationHandleLayout {
    let axis = print_axis_to_world(print_axis);
    let radial = print_axis_to_world(rotation_handle_radial(print_axis));
    let tangent = axis.cross(radial).normalize_or_zero();
    let center = origin + radial * radius;
    let half_length = radius * 0.13;
    let start = center - tangent * half_length;
    let end = center + tangent * half_length;

    RotationHandleLayout {
        radial,
        tangent,
        center,
        start,
        end,
    }
}

pub(super) fn rotation_handle_radial(axis: Vec3) -> Vec3 {
    if axis == Vec3::X {
        Vec3::NEG_Y
    } else if axis == Vec3::Y {
        Vec3::Z
    } else {
        Vec3::X
    }
}

pub(super) fn world_axis_to_rotation_print_axis(axis: Vec3) -> Option<Vec3> {
    if axis.abs_diff_eq(print_axis_to_world(Vec3::X), f32::EPSILON) {
        Some(Vec3::X)
    } else if axis.abs_diff_eq(print_axis_to_world(Vec3::Y), f32::EPSILON) {
        Some(Vec3::Y)
    } else if axis.abs_diff_eq(print_axis_to_world(Vec3::Z), f32::EPSILON) {
        Some(Vec3::Z)
    } else {
        None
    }
}

pub(super) fn rotation_axis_label(axis: Vec3) -> &'static str {
    if axis == Vec3::X {
        "X"
    } else if axis == Vec3::Y {
        "Y"
    } else {
        "Z"
    }
}

pub(super) fn normalized_degrees(angle: f32) -> f32 {
    angle.to_degrees().rem_euclid(360.0)
}
