use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};

use super::{ORIENTATION_AXIS_LABEL_SIZE, ORIENTATION_FACE_INSET, OrientationBlockSeed};
use crate::app::coordinates::print_axis_to_world;

pub(super) fn orientation_block_mesh(block: &OrientationBlockSeed) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();

    for axis in 0..3 {
        let step = block.direction[axis];
        if step != 0.0 {
            add_orientation_surface_quad(
                axis,
                step.signum(),
                block.direction,
                &mut positions,
                &mut normals,
                &mut uvs,
            );
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

pub(super) fn orientation_axis_label_mesh() -> Mesh {
    let half = ORIENTATION_AXIS_LABEL_SIZE * 0.5;
    let positions = vec![
        [-half, -half, 0.0],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 6];
    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn add_orientation_surface_quad(
    axis: usize,
    sign: f32,
    direction: Vec3,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
) {
    let plane = sign * 0.5;
    let mut normal = Vec3::ZERO;
    normal[axis] = sign;
    let world_normal = print_axis_to_world(normal).to_array();
    let (right, up) = orientation_face_texture_basis(normal);
    let (right_axis, right_sign) = orientation_signed_axis(right);
    let (up_axis, up_sign) = orientation_signed_axis(up);
    let (right_min, right_max) = orientation_axis_range(direction[right_axis]);
    let (up_min, up_max) = orientation_axis_range(direction[up_axis]);
    let left_coord = orientation_signed_coord(right_min, right_max, -right_sign);
    let right_coord = orientation_signed_coord(right_min, right_max, right_sign);
    let down_coord = orientation_signed_coord(up_min, up_max, -up_sign);
    let up_coord = orientation_signed_coord(up_min, up_max, up_sign);

    let quad_positions = [
        (
            orientation_surface_position(axis, plane, right_axis, left_coord, up_axis, down_coord),
            [0.0, 1.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, right_coord, up_axis, down_coord),
            [1.0, 1.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, right_coord, up_axis, up_coord),
            [1.0, 0.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, left_coord, up_axis, up_coord),
            [0.0, 0.0],
        ),
    ];

    for triangle_index in [0, 1, 2, 0, 2, 3] {
        let (position, uv) = quad_positions[triangle_index];
        positions.push(print_axis_to_world(position).to_array());
        normals.push(world_normal);
        uvs.push(uv);
    }
}

fn orientation_face_texture_basis(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.z != 0.0 {
        Vec3::Y * normal.z.signum()
    } else {
        Vec3::Z
    };
    let right = (-normal).cross(up);

    (right, up)
}

fn orientation_signed_axis(axis: Vec3) -> (usize, f32) {
    if axis.x != 0.0 {
        (0, axis.x.signum())
    } else if axis.y != 0.0 {
        (1, axis.y.signum())
    } else {
        (2, axis.z.signum())
    }
}

fn orientation_signed_coord(min: f32, max: f32, sign: f32) -> f32 {
    if sign > 0.0 { max } else { min }
}

fn orientation_surface_position(
    plane_axis: usize,
    plane: f32,
    right_axis: usize,
    right_coord: f32,
    up_axis: usize,
    up_coord: f32,
) -> Vec3 {
    let mut position = Vec3::ZERO;
    position[plane_axis] = plane;
    position[right_axis] = right_coord;
    position[up_axis] = up_coord;
    position
}

pub(super) fn orientation_axis_range(step: f32) -> (f32, f32) {
    if step > 0.0 {
        (ORIENTATION_FACE_INSET, 0.5)
    } else if step < 0.0 {
        (-0.5, -ORIENTATION_FACE_INSET)
    } else {
        (-ORIENTATION_FACE_INSET, ORIENTATION_FACE_INSET)
    }
}
