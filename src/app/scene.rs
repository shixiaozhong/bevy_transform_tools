use bevy::prelude::*;

use super::camera::spawn_orbit_camera;

pub(super) fn setup_scene(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            intensity: 500.0,
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(-4.0, 5.0, -3.0),
    ));

    spawn_orbit_camera(&mut commands);
}
