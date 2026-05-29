use bevy::{
    ecs::system::SystemParam,
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    picking::pointer::{PointerInteraction, PointerMap},
    prelude::*,
    ui::IsDefaultUiCamera,
};

use super::{
    CAMERA_DISTANCE_SCALE, CAMERA_MAX_DISTANCE_SCALE, CAMERA_PITCH, CAMERA_YAW, GRID_HALF_EXTENT,
    ORBIT_INERTIA_DAMPING, ORBIT_SENSITIVITY, ORBIT_VELOCITY_EPSILON,
    interaction::{GizmoDrag, pointer_is_over_model},
    model::{ImportedModel, ModelDrag},
};

#[derive(Resource, Default)]
pub(super) struct OrbitDrag {
    active: bool,
    suppress_clear_click_frames: u8,
}

impl OrbitDrag {
    pub(super) fn take_suppressed_clear_click(&mut self) -> bool {
        let suppress = self.suppress_clear_click_frames > 0;
        self.suppress_clear_click_frames = 0;
        suppress
    }

    fn suppress_next_clear_click(&mut self) {
        self.suppress_clear_click_frames = 2;
    }

    fn decay_clear_click_suppression(&mut self) {
        self.suppress_clear_click_frames = self.suppress_clear_click_frames.saturating_sub(1);
    }
}

#[derive(Component)]
pub(super) struct OrbitCamera {
    target: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
    yaw_velocity: f32,
    pitch_velocity: f32,
}

#[derive(SystemParam)]
pub(super) struct OrbitInput<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    mouse_motion: Res<'w, AccumulatedMouseMotion>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    pointer_map: Res<'w, PointerMap>,
    pointer_interactions: Query<'w, 's, &'static PointerInteraction>,
}

pub(super) fn spawn_orbit_camera(commands: &mut Commands) {
    let orbit = default_orbit_camera();
    let mut camera_transform = Transform::default();
    apply_orbit_transform(&mut camera_transform, &orbit);
    commands.spawn((
        Camera3d::default(),
        camera_transform,
        orbit,
        IsDefaultUiCamera,
    ));
}

pub(super) fn orbit_camera(
    time: Res<Time>,
    mut input: OrbitInput,
    camera: Single<(&mut Transform, &mut OrbitCamera), With<OrbitCamera>>,
    model_drag: Res<ModelDrag>,
    gizmo_drag: Res<GizmoDrag>,
    mut orbit_drag: ResMut<OrbitDrag>,
    models: Query<&ImportedModel>,
) {
    let (mut transform, mut orbit) = camera.into_inner();
    let is_blocked = model_drag.active.is_some() || gizmo_drag.is_active();

    update_orbit_drag_state(
        &input.buttons,
        is_blocked,
        &mut orbit_drag,
        &models,
        &input.pointer_map,
        &input.pointer_interactions,
    );
    update_orbit_rotation(
        &input.buttons,
        input.mouse_motion.delta,
        time.delta_secs(),
        is_blocked,
        orbit_drag.active,
        &mut orbit,
        &mut orbit_drag,
    );
    update_orbit_zoom(&mut input.mouse_wheel, &mut orbit, &mut orbit_drag);
    apply_orbit_transform(&mut transform, &orbit);
}

fn update_orbit_drag_state(
    buttons: &ButtonInput<MouseButton>,
    is_blocked: bool,
    orbit_drag: &mut OrbitDrag,
    models: &Query<&ImportedModel>,
    pointer_map: &PointerMap,
    pointer_interactions: &Query<&PointerInteraction>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        orbit_drag.active =
            !is_blocked && !pointer_is_over_model(models, pointer_map, pointer_interactions);
    }

    if buttons.just_released(MouseButton::Left) || is_blocked {
        orbit_drag.active = false;
    }
}

fn update_orbit_rotation(
    buttons: &ButtonInput<MouseButton>,
    mouse_delta: Vec2,
    delta_secs: f32,
    is_blocked: bool,
    orbit_drag_active: bool,
    orbit: &mut OrbitCamera,
    orbit_drag: &mut OrbitDrag,
) {
    if is_blocked {
        stop_orbit_inertia(orbit);
    } else if buttons.pressed(MouseButton::Left) && orbit_drag_active {
        apply_orbit_input(orbit, mouse_delta, delta_secs);
        if mouse_delta.length_squared() > 0.0 {
            orbit_drag.suppress_next_clear_click();
        }
    } else {
        orbit_drag.decay_clear_click_suppression();
        apply_orbit_inertia(orbit, delta_secs);
    }
}

fn apply_orbit_input(orbit: &mut OrbitCamera, mouse_delta: Vec2, delta_secs: f32) {
    let yaw_delta = -mouse_delta.x * ORBIT_SENSITIVITY;
    let pitch_delta = mouse_delta.y * ORBIT_SENSITIVITY;

    orbit.yaw += yaw_delta;
    orbit.pitch = clamp_pitch(orbit.pitch + pitch_delta);

    if delta_secs > f32::EPSILON && mouse_delta != Vec2::ZERO {
        orbit.yaw_velocity = yaw_delta / delta_secs;
        orbit.pitch_velocity = pitch_delta / delta_secs;
    } else {
        stop_orbit_inertia(orbit);
    }
}

fn apply_orbit_inertia(orbit: &mut OrbitCamera, delta_secs: f32) {
    let velocity = Vec2::new(orbit.yaw_velocity, orbit.pitch_velocity);
    if delta_secs <= f32::EPSILON
        || velocity.length_squared() < ORBIT_VELOCITY_EPSILON * ORBIT_VELOCITY_EPSILON
    {
        stop_orbit_inertia(orbit);
        return;
    }

    orbit.yaw += orbit.yaw_velocity * delta_secs;
    orbit.pitch = clamp_pitch(orbit.pitch + orbit.pitch_velocity * delta_secs);

    let decay = (-ORBIT_INERTIA_DAMPING * delta_secs).exp();
    orbit.yaw_velocity *= decay;
    orbit.pitch_velocity *= decay;
}

fn update_orbit_zoom(
    mouse_wheel: &mut MessageReader<MouseWheel>,
    orbit: &mut OrbitCamera,
    orbit_drag: &mut OrbitDrag,
) {
    for wheel in mouse_wheel.read() {
        orbit.distance = (orbit.distance - wheel.y * 0.35).clamp(1.0, max_camera_distance());
        orbit_drag.suppress_next_clear_click();
    }
}

fn stop_orbit_inertia(orbit: &mut OrbitCamera) {
    orbit.yaw_velocity = 0.0;
    orbit.pitch_velocity = 0.0;
}

pub(super) fn set_orbit_view_direction(orbit: &mut OrbitCamera, direction: Vec3) {
    let direction = direction.normalize_or_zero();
    if direction.length_squared() < f32::EPSILON {
        return;
    }

    orbit.yaw = direction.x.atan2(direction.z);
    orbit.pitch = clamp_pitch(direction.y.asin());
    stop_orbit_inertia(orbit);
}

pub(super) fn sync_orbit_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    apply_orbit_transform(transform, orbit);
}

fn default_orbit_camera() -> OrbitCamera {
    OrbitCamera {
        target: Vec3::ZERO,
        distance: default_camera_distance(),
        yaw: CAMERA_YAW,
        pitch: CAMERA_PITCH,
        yaw_velocity: 0.0,
        pitch_velocity: 0.0,
    }
}

fn default_camera_distance() -> f32 {
    let half_extent = GRID_HALF_EXTENT as f32;
    let half_diagonal = Vec2::splat(half_extent).length();
    (half_diagonal * CAMERA_DISTANCE_SCALE).max(7.8)
}

fn max_camera_distance() -> f32 {
    default_camera_distance() * CAMERA_MAX_DISTANCE_SCALE
}

fn apply_orbit_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let horizontal = orbit.distance * orbit.pitch.cos();
    transform.translation = orbit.target
        + Vec3::new(
            horizontal * orbit.yaw.sin(),
            orbit.distance * orbit.pitch.sin(),
            horizontal * orbit.yaw.cos(),
        );
    transform.look_at(orbit.target, Vec3::Y);
}

fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(-1.25, 1.25)
}
