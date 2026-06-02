use bevy::prelude::*;

use crate::state::{self, ModelInfoSnapshot, TransformSnapshot};

use super::{
    ImportedModel, ModelDrag, SelectedModel,
    transform::{model_visual_center, selected_world_bounds},
};
use crate::app::{coordinates::world_rotation_to_print_degrees, gizmo_interaction::GizmoDrag};

pub(in crate::app) fn update_api_state_from_scene(
    selected: Res<SelectedModel>,
    gizmo_drag: Res<GizmoDrag>,
    model_drag: Res<ModelDrag>,
    models: Query<(&ImportedModel, &Transform)>,
) {
    let refresh_model_info = !gizmo_drag.is_active() && model_drag.active.is_none();
    state::sync_api_state(
        selected.ids().iter().copied(),
        selected_world_bounds(&selected, &models)
            .map(|(min, max)| (min.to_array(), max.to_array())),
        models.iter().map(|(model, transform)| {
            (
                model.id,
                TransformSnapshot::from_transform_with_visual_position_and_rotation(
                    transform,
                    model_visual_center(model, transform),
                    world_rotation_to_print_degrees(transform.rotation),
                ),
                refresh_model_info.then(|| model_info_snapshot(model, transform)),
            )
        }),
    );
}

pub(in crate::app::model) fn model_info_snapshot(
    model: &ImportedModel,
    transform: &Transform,
) -> ModelInfoSnapshot {
    let size = model
        .bounds
        .map(|bounds| bounds.size * transform.scale.abs())
        .unwrap_or(Vec3::ZERO);
    let volume = model.volume
        * f64::from(transform.scale.x.abs())
        * f64::from(transform.scale.y.abs())
        * f64::from(transform.scale.z.abs());

    ModelInfoSnapshot {
        name: model.name.clone(),
        size: size.to_array(),
        volume,
        triangle_count: model.triangle_count,
    }
}
