use bevy::prelude::*;

use super::{
    OrientationBlockMaterials, OrientationBlockShape, OrientationCamera, OrientationInteraction,
    OrientationViewTarget, mesh::orientation_axis_range,
};
use crate::app::{
    camera::{OrbitCamera, set_orbit_view_direction, sync_orbit_transform},
    coordinates::{print_axis_to_world, world_axis_to_print},
};

pub(in crate::app) fn orient_camera_from_cube_click(
    click: On<Pointer<Click>>,
    targets: Query<&OrientationViewTarget>,
    interaction: Res<OrientationInteraction>,
    camera: Single<(&mut Transform, &mut OrbitCamera), With<OrbitCamera>>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let target_entity = interaction.hovered_entity.unwrap_or(click.entity);
    let Ok(target) = targets.get(target_entity) else {
        return;
    };

    let (mut transform, mut orbit) = camera.into_inner();
    set_orbit_view_direction(&mut orbit, target.direction);
    sync_orbit_transform(&mut transform, &orbit);
}

pub(in crate::app) fn update_orientation_interaction(
    window: Single<&Window>,
    orientation_camera: Single<(&Camera, &GlobalTransform), With<OrientationCamera>>,
    mut interaction: ResMut<OrientationInteraction>,
    mut blocks: Query<(
        Entity,
        &OrientationBlockShape,
        &OrientationBlockMaterials,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let hovered_entity = cursor_hovered_orientation_block(&window, *orientation_camera, &blocks);

    if interaction.hovered_entity == hovered_entity {
        return;
    }

    interaction.hovered_entity = hovered_entity;
    interaction.pointer_over = hovered_entity.is_some();

    for (entity, _, block_materials, mut material) in &mut blocks {
        material.0 = if Some(entity) == hovered_entity {
            block_materials.hover.clone()
        } else {
            block_materials.base.clone()
        };
    }
}

fn cursor_hovered_orientation_block(
    window: &Window,
    (camera, camera_transform): (&Camera, &GlobalTransform),
    blocks: &Query<(
        Entity,
        &OrientationBlockShape,
        &OrientationBlockMaterials,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<Entity> {
    let cursor = window.cursor_position()?;
    if !cursor_is_in_orientation_viewport(window, camera, cursor) {
        return None;
    }

    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let mut best_hit = None::<(f32, Entity)>;

    for (entity, shape, _, _) in blocks {
        if let Some(distance) = ray_orientation_block_distance(&ray, shape.direction)
            && best_hit
                .map(|(best_distance, _)| distance < best_distance)
                .unwrap_or(true)
        {
            best_hit = Some((distance, entity));
        }
    }

    best_hit.map(|(_, entity)| entity)
}

fn cursor_is_in_orientation_viewport(window: &Window, camera: &Camera, cursor: Vec2) -> bool {
    let Some(viewport) = &camera.viewport else {
        return true;
    };

    let scale_factor = window.resolution.scale_factor();
    let cursor = (cursor * scale_factor).round().as_uvec2();
    let min = viewport.physical_position;
    let max = min + viewport.physical_size;

    cursor.x >= min.x && cursor.x <= max.x && cursor.y >= min.y && cursor.y <= max.y
}

fn ray_orientation_block_distance(ray: &Ray3d, direction: Vec3) -> Option<f32> {
    let mut best_distance = None::<f32>;

    for axis in 0..3 {
        let step = direction[axis];
        if step == 0.0 {
            continue;
        }

        let sign = step.signum();
        let mut normal = Vec3::ZERO;
        normal[axis] = sign;
        let normal = print_axis_to_world(normal);

        let mut plane_origin = Vec3::ZERO;
        plane_origin[axis] = sign * 0.5;
        let plane_origin = print_axis_to_world(plane_origin);

        let Some(distance) = ray.intersect_plane(plane_origin, InfinitePlane3d::new(normal)) else {
            continue;
        };
        let point = world_axis_to_print(ray.get_point(distance));

        let u_axis = (axis + 1) % 3;
        let v_axis = (axis + 2) % 3;
        let (u_min, u_max) = orientation_axis_range(direction[u_axis]);
        let (v_min, v_max) = orientation_axis_range(direction[v_axis]);

        if (point[axis] - 0.5 * sign).abs() <= 0.002
            && point[u_axis] >= u_min - 0.002
            && point[u_axis] <= u_max + 0.002
            && point[v_axis] >= v_min - 0.002
            && point[v_axis] <= v_max + 0.002
            && best_distance
                .map(|best_distance| distance < best_distance)
                .unwrap_or(true)
        {
            best_distance = Some(distance);
        }
    }

    best_distance
}
