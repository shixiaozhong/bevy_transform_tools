use bevy::prelude::*;

use crate::mesh::MeshBounds;

use super::{
    GRID_HALF_EXTENT,
    model::{ImportedModel, SelectedModel, model_visual_center},
    tool::ActiveTool,
};

pub(super) fn draw_grid_and_selection(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    models: Query<(&ImportedModel, &Transform)>,
    mut gizmos: Gizmos,
) {
    draw_ground_grid(&mut gizmos);
    draw_selected_model_tools(&mut gizmos, selected.0, active_tool.is_move(), &models);
}

fn draw_selected_model_tools(
    gizmos: &mut Gizmos,
    selected_id: Option<u32>,
    show_move_gizmo: bool,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(selected_id) = selected_id else {
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }

        if !show_move_gizmo && let Some(bounds) = model.bounds {
            draw_model_aabb(gizmos, bounds, transform);
        }
        if show_move_gizmo {
            draw_move_gizmo(gizmos, model, transform);
        }

        break;
    }
}

fn draw_move_gizmo(gizmos: &mut Gizmos, model: &ImportedModel, transform: &Transform) {
    let origin = model_visual_center(model, transform);
    let length = move_gizmo_length(model, transform);
    let x_color = Color::srgb(0.95, 0.05, 0.04);
    let y_color = Color::srgb(0.05, 0.75, 0.12);
    let z_color = Color::srgb(0.04, 0.16, 0.95);

    gizmos.arrow(origin, origin + Vec3::X * length, x_color);
    gizmos.arrow(origin, origin + Vec3::Y * length, y_color);
    gizmos.arrow(origin, origin + Vec3::Z * length, z_color);
    gizmos.sphere(origin, 0.07 * length, Color::srgb(0.08, 0.52, 0.48));
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

fn move_gizmo_length(model: &ImportedModel, transform: &Transform) -> f32 {
    let scale = transform.scale.abs().max_element().max(1.0);
    let model_size = model
        .bounds
        .map(|bounds| bounds.size.max_element().abs() * scale * 0.7)
        .unwrap_or(0.0);
    model_size.max(2.2 * scale)
}
