use bevy::{
    picking::pointer::{PointerId, PointerInteraction, PointerMap},
    prelude::*,
};

use crate::state::{self, ToolModeSpec};

use super::{
    camera::{OrbitCamera, OrbitDrag},
    cut::CutPreview,
    gizmo_interaction::GizmoDrag,
    model::{ImportedModel, ModelDrag, ModelDragState, SelectedModel, model_visual_center},
    orientation::{OrientationInteraction, OrientationViewTarget},
    tool::ActiveTool,
};

pub(super) fn select_model_on_click(
    click: On<Pointer<Click>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    models: Query<&ImportedModel>,
    mut selected: ResMut<SelectedModel>,
) {
    let Ok(model) = models.get(click.entity) else {
        return;
    };

    if keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        selected.toggle(model.id);
    } else {
        selected.set_single(model.id);
    }
    state::remember_selected_models(selected.ids().iter().copied());
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

    selected.clear();
    model_drag.active = None;
    active_tool.set_mode(ToolModeSpec::None);
    state::remember_selection(None);
    state::remember_active_tool(ToolModeSpec::None);
}

pub(super) fn start_model_drag(
    drag: On<Pointer<DragStart>>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    models: Query<(&ImportedModel, &Transform)>,
    active_tool: Res<ActiveTool>,
    cut_preview: Res<CutPreview>,
    mut model_drag: ResMut<ModelDrag>,
    mut selected: ResMut<SelectedModel>,
) {
    if drag.button != PointerButton::Primary
        || active_tool.is_move()
        || active_tool.is_rotate()
        || active_tool.is_scale()
        || cut_preview.active_plane().is_some()
    {
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

    selected.set_single(model.id);
    state::remember_selection(Some(model.id));
    model_drag.active = Some(ModelDragState {
        id: model.id,
        grab_offset: center - pointer_world,
        plane_y,
    });
}

pub(super) fn update_model_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    gizmo_drag: Res<GizmoDrag>,
    cut_preview: Res<CutPreview>,
    mut model_drag: ResMut<ModelDrag>,
    mut models: Query<(&ImportedModel, &mut Transform)>,
) {
    if buttons.just_released(MouseButton::Left) {
        model_drag.active = None;
        return;
    }

    if gizmo_drag.is_active() || cut_preview.active_plane().is_some() {
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
