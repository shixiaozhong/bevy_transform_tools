use bevy::prelude::*;

use super::{camera::spawn_orbit_camera, drawing::spawn_orientation_overlay};

pub(super) fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
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
    spawn_orientation_overlay(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &asset_server,
    );
}
