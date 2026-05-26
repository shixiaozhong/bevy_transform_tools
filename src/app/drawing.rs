use bevy::prelude::*;

use crate::mesh::MeshBounds;

use super::{
    GRID_HALF_EXTENT,
    model::{ImportedModel, SelectedModel},
};

pub(super) fn draw_grid_and_selection(
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &Transform)>,
    mut gizmos: Gizmos,
) {
    draw_ground_grid(&mut gizmos);
    draw_selected_model_bounds(&mut gizmos, selected.0, &models);
}

fn draw_selected_model_bounds(
    gizmos: &mut Gizmos,
    selected_id: Option<u32>,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(selected_id) = selected_id else {
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }

        if let Some(bounds) = model.bounds {
            draw_model_aabb(gizmos, bounds, transform);
        }

        break;
    }
}

fn draw_model_aabb(gizmos: &mut Gizmos, bounds: MeshBounds, transform: &Transform) {
    let corners = world_aabb_corners(bounds, transform);
    let color = Color::srgb(1.0, 0.86, 0.18);

    for (start, end) in AABB_EDGES {
        gizmos.line(corners[start], corners[end], color);
    }
}

fn world_aabb_corners(bounds: MeshBounds, transform: &Transform) -> [Vec3; 8] {
    let half = bounds.size * 0.5;
    let local_corners = [
        bounds.center + Vec3::new(-half.x, -half.y, -half.z),
        bounds.center + Vec3::new(half.x, -half.y, -half.z),
        bounds.center + Vec3::new(half.x, half.y, -half.z),
        bounds.center + Vec3::new(-half.x, half.y, -half.z),
        bounds.center + Vec3::new(-half.x, -half.y, half.z),
        bounds.center + Vec3::new(half.x, -half.y, half.z),
        bounds.center + Vec3::new(half.x, half.y, half.z),
        bounds.center + Vec3::new(-half.x, half.y, half.z),
    ];

    let mut min = transform.transform_point(local_corners[0]);
    let mut max = min;
    for corner in local_corners.iter().skip(1) {
        let world = transform.transform_point(*corner);
        min = min.min(world);
        max = max.max(world);
    }

    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
}

fn draw_ground_grid(gizmos: &mut Gizmos) {
    let size = GRID_HALF_EXTENT;
    let color = Color::srgba(0.48, 0.52, 0.58, 0.32);
    let axis_color = Color::srgba(0.82, 0.84, 0.88, 0.5);

    for i in -size..=size {
        let i = i as f32;
        let c = if i.abs() < f32::EPSILON {
            axis_color
        } else {
            color
        };
        gizmos.line(
            Vec3::new(i, 0.0, -size as f32),
            Vec3::new(i, 0.0, size as f32),
            c,
        );
        gizmos.line(
            Vec3::new(-size as f32, 0.0, i),
            Vec3::new(size as f32, 0.0, i),
            c,
        );
    }
}

const AABB_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];
