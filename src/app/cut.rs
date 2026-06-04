use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use crate::{
    mesh::clip::{CutAxis, CutPlane},
    state,
};

use super::{
    camera::OrbitCamera,
    model::{ImportedModel, SelectedModel, selected_world_bounds},
};

#[derive(Resource, Default)]
pub(super) struct CutPreview {
    pub(super) visible: bool,
    pub(super) plane: Option<CutPlane>,
    pub(super) drag: Option<CutPreviewDrag>,
}

#[derive(Component)]
pub(super) struct CutPreviewPlaneVisual;

#[derive(Clone, Copy)]
pub(super) struct CutPreviewDrag {
    axis: CutAxis,
    start_position: f32,
    start_cursor: Vec2,
    screen_axis: Vec2,
    units_per_pixel: f32,
    min_position: f32,
    max_position: f32,
}

impl CutPreview {
    pub(super) fn active_plane(&self) -> Option<CutPlane> {
        self.visible.then_some(self.plane?)
    }

    pub(super) fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
}

pub(super) fn sync_cut_preview_from_api(
    mut preview: ResMut<CutPreview>,
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &Transform)>,
) {
    if let Some(update) = state::take_cut_preview_update() {
        preview.visible = update.visible;
        preview.plane = update
            .plane
            .map(|plane| clamp_plane_to_selection(plane, &selected, &models));
        if !update.visible {
            preview.drag = None;
        }
    }
}

pub(super) fn spawn_cut_preview_plane_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(cut_preview_plane_mesh([Vec3::ZERO; 4], Vec3::Y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.58, 0.60, 0.58, 0.34),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::IDENTITY,
        Visibility::Hidden,
        Pickable::IGNORE,
        NotShadowCaster,
        NotShadowReceiver,
        NoFrustumCulling,
        CutPreviewPlaneVisual,
        Name::new("Cut Preview Plane"),
    ));
}

pub(super) fn update_cut_preview_plane_visual(
    preview: Res<CutPreview>,
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visuals: Query<(&Mesh3d, &mut Visibility), With<CutPreviewPlaneVisual>>,
) {
    let Ok((mesh_handle, mut visibility)) = visuals.single_mut() else {
        return;
    };

    let Some(plane) = preview.active_plane() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some((min, max)) = selected_world_bounds(&selected, &models) else {
        *visibility = Visibility::Hidden;
        return;
    };

    let plane = clamp_plane_to_bounds(plane, min, max);
    let normal = axis_vector(plane.axis);
    if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
        *mesh = cut_preview_plane_mesh(plane_corners(min, max, plane), normal);
    }
    *visibility = Visibility::Visible;
}

pub(super) fn begin_cut_preview_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &Transform)>,
    mut preview: ResMut<CutPreview>,
) {
    if !buttons.just_pressed(MouseButton::Left) || preview.drag.is_some() {
        return;
    }
    let Some(plane) = preview.active_plane() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some((min, max)) = selected_world_bounds(&selected, &models) else {
        return;
    };

    let (min_position, max_position) = axis_range(min, max, plane.axis);
    let plane = CutPlane {
        position: plane.position.clamp(min_position, max_position),
        ..plane
    };
    preview.plane = Some(plane);

    let normal = axis_vector(plane.axis);
    let center = plane_center(min, max, plane);
    let size = (max - min).max_element().max(1.0);
    let (camera, camera_transform) = *camera;
    let cursor_on_plane = camera
        .viewport_to_world(camera_transform, cursor)
        .ok()
        .and_then(|ray| ray.plane_intersection_point(center, InfinitePlane3d::new(normal)));
    let cursor_hits_plane = cursor_on_plane
        .map(|point| point_inside_plane_rect(point, min, max, plane))
        .unwrap_or(false);
    let Ok(screen_center) = camera.world_to_viewport(camera_transform, center) else {
        return;
    };
    let Ok(screen_end) = camera.world_to_viewport(camera_transform, center + normal * size) else {
        return;
    };
    let screen_vector = screen_end - screen_center;
    let screen_length = screen_vector.length();
    if screen_length < 8.0 {
        return;
    }

    let screen_axis = screen_vector / screen_length;
    let distance_to_axis = distance_to_segment(cursor, screen_center, screen_end);
    let distance_to_center = cursor.distance(screen_center);
    if !cursor_hits_plane && distance_to_axis > 18.0 && distance_to_center > 42.0 {
        return;
    }

    preview.drag = Some(CutPreviewDrag {
        axis: plane.axis,
        start_position: plane.position,
        start_cursor: cursor,
        screen_axis,
        units_per_pixel: size / screen_length,
        min_position,
        max_position,
    });
}

pub(super) fn update_cut_preview_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut preview: ResMut<CutPreview>,
) {
    if buttons.just_released(MouseButton::Left) {
        preview.drag = None;
        return;
    }

    let Some(drag) = preview.drag else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let offset = (cursor - drag.start_cursor).dot(drag.screen_axis) * drag.units_per_pixel;
    let position = (drag.start_position + offset).clamp(drag.min_position, drag.max_position);
    preview.plane = Some(CutPlane {
        axis: drag.axis,
        position,
    });
    state::remember_cut_preview_position(drag.axis, position);
}

pub(super) fn draw_cut_preview(
    gizmos: &mut Gizmos,
    preview: &CutPreview,
    selected: &SelectedModel,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(plane) = preview.active_plane() else {
        return;
    };
    if selected.primary().is_none() {
        return;
    }
    let Some((min, max)) = selected_world_bounds(selected, models) else {
        return;
    };
    let plane = clamp_plane_to_bounds(plane, min, max);

    let corners = plane_corners(min, max, plane);
    let outline = Color::srgba(0.82, 0.84, 0.82, 0.42);
    for index in 0..4 {
        gizmos.line(corners[index], corners[(index + 1) % 4], outline);
    }
}

fn plane_center(min: Vec3, max: Vec3, plane: CutPlane) -> Vec3 {
    let mut center = (min + max) * 0.5;
    center[axis_index(plane.axis)] = plane.position;
    center
}

fn plane_corners(min: Vec3, max: Vec3, plane: CutPlane) -> [Vec3; 4] {
    let padding = (max - min).max_element().max(1.0) * 0.08;
    let mut a = min - Vec3::splat(padding);
    let mut b = max + Vec3::splat(padding);
    a[axis_index(plane.axis)] = plane.position;
    b[axis_index(plane.axis)] = plane.position;

    match plane.axis {
        CutAxis::X => [
            Vec3::new(plane.position, a.y, a.z),
            Vec3::new(plane.position, b.y, a.z),
            Vec3::new(plane.position, b.y, b.z),
            Vec3::new(plane.position, a.y, b.z),
        ],
        CutAxis::Y => [
            Vec3::new(a.x, plane.position, a.z),
            Vec3::new(b.x, plane.position, a.z),
            Vec3::new(b.x, plane.position, b.z),
            Vec3::new(a.x, plane.position, b.z),
        ],
        CutAxis::Z => [
            Vec3::new(a.x, a.y, plane.position),
            Vec3::new(b.x, a.y, plane.position),
            Vec3::new(b.x, b.y, plane.position),
            Vec3::new(a.x, b.y, plane.position),
        ],
    }
}

fn cut_preview_plane_mesh(corners: [Vec3; 4], normal: Vec3) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        corners.map(|corner| corner.to_array()).to_vec(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal.to_array(); 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh
}

fn point_inside_plane_rect(point: Vec3, min: Vec3, max: Vec3, plane: CutPlane) -> bool {
    let padding = (max - min).max_element().max(1.0) * 0.08;
    match plane.axis {
        CutAxis::X => {
            point.y >= min.y - padding
                && point.y <= max.y + padding
                && point.z >= min.z - padding
                && point.z <= max.z + padding
        }
        CutAxis::Y => {
            point.x >= min.x - padding
                && point.x <= max.x + padding
                && point.z >= min.z - padding
                && point.z <= max.z + padding
        }
        CutAxis::Z => {
            point.x >= min.x - padding
                && point.x <= max.x + padding
                && point.y >= min.y - padding
                && point.y <= max.y + padding
        }
    }
}

fn axis_vector(axis: CutAxis) -> Vec3 {
    match axis {
        CutAxis::X => Vec3::X,
        CutAxis::Y => Vec3::Y,
        CutAxis::Z => Vec3::Z,
    }
}

fn axis_range(min: Vec3, max: Vec3, axis: CutAxis) -> (f32, f32) {
    let axis = axis_index(axis);
    (min[axis], max[axis])
}

fn clamp_plane_to_selection(
    plane: CutPlane,
    selected: &SelectedModel,
    models: &Query<(&ImportedModel, &Transform)>,
) -> CutPlane {
    selected_world_bounds(selected, models)
        .map(|(min, max)| clamp_plane_to_bounds(plane, min, max))
        .unwrap_or(plane)
}

fn clamp_plane_to_bounds(plane: CutPlane, min: Vec3, max: Vec3) -> CutPlane {
    let (min_position, max_position) = axis_range(min, max, plane.axis);
    CutPlane {
        position: plane.position.clamp(min_position, max_position),
        ..plane
    }
}

fn axis_index(axis: CutAxis) -> usize {
    match axis {
        CutAxis::X => 0,
        CutAxis::Y => 1,
        CutAxis::Z => 2,
    }
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
