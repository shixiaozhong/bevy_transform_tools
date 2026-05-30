use bevy::{
    asset::RenderAssetUsages,
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};

use crate::mesh::MeshBounds;

use super::{
    GRID_HALF_EXTENT,
    camera::{OrbitCamera, set_orbit_view_direction, sync_orbit_transform},
    interaction::{GizmoDrag, RotateGizmoHover},
    model::{ImportedModel, SelectedModel, model_visual_center},
    tool::ActiveTool,
};

const ORIENTATION_MARGIN: f32 = 32.0;
const ORIENTATION_VIEWPORT_SIZE: f32 = 196.0;
const ORIENTATION_VIEW_HEIGHT: f32 = 2.58;
const ORIENTATION_CAMERA_DISTANCE: f32 = 4.0;
const ORIENTATION_AXIS_LENGTH: f32 = 1.18;
const ORIENTATION_SCENE_LAYER: usize = 1;
const ORIENTATION_EDGE_BLOCK_WIDTH: f32 = 0.24;
const ORIENTATION_FACE_INSET: f32 = 0.5 - ORIENTATION_EDGE_BLOCK_WIDTH;
const ORIENTATION_AXIS_LABEL_OFFSET: f32 = 0.18;
const ORIENTATION_AXIS_LABEL_SIZE: f32 = 0.22;
const ORIENTATION_LABEL_TEXTURE_SIZE: u32 = 128;
const ORIENTATION_LABEL_GLYPH_SCALE: u32 = 11;
const ORIENTATION_LABEL_GLYPH_SPACING: u32 = 6;
const ROTATION_ANGLE_LABEL_OFFSET: Vec2 = Vec2::new(12.0, 10.0);
const SCALE_HANDLE_COUNT: usize = 9;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub(super) struct OrientationGizmos;

#[derive(Component)]
pub(super) struct OrientationCamera;

#[derive(Component)]
pub(super) struct OrientationViewTarget {
    direction: Vec3,
}

#[derive(Component)]
pub(super) struct OrientationBlockShape {
    direction: Vec3,
}

#[derive(Component)]
pub(super) struct OrientationAxisLabel {
    axis: Vec3,
}

#[derive(Component)]
pub(super) struct RotationAngleLabel;

#[derive(Component)]
pub(super) struct RotationAngleText;

#[derive(Component)]
pub(super) struct ScaleHandleVisual {
    index: usize,
}

#[derive(Resource, Default)]
pub(super) struct OrientationInteraction {
    pub(super) hovered_entity: Option<Entity>,
    pub(super) pointer_over: bool,
}

#[derive(Component)]
pub(super) struct OrientationBlockMaterials {
    base: Handle<StandardMaterial>,
    hover: Handle<StandardMaterial>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OrientationBlockKind {
    Face,
    Edge,
    Corner,
}

pub(super) fn spawn_orientation_overlay(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    asset_server: &AssetServer,
) {
    spawn_orientation_cameras(commands);
    spawn_orientation_light(commands);
    spawn_orientation_cube_blocks(commands, meshes, materials, asset_server);
    spawn_orientation_axis_labels(commands, meshes, materials, images);
    spawn_rotation_angle_label(commands);
    spawn_scale_handle_visuals(commands, meshes, materials);
}

fn spawn_scale_handle_visuals(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for index in 0..SCALE_HANDLE_COUNT {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.78, 0.82),
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(0.001)),
            Visibility::Hidden,
            ScaleHandleVisual { index },
            Name::new(format!("Scale Handle Visual {index}")),
        ));
    }
}

fn spawn_rotation_angle_label(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.07, 0.88)),
        RotationAngleLabel,
        children![(
            Text::new("X: 0.00"),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.94)),
            RotationAngleText,
        )],
    ));
}

fn spawn_orientation_cameras(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: ORIENTATION_VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_3d()
        }),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(2.2, 1.9, 2.4).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(ORIENTATION_SCENE_LAYER),
        OrientationCamera,
        Name::new("Orientation 3D Camera"),
    ));
}

fn spawn_orientation_light(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 7_500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(ORIENTATION_SCENE_LAYER),
        Name::new("Orientation Light"),
    ));
}

fn plain_orientation_block_materials(
    materials: &mut Assets<StandardMaterial>,
) -> OrientationBlockMaterials {
    let base = materials.add(StandardMaterial {
        base_color: Color::srgb(0.74, 0.75, 0.76),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let hover = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.58, 0.32),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    OrientationBlockMaterials { base, hover }
}

fn labeled_orientation_block_materials(
    base_texture_path: &'static str,
    hover_texture_path: &'static str,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> OrientationBlockMaterials {
    let base_texture = asset_server.load(base_texture_path);
    let hover_texture = asset_server.load(hover_texture_path);
    let base = materials.add(StandardMaterial {
        base_color_texture: Some(base_texture),
        perceptual_roughness: 0.92,
        metallic: 0.0,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let hover = materials.add(StandardMaterial {
        base_color_texture: Some(hover_texture),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    OrientationBlockMaterials { base, hover }
}

fn spawn_orientation_cube_blocks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    for block in orientation_cube_blocks() {
        let block_materials = orientation_block_materials(&block, materials, asset_server);
        let base_material = block_materials.base.clone();
        commands
            .spawn((
                Mesh3d(meshes.add(orientation_block_mesh(&block))),
                MeshMaterial3d(base_material),
                Transform::default(),
                RenderLayers::layer(ORIENTATION_SCENE_LAYER),
                OrientationViewTarget {
                    direction: print_axis_to_world(block.direction.normalize()),
                },
                OrientationBlockShape {
                    direction: block.direction,
                },
                block_materials,
                Name::new(format!("Orientation {} View Block", block.name)),
            ))
            .insert(Pickable::default());
    }
}

fn orientation_block_materials(
    block: &OrientationBlockSeed,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> OrientationBlockMaterials {
    if block.kind == OrientationBlockKind::Face {
        let (base_path, hover_path) = orientation_face_texture_paths(block.direction);
        labeled_orientation_block_materials(base_path, hover_path, materials, asset_server)
    } else {
        plain_orientation_block_materials(materials)
    }
}

fn spawn_orientation_axis_labels(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    for (label, axis, color) in [
        ("x", Vec3::X, [204, 105, 105, 255]),
        ("y", Vec3::Y, [111, 194, 119, 255]),
        ("z", Vec3::Z, [123, 130, 204, 255]),
    ] {
        let texture = images.add(orientation_label_texture(label, color, [0, 0, 0, 0]));
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(orientation_axis_label_mesh())),
            MeshMaterial3d(material),
            Transform::default(),
            RenderLayers::layer(ORIENTATION_SCENE_LAYER),
            OrientationAxisLabel { axis },
            Name::new(format!("Orientation {label} Axis Label")),
        ));
    }
}

pub(super) fn draw_grid_and_selection(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    gizmo_drag: Res<GizmoDrag>,
    rotate_hover: Res<RotateGizmoHover>,
    models: Query<(&ImportedModel, &Transform)>,
    mut scale_handles: Query<
        (
            &ScaleHandleVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<ImportedModel>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmos: Gizmos,
) {
    draw_ground_grid(&mut gizmos);
    update_scale_handle_visuals(
        selected.0,
        &active_tool,
        &models,
        &mut scale_handles,
        &mut materials,
    );
    draw_selected_model_tools(
        &mut gizmos,
        selected.0,
        &active_tool,
        &gizmo_drag,
        &rotate_hover,
        &models,
    );
}

pub(super) fn update_rotation_angle_label(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    gizmo_drag: Res<GizmoDrag>,
    rotate_hover: Res<RotateGizmoHover>,
    camera: Single<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    models: Query<(&ImportedModel, &Transform)>,
    mut label: Single<&mut Node, With<RotationAngleLabel>>,
    mut text: Single<&mut Text, With<RotationAngleText>>,
) {
    if active_tool.is_scale() {
        update_scale_label(
            selected,
            &gizmo_drag,
            camera.into_inner(),
            &models,
            &mut label,
            &mut text,
        );
        return;
    }

    let Some((axis, angle)) = rotation_angle_label_state(&gizmo_drag, &rotate_hover) else {
        label.display = Display::None;
        return;
    };

    if !active_tool.is_rotate() {
        label.display = Display::None;
        return;
    }

    let Some(selected_id) = selected.0 else {
        label.display = Display::None;
        return;
    };

    let (camera, camera_transform) = *camera;
    for (model, transform) in &models {
        if model.id != selected_id {
            continue;
        }

        let origin = model_visual_center(model, transform);
        let radius = rotate_gizmo_radius(model, transform);
        let Some(print_axis) = world_axis_to_rotation_print_axis(axis) else {
            label.display = Display::None;
            return;
        };
        let handle_position =
            origin + print_axis_to_world(rotation_handle_radial(print_axis)) * radius;
        let Ok(screen_position) = camera.world_to_viewport(camera_transform, handle_position)
        else {
            label.display = Display::None;
            return;
        };

        label.display = Display::Flex;
        label.left = Val::Px(screen_position.x + ROTATION_ANGLE_LABEL_OFFSET.x);
        label.top = Val::Px(screen_position.y + ROTATION_ANGLE_LABEL_OFFSET.y);
        text.0 = format!(
            "{}: {:.2}",
            rotation_axis_label(print_axis),
            normalized_degrees(angle)
        );
        return;
    }

    label.display = Display::None;
}

fn update_scale_label(
    selected: Res<SelectedModel>,
    gizmo_drag: &GizmoDrag,
    (camera, camera_transform): (&Camera, &GlobalTransform),
    models: &Query<(&ImportedModel, &Transform)>,
    label: &mut Node,
    text: &mut Text,
) {
    let Some(component) = gizmo_drag.active_scale() else {
        label.display = Display::None;
        return;
    };
    let Some(selected_id) = selected.0 else {
        label.display = Display::None;
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }
        let position = model_visual_center(model, transform);
        let Ok(screen_position) = camera.world_to_viewport(camera_transform, position) else {
            label.display = Display::None;
            return;
        };

        label.display = Display::Flex;
        label.left = Val::Px(screen_position.x + 28.0);
        label.top = Val::Px(screen_position.y + 28.0);
        text.0 = match component {
            Some(0) => format!("X: {:.2}%", transform.scale.x * 100.0),
            Some(2) => format!("Y: {:.2}%", transform.scale.z * 100.0),
            Some(1) => format!("Z: {:.2}%", transform.scale.y * 100.0),
            _ => format!(
                "X: {:.2}%\nY: {:.2}%\nZ: {:.2}%",
                transform.scale.x * 100.0,
                transform.scale.z * 100.0,
                transform.scale.y * 100.0
            ),
        };
        return;
    }

    label.display = Display::None;
}

fn rotation_angle_label_state(
    gizmo_drag: &GizmoDrag,
    rotate_hover: &RotateGizmoHover,
) -> Option<(Vec3, f32)> {
    gizmo_drag
        .active_rotation()
        .or_else(|| rotate_hover.axis.map(|axis| (axis, rotate_hover.angle)))
}

pub(super) fn draw_orientation_overlay(
    orientation_camera: Single<&GlobalTransform, With<OrientationCamera>>,
    mut axis_labels: Query<(&OrientationAxisLabel, &mut Transform)>,
    mut orientation_gizmos: Gizmos<OrientationGizmos>,
) {
    update_orientation_axis_label_transforms(*orientation_camera, &mut axis_labels);
    draw_orientation_axes(&mut orientation_gizmos);
}

pub(super) fn update_orientation_camera(
    window: Single<&Window>,
    main_camera: Single<&GlobalTransform, With<OrbitCamera>>,
    orientation_camera: Single<(&mut Camera, &mut Transform), With<OrientationCamera>>,
) {
    let (mut camera, mut transform) = orientation_camera.into_inner();
    let scale_factor = window.resolution.scale_factor();
    let viewport_size = (ORIENTATION_VIEWPORT_SIZE * scale_factor).round() as u32;
    let viewport_margin = (ORIENTATION_MARGIN * scale_factor).round() as u32;
    let viewport_y = window
        .physical_height()
        .saturating_sub(viewport_margin + viewport_size);
    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(viewport_margin, viewport_y),
        physical_size: UVec2::splat(viewport_size),
        ..default()
    });

    let view_to_camera = main_camera.rotation() * Vec3::Z;
    transform.translation = view_to_camera * ORIENTATION_CAMERA_DISTANCE;
    transform.look_at(Vec3::ZERO, Vec3::Y);
}

pub(super) fn orient_camera_from_cube_click(
    click: On<Pointer<Click>>,
    targets: Query<&OrientationViewTarget>,
    interaction: Res<OrientationInteraction>,
    camera: Single<(&mut Transform, &mut OrbitCamera), With<OrbitCamera>>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let target_entity = interaction.hovered_entity.unwrap_or(click.entity);
    let Ok(target) = targets.get(target_entity) else {
        return;
    };

    let (mut transform, mut orbit) = camera.into_inner();
    set_orbit_view_direction(&mut orbit, target.direction);
    sync_orbit_transform(&mut transform, &orbit);
}

pub(super) fn update_orientation_interaction(
    window: Single<&Window>,
    orientation_camera: Single<(&Camera, &GlobalTransform), With<OrientationCamera>>,
    mut interaction: ResMut<OrientationInteraction>,
    mut blocks: Query<(
        Entity,
        &OrientationBlockShape,
        &OrientationBlockMaterials,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let hovered_entity = cursor_hovered_orientation_block(&window, *orientation_camera, &blocks);

    if interaction.hovered_entity == hovered_entity {
        return;
    }

    interaction.hovered_entity = hovered_entity;
    interaction.pointer_over = hovered_entity.is_some();

    for (entity, _, block_materials, mut material) in &mut blocks {
        material.0 = if Some(entity) == hovered_entity {
            block_materials.hover.clone()
        } else {
            block_materials.base.clone()
        };
    }
}

fn cursor_hovered_orientation_block(
    window: &Window,
    (camera, camera_transform): (&Camera, &GlobalTransform),
    blocks: &Query<(
        Entity,
        &OrientationBlockShape,
        &OrientationBlockMaterials,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<Entity> {
    let cursor = window.cursor_position()?;
    if !cursor_is_in_orientation_viewport(window, camera, cursor) {
        return None;
    }

    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let mut best_hit = None::<(f32, Entity)>;

    for (entity, shape, _, _) in blocks {
        if let Some(distance) = ray_orientation_block_distance(&ray, shape.direction)
            && best_hit
                .map(|(best_distance, _)| distance < best_distance)
                .unwrap_or(true)
        {
            best_hit = Some((distance, entity));
        }
    }

    best_hit.map(|(_, entity)| entity)
}

fn cursor_is_in_orientation_viewport(window: &Window, camera: &Camera, cursor: Vec2) -> bool {
    let Some(viewport) = &camera.viewport else {
        return true;
    };

    let scale_factor = window.resolution.scale_factor();
    let cursor = (cursor * scale_factor).round().as_uvec2();
    let min = viewport.physical_position;
    let max = min + viewport.physical_size;

    cursor.x >= min.x && cursor.x <= max.x && cursor.y >= min.y && cursor.y <= max.y
}

fn ray_orientation_block_distance(ray: &Ray3d, direction: Vec3) -> Option<f32> {
    let mut best_distance = None::<f32>;

    for axis in 0..3 {
        let step = direction[axis];
        if step == 0.0 {
            continue;
        }

        let sign = step.signum();
        let mut normal = Vec3::ZERO;
        normal[axis] = sign;
        let normal = print_axis_to_world(normal);

        let mut plane_origin = Vec3::ZERO;
        plane_origin[axis] = sign * 0.5;
        let plane_origin = print_axis_to_world(plane_origin);

        let Some(distance) = ray.intersect_plane(plane_origin, InfinitePlane3d::new(normal)) else {
            continue;
        };
        let point = world_axis_to_print(ray.get_point(distance));

        let u_axis = (axis + 1) % 3;
        let v_axis = (axis + 2) % 3;
        let (u_min, u_max) = orientation_axis_range(direction[u_axis]);
        let (v_min, v_max) = orientation_axis_range(direction[v_axis]);

        if (point[axis] - 0.5 * sign).abs() <= 0.002
            && point[u_axis] >= u_min - 0.002
            && point[u_axis] <= u_max + 0.002
            && point[v_axis] >= v_min - 0.002
            && point[v_axis] <= v_max + 0.002
            && best_distance
                .map(|best_distance| distance < best_distance)
                .unwrap_or(true)
        {
            best_distance = Some(distance);
        }
    }

    best_distance
}

fn draw_selected_model_tools(
    gizmos: &mut Gizmos,
    selected_id: Option<u32>,
    active_tool: &ActiveTool,
    gizmo_drag: &GizmoDrag,
    rotate_hover: &RotateGizmoHover,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(selected_id) = selected_id else {
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }

        if !active_tool.is_move()
            && !active_tool.is_rotate()
            && !active_tool.is_scale()
            && let Some(bounds) = model.bounds
        {
            draw_model_aabb(gizmos, bounds, transform);
        }
        if active_tool.is_move() {
            draw_move_gizmo(gizmos, model, transform);
        }
        if active_tool.is_rotate() {
            draw_rotate_gizmo(
                gizmos,
                model,
                transform,
                gizmo_drag.active_rotation(),
                rotate_hover.axis,
            );
        }
        if active_tool.is_scale() {
            draw_scale_gizmo(gizmos, model, transform);
        }

        break;
    }
}

fn draw_move_gizmo(gizmos: &mut Gizmos, model: &ImportedModel, transform: &Transform) {
    let origin = model_visual_center(model, transform);
    let length = move_gizmo_length(model, transform);
    let x_color = Color::srgb(0.95, 0.05, 0.04);
    let y_color = Color::srgb(0.05, 0.75, 0.12);
    let z_color = Color::srgb(0.04, 0.16, 0.95);

    gizmos.arrow(origin, origin + Vec3::X * length, x_color);
    gizmos.arrow(origin, origin + Vec3::Y * length, y_color);
    gizmos.arrow(origin, origin + Vec3::Z * length, z_color);
    gizmos.sphere(origin, 0.07 * length, Color::srgb(0.08, 0.52, 0.48));
}

fn draw_scale_gizmo(gizmos: &mut Gizmos, model: &ImportedModel, transform: &Transform) {
    let (min, max) = scale_gizmo_bounds(model, transform);
    let bottom_y = min.y;
    let red = Color::srgb(0.95, 0.05, 0.04);

    let base_corners = [
        Vec3::new(min.x, bottom_y, min.z),
        Vec3::new(max.x, bottom_y, min.z),
        Vec3::new(max.x, bottom_y, max.z),
        Vec3::new(min.x, bottom_y, max.z),
    ];
    for index in 0..4 {
        gizmos.line(base_corners[index], base_corners[(index + 1) % 4], red);
    }
}

fn update_scale_handle_visuals(
    selected_id: Option<u32>,
    active_tool: &ActiveTool,
    models: &Query<(&ImportedModel, &Transform)>,
    scale_handles: &mut Query<
        (
            &ScaleHandleVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<ImportedModel>,
    >,
    materials: &mut Assets<StandardMaterial>,
) {
    let Some(selected_id) = selected_id.filter(|_| active_tool.is_scale()) else {
        for (_, _, mut visibility, _) in scale_handles {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let Some((model, transform)) = models.iter().find(|(model, _)| model.id == selected_id) else {
        for (_, _, mut visibility, _) in scale_handles {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    let handle_size = move_gizmo_length(model, transform) * 0.055;
    let handles = scale_handle_visual_layout(model, transform);

    for (handle, mut handle_transform, mut visibility, mut material) in scale_handles {
        let Some((position, color)) = handles.get(handle.index).copied() else {
            *visibility = Visibility::Hidden;
            continue;
        };

        handle_transform.translation = position;
        handle_transform.rotation = Quat::IDENTITY;
        handle_transform.scale = Vec3::splat(handle_size);
        *visibility = Visibility::Visible;

        if let Some(existing) = materials.get_mut(&material.0) {
            existing.base_color = color;
        } else {
            material.0 = materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.82,
                metallic: 0.0,
                ..default()
            });
        }
    }
}

fn scale_handle_visual_layout(model: &ImportedModel, transform: &Transform) -> [(Vec3, Color); 9] {
    let (min, max) = scale_gizmo_bounds(model, transform);
    let center = (min + max) * 0.5;
    let bottom_y = min.y;
    let top_y = max.y;
    let red = Color::srgb(0.95, 0.05, 0.04);
    let green = Color::srgb(0.05, 0.78, 0.12);
    let blue = Color::srgb(0.04, 0.16, 0.95);
    let cyan = Color::srgb(0.0, 0.78, 0.82);

    [
        (Vec3::new(min.x, bottom_y, min.z), cyan),
        (Vec3::new(max.x, bottom_y, min.z), cyan),
        (Vec3::new(max.x, bottom_y, max.z), cyan),
        (Vec3::new(min.x, bottom_y, max.z), cyan),
        (Vec3::new(min.x, bottom_y, center.z), red),
        (Vec3::new(max.x, bottom_y, center.z), red),
        (Vec3::new(center.x, bottom_y, min.z), green),
        (Vec3::new(center.x, bottom_y, max.z), green),
        (Vec3::new(center.x, top_y, center.z), blue),
    ]
}

fn draw_rotate_gizmo(
    gizmos: &mut Gizmos,
    model: &ImportedModel,
    transform: &Transform,
    active_rotation: Option<(Vec3, f32)>,
    hovered_axis: Option<Vec3>,
) {
    let origin = model_visual_center(model, transform);
    let radius = rotate_gizmo_radius(model, transform);
    let focused_axis = active_rotation.map(|(axis, _)| axis).or(hovered_axis);
    let axes = [
        (Vec3::X, Color::srgb(0.95, 0.05, 0.04)),
        (Vec3::Y, Color::srgb(0.05, 0.78, 0.12)),
        (Vec3::Z, Color::srgb(0.04, 0.16, 0.95)),
    ];

    for (print_axis, color) in axes {
        let world_axis = print_axis_to_world(print_axis);
        if focused_axis.is_some_and(|axis| axis != world_axis) {
            continue;
        }

        let is_active = active_rotation.is_some_and(|(active_axis, _)| active_axis == world_axis);
        let is_hovered = hovered_axis == Some(world_axis);
        let ring_color = if is_active {
            Color::srgb(0.96, 0.96, 0.92)
        } else {
            color
        };
        gizmos
            .circle(
                Isometry3d::new(origin, Quat::from_rotation_arc(Vec3::Z, world_axis)),
                radius,
                ring_color,
            )
            .resolution(128);
        draw_rotation_axis_handle(gizmos, origin, print_axis, radius, color);

        if is_active || is_hovered {
            let angle = active_rotation
                .filter(|(axis, _)| *axis == world_axis)
                .map(|(_, angle)| angle)
                .unwrap_or(0.0);
            draw_rotation_ticks(gizmos, origin, print_axis, radius, angle);
        }
    }

    gizmos.sphere(origin, 0.045 * radius, Color::srgb(0.08, 0.52, 0.48));
}

fn draw_rotation_axis_handle(
    gizmos: &mut Gizmos,
    origin: Vec3,
    print_axis: Vec3,
    radius: f32,
    color: Color,
) {
    let axis = print_axis_to_world(print_axis);
    let radial = print_axis_to_world(rotation_handle_radial(print_axis));
    let tangent = axis.cross(radial).normalize_or_zero();
    if tangent.length_squared() < f32::EPSILON {
        return;
    }

    let center = origin + radial * radius;
    let half_length = radius * 0.13;
    let tip_length = radius * 0.065;
    let start = center - tangent * half_length;
    let end = center + tangent * half_length;

    gizmos
        .arrow(start, end, color)
        .with_double_end()
        .with_tip_length(tip_length);

    let grip_half = radius * 0.045;
    let grip_width = radius * 0.055;
    let side = radial.cross(tangent).normalize_or_zero();
    let corners = [
        center - tangent * grip_half - side * grip_width,
        center + tangent * grip_half - side * grip_width,
        center + tangent * grip_half + side * grip_width,
        center - tangent * grip_half + side * grip_width,
    ];
    for index in 0..4 {
        gizmos.line(corners[index], corners[(index + 1) % 4], color);
    }
}

fn rotation_handle_radial(axis: Vec3) -> Vec3 {
    if axis == Vec3::X {
        Vec3::NEG_Y
    } else if axis == Vec3::Y {
        Vec3::Z
    } else {
        Vec3::X
    }
}

fn draw_rotation_ticks(
    gizmos: &mut Gizmos,
    origin: Vec3,
    print_axis: Vec3,
    radius: f32,
    angle_delta: f32,
) {
    let axis = print_axis_to_world(print_axis);
    let u = print_axis_to_world(rotation_handle_radial(print_axis));
    let v = axis.cross(u).normalize_or_zero();
    let color = Color::srgb(0.96, 0.96, 0.92);
    let tick_count = 72;

    for index in 0..tick_count {
        let angle = index as f32 / tick_count as f32 * std::f32::consts::TAU;
        let direction = u * angle.cos() + v * angle.sin();
        let is_major = index % 6 == 0;
        let length = if is_major { 0.12 } else { 0.065 } * radius;
        gizmos.line(
            origin + direction * (radius - length),
            origin + direction * (radius + length * 0.35),
            color,
        );
    }

    let reference = u;
    let current = u * angle_delta.cos() + v * angle_delta.sin();
    gizmos.line(origin, origin + reference * radius, color);
    gizmos.line(origin, origin + current * radius, color);
}

fn draw_model_aabb(gizmos: &mut Gizmos, bounds: MeshBounds, transform: &Transform) {
    let corners = world_aabb_corners(bounds, transform);
    let color = Color::srgb(1.0, 0.86, 0.18);

    for (start, end) in AABB_EDGES {
        gizmos.line(corners[start], corners[end], color);
    }
}

fn world_aabb_corners(bounds: MeshBounds, transform: &Transform) -> [Vec3; 8] {
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

    let mut min = transform.transform_point(local_corners[0]);
    let mut max = min;
    for corner in local_corners.iter().skip(1) {
        let world = transform.transform_point(*corner);
        min = min.min(world);
        max = max.max(world);
    }

    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
}

fn draw_ground_grid(gizmos: &mut Gizmos) {
    let size = GRID_HALF_EXTENT;
    let color = Color::srgba(0.48, 0.52, 0.58, 0.32);
    let axis_color = Color::srgba(0.82, 0.84, 0.88, 0.5);

    for i in -size..=size {
        let i = i as f32;
        let c = if i.abs() < f32::EPSILON {
            axis_color
        } else {
            color
        };
        gizmos.line(
            Vec3::new(i, 0.0, -size as f32),
            Vec3::new(i, 0.0, size as f32),
            c,
        );
        gizmos.line(
            Vec3::new(-size as f32, 0.0, i),
            Vec3::new(size as f32, 0.0, i),
            c,
        );
    }
}

fn draw_orientation_axes(gizmos: &mut Gizmos<OrientationGizmos>) {
    for (axis, color) in [
        (Vec3::X, Color::srgb(0.80, 0.42, 0.42)),
        (Vec3::Y, Color::srgb(0.44, 0.76, 0.47)),
        (Vec3::Z, Color::srgb(0.48, 0.51, 0.80)),
    ] {
        let axis_origin = print_axis_to_world(ORIENTATION_AXIS_CORNER);
        let axis_end = axis_origin + print_axis_to_world(axis) * ORIENTATION_AXIS_LENGTH;
        gizmos.arrow(axis_origin, axis_end, color);
    }
    gizmos.sphere(
        print_axis_to_world(ORIENTATION_AXIS_CORNER),
        0.062,
        Color::srgb(0.92, 0.94, 0.96),
    );
}

fn update_orientation_axis_label_transforms(
    camera_transform: &GlobalTransform,
    axis_labels: &mut Query<(&OrientationAxisLabel, &mut Transform)>,
) {
    for (label, mut transform) in axis_labels {
        let position = ORIENTATION_AXIS_CORNER
            + label.axis * (ORIENTATION_AXIS_LENGTH + ORIENTATION_AXIS_LABEL_OFFSET);
        transform.translation = print_axis_to_world(position);
        transform.rotation = camera_transform.rotation();
    }
}

fn print_axis_to_world(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, axis.z, -axis.y)
}

fn world_axis_to_print(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, -axis.z, axis.y)
}

fn orientation_block_mesh(block: &OrientationBlockSeed) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();

    for axis in 0..3 {
        let step = block.direction[axis];
        if step != 0.0 {
            add_orientation_surface_quad(
                axis,
                step.signum(),
                block.direction,
                &mut positions,
                &mut normals,
                &mut uvs,
            );
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn orientation_axis_label_mesh() -> Mesh {
    let half = ORIENTATION_AXIS_LABEL_SIZE * 0.5;
    let positions = vec![
        [-half, -half, 0.0],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 6];
    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn add_orientation_surface_quad(
    axis: usize,
    sign: f32,
    direction: Vec3,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
) {
    let plane = sign * 0.5;
    let mut normal = Vec3::ZERO;
    normal[axis] = sign;
    let world_normal = print_axis_to_world(normal).to_array();
    let (right, up) = orientation_face_texture_basis(normal);
    let (right_axis, right_sign) = orientation_signed_axis(right);
    let (up_axis, up_sign) = orientation_signed_axis(up);
    let (right_min, right_max) = orientation_axis_range(direction[right_axis]);
    let (up_min, up_max) = orientation_axis_range(direction[up_axis]);
    let left_coord = orientation_signed_coord(right_min, right_max, -right_sign);
    let right_coord = orientation_signed_coord(right_min, right_max, right_sign);
    let down_coord = orientation_signed_coord(up_min, up_max, -up_sign);
    let up_coord = orientation_signed_coord(up_min, up_max, up_sign);

    let quad_positions = [
        (
            orientation_surface_position(axis, plane, right_axis, left_coord, up_axis, down_coord),
            [0.0, 1.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, right_coord, up_axis, down_coord),
            [1.0, 1.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, right_coord, up_axis, up_coord),
            [1.0, 0.0],
        ),
        (
            orientation_surface_position(axis, plane, right_axis, left_coord, up_axis, up_coord),
            [0.0, 0.0],
        ),
    ];

    for triangle_index in [0, 1, 2, 0, 2, 3] {
        let (position, uv) = quad_positions[triangle_index];
        positions.push(print_axis_to_world(position).to_array());
        normals.push(world_normal);
        uvs.push(uv);
    }
}

fn orientation_face_texture_basis(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.z != 0.0 {
        Vec3::Y * normal.z.signum()
    } else {
        Vec3::Z
    };
    let right = (-normal).cross(up);

    (right, up)
}

fn orientation_signed_axis(axis: Vec3) -> (usize, f32) {
    if axis.x != 0.0 {
        (0, axis.x.signum())
    } else if axis.y != 0.0 {
        (1, axis.y.signum())
    } else {
        (2, axis.z.signum())
    }
}

fn orientation_signed_coord(min: f32, max: f32, sign: f32) -> f32 {
    if sign > 0.0 { max } else { min }
}

fn orientation_surface_position(
    plane_axis: usize,
    plane: f32,
    right_axis: usize,
    right_coord: f32,
    up_axis: usize,
    up_coord: f32,
) -> Vec3 {
    let mut position = Vec3::ZERO;
    position[plane_axis] = plane;
    position[right_axis] = right_coord;
    position[up_axis] = up_coord;
    position
}

fn orientation_axis_range(step: f32) -> (f32, f32) {
    if step > 0.0 {
        (ORIENTATION_FACE_INSET, 0.5)
    } else if step < 0.0 {
        (-0.5, -ORIENTATION_FACE_INSET)
    } else {
        (-ORIENTATION_FACE_INSET, ORIENTATION_FACE_INSET)
    }
}

fn orientation_face_texture_paths(direction: Vec3) -> (&'static str, &'static str) {
    if direction.z > 0.0 {
        ("viewcube/top.png", "viewcube/top-hover.png")
    } else if direction.z < 0.0 {
        ("viewcube/bottom.png", "viewcube/bottom-hover.png")
    } else if direction.y > 0.0 {
        ("viewcube/back.png", "viewcube/back-hover.png")
    } else if direction.y < 0.0 {
        ("viewcube/front.png", "viewcube/front-hover.png")
    } else if direction.x < 0.0 {
        ("viewcube/left.png", "viewcube/left-hover.png")
    } else {
        ("viewcube/right.png", "viewcube/right-hover.png")
    }
}

fn orientation_label_texture(label: &str, text_color: [u8; 4], background: [u8; 4]) -> Image {
    let size = ORIENTATION_LABEL_TEXTURE_SIZE;
    let mut pixels = vec![0; (size * size * 4) as usize];

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.copy_from_slice(&background);
    }

    let glyphs = label.chars().collect::<Vec<_>>();
    let glyph_width = 5 * ORIENTATION_LABEL_GLYPH_SCALE;
    let glyph_height = 7 * ORIENTATION_LABEL_GLYPH_SCALE;
    let text_width = glyph_width * glyphs.len() as u32
        + ORIENTATION_LABEL_GLYPH_SPACING * glyphs.len().saturating_sub(1) as u32;
    let mut x = (size.saturating_sub(text_width)) / 2;
    let y = (size.saturating_sub(glyph_height)) / 2;

    for glyph in glyphs {
        draw_orientation_glyph(&mut pixels, size, glyph, x, y, text_color);
        x += glyph_width + ORIENTATION_LABEL_GLYPH_SPACING;
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn draw_orientation_glyph(
    pixels: &mut [u8],
    texture_size: u32,
    glyph: char,
    x: u32,
    y: u32,
    color: [u8; 4],
) {
    for (row, pattern) in orientation_glyph_pattern(glyph).iter().enumerate() {
        for (column, value) in pattern.as_bytes().iter().enumerate() {
            if *value == b'#' {
                draw_orientation_label_cell(
                    pixels,
                    texture_size,
                    x + column as u32 * ORIENTATION_LABEL_GLYPH_SCALE,
                    y + row as u32 * ORIENTATION_LABEL_GLYPH_SCALE,
                    color,
                );
            }
        }
    }
}

fn draw_orientation_label_cell(
    pixels: &mut [u8],
    texture_size: u32,
    x: u32,
    y: u32,
    color: [u8; 4],
) {
    for py in y..(y + ORIENTATION_LABEL_GLYPH_SCALE) {
        for px in x..(x + ORIENTATION_LABEL_GLYPH_SCALE) {
            if px >= texture_size || py >= texture_size {
                continue;
            }
            let index = ((py * texture_size + px) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn orientation_glyph_pattern(glyph: char) -> [&'static str; 7] {
    match glyph {
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        'X' | 'x' => [
            "#...#", ".#.#.", "..#..", "..#..", "..#..", ".#.#.", "#...#",
        ],
        'Y' | 'y' => [
            "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' | 'z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}

struct OrientationBlockSeed {
    name: String,
    direction: Vec3,
    kind: OrientationBlockKind,
}

fn orientation_cube_blocks() -> Vec<OrientationBlockSeed> {
    let mut blocks = Vec::with_capacity(26);
    let offsets = [-1.0, 0.0, 1.0];

    for x in offsets {
        for y in offsets {
            for z in offsets {
                let direction = Vec3::new(x, y, z);
                let axis_count = [x, y, z].into_iter().filter(|step| *step != 0.0).count();
                let kind = match axis_count {
                    1 => OrientationBlockKind::Face,
                    2 => OrientationBlockKind::Edge,
                    3 => OrientationBlockKind::Corner,
                    _ => continue,
                };

                blocks.push(OrientationBlockSeed {
                    name: orientation_block_name(direction),
                    direction,
                    kind,
                });
            }
        }
    }

    blocks
}

fn orientation_block_name(direction: Vec3) -> String {
    format!(
        "{}{}{}",
        orientation_axis_name("X", direction.x),
        orientation_axis_name("Y", direction.y),
        orientation_axis_name("Z", direction.z),
    )
}

fn orientation_axis_name(axis: &str, value: f32) -> String {
    if value > 0.0 {
        format!("+{axis}")
    } else if value < 0.0 {
        format!("-{axis}")
    } else {
        String::new()
    }
}

const AABB_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

const ORIENTATION_AXIS_CORNER: Vec3 = Vec3::new(-0.5, -0.5, -0.5);

fn move_gizmo_length(model: &ImportedModel, transform: &Transform) -> f32 {
    let scale = transform.scale.abs().max_element().max(1.0);
    let model_size = model
        .bounds
        .map(|bounds| bounds.size.max_element().abs() * scale * 0.7)
        .unwrap_or(0.0);
    model_size.max(2.2 * scale)
}

fn rotate_gizmo_radius(model: &ImportedModel, transform: &Transform) -> f32 {
    move_gizmo_length(model, transform) * 0.86
}

fn scale_gizmo_bounds(model: &ImportedModel, transform: &Transform) -> (Vec3, Vec3) {
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

fn world_axis_to_rotation_print_axis(axis: Vec3) -> Option<Vec3> {
    if axis.abs_diff_eq(print_axis_to_world(Vec3::X), f32::EPSILON) {
        Some(Vec3::X)
    } else if axis.abs_diff_eq(print_axis_to_world(Vec3::Y), f32::EPSILON) {
        Some(Vec3::Y)
    } else if axis.abs_diff_eq(print_axis_to_world(Vec3::Z), f32::EPSILON) {
        Some(Vec3::Z)
    } else {
        None
    }
}

fn rotation_axis_label(axis: Vec3) -> &'static str {
    if axis == Vec3::X {
        "X"
    } else if axis == Vec3::Y {
        "Y"
    } else {
        "Z"
    }
}

fn normalized_degrees(angle: f32) -> f32 {
    angle.to_degrees().rem_euclid(360.0)
}
