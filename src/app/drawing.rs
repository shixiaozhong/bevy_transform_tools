use bevy::{
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    picking::pointer::{PointerId, PointerInteraction, PointerMap},
    prelude::*,
};

use crate::mesh::MeshBounds;

use super::{
    GRID_HALF_EXTENT,
    camera::{OrbitCamera, set_orbit_view_direction},
    model::{ImportedModel, SelectedModel, model_visual_center},
    tool::ActiveTool,
};

const ORIENTATION_MARGIN: f32 = 32.0;
const ORIENTATION_VIEWPORT_SIZE: f32 = 196.0;
const ORIENTATION_VIEW_HEIGHT: f32 = 2.58;
const ORIENTATION_CAMERA_DISTANCE: f32 = 4.0;
const ORIENTATION_AXIS_LENGTH: f32 = 1.12;
const ORIENTATION_LABEL_OFFSET: f32 = 18.0;
const ORIENTATION_CUBE_SIZE: f32 = ORIENTATION_VIEWPORT_SIZE / ORIENTATION_VIEW_HEIGHT;
const ORIENTATION_SCENE_LAYER: usize = 1;
const ORIENTATION_LABEL_LAYER: usize = 2;
const ORIENTATION_EDGE_BLOCK_WIDTH: f32 = 0.24;
const ORIENTATION_CENTER_BLOCK_LENGTH: f32 = 1.0 - ORIENTATION_EDGE_BLOCK_WIDTH * 2.0;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub(super) struct OrientationGizmos;

#[derive(Component)]
pub(super) struct OrientationAxisLabel {
    axis: Vec3,
}

#[derive(Component)]
pub(super) struct OrientationCamera;

#[derive(Component)]
pub(super) struct OrientationViewTarget {
    direction: Vec3,
}

#[derive(Resource, Default)]
pub(super) struct OrientationInteraction {
    hovered_entity: Option<Entity>,
    pub(super) pointer_over: bool,
}

#[derive(Resource)]
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
) {
    spawn_orientation_cameras(commands);
    spawn_orientation_light(commands);

    let block_materials = orientation_block_materials(materials);
    let base_material = block_materials.base.clone();
    commands.insert_resource(block_materials);

    spawn_orientation_cube_blocks(commands, meshes, &base_material);
    spawn_orientation_axis_labels(commands);
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

    commands.spawn((
        Camera2d,
        Camera {
            order: 11,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(ORIENTATION_LABEL_LAYER),
        Name::new("Orientation Label Camera"),
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

fn orientation_block_materials(
    materials: &mut Assets<StandardMaterial>,
) -> OrientationBlockMaterials {
    let base = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.44, 0.45),
        perceptual_roughness: 0.92,
        unlit: true,
        ..default()
    });
    let hover = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.48, 0.16),
        perceptual_roughness: 0.86,
        unlit: true,
        ..default()
    });

    OrientationBlockMaterials { base, hover }
}

fn spawn_orientation_cube_blocks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    base_material: &Handle<StandardMaterial>,
) {
    for block in orientation_cube_blocks() {
        commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(block.size.x, block.size.z, block.size.y))),
                MeshMaterial3d(base_material.clone()),
                Transform {
                    translation: print_axis_to_world(block.center),
                    ..default()
                },
                RenderLayers::layer(ORIENTATION_SCENE_LAYER),
                OrientationViewTarget {
                    direction: print_axis_to_world(block.direction),
                },
                Name::new(format!("Orientation {} View Block", block.name)),
            ))
            .insert(Pickable::default());
    }
}

fn spawn_orientation_axis_labels(commands: &mut Commands) {
    for (name, axis, color) in [
        ("X", Vec3::X, Color::srgb(0.95, 0.05, 0.04)),
        ("Y", Vec3::Y, Color::srgb(0.05, 0.75, 0.12)),
        ("Z", Vec3::Z, Color::srgb(0.10, 0.18, 1.00)),
    ] {
        commands.spawn((
            Text2d::new(name),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(color),
            TextLayout::new_with_justify(Justify::Center),
            Transform::from_translation(Vec3::ZERO),
            RenderLayers::layer(ORIENTATION_LABEL_LAYER),
            OrientationAxisLabel { axis },
            Name::new(format!("Orientation {name} Label")),
        ));
    }
}

pub(super) fn draw_grid_and_selection(
    selected: Res<SelectedModel>,
    active_tool: Res<ActiveTool>,
    models: Query<(&ImportedModel, &Transform)>,
    mut gizmos: Gizmos,
) {
    draw_ground_grid(&mut gizmos);
    draw_selected_model_tools(&mut gizmos, selected.0, active_tool.is_move(), &models);
}

pub(super) fn draw_orientation_overlay(
    window: Single<&Window>,
    camera: Single<&GlobalTransform, With<OrbitCamera>>,
    mut axis_labels: Query<(&OrientationAxisLabel, &mut Transform)>,
    mut orientation_gizmos: Gizmos<OrientationGizmos>,
) {
    draw_orientation_axes(&mut orientation_gizmos, &window, *camera, &mut axis_labels);
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
    orbit: Single<&mut OrbitCamera>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok(target) = targets.get(click.entity) else {
        return;
    };

    let mut orbit = orbit.into_inner();
    set_orbit_view_direction(&mut orbit, target.direction);
}

pub(super) fn update_orientation_interaction(
    materials: Res<OrientationBlockMaterials>,
    pointer_map: Res<PointerMap>,
    pointer_interactions: Query<&PointerInteraction>,
    mut interaction: ResMut<OrientationInteraction>,
    mut blocks: Query<(
        Entity,
        &OrientationViewTarget,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let hovered = pointer_hovered_orientation_block(&pointer_map, &pointer_interactions, &blocks);
    let hovered_entity = hovered.map(|(entity, _)| entity);

    if interaction.hovered_entity == hovered_entity {
        return;
    }

    interaction.hovered_entity = hovered_entity;
    interaction.pointer_over = hovered_entity.is_some();

    for (entity, _, mut material) in &mut blocks {
        material.0 = if Some(entity) == hovered_entity {
            materials.hover.clone()
        } else {
            materials.base.clone()
        };
    }
}

fn pointer_hovered_orientation_block(
    pointer_map: &PointerMap,
    pointer_interactions: &Query<&PointerInteraction>,
    blocks: &Query<(
        Entity,
        &OrientationViewTarget,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) -> Option<(Entity, Vec3)> {
    pointer_map
        .get_entity(PointerId::Mouse)
        .and_then(|entity| pointer_interactions.get(entity).ok())
        .and_then(PointerInteraction::get_nearest_hit)
        .and_then(|(entity, _)| blocks.get(*entity).ok())
        .map(|(entity, target, _)| (entity, target.direction))
}

fn draw_selected_model_tools(
    gizmos: &mut Gizmos,
    selected_id: Option<u32>,
    show_move_gizmo: bool,
    models: &Query<(&ImportedModel, &Transform)>,
) {
    let Some(selected_id) = selected_id else {
        return;
    };

    for (model, transform) in models {
        if model.id != selected_id {
            continue;
        }

        if !show_move_gizmo && let Some(bounds) = model.bounds {
            draw_model_aabb(gizmos, bounds, transform);
        }
        if show_move_gizmo {
            draw_move_gizmo(gizmos, model, transform);
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

fn draw_orientation_axes(
    gizmos: &mut Gizmos<OrientationGizmos>,
    window: &Window,
    camera_transform: &GlobalTransform,
    axis_labels: &mut Query<(&OrientationAxisLabel, &mut Transform)>,
) {
    let origin = orientation_origin(window);
    let projection = OrientationProjection::from_camera(camera_transform);

    for (axis, color) in [
        (Vec3::X, Color::srgb(0.95, 0.05, 0.04)),
        (Vec3::Y, Color::srgb(0.05, 0.75, 0.12)),
        (Vec3::Z, Color::srgb(0.10, 0.18, 1.00)),
    ] {
        let axis_origin = print_axis_to_world(ORIENTATION_AXIS_CORNER);
        let axis_end = axis_origin + print_axis_to_world(axis) * ORIENTATION_AXIS_LENGTH;
        gizmos.arrow(axis_origin, axis_end, color);
    }
    gizmos.sphere(
        print_axis_to_world(ORIENTATION_AXIS_CORNER),
        0.045,
        Color::srgb(0.84, 0.88, 0.92),
    );

    for (label, mut transform) in axis_labels {
        let axis_end = ORIENTATION_AXIS_CORNER + label.axis * ORIENTATION_AXIS_LENGTH;
        let direction = project_print_axis(label.axis, projection).normalize_or_zero();
        let position = project_print_position(axis_end, origin, projection)
            + direction * ORIENTATION_LABEL_OFFSET;
        transform.translation = Vec3::new(position.x, position.y, 1.0);
    }
}

#[derive(Clone, Copy)]
struct OrientationProjection {
    screen_right: Vec3,
    screen_up: Vec3,
}

impl OrientationProjection {
    fn from_camera(camera_transform: &GlobalTransform) -> Self {
        let rotation = camera_transform.rotation();

        Self {
            screen_right: rotation * Vec3::X,
            screen_up: rotation * Vec3::Y,
        }
    }
}

fn orientation_origin(window: &Window) -> Vec2 {
    Vec2::new(
        -window.width() * 0.5 + ORIENTATION_MARGIN + ORIENTATION_VIEWPORT_SIZE * 0.5,
        -window.height() * 0.5 + ORIENTATION_MARGIN + ORIENTATION_VIEWPORT_SIZE * 0.5,
    )
}

fn project_print_position(position: Vec3, origin: Vec2, projection: OrientationProjection) -> Vec2 {
    origin + project_print_axis(position, projection) * ORIENTATION_CUBE_SIZE
}

fn project_print_axis(axis: Vec3, projection: OrientationProjection) -> Vec2 {
    let world_axis = print_axis_to_world(axis);
    Vec2::new(
        world_axis.dot(projection.screen_right),
        world_axis.dot(projection.screen_up),
    )
}

fn print_axis_to_world(axis: Vec3) -> Vec3 {
    Vec3::new(axis.x, axis.z, axis.y)
}

struct OrientationBlockSeed {
    name: String,
    center: Vec3,
    size: Vec3,
    direction: Vec3,
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
                    center: orientation_block_center(direction),
                    size: orientation_block_size(direction, kind),
                    direction: direction.normalize(),
                });
            }
        }
    }

    blocks
}

fn orientation_block_center(direction: Vec3) -> Vec3 {
    let offset = 0.5 - ORIENTATION_EDGE_BLOCK_WIDTH * 0.5;
    Vec3::new(
        orientation_axis_offset(direction.x, offset),
        orientation_axis_offset(direction.y, offset),
        orientation_axis_offset(direction.z, offset),
    )
}

fn orientation_axis_offset(step: f32, offset: f32) -> f32 {
    if step == 0.0 {
        0.0
    } else {
        step.signum() * offset
    }
}

fn orientation_block_size(direction: Vec3, kind: OrientationBlockKind) -> Vec3 {
    let mut size = Vec3::splat(ORIENTATION_EDGE_BLOCK_WIDTH);

    for axis in 0..3 {
        let step = direction[axis];
        if step != 0.0 {
            continue;
        }

        size[axis] = match kind {
            OrientationBlockKind::Face | OrientationBlockKind::Edge => {
                ORIENTATION_CENTER_BLOCK_LENGTH
            }
            OrientationBlockKind::Corner => ORIENTATION_EDGE_BLOCK_WIDTH,
        };
    }

    size
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
