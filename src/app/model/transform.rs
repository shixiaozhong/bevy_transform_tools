use bevy::prelude::*;

use crate::mesh::MeshBounds;

use super::{ImportedModel, SelectedModel};

pub(in crate::app) fn model_visual_center(model: &ImportedModel, transform: &Transform) -> Vec3 {
    model
        .bounds
        .map(|bounds| transform.transform_point(bounds.center))
        .unwrap_or(transform.translation)
}

pub(in crate::app) fn model_world_bounds(
    model: &ImportedModel,
    transform: &Transform,
) -> (Vec3, Vec3) {
    let Some(bounds) = model.bounds else {
        let half = Vec3::splat(0.5);
        return (transform.translation - half, transform.translation + half);
    };

    let half = bounds.size * 0.5;
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for x in [-half.x, half.x] {
        for y in [-half.y, half.y] {
            for z in [-half.z, half.z] {
                let point = transform.transform_point(bounds.center + Vec3::new(x, y, z));
                min = min.min(point);
                max = max.max(point);
            }
        }
    }

    (min, max)
}

pub(in crate::app) fn selected_world_bounds(
    selected: &SelectedModel,
    models: &Query<(&ImportedModel, &Transform)>,
) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut found = false;

    for (model, transform) in models {
        if !selected.contains(model.id) {
            continue;
        }

        let (model_min, model_max) = model_world_bounds(model, transform);
        min = min.min(model_min);
        max = max.max(model_max);
        found = true;
    }

    found.then_some((min, max))
}

pub(in crate::app::model) fn center_model_on_platform(
    transform: &mut Transform,
    local_center: Vec3,
) {
    transform.translation -= transform.rotation * (local_center * transform.scale);
}

pub(in crate::app::model) fn center_model_on_origin(
    model: &ImportedModel,
    transform: &mut Transform,
) {
    let center = model_visual_center(model, transform);
    transform.translation.x -= center.x;
    transform.translation.z -= center.z;
}

pub(in crate::app::model) fn drop_model_to_build_plate(
    model: &ImportedModel,
    transform: &mut Transform,
) {
    let Some(bounds) = model.bounds else {
        transform.translation.y = 0.0;
        return;
    };

    let min_y = transformed_bounds_min_y(bounds, transform);
    transform.translation.y -= min_y;
}

pub(in crate::app::model) fn set_model_rotation(
    model: &ImportedModel,
    transform: &mut Transform,
    rotation: Quat,
) {
    let center = model_visual_center(model, transform);
    transform.rotation = rotation.normalize();
    recenter_model_visual_center(model, transform, center);
}

pub(in crate::app::model) fn set_model_scale(
    model: &ImportedModel,
    transform: &mut Transform,
    scale: Vec3,
) {
    let center = model_visual_center(model, transform);
    transform.scale = scale;
    recenter_model_visual_center(model, transform, center);
}

fn recenter_model_visual_center(model: &ImportedModel, transform: &mut Transform, center: Vec3) {
    if let Some(bounds) = model.bounds {
        transform.translation = center - transform.rotation * (bounds.center * transform.scale);
    } else {
        transform.translation = center;
    }
}

fn transformed_bounds_min_y(bounds: MeshBounds, transform: &Transform) -> f32 {
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

    local_corners
        .into_iter()
        .map(|corner| transform.transform_point(corner).y)
        .fold(f32::INFINITY, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model(id: u32, bounds: MeshBounds) -> ImportedModel {
        ImportedModel {
            id,
            name: format!("model-{id}"),
            bounds: Some(bounds),
            triangle_count: 12,
            volume: 1.0,
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn drop_to_build_plate_moves_each_model_independently() {
        let bounds = MeshBounds {
            center: Vec3::ZERO,
            size: Vec3::new(2.0, 4.0, 2.0),
        };
        let first = test_model(1, bounds);
        let second = test_model(2, bounds);
        let mut first_transform = Transform::from_xyz(0.0, 6.0, 0.0);
        let mut second_transform = Transform::from_xyz(0.0, -3.0, 0.0);

        drop_model_to_build_plate(&first, &mut first_transform);
        drop_model_to_build_plate(&second, &mut second_transform);

        assert_close(transformed_bounds_min_y(bounds, &first_transform), 0.0);
        assert_close(transformed_bounds_min_y(bounds, &second_transform), 0.0);
        assert_close(first_transform.translation.y, 2.0);
        assert_close(second_transform.translation.y, 2.0);
    }

    #[test]
    fn center_model_on_origin_moves_visual_center_to_platform_origin() {
        let bounds = MeshBounds {
            center: Vec3::new(1.0, 0.0, -2.0),
            size: Vec3::new(2.0, 2.0, 2.0),
        };
        let model = test_model(1, bounds);
        let mut transform = Transform::from_xyz(5.0, 0.0, 7.0);

        center_model_on_origin(&model, &mut transform);

        let center = model_visual_center(&model, &transform);
        assert_close(center.x, 0.0);
        assert_close(center.z, 0.0);
    }
}
