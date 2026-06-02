use bevy::prelude::*;

mod move_gizmo;
mod rotate_gizmo;
mod scale_gizmo;

use crate::state;

use super::{
    camera::OrbitCamera,
    coordinates::print_axis_to_world,
    gizmo_layout::{ScaleDragMode, move_gizmo_length, rotate_gizmo_radius},
    model::{ImportedModel, SelectedModel, selected_world_bounds},
    tool::ActiveTool,
};
use move_gizmo::pick_move_gizmo_axis;
use rotate_gizmo::{
    cursor_rotation_vector, hovered_rotate_gizmo_handle_axis, pick_rotate_gizmo_axis,
    signed_rotation_angle,
};
use scale_gizmo::pick_scale_gizmo_handle;

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
        match &self.active {
            Some(GizmoDragState {
                mode:
                    GizmoDragMode::Rotate {
                        axis, angle_delta, ..
                    },
                ..
            }) => Some((*axis, *angle_delta)),
            _ => None,
        }
    }

    pub(super) fn active_scale(&self) -> Option<Option<usize>> {
        match &self.active {
            Some(GizmoDragState {
                mode: GizmoDragMode::Scale { mode, .. },
                ..
            }) => Some(match *mode {
                ScaleDragMode::Uniform => None,
                ScaleDragMode::Axis { component } => Some(component),
            }),
            _ => None,
        }
    }

    fn suppress_next_clear_click(&mut self) {
        self.suppress_clear_click_frames = 2;
    }

    pub(super) fn take_suppressed_clear_click(&mut self) -> bool {
        let suppress = self.suppress_clear_click_frames > 0;
        self.suppress_clear_click_frames = 0;
        suppress
    }

    fn decay_clear_click_suppression(&mut self) {
        self.suppress_clear_click_frames = self.suppress_clear_click_frames.saturating_sub(1);
    }
}

#[derive(Clone)]
struct GizmoDragState {
    mode: GizmoDragMode,
}

#[derive(Clone)]
enum GizmoDragMode {
    Move {
        axis: Vec3,
        screen_axis: Vec2,
        start_cursor: Vec2,
        start_translations: Vec<(u32, Vec3)>,
        units_per_pixel: f32,
    },
    Rotate {
        axis: Vec3,
        origin: Vec3,
        start_vector: Vec3,
        start_transforms: Vec<(u32, Transform)>,
        angle_delta: f32,
    },
    Scale {
        mode: ScaleDragMode,
        screen_axis: Vec2,
        start_cursor: Vec2,
        start_transforms: Vec<(u32, Transform)>,
        origin: Vec3,
    },
}

pub(super) fn begin_gizmo_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    models: Query<(&ImportedModel, &Transform)>,
    mut drag: ResMut<GizmoDrag>,
) {
    if (!active_tool.is_move() && !active_tool.is_rotate() && !active_tool.is_scale())
        || !buttons.just_pressed(MouseButton::Left)
        || drag.active.is_some()
    {
        return;
    }

    if selected.primary().is_none() {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let Some((min, max)) = selected_world_bounds(&selected, &models) else {
        return;
    };
    let origin = (min + max) * 0.5;
    let size = max - min;
    let start_transforms = selected_start_transforms(&selected, &models);
    let (camera, camera_transform) = *camera;

    if active_tool.is_move() {
        let length = move_gizmo_length(size);
        if let Some(active) = pick_move_gizmo_axis(
            cursor,
            camera,
            camera_transform,
            origin,
            length,
            start_transforms
                .iter()
                .map(|(id, transform)| (*id, transform.translation))
                .collect(),
        ) {
            drag.active = Some(active);
        }
    } else if active_tool.is_rotate() {
        let radius = rotate_gizmo_radius(size);
        if let Some(active) = pick_rotate_gizmo_axis(
            cursor,
            camera,
            camera_transform,
            origin,
            radius,
            start_transforms,
        ) {
            drag.active = Some(active);
        }
    } else if active_tool.is_scale()
        && let Some(active) = pick_scale_gizmo_handle(
            cursor,
            camera,
            camera_transform,
            (min, max),
            start_transforms,
        )
    {
        drag.active = Some(active);
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

    if let Some(axis) = state::rotation_focus_axis() {
        let print_axis = match axis {
            0 => Vec3::X,
            1 => Vec3::Y,
            2 => Vec3::Z,
            _ => return,
        };
        hover.axis = Some(print_axis_to_world(print_axis));
        return;
    }

    if selected.primary().is_none() {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let Some((min, max)) = selected_world_bounds(&selected, &models) else {
        return;
    };
    let origin = (min + max) * 0.5;
    let radius = rotate_gizmo_radius(max - min);
    let (camera, camera_transform) = *camera;
    if let Some((axis, angle)) =
        hovered_rotate_gizmo_handle_axis(cursor, camera, camera_transform, origin, radius)
    {
        hover.axis = Some(axis);
        hover.angle = angle;
    }
}

pub(super) fn update_gizmo_drag(
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
            start_translations,
            units_per_pixel,
        } => {
            let offset = (cursor - *start_cursor).dot(*screen_axis) * *units_per_pixel;
            for (model, mut transform) in &mut models {
                if let Some((_, start_translation)) =
                    start_translations.iter().find(|(id, _)| *id == model.id)
                {
                    transform.translation = *start_translation + *axis * offset;
                }
            }
        }
        GizmoDragMode::Rotate {
            axis,
            origin,
            start_vector,
            start_transforms,
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
            let delta = Quat::from_axis_angle(*axis, angle);

            for (model, mut transform) in &mut models {
                if let Some((_, start_transform)) =
                    start_transforms.iter().find(|(id, _)| *id == model.id)
                {
                    transform.translation =
                        *origin + delta * (start_transform.translation - *origin);
                    transform.rotation = delta * start_transform.rotation;
                }
            }
        }
        GizmoDragMode::Scale {
            mode,
            screen_axis,
            start_cursor,
            start_transforms,
            origin,
        } => {
            let delta = (cursor - *start_cursor).dot(*screen_axis);
            let factor = (1.0 + delta * 0.01).clamp(0.05, 20.0);
            let scale_factor = match *mode {
                ScaleDragMode::Uniform => Vec3::splat(factor),
                ScaleDragMode::Axis { component } => {
                    let mut scale_factor = Vec3::ONE;
                    scale_factor[component] = factor;
                    scale_factor
                }
            };
            for (model, mut transform) in &mut models {
                if let Some((_, start_transform)) =
                    start_transforms.iter().find(|(id, _)| *id == model.id)
                {
                    transform.translation =
                        *origin + scale_factor * (start_transform.translation - *origin);
                    transform.scale = start_transform.scale * scale_factor;
                }
            }
        }
    }
}

fn selected_start_transforms(
    selected: &SelectedModel,
    models: &Query<(&ImportedModel, &Transform)>,
) -> Vec<(u32, Transform)> {
    models
        .iter()
        .filter(|(model, _)| selected.contains(model.id))
        .map(|(model, transform)| (model.id, *transform))
        .collect()
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
