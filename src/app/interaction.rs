use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::Indices,
    picking::pointer::{PointerId, PointerInteraction, PointerMap},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use crate::state::{self, ToolModeSpec};

use super::{
    camera::{OrbitCamera, OrbitDrag},
    cut::CutPreview,
    gizmo_interaction::GizmoDrag,
    model::{
        ImportedModel, ModelDrag, ModelDragState, SelectedModel, model_visual_center,
        place_model_face_on_build_plate,
    },
    orientation::{OrientationInteraction, OrientationViewTarget},
    tool::ActiveTool,
};

pub(super) fn select_model_on_click(
    click: On<Pointer<Click>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    active_tool: Res<ActiveTool>,
    models: Query<&ImportedModel>,
    mut selected: ResMut<SelectedModel>,
) {
    if active_tool.is_bottom_face() {
        return;
    }

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

#[derive(Resource, Default)]
pub(super) struct BottomFaceHover {
    triangle: Option<BottomFaceHoverTriangle>,
}

#[derive(Component)]
pub(super) struct BottomFaceHoverVisual;

#[derive(Clone, Copy)]
struct BottomFaceHoverTriangle {
    vertices: [Vec3; 3],
    normal: Vec3,
}

pub(super) fn place_bottom_face_on_click(
    click: On<Pointer<Click>>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    mut active_tool: ResMut<ActiveTool>,
    mut models: ParamSet<(
        Query<(Entity, &ImportedModel, &GlobalTransform)>,
        Query<(&ImportedModel, &mut Transform)>,
    )>,
) {
    if !active_tool.is_bottom_face() || click.button != PointerButton::Primary {
        return;
    }

    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, click.pointer_location.position)
    else {
        return;
    };

    let Some((entity, hit)) = models
        .p0()
        .iter()
        .filter(|(_, model, _)| selected.contains(model.id))
        .filter_map(|(entity, model, global_transform)| {
            pick_model_bottom_face(model, global_transform, ray.origin, *ray.direction)
                .map(|hit| (entity, hit))
        })
        .min_by(|(_, a), (_, b)| a.distance_squared.total_cmp(&b.distance_squared))
    else {
        return;
    };

    let mut writable_models = models.p1();
    let Ok((model, mut transform)) = writable_models.get_mut(entity) else {
        return;
    };

    place_model_face_on_build_plate(model, &mut transform, hit.visible_normal_world);
    active_tool.set_mode(ToolModeSpec::None);
    state::remember_active_tool(ToolModeSpec::None);
}

pub(super) fn spawn_bottom_face_hover_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(bottom_face_hover_mesh([Vec3::ZERO; 3], Vec3::Y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.86, 0.04),
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
        BottomFaceHoverVisual,
        Name::new("Bottom Face Hover"),
    ));
}

pub(super) fn update_bottom_face_hover(
    active_tool: Res<ActiveTool>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &GlobalTransform)>,
    mut hover: ResMut<BottomFaceHover>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visuals: Query<(&Mesh3d, &mut Visibility), With<BottomFaceHoverVisual>>,
) {
    let Ok((mesh_handle, mut visibility)) = visuals.single_mut() else {
        return;
    };

    let Some(triangle) =
        bottom_face_hover_triangle(&active_tool, &window, *camera, &selected, &models)
    else {
        hover.triangle = None;
        *visibility = Visibility::Hidden;
        return;
    };

    hover.triangle = Some(triangle);
    if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
        *mesh = bottom_face_hover_mesh(triangle.vertices, triangle.normal);
    }
    *visibility = Visibility::Visible;
}

pub(super) fn draw_bottom_face_hover(hover: &BottomFaceHover, gizmos: &mut Gizmos) {
    let Some(triangle) = hover.triangle else {
        return;
    };

    let offset = triangle.normal * 0.025;
    let vertices = triangle.vertices.map(|vertex| vertex + offset);
    gizmos.lineloop(vertices, Color::srgb(1.0, 0.92, 0.04));
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
        || active_tool.is_bottom_face()
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
    state::set_cut_preview(state::CutPreviewUpdate {
        visible: false,
        plane: None,
    });
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
        || active_tool.is_cut()
        || active_tool.is_bottom_face()
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

#[derive(Clone, Copy)]
struct TriangleHit {
    distance_squared: f32,
    visible_normal_world: Vec3,
    vertices_world: [Vec3; 3],
}

fn pick_model_bottom_face(
    model: &ImportedModel,
    global_transform: &GlobalTransform,
    ray_origin_world: Vec3,
    ray_direction_world: Vec3,
) -> Option<TriangleHit> {
    pick_model_triangle(
        model,
        global_transform,
        ray_origin_world,
        ray_direction_world,
    )
}

fn pick_model_triangle(
    model: &ImportedModel,
    global_transform: &GlobalTransform,
    ray_origin_world: Vec3,
    ray_direction_world: Vec3,
) -> Option<TriangleHit> {
    let inverse = global_transform.affine().inverse();
    let local_origin = inverse.transform_point3(ray_origin_world);
    let local_direction = inverse
        .transform_vector3(ray_direction_world)
        .try_normalize()?;

    model
        .mesh
        .positions
        .chunks_exact(3)
        .filter_map(|triangle| {
            let a = Vec3::from_array(triangle[0]);
            let b = Vec3::from_array(triangle[1]);
            let c = Vec3::from_array(triangle[2]);
            let local_hit = ray_triangle_intersection(local_origin, local_direction, a, b, c)?;
            let world_a = global_transform.transform_point(a);
            let world_b = global_transform.transform_point(b);
            let world_c = global_transform.transform_point(c);
            let mut visible_normal_world = (world_b - world_a)
                .cross(world_c - world_a)
                .try_normalize()?;
            if visible_normal_world.dot(ray_direction_world) > 0.0 {
                visible_normal_world = -visible_normal_world;
            }
            let world_hit = global_transform.transform_point(local_hit);
            Some(TriangleHit {
                distance_squared: world_hit.distance_squared(ray_origin_world),
                visible_normal_world,
                vertices_world: [world_a, world_b, world_c],
            })
        })
        .min_by(|a, b| a.distance_squared.total_cmp(&b.distance_squared))
}

fn ray_triangle_intersection(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<Vec3> {
    let edge1 = b - a;
    let edge2 = c - a;
    let h = direction.cross(edge2);
    let det = edge1.dot(h);
    if det.abs() <= 0.000001 {
        return None;
    }

    let inv_det = det.recip();
    let s = origin - a;
    let u = inv_det * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = inv_det * direction.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = inv_det * edge2.dot(q);
    (t > 0.000001).then_some(origin + direction * t)
}

fn bottom_face_hover_triangle(
    active_tool: &ActiveTool,
    window: &Window,
    camera: (&Camera, &GlobalTransform),
    selected: &SelectedModel,
    models: &Query<(&ImportedModel, &GlobalTransform)>,
) -> Option<BottomFaceHoverTriangle> {
    if !active_tool.is_bottom_face() || selected.len() == 0 {
        return None;
    }

    let cursor = window.cursor_position()?;
    let (camera, camera_transform) = camera;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    models
        .iter()
        .filter(|(model, _)| selected.contains(model.id))
        .filter_map(|(model, global_transform)| {
            pick_model_bottom_face(model, global_transform, ray.origin, *ray.direction)
        })
        .min_by(|a, b| a.distance_squared.total_cmp(&b.distance_squared))
        .map(|hit| BottomFaceHoverTriangle {
            vertices: hit.vertices_world,
            normal: hit.visible_normal_world,
        })
}

fn bottom_face_hover_mesh(vertices: [Vec3; 3], normal: Vec3) -> Mesh {
    let offset = normal.try_normalize().unwrap_or(Vec3::Y) * 0.035;
    let positions = vertices.map(|vertex| (vertex + offset).to_array()).to_vec();
    let normals = vec![normal.to_array(); 3];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
    mesh
}
