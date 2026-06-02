use bevy::prelude::*;

use crate::mesh::MeshBounds;

use super::{
    GRID_HALF_EXTENT,
    camera::OrbitCamera,
    coordinates::print_axis_to_world,
    gizmo_draw::{draw_move_gizmo, draw_rotate_gizmo, draw_scale_gizmo},
    gizmo_interaction::{GizmoDrag, RotateGizmoHover},
    gizmo_layout::{
        move_gizmo_length, normalized_degrees, rotate_gizmo_radius, rotation_axis_label,
        rotation_handle_radial, scale_handle_layout, world_axis_to_rotation_print_axis,
    },
    model::{ImportedModel, SelectedModel, model_visual_center, selected_world_bounds},
    tool::ActiveTool,
};

const ROTATION_ANGLE_LABEL_OFFSET: Vec2 = Vec2::new(12.0, 10.0);
const SCALE_HANDLE_COUNT: usize = 9;

#[derive(Component)]
pub(super) struct RotationAngleLabel;

#[derive(Component)]
pub(super) struct RotationAngleText;

#[derive(Component)]
pub(super) struct ScaleHandleVisual {
    index: usize,
}

pub(super) fn spawn_drawing_overlays(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    spawn_rotation_angle_label(commands);
    spawn_scale_handle_visuals(commands, meshes, materials);
}

fn spawn_scale_handle_visuals(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for index in 0..SCALE_HANDLE_COUNT {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.78, 0.82),
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(0.001)),
            Visibility::Hidden,
            ScaleHandleVisual { index },
            Name::new(format!("Scale Handle Visual {index}")),
        ));
    }
}

fn spawn_rotation_angle_label(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.07, 0.88)),
        RotationAngleLabel,
        children![(
            Text::new("X: 0.00"),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.94)),
            RotationAngleText,
        )],
    ));
}

pub(super) fn draw_grid_and_selection(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    gizmo_drag: Res<GizmoDrag>,
    rotate_hover: Res<RotateGizmoHover>,
    models: Query<(&ImportedModel, &Transform)>,
    mut scale_handles: Query<
        (
            &ScaleHandleVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<ImportedModel>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmos: Gizmos,
) {
    draw_ground_grid(&mut gizmos);
    update_scale_handle_visuals(
        &selected,
        &active_tool,
        &models,
        &mut scale_handles,
        &mut materials,
    );
    draw_selected_model_tools(
        &mut gizmos,
        &selected,
        &active_tool,
        &gizmo_drag,
        &rotate_hover,
        &models,
    );
}

pub(super) fn update_rotation_angle_label(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    gizmo_drag: Res<GizmoDrag>,
    rotate_hover: Res<RotateGizmoHover>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    models: Query<(&ImportedModel, &Transform)>,
    mut label: Single<&mut Node, With<RotationAngleLabel>>,
    mut text: Single<&mut Text, With<RotationAngleText>>,
) {
    if active_tool.is_scale() {
        update_scale_label(
            selected,
            &gizmo_drag,
            camera.into_inner(),
            &models,
            &mut label,
            &mut text,
        );
        return;
    }

    let Some((axis, angle)) = rotation_angle_label_state(&gizmo_drag, &rotate_hover) else {
        label.display = Display::None;
        return;
    };

    if !active_tool.is_rotate() {
        label.display = Display::None;
        return;
    }

    let Some(selected_id) = selected.primary() else {
        label.display = Display::None;
        return;
    };

    let (camera, camera_transform) = *camera;
    for (model, _) in &models {
        if model.id != selected_id {
            continue;
        }

        let Some((min, max)) = selected_world_bounds(&selected, &models) else {
            label.display = Display::None;
            return;
        };
        let origin = (min + max) * 0.5;
        let radius = rotate_gizmo_radius(max - min);
        let Some(print_axis) = world_axis_to_rotation_print_axis(axis) else {
            label.display = Display::None;
            return;
        };
        let handle_position =
            origin + print_axis_to_world(rotation_handle_radial(print_axis)) * radius;
        let Ok(screen_position) = camera.world_to_viewport(camera_transform, handle_position)
        else {
            label.display = Display::None;
            return;
        };

        label.display = Display::Flex;
        label.left = Val::Px(screen_position.x + ROTATION_ANGLE_LABEL_OFFSET.x);
        label.top = Val::Px(screen_position.y + ROTATION_ANGLE_LABEL_OFFSET.y);
        text.0 = format!(
            "{}: {:.2}",
            rotation_axis_label(print_axis),
            normalized_degrees(angle)
        );
        return;
    }

    label.display = Display::None;
}

fn update_scale_label(
    selected: Res<SelectedModel>,
    gizmo_drag: &GizmoDrag,
    (camera, camera_transform): (&Camera, &GlobalTransform),
    models: &Query<(&ImportedModel, &Transform)>,
    label: &mut Node,
    text: &mut Text,
) {
    let Some(component) = gizmo_drag.active_scale() else {
        label.display = Display::None;
        return;
    };
    let Some(selected_id) = selected.primary() else {
        label.display = Display::None;
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }
        let position = selected_world_bounds(&selected, models)
            .map(|(min, max)| (min + max) * 0.5)
            .unwrap_or_else(|| model_visual_center(model, transform));
        let Ok(screen_position) = camera.world_to_viewport(camera_transform, position) else {
            label.display = Display::None;
            return;
        };

        label.display = Display::Flex;
        label.left = Val::Px(screen_position.x + 28.0);
        label.top = Val::Px(screen_position.y + 28.0);
        text.0 = match component {
            Some(0) => format!("X: {:.2}%", transform.scale.x * 100.0),
            Some(2) => format!("Y: {:.2}%", transform.scale.z * 100.0),
            Some(1) => format!("Z: {:.2}%", transform.scale.y * 100.0),
            _ => format!(
                "X: {:.2}%\nY: {:.2}%\nZ: {:.2}%",
                transform.scale.x * 100.0,
                transform.scale.z * 100.0,
                transform.scale.y * 100.0
            ),
        };
        return;
    }

    label.display = Display::None;
}

fn rotation_angle_label_state(
    gizmo_drag: &GizmoDrag,
    rotate_hover: &RotateGizmoHover,
) -> Option<(Vec3, f32)> {
    gizmo_drag
        .active_rotation()
        .or_else(|| rotate_hover.axis.map(|axis| (axis, rotate_hover.angle)))
}

fn draw_selected_model_tools(
    gizmos: &mut Gizmos,
    selected: &SelectedModel,
    active_tool: &ActiveTool,
    gizmo_drag: &GizmoDrag,
    rotate_hover: &RotateGizmoHover,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(primary_id) = selected.primary() else {
        return;
    };
    let Some((selection_min, selection_max)) = selected_world_bounds(selected, models) else {
        return;
    };
    let selection_center = (selection_min + selection_max) * 0.5;
    let selection_size = selection_max - selection_min;
    let show_selection_bounds =
        !active_tool.is_move() && !active_tool.is_rotate() && !active_tool.is_scale();

    if show_selection_bounds && selected.len() > 1 {
        draw_world_aabb(gizmos, selection_min, selection_max);
    }

    for (model, transform) in models {
        if show_selection_bounds
            && selected.len() == 1
            && selected.contains(model.id)
            && let Some(bounds) = model.bounds
        {
            draw_model_aabb(gizmos, bounds, transform);
        }

        if model.id != primary_id {
            continue;
        }

        if active_tool.is_move() {
            draw_move_gizmo(gizmos, selection_center, selection_size);
        }
        if active_tool.is_rotate() {
            draw_rotate_gizmo(
                gizmos,
                selection_center,
                selection_size,
                gizmo_drag.active_rotation(),
                rotate_hover.axis,
            );
        }
        if active_tool.is_scale() {
            draw_scale_gizmo(gizmos, (selection_min, selection_max));
        }
    }
}

fn update_scale_handle_visuals(
    selected: &SelectedModel,
    active_tool: &ActiveTool,
    models: &Query<(&ImportedModel, &Transform)>,
    scale_handles: &mut Query<
        (
            &ScaleHandleVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<ImportedModel>,
    >,
    materials: &mut Assets<StandardMaterial>,
) {
    if !active_tool.is_scale() || selected.primary().is_none() {
        for (_, _, mut visibility, _) in scale_handles {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let Some(bounds) = selected_world_bounds(selected, models) else {
        for (_, _, mut visibility, _) in scale_handles {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let handle_size = move_gizmo_length(bounds.1 - bounds.0) * 0.055;
    let handles = scale_handle_layout(bounds);

    for (handle, mut handle_transform, mut visibility, mut material) in scale_handles {
        let Some(handle_layout) = handles.get(handle.index).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };

        handle_transform.translation = handle_layout.position;
        handle_transform.rotation = Quat::IDENTITY;
        handle_transform.scale = Vec3::splat(handle_size);
        *visibility = Visibility::Visible;

        if let Some(existing) = materials.get_mut(&material.0) {
            existing.base_color = handle_layout.color;
        } else {
            material.0 = materials.add(StandardMaterial {
                base_color: handle_layout.color,
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            });
        }
    }
}

fn draw_model_aabb(gizmos: &mut Gizmos, bounds: MeshBounds, transform: &Transform) {
    let corners = world_aabb_corners(bounds, transform);
    draw_aabb_corners(gizmos, corners);
}

fn draw_world_aabb(gizmos: &mut Gizmos, min: Vec3, max: Vec3) {
    draw_aabb_corners(
        gizmos,
        [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ],
    );
}

fn draw_aabb_corners(gizmos: &mut Gizmos, corners: [Vec3; 8]) {
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
    let color = Color::srgba(0.18, 0.20, 0.21, 0.34);
    let axis_color = Color::srgba(0.12, 0.13, 0.14, 0.58);

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
