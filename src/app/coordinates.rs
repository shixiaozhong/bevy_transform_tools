use bevy::prelude::*;

pub(super) fn print_axis_to_world(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, axis.z, -axis.y)
}

pub(super) fn world_axis_to_print(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, -axis.z, axis.y)
}

pub(super) fn print_axis_colors() -> [(Vec3, Color); 3] {
    [
        (Vec3::X, axis_x_color()),
        (Vec3::Y, axis_y_color()),
        (Vec3::Z, axis_z_color()),
    ]
}

pub(super) fn axis_x_color() -> Color {
    Color::srgb(204.0 / 255.0, 105.0 / 255.0, 105.0 / 255.0)
}

pub(super) fn axis_y_color() -> Color {
    Color::srgb(111.0 / 255.0, 194.0 / 255.0, 119.0 / 255.0)
}

pub(super) fn axis_z_color() -> Color {
    Color::srgb(123.0 / 255.0, 130.0 / 255.0, 204.0 / 255.0)
}

pub(super) fn print_rotation_degrees_to_world(rotation_degrees: [f32; 3]) -> Quat {
    let print_rotation = Quat::from_euler(
        EulerRot::XYZ,
        rotation_degrees[0].to_radians(),
        rotation_degrees[1].to_radians(),
        rotation_degrees[2].to_radians(),
    );
    let world_from_print = print_rotation_basis();
    let print_from_world = world_from_print.transpose();
    Quat::from_mat3(&(world_from_print * Mat3::from_quat(print_rotation) * print_from_world))
        .normalize()
}

pub(super) fn world_rotation_to_print_degrees(rotation: Quat) -> [f32; 3] {
    let world_from_print = print_rotation_basis();
    let print_from_world = world_from_print.transpose();
    let print_rotation = Quat::from_mat3(
        &(print_from_world * Mat3::from_quat(rotation.normalize()) * world_from_print),
    )
    .normalize();
    let (x, y, z) = print_rotation.to_euler(EulerRot::XYZ);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

fn print_rotation_basis() -> Mat3 {
    Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)
}
