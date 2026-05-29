use bevy::{
    picking::pointer::{PointerId, PointerInteraction, PointerMap},
    prelude::*,
};

use crate::state::{self, ToolModeSpec};

use super::{
    camera::{OrbitCamera, OrbitDrag},
    drawing::{OrientationInteraction, OrientationViewTarget},
    model::{ImportedModel, ModelDrag, ModelDragState, SelectedModel, model_visual_center},
    tool::ActiveTool,
};

#[derive(Resource, Default)]
pub(super) struct GizmoDrag {
    active: Option<GizmoDragState>,
    suppress_clear_click_frames: u8,
}

#[derive(Resource, Default)]
pub(super) struct RotateGizmoHover {
    pub(super) axis: Option<Vec3>,
    pub(super) angle: f32,
}

impl GizmoDrag {
    pub(super) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(super) fn active_rotation(&self) -> Option<(Vec3, f32)> {
        match self.active {
            Some(GizmoDragState {
                mode:
                    GizmoDragMode::Rotate {
                        axis, angle_delta, ..
                    },
                ..
            }) => Some((axis, angle_delta)),
            _ => None,
        }
    }

    fn suppress_next_clear_click(&mut self) {
        self.suppress_clear_click_frames = 2;
    }

    fn take_suppressed_clear_click(&mut self) -> bool {
        let suppress = self.suppress_clear_click_frames > 0;
        self.suppress_clear_click_frames = 0;
        suppress
    }

    fn decay_clear_click_suppression(&mut self) {
        self.suppress_clear_click_frames = self.suppress_clear_click_frames.saturating_sub(1);
    }
}

#[derive(Clone, Copy)]
struct GizmoDragState {
    id: u32,
    mode: GizmoDragMode,
}

#[derive(Clone, Copy)]
enum GizmoDragMode {
    Move {
        axis: Vec3,
        screen_axis: Vec2,
        start_cursor: Vec2,
        start_translation: Vec3,
        units_per_pixel: f32,
    },
    Rotate {
        axis: Vec3,
        origin: Vec3,
        start_vector: Vec3,
        start_rotation: Quat,
        angle_delta: f32,
    },
}

pub(super) fn select_model_on_click(
    click: On<Pointer<Click>>,
    models: Query<&ImportedModel>,
    mut selected: ResMut<SelectedModel>,
) {
    let Ok(model) = models.get(click.entity) else {
        return;
    };

    selected.0 = Some(model.id);
    state::remember_selection(Some(model.id));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clear_selection_on_non_model_click(
    click: On<Pointer<Click>>,
    models: Query<&ImportedModel>,
    orientation_targets: Query<(), With<OrientationViewTarget>>,
    pointer_map: Res<PointerMap>,
    pointer_interactions: Query<&PointerInteraction>,
    orientation_interaction: Res<OrientationInteraction>,
    mut selected: ResMut<SelectedModel>,
    mut model_drag: ResMut<ModelDrag>,
    mut gizmo_drag: ResMut<GizmoDrag>,
    mut orbit_drag: ResMut<OrbitDrag>,
    mut active_tool: ResMut<ActiveTool>,
) {
    if click.button != PointerButton::Primary
        || orientation_targets.contains(click.entity)
        || orientation_interaction.pointer_over
        || pointer_is_over_model(&models, &pointer_map, &pointer_interactions)
    {
        return;
    }

    if gizmo_drag.is_active()
        || gizmo_drag.take_suppressed_clear_click()
        || orbit_drag.take_suppressed_clear_click()
    {
        return;
    }

    selected.0 = None;
    model_drag.active = None;
    active_tool.set_mode(ToolModeSpec::None);
    state::remember_selection(None);
}

pub(super) fn start_model_drag(
    drag: On<Pointer<DragStart>>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    models: Query<(&ImportedModel, &Transform)>,
    active_tool: Res<ActiveTool>,
    mut model_drag: ResMut<ModelDrag>,
    mut selected: ResMut<SelectedModel>,
) {
    if drag.button != PointerButton::Primary || active_tool.is_move() || active_tool.is_rotate() {
        return;
    }

    let Ok((model, transform)) = models.get(drag.entity) else {
        return;
    };
    let center = model_visual_center(model, transform);
    let (camera, camera_transform) = *camera;
    let plane_y = center.y;
    let Some(pointer_world) = cursor_on_horizontal_plane(
        drag.pointer_location.position,
        camera,
        camera_transform,
        plane_y,
    ) else {
        return;
    };

    selected.0 = Some(model.id);
    state::remember_selection(Some(model.id));
    model_drag.active = Some(ModelDragState {
        id: model.id,
        grab_offset: center - pointer_world,
        plane_y,
    });
}

pub(super) fn begin_move_gizmo_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    models: Query<(&ImportedModel, &Transform)>,
    mut drag: ResMut<GizmoDrag>,
) {
    if (!active_tool.is_move() && !active_tool.is_rotate())
        || !buttons.just_pressed(MouseButton::Left)
        || drag.active.is_some()
    {
        return;
    }

    let Some(selected_id) = selected.0 else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    for (model, transform) in &models {
        if model.id != selected_id {
            continue;
        }

        let origin = model_visual_center(model, transform);
        if active_tool.is_move() {
            let length = move_gizmo_length(model, transform);
            if let Some(active) = pick_move_gizmo_axis(
                selected_id,
                cursor,
                camera,
                camera_transform,
                origin,
                length,
                transform.translation,
            ) {
                drag.active = Some(active);
            }
        } else if active_tool.is_rotate() {
            let radius = rotate_gizmo_radius(model, transform);
            if let Some(active) = pick_rotate_gizmo_axis(
                selected_id,
                cursor,
                camera,
                camera_transform,
                origin,
                radius,
                transform.rotation,
            ) {
                drag.active = Some(active);
            }
        }
        break;
    }
}

pub(super) fn update_rotate_gizmo_hover(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    gizmo_drag: Res<GizmoDrag>,
    models: Query<(&ImportedModel, &Transform)>,
    mut hover: ResMut<RotateGizmoHover>,
) {
    hover.axis = None;
    hover.angle = 0.0;

    if !active_tool.is_rotate() {
        return;
    }

    if let Some((axis, _)) = gizmo_drag.active_rotation() {
        hover.axis = Some(axis);
        return;
    }

    let Some(selected_id) = selected.0 else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    for (model, transform) in &models {
        if model.id != selected_id {
            continue;
        }

        let origin = model_visual_center(model, transform);
        let radius = rotate_gizmo_radius(model, transform);
        if let Some((axis, angle)) =
            hovered_rotate_gizmo_handle_axis(cursor, camera, camera_transform, origin, radius)
        {
            hover.axis = Some(axis);
            hover.angle = angle;
        }
        break;
    }
}

pub(super) fn update_move_gizmo_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    mut drag: ResMut<GizmoDrag>,
    mut models: Query<(&ImportedModel, &mut Transform)>,
) {
    if buttons.just_released(MouseButton::Left) {
        if drag.active.is_some() {
            drag.active = None;
            drag.suppress_next_clear_click();
        }
        return;
    }

    drag.decay_clear_click_suppression();

    let Some(active) = drag.active.as_mut() else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    match &mut active.mode {
        GizmoDragMode::Move {
            axis,
            screen_axis,
            start_cursor,
            start_translation,
            units_per_pixel,
        } => {
            let offset = (cursor - *start_cursor).dot(*screen_axis) * *units_per_pixel;
            for (model, mut transform) in &mut models {
                if model.id == active.id {
                    transform.translation = *start_translation + *axis * offset;
                    break;
                }
            }
        }
        GizmoDragMode::Rotate {
            axis,
            origin,
            start_vector,
            start_rotation,
            angle_delta,
        } => {
            let (camera, camera_transform) = *camera;
            let Some(current_vector) =
                cursor_rotation_vector(cursor, camera, camera_transform, *origin, *axis)
            else {
                return;
            };
            let angle = signed_rotation_angle(*start_vector, current_vector, *axis);
            *angle_delta = angle;

            for (model, mut transform) in &mut models {
                if model.id == active.id {
                    transform.rotation = Quat::from_axis_angle(*axis, angle) * *start_rotation;
                    if let Some(bounds) = model.bounds {
                        transform.translation =
                            *origin - transform.rotation * (bounds.center * transform.scale);
                    } else {
                        transform.translation = *origin;
                    }
                    break;
                }
            }
        }
    }
}

pub(super) fn update_model_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    gizmo_drag: Res<GizmoDrag>,
    mut model_drag: ResMut<ModelDrag>,
    mut models: Query<(&ImportedModel, &mut Transform)>,
) {
    if buttons.just_released(MouseButton::Left) {
        model_drag.active = None;
        return;
    }

    if gizmo_drag.active.is_some() {
        return;
    }

    let Some(active) = model_drag.active else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    let Some(pointer_world) =
        cursor_on_horizontal_plane(cursor, camera, camera_transform, active.plane_y)
    else {
        return;
    };

    for (model, mut transform) in &mut models {
        if model.id == active.id {
            let desired_center = pointer_world + active.grab_offset;
            let current_center = model_visual_center(model, &transform);
            transform.translation += desired_center - current_center;
            break;
        }
    }
}

pub(super) fn pointer_is_over_model(
    models: &Query<&ImportedModel>,
    pointer_map: &PointerMap,
    pointer_interactions: &Query<&PointerInteraction>,
) -> bool {
    pointer_map
        .get_entity(PointerId::Mouse)
        .and_then(|entity| pointer_interactions.get(entity).ok())
        .and_then(PointerInteraction::get_nearest_hit)
        .is_some_and(|(entity, _)| models.contains(*entity))
}

fn cursor_on_horizontal_plane(
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    plane_y: f32,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    ray.plane_intersection_point(Vec3::new(0.0, plane_y, 0.0), InfinitePlane3d::new(Vec3::Y))
}

fn pick_move_gizmo_axis(
    id: u32,
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    length: f32,
    start_translation: Vec3,
) -> Option<GizmoDragState> {
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
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
            id,
            mode: GizmoDragMode::Move {
                axis,
                screen_axis: screen_vector / screen_length,
                start_cursor: cursor,
                start_translation,
                units_per_pixel: length / screen_length,
            },
        };

        if best
            .map(|(best_distance, _)| distance < best_distance)
            .unwrap_or(true)
        {
            best = Some((distance, drag));
        }
    }

    best.map(|(_, drag)| drag)
}

fn pick_rotate_gizmo_axis(
    id: u32,
    cursor: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    origin: Vec3,
    radius: f32,
    start_rotation: Quat,
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
            id,
            mode: GizmoDragMode::Rotate {
                axis,
                origin,
                start_vector,
                start_rotation,
                angle_delta: 0.0,
            },
        };

        if best
            .map(|(best_distance, _)| distance < best_distance)
            .unwrap_or(true)
        {
            best = Some((distance, drag));
        }
    }

    best.map(|(_, drag)| drag)
}

fn hovered_rotate_gizmo_handle_axis(
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
    let (_, _, _, center, start, end) = rotation_handle_geometry(origin, radius, print_axis);
    let start = camera.world_to_viewport(camera_transform, start).ok()?;
    let end = camera.world_to_viewport(camera_transform, end).ok()?;
    let center = camera.world_to_viewport(camera_transform, center).ok()?;
    let distance = distance_to_segment(cursor, start, end).min(cursor.distance(center));

    if distance <= 18.0 {
        Some(distance)
    } else {
        None
    }
}

fn cursor_rotation_vector(
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

fn signed_rotation_angle(start: Vec3, current: Vec3, axis: Vec3) -> f32 {
    axis.dot(start.cross(current)).atan2(start.dot(current))
}

fn rotation_handle_geometry(
    origin: Vec3,
    radius: f32,
    print_axis: Vec3,
) -> (Vec3, Vec3, Vec3, Vec3, Vec3, Vec3) {
    let axis = print_axis_to_world(print_axis);
    let radial = print_axis_to_world(rotation_handle_radial(print_axis));
    let tangent = axis.cross(radial).normalize_or_zero();
    let center = origin + radial * radius;
    let half_length = radius * 0.13;
    let start = center - tangent * half_length;
    let end = center + tangent * half_length;

    (axis, radial, tangent, center, start, end)
}

fn rotation_handle_radial(axis: Vec3) -> Vec3 {
    if axis == Vec3::X {
        Vec3::NEG_Y
    } else if axis == Vec3::Y {
        Vec3::Z
    } else {
        Vec3::X
    }
}

fn print_axis_to_world(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, axis.z, -axis.y)
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared < f32::EPSILON {
        return point.distance(start);
    }

    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn move_gizmo_length(model: &ImportedModel, transform: &Transform) -> f32 {
    let scale = transform.scale.abs().max_element().max(1.0);
    let model_size = model
        .bounds
        .map(|bounds| bounds.size.max_element().abs() * scale * 0.7)
        .unwrap_or(0.0);
    model_size.max(2.2 * scale)
}

fn rotate_gizmo_radius(model: &ImportedModel, transform: &Transform) -> f32 {
    move_gizmo_length(model, transform) * 0.86
}
