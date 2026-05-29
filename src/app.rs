use bevy::{
    camera::visibility::RenderLayers,
    gizmos::prelude::GizmoConfig,
    prelude::*,
    window::{PresentMode, WindowTheme},
};

mod camera;
mod drawing;
mod interaction;
mod model;
mod scene;
mod tool;

use camera::{OrbitDrag, orbit_camera};
use drawing::{
    OrientationGizmos, OrientationInteraction, draw_grid_and_selection, draw_orientation_overlay,
    orient_camera_from_cube_click, update_orientation_camera, update_orientation_interaction,
    update_rotation_angle_label,
};
use interaction::{
    GizmoDrag, RotateGizmoHover, begin_move_gizmo_drag, clear_selection_on_non_model_click,
    update_model_drag, update_move_gizmo_drag, update_rotate_gizmo_hover,
};
use model::{ModelDrag, SelectedModel, apply_model_commands, update_api_state_from_scene};
use scene::setup_scene;
use tool::ActiveTool;

pub(super) const GRID_HALF_EXTENT: i32 = 50;
pub(super) const CAMERA_YAW: f32 = 0.62;
pub(super) const CAMERA_PITCH: f32 = 0.58;
pub(super) const CAMERA_DISTANCE_SCALE: f32 = 1.8;
pub(super) const CAMERA_MAX_DISTANCE_SCALE: f32 = 4.0;
pub(super) const ORBIT_SENSITIVITY: f32 = 0.006;
pub(super) const ORBIT_INERTIA_DAMPING: f32 = 5.5;
pub(super) const ORBIT_VELOCITY_EPSILON: f32 = 0.001;

pub fn run_app() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.035, 0.045)))
        .insert_resource(SelectedModel::default())
        .insert_resource(ModelDrag::default())
        .insert_resource(GizmoDrag::default())
        .insert_resource(RotateGizmoHover::default())
        .insert_resource(OrbitDrag::default())
        .insert_resource(ActiveTool::default())
        .insert_resource(OrientationInteraction::default())
        .insert_gizmo_config::<OrientationGizmos>(
            OrientationGizmos,
            GizmoConfig {
                line: bevy::gizmos::config::GizmoLineConfig {
                    width: 4.0,
                    ..default()
                },
                depth_bias: -0.25,
                render_layers: RenderLayers::layer(1),
                ..default()
            },
        )
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Transform Tools".into(),
                resolution: (1280, 720).into(),
                present_mode: PresentMode::AutoVsync,
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                window_theme: Some(WindowTheme::Dark),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_observer(clear_selection_on_non_model_click)
        .add_observer(orient_camera_from_cube_click)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                apply_model_commands,
                begin_move_gizmo_drag,
                update_rotate_gizmo_hover,
                update_move_gizmo_drag,
                update_model_drag,
                orbit_camera,
                update_api_state_from_scene,
                update_orientation_camera,
                update_orientation_interaction,
                update_rotation_angle_label,
                draw_grid_and_selection,
                draw_orientation_overlay,
            )
                .chain(),
        )
        .run();
}
