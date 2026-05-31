use bevy::{color::LinearRgba, prelude::*};

use crate::{
    mesh::{MeshBounds, MeshData},
    state::{self, ModelCommand, ModelInfoSnapshot, TransformSnapshot},
};

use super::{
    interaction::{select_model_on_click, start_model_drag},
    tool::ActiveTool,
};

#[derive(Component)]
pub(super) struct ImportedModel {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) bounds: Option<MeshBounds>,
    pub(super) triangle_count: usize,
    pub(super) volume: f64,
}

#[derive(Resource, Default)]
pub(super) struct SelectedModel(pub(super) Option<u32>);

#[derive(Resource, Default)]
pub(super) struct ModelDrag {
    pub(super) active: Option<ModelDragState>,
}

#[derive(Clone, Copy)]
pub(super) struct ModelDragState {
    pub(super) id: u32,
    pub(super) grab_offset: Vec3,
    pub(super) plane_y: f32,
}

pub(super) fn apply_model_commands(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selected: ResMut<SelectedModel>,
    mut active_tool: ResMut<ActiveTool>,
    mut models: Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for command in state::drain_commands() {
        match command {
            ModelCommand::Load {
                id,
                name,
                mesh,
                transform,
            } => spawn_imported_model(
                &mut commands,
                &mut meshes,
                &mut materials,
                id,
                name,
                mesh,
                transform,
            ),
            ModelCommand::SetTransform { id, transform } => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        *model_transform = transform.to_transform();
                    }
                }
            }
            ModelCommand::SetColor { id, color } => {
                for (_, model, _, material) in &mut models {
                    if model.id == id {
                        if let Some(material) = materials.get_mut(&material.0) {
                            apply_model_color(material, color);
                        }
                        break;
                    }
                }
            }
            ModelCommand::SetTranslation { id, translation } => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        let desired_center = Vec3::from_array(translation);
                        let current_center = model_visual_center(model, &model_transform);
                        model_transform.translation += desired_center - current_center;
                        break;
                    }
                }
            }
            ModelCommand::TranslateBy { id, delta } => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        model_transform.translation += Vec3::from_array(delta);
                        break;
                    }
                }
            }
            ModelCommand::SetRotation {
                id,
                rotation_degrees,
            } => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        set_model_rotation(
                            model,
                            &mut model_transform,
                            print_rotation_degrees_to_world(rotation_degrees),
                        );
                        break;
                    }
                }
            }
            ModelCommand::RotateBy { id, delta_degrees } => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        let delta = print_rotation_degrees_to_world(delta_degrees);
                        let rotation = delta * model_transform.rotation;
                        set_model_rotation(model, &mut model_transform, rotation);
                        break;
                    }
                }
            }
            ModelCommand::CenterOnOrigin(id) => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        center_model_on_origin(model, &mut model_transform);
                        break;
                    }
                }
            }
            ModelCommand::DropToBuildPlate(id) => {
                for (_, model, mut model_transform, _) in &mut models {
                    if model.id == id {
                        drop_model_to_build_plate(model, &mut model_transform);
                        break;
                    }
                }
            }
            ModelCommand::SetActiveTool(mode) => active_tool.set_mode(mode),
            ModelCommand::RemoveAll => {
                for (entity, _, _, _) in &mut models {
                    commands.entity(entity).despawn();
                }
                selected.0 = None;
                active_tool.set_mode(state::ToolModeSpec::None);
            }
            ModelCommand::Select(id) => {
                if models.iter_mut().any(|(_, model, _, _)| model.id == id) {
                    selected.0 = Some(id);
                }
            }
        }
    }
}

fn spawn_imported_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: u32,
    name: String,
    mesh: MeshData,
    transform: state::TransformSpec,
) {
    let default_color = [0.62, 0.69, 0.78];
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(default_color[0], default_color[1], default_color[2]),
        emissive: model_emissive(default_color),
        metallic: 0.0,
        perceptual_roughness: 0.46,
        reflectance: 0.56,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let bounds = mesh.bounds();
    let triangle_count = mesh.positions.len() / 3;
    let volume = mesh.volume();
    let mut model_transform = transform.to_transform();
    if let Some(bounds) = bounds {
        center_model_on_platform(&mut model_transform, bounds.center);
    }

    commands
        .spawn((
            Mesh3d(meshes.add(mesh.into_mesh())),
            MeshMaterial3d(material),
            model_transform,
            Name::new(name.clone()),
            ImportedModel {
                id,
                name,
                bounds,
                triangle_count,
                volume,
            },
        ))
        .observe(select_model_on_click)
        .observe(start_model_drag);
}

pub(super) fn update_api_state_from_scene(
    selected: Res<SelectedModel>,
    models: Query<(&ImportedModel, &Transform)>,
) {
    state::sync_api_state(
        selected.0,
        models.iter().map(|(model, transform)| {
            (
                model.id,
                TransformSnapshot::from_transform_with_visual_position_and_rotation(
                    transform,
                    model_visual_center(model, transform),
                    world_rotation_to_print_degrees(transform.rotation),
                ),
                model_info_snapshot(model, transform),
            )
        }),
    );
}

pub(super) fn model_visual_center(model: &ImportedModel, transform: &Transform) -> Vec3 {
    model
        .bounds
        .map(|bounds| transform.transform_point(bounds.center))
        .unwrap_or(transform.translation)
}

fn center_model_on_platform(transform: &mut Transform, local_center: Vec3) {
    transform.translation -= transform.rotation * (local_center * transform.scale);
}

fn center_model_on_origin(model: &ImportedModel, transform: &mut Transform) {
    let center = model_visual_center(model, transform);
    transform.translation.x -= center.x;
    transform.translation.z -= center.z;
}

fn drop_model_to_build_plate(model: &ImportedModel, transform: &mut Transform) {
    let Some(bounds) = model.bounds else {
        transform.translation.y = 0.0;
        return;
    };

    let min_y = transformed_bounds_min_y(bounds, transform);
    transform.translation.y -= min_y;
}

fn set_model_rotation(model: &ImportedModel, transform: &mut Transform, rotation: Quat) {
    let center = model_visual_center(model, transform);
    transform.rotation = rotation.normalize();
    recenter_model_visual_center(model, transform, center);
}

fn recenter_model_visual_center(model: &ImportedModel, transform: &mut Transform, center: Vec3) {
    if let Some(bounds) = model.bounds {
        transform.translation = center - transform.rotation * (bounds.center * transform.scale);
    } else {
        transform.translation = center;
    }
}

fn transformed_bounds_min_y(bounds: MeshBounds, transform: &Transform) -> f32 {
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

    local_corners
        .into_iter()
        .map(|corner| transform.transform_point(corner).y)
        .fold(f32::INFINITY, f32::min)
}

fn print_rotation_degrees_to_world(rotation_degrees: [f32; 3]) -> Quat {
    let print_rotation = Quat::from_euler(
        EulerRot::XYZ,
        rotation_degrees[0].to_radians(),
        rotation_degrees[1].to_radians(),
        rotation_degrees[2].to_radians(),
    );
    let world_from_print = print_rotation_basis();
    let print_from_world = world_from_print.transpose();
    Quat::from_mat3(&(world_from_print * Mat3::from_quat(print_rotation) * print_from_world))
        .normalize()
}

fn world_rotation_to_print_degrees(rotation: Quat) -> [f32; 3] {
    let world_from_print = print_rotation_basis();
    let print_from_world = world_from_print.transpose();
    let print_rotation = Quat::from_mat3(
        &(print_from_world * Mat3::from_quat(rotation.normalize()) * world_from_print),
    )
    .normalize();
    let (x, y, z) = print_rotation.to_euler(EulerRot::XYZ);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

fn print_rotation_basis() -> Mat3 {
    Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)
}

fn model_info_snapshot(model: &ImportedModel, transform: &Transform) -> ModelInfoSnapshot {
    let size = model
        .bounds
        .map(|bounds| bounds.size * transform.scale.abs())
        .unwrap_or(Vec3::ZERO);
    let volume = model.volume
        * f64::from(transform.scale.x.abs())
        * f64::from(transform.scale.y.abs())
        * f64::from(transform.scale.z.abs());

    ModelInfoSnapshot {
        name: model.name.clone(),
        size: size.to_array(),
        volume,
        triangle_count: model.triangle_count,
    }
}

fn apply_model_color(material: &mut StandardMaterial, color: [f32; 3]) {
    material.base_color = Color::srgb(color[0], color[1], color[2]);
    material.emissive = model_emissive(color);
}

fn model_emissive(color: [f32; 3]) -> LinearRgba {
    let linear = Color::srgb(color[0], color[1], color[2]).to_linear();
    LinearRgba::rgb(
        linear.red * 0.018,
        linear.green * 0.018,
        linear.blue * 0.018,
    )
}
