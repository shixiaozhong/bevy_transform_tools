use bevy::prelude::*;

use crate::{
    mesh::{MeshBounds, MeshData},
    state::{self, ModelCommand, TransformSnapshot},
};

use super::interaction::{select_model_on_click, start_model_drag};

#[derive(Component)]
pub(super) struct ImportedModel {
    pub(super) id: u32,
    pub(super) bounds: Option<MeshBounds>,
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
    mut models: Query<(Entity, &ImportedModel, &mut Transform)>,
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
                for (_, model, mut model_transform) in &mut models {
                    if model.id == id {
                        *model_transform = transform.to_transform();
                    }
                }
            }
            ModelCommand::RemoveAll => {
                for (entity, _, _) in &mut models {
                    commands.entity(entity).despawn();
                }
                selected.0 = None;
            }
            ModelCommand::Select(id) => {
                if models.iter_mut().any(|(_, model, _)| model.id == id) {
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
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.69, 0.78),
        metallic: 0.0,
        perceptual_roughness: 0.72,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let bounds = mesh.bounds();
    let mut model_transform = transform.to_transform();
    if let Some(bounds) = bounds {
        center_model_on_platform(&mut model_transform, bounds.center);
    }

    commands
        .spawn((
            Mesh3d(meshes.add(mesh.into_mesh())),
            MeshMaterial3d(material),
            model_transform,
            Name::new(name),
            ImportedModel { id, bounds },
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
        models
            .iter()
            .map(|(model, transform)| (model.id, TransformSnapshot::from_transform(transform))),
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
