use bevy::{color::LinearRgba, prelude::*};

use crate::{
    mesh::MeshData,
    state::{self},
};

use super::{ImportedModel, transform::center_model_on_platform};
use crate::app::interaction::{select_model_on_click, start_model_drag};

pub(in crate::app::model) fn spawn_imported_model(
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
    let metadata = imported_model_metadata(id, name.clone(), mesh.clone());
    let mut model_transform = transform.to_transform();
    if let Some(bounds) = metadata.bounds {
        center_model_on_platform(&mut model_transform, bounds.center);
    }

    commands
        .spawn((
            Mesh3d(meshes.add(mesh.into_mesh())),
            MeshMaterial3d(material),
            model_transform,
            Name::new(name.clone()),
            metadata,
        ))
        .observe(select_model_on_click)
        .observe(start_model_drag);
}

pub(in crate::app::model) fn imported_model_metadata(
    id: u32,
    name: String,
    mesh: MeshData,
) -> ImportedModel {
    let bounds = mesh.bounds();
    let triangle_count = mesh.positions.len() / 3;
    let volume = mesh.volume();
    ImportedModel {
        id,
        name,
        mesh,
        bounds,
        triangle_count,
        volume,
    }
}

pub(in crate::app::model) fn spawn_cut_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    id: u32,
    name: String,
    mesh: MeshData,
    material: MeshMaterial3d<StandardMaterial>,
) {
    commands
        .spawn((
            Mesh3d(meshes.add(mesh.clone().into_mesh())),
            material,
            Transform::IDENTITY,
            Name::new(name.clone()),
            imported_model_metadata(id, name, mesh),
        ))
        .observe(select_model_on_click)
        .observe(start_model_drag);
}

pub(in crate::app::model) fn apply_model_color(material: &mut StandardMaterial, color: [f32; 3]) {
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
