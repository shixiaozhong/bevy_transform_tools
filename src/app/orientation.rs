use bevy::{
    camera::{ScalingMode, Viewport, visibility::RenderLayers},
    prelude::*,
};

mod interaction;
mod mesh;
mod textures;

pub(super) use interaction::{orient_camera_from_cube_click, update_orientation_interaction};

use super::{
    camera::OrbitCamera,
    coordinates::{print_axis_colors, print_axis_to_world},
};
use mesh::{orientation_axis_label_mesh, orientation_block_mesh};
use textures::{orientation_face_texture_paths, orientation_label_texture};

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

struct OrientationBlockSeed {
    name: String,
    direction: Vec3,
    kind: OrientationBlockKind,
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

fn draw_orientation_axes(gizmos: &mut Gizmos<OrientationGizmos>) {
    for (axis, color) in print_axis_colors() {
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

const ORIENTATION_AXIS_CORNER: Vec3 = Vec3::new(-0.5, -0.5, -0.5);
