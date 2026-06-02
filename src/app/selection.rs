use bevy::prelude::*;

use super::model::{ImportedModel, SelectedModel, model_world_bounds};

pub(super) fn selected_bounds_in_scene(
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<(Vec3, Vec3)> {
    bounds_union(models.iter_mut().filter_map(|(_, model, transform, _)| {
        selected
            .contains(model.id)
            .then(|| model_world_bounds(model, &transform))
    }))
}

pub(super) fn selected_center_and_primary_rotation(
    primary_id: u32,
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<(Vec3, Quat)> {
    let (min, max) = selected_bounds_in_scene(selected, models)?;
    let center = group_center((min, max));
    let primary_rotation = models
        .iter_mut()
        .find(|(_, model, _, _)| model.id == primary_id)
        .map(|(_, _, transform, _)| transform.rotation)?;
    Some((center, primary_rotation))
}

pub(super) fn selected_center_and_primary_scale(
    primary_id: u32,
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<(Vec3, Vec3)> {
    let (min, max) = selected_bounds_in_scene(selected, models)?;
    let center = group_center((min, max));
    let primary_scale = models
        .iter_mut()
        .find(|(_, model, _, _)| model.id == primary_id)
        .map(|(_, _, transform, _)| transform.scale)?;
    Some((center, primary_scale))
}

pub(super) fn translate_selected_models(
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    delta: Vec3,
) {
    for (_, model, mut transform, _) in models {
        if selected.contains(model.id) {
            translate_transform(&mut transform, delta);
        }
    }
}

pub(super) fn rotate_selected_models(
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    center: Vec3,
    delta: Quat,
) {
    for (_, model, mut transform, _) in models {
        if selected.contains(model.id) {
            rotate_transform_around(&mut transform, center, delta);
        }
    }
}

pub(super) fn scale_selected_models(
    selected: &SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    center: Vec3,
    factor: Vec3,
) {
    for (_, model, mut transform, _) in models {
        if selected.contains(model.id) {
            scale_transform_around(&mut transform, center, factor);
        }
    }
}

pub(super) fn scale_factor_component(desired: f32, current: f32) -> f32 {
    if current.abs() > 0.0001 {
        desired / current
    } else {
        1.0
    }
}

pub(super) fn bounds_union(bounds: impl IntoIterator<Item = (Vec3, Vec3)>) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut found = false;

    for (model_min, model_max) in bounds {
        min = min.min(model_min);
        max = max.max(model_max);
        found = true;
    }

    found.then_some((min, max))
}

pub(super) fn group_center(bounds: (Vec3, Vec3)) -> Vec3 {
    (bounds.0 + bounds.1) * 0.5
}

pub(super) fn translate_transform(transform: &mut Transform, delta: Vec3) {
    transform.translation += delta;
}

pub(super) fn rotate_transform_around(transform: &mut Transform, center: Vec3, delta: Quat) {
    transform.translation = center + delta * (transform.translation - center);
    transform.rotation = delta * transform.rotation;
}

pub(super) fn scale_transform_around(transform: &mut Transform, center: Vec3, factor: Vec3) {
    transform.translation = center + factor * (transform.translation - center);
    transform.scale *= factor;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            actual.abs_diff_eq(expected, 0.0001),
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn bounds_union_uses_all_selected_model_bounds() {
        let bounds = bounds_union([
            (Vec3::new(-2.0, 0.0, -1.0), Vec3::new(1.0, 4.0, 2.0)),
            (Vec3::new(3.0, -1.0, -4.0), Vec3::new(5.0, 2.0, 1.0)),
        ])
        .unwrap();

        assert_vec3_close(bounds.0, Vec3::new(-2.0, -1.0, -4.0));
        assert_vec3_close(bounds.1, Vec3::new(5.0, 4.0, 2.0));
        assert_vec3_close(group_center(bounds), Vec3::new(1.5, 1.5, -1.0));
    }

    #[test]
    fn translate_transform_applies_delta() {
        let mut transform = Transform::from_xyz(1.0, 2.0, 3.0);

        translate_transform(&mut transform, Vec3::new(4.0, -1.0, 2.0));

        assert_vec3_close(transform.translation, Vec3::new(5.0, 1.0, 5.0));
    }

    #[test]
    fn rotate_transform_uses_group_center() {
        let mut transform = Transform::from_xyz(2.0, 0.0, 0.0);
        let delta = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        rotate_transform_around(&mut transform, Vec3::ZERO, delta);

        assert_vec3_close(transform.translation, Vec3::new(0.0, 0.0, -2.0));
        assert!(transform.rotation.abs_diff_eq(delta, 0.0001));
    }

    #[test]
    fn scale_transform_uses_group_center() {
        let mut transform = Transform::from_xyz(3.0, 1.0, -2.0);

        scale_transform_around(
            &mut transform,
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 3.0, 0.5),
        );

        assert_vec3_close(transform.translation, Vec3::new(5.0, 1.0, -0.5));
        assert_vec3_close(transform.scale, Vec3::new(2.0, 3.0, 0.5));
    }

    #[test]
    fn scale_factor_component_keeps_zero_axis_stable() {
        assert_eq!(scale_factor_component(3.0, 0.0), 1.0);
        assert_eq!(scale_factor_component(6.0, 2.0), 3.0);
    }
}
