use bevy::prelude::*;

use super::{
    coordinates::{axis_x_color, print_axis_colors, print_axis_to_world},
    gizmo_layout::{
        move_gizmo_length, rotate_gizmo_radius, rotation_handle_layout, rotation_handle_radial,
    },
};

pub(super) fn draw_move_gizmo(gizmos: &mut Gizmos, origin: Vec3, size: Vec3) {
    let length = move_gizmo_length(size);
    for (print_axis, color) in print_axis_colors() {
        let world_axis = print_axis_to_world(print_axis);
        gizmos.arrow(origin, origin + world_axis * length, color);
    }
    gizmos.sphere(origin, 0.07 * length, Color::srgb(0.08, 0.52, 0.48));
}

pub(super) fn draw_scale_gizmo(gizmos: &mut Gizmos, bounds: (Vec3, Vec3)) {
    let (min, max) = bounds;
    let bottom_y = min.y;
    let red = axis_x_color();

    let base_corners = [
        Vec3::new(min.x, bottom_y, min.z),
        Vec3::new(max.x, bottom_y, min.z),
        Vec3::new(max.x, bottom_y, max.z),
        Vec3::new(min.x, bottom_y, max.z),
    ];
    for index in 0..4 {
        gizmos.line(base_corners[index], base_corners[(index + 1) % 4], red);
    }
}

pub(super) fn draw_rotate_gizmo(
    gizmos: &mut Gizmos,
    origin: Vec3,
    size: Vec3,
    active_rotation: Option<(Vec3, f32)>,
    hovered_axis: Option<Vec3>,
) {
    let radius = rotate_gizmo_radius(size);
    let focused_axis = active_rotation.map(|(axis, _)| axis).or(hovered_axis);

    for (print_axis, color) in print_axis_colors() {
        let world_axis = print_axis_to_world(print_axis);
        if focused_axis.is_some_and(|axis| axis != world_axis) {
            continue;
        }

        let is_active = active_rotation.is_some_and(|(active_axis, _)| active_axis == world_axis);
        let is_hovered = hovered_axis == Some(world_axis);
        let ring_color = if is_active {
            Color::srgb(0.96, 0.96, 0.92)
        } else {
            color
        };
        gizmos
            .circle(
                Isometry3d::new(origin, Quat::from_rotation_arc(Vec3::Z, world_axis)),
                radius,
                ring_color,
            )
            .resolution(128);
        draw_rotation_axis_handle(gizmos, origin, print_axis, radius, color);

        if is_active || is_hovered {
            let angle = active_rotation
                .filter(|(axis, _)| *axis == world_axis)
                .map(|(_, angle)| angle)
                .unwrap_or(0.0);
            draw_rotation_ticks(gizmos, origin, print_axis, radius, angle);
            draw_rotation_direction_arrows(gizmos, origin, print_axis, radius, color);
        }
    }

    gizmos.sphere(origin, 0.045 * radius, Color::srgb(0.08, 0.52, 0.48));
}

fn draw_rotation_axis_handle(
    gizmos: &mut Gizmos,
    origin: Vec3,
    print_axis: Vec3,
    radius: f32,
    color: Color,
) {
    let layout = rotation_handle_layout(origin, radius, print_axis);
    if layout.tangent.length_squared() < f32::EPSILON {
        return;
    }

    let tip_length = radius * 0.065;

    gizmos
        .arrow(layout.start, layout.end, color)
        .with_double_end()
        .with_tip_length(tip_length);

    let grip_half = radius * 0.045;
    let grip_width = radius * 0.055;
    let side = layout.radial.cross(layout.tangent).normalize_or_zero();
    let corners = [
        layout.center - layout.tangent * grip_half - side * grip_width,
        layout.center + layout.tangent * grip_half - side * grip_width,
        layout.center + layout.tangent * grip_half + side * grip_width,
        layout.center - layout.tangent * grip_half + side * grip_width,
    ];
    for index in 0..4 {
        gizmos.line(corners[index], corners[(index + 1) % 4], color);
    }
}

fn draw_rotation_ticks(
    gizmos: &mut Gizmos,
    origin: Vec3,
    print_axis: Vec3,
    radius: f32,
    angle_delta: f32,
) {
    let axis = print_axis_to_world(print_axis);
    let u = print_axis_to_world(rotation_handle_radial(print_axis));
    let v = axis.cross(u).normalize_or_zero();
    let color = Color::srgb(0.96, 0.96, 0.92);
    let tick_count = 72;

    for index in 0..tick_count {
        let angle = index as f32 / tick_count as f32 * std::f32::consts::TAU;
        let direction = u * angle.cos() + v * angle.sin();
        let is_major = index % 6 == 0;
        let length = if is_major { 0.12 } else { 0.065 } * radius;
        gizmos.line(
            origin + direction * (radius - length),
            origin + direction * (radius + length * 0.35),
            color,
        );
    }

    let reference = u;
    let current = u * angle_delta.cos() + v * angle_delta.sin();
    gizmos.line(origin, origin + reference * radius, color);
    gizmos.line(origin, origin + current * radius, color);
}

fn draw_rotation_direction_arrows(
    gizmos: &mut Gizmos,
    origin: Vec3,
    print_axis: Vec3,
    radius: f32,
    color: Color,
) {
    let axis = print_axis_to_world(print_axis);
    let u = print_axis_to_world(rotation_handle_radial(print_axis));
    let v = axis.cross(u).normalize_or_zero();
    if v.length_squared() < f32::EPSILON {
        return;
    }

    let arc_radius = radius * 0.94;
    for (start_angle, end_angle) in [(-0.85, 0.42), (2.30, 3.57)] {
        draw_rotation_direction_arc(
            gizmos,
            origin,
            u,
            v,
            arc_radius,
            start_angle,
            end_angle,
            color,
        );
    }
}

fn draw_rotation_direction_arc(
    gizmos: &mut Gizmos,
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color,
) {
    let segments = 20;
    let mut previous = origin + rotation_arc_direction(u, v, start_angle) * radius;
    for index in 1..=segments {
        let t = index as f32 / segments as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let current = origin + rotation_arc_direction(u, v, angle) * radius;
        gizmos.line(previous, current, color);
        previous = current;
    }

    let tip = origin + rotation_arc_direction(u, v, end_angle) * radius;
    let tangent = rotation_arc_tangent(u, v, end_angle);
    let arrow_start = tip - tangent * radius * 0.15;
    gizmos
        .arrow(arrow_start, tip, color)
        .with_tip_length(radius * 0.065);
}

fn rotation_arc_direction(u: Vec3, v: Vec3, angle: f32) -> Vec3 {
    u * angle.cos() + v * angle.sin()
}

fn rotation_arc_tangent(u: Vec3, v: Vec3, angle: f32) -> Vec3 {
    (-u * angle.sin() + v * angle.cos()).normalize_or_zero()
}
