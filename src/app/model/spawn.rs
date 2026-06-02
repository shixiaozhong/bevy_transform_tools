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
