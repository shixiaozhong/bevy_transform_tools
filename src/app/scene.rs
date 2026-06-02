use bevy::prelude::*;

use super::{
    camera::{OrbitCamera, spawn_orbit_camera},
    drawing::spawn_drawing_overlays,
    orientation::spawn_orientation_overlay,
};

#[derive(Component)]
pub(super) struct CameraFillLight;

pub(super) fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 9_500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-5.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 1_500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(5.0, 5.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 1_700.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::default(),
        CameraFillLight,
    ));

    commands.spawn((
        PointLight {
            intensity: 850.0,
            range: 24.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 4.0),
    ));

    spawn_orbit_camera(&mut commands);
    spawn_orientation_overlay(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &asset_server,
    );
    spawn_drawing_overlays(&mut commands, &mut meshes, &mut materials);
}

pub(super) fn sync_camera_fill_light(
    camera: Single<&GlobalTransform, With<OrbitCamera>>,
    mut lights: Query<&mut Transform, (With<CameraFillLight>, Without<OrbitCamera>)>,
) {
    let camera_transform = camera.compute_transform();
    for mut light_transform in &mut lights {
        *light_transform = camera_transform;
    }
}
