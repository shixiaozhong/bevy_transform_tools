use bevy::prelude::*;

use crate::state::{self, ModelCommand};

use super::{
    ImportedModel, SelectedModel,
    spawn::{imported_model_metadata, spawn_cut_model, spawn_imported_model},
    transform::{
        center_model_on_origin, drop_model_to_build_plate, model_visual_center, set_model_rotation,
        set_model_scale,
    },
};
use crate::app::{
    coordinates::print_rotation_degrees_to_world,
    selection::{
        rotate_selected_models, scale_factor_component, scale_selected_models,
        selected_bounds_in_scene, selected_center_and_primary_rotation,
        selected_center_and_primary_scale, translate_selected_models,
    },
    tool::ActiveTool,
};
use crate::mesh::{
    MeshData,
    clip::{CutKeep, CutPlane, cut_mesh},
};

pub(in crate::app) fn apply_model_commands(
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
                            super::spawn::apply_model_color(material, color);
                        }
                        break;
                    }
                }
            }
            ModelCommand::SetTranslation { id, translation } => {
                if selected.primary() == Some(id)
                    && selected.len() > 1
                    && let Some((min, max)) = selected_bounds_in_scene(&selected, &mut models)
                {
                    let delta = Vec3::from_array(translation) - (min + max) * 0.5;
                    translate_selected_models(&selected, &mut models, delta);
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            let desired_center = Vec3::from_array(translation);
                            let current_center = model_visual_center(model, &model_transform);
                            model_transform.translation += desired_center - current_center;
                            break;
                        }
                    }
                }
            }
            ModelCommand::TranslateBy { id, delta } => {
                if selected.primary() == Some(id) && selected.len() > 1 {
                    translate_selected_models(&selected, &mut models, Vec3::from_array(delta));
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            model_transform.translation += Vec3::from_array(delta);
                            break;
                        }
                    }
                }
            }
            ModelCommand::SetRotation {
                id,
                rotation_degrees,
            } => {
                if selected.primary() == Some(id)
                    && selected.len() > 1
                    && let Some((center, primary_rotation)) =
                        selected_center_and_primary_rotation(id, &selected, &mut models)
                {
                    let delta = print_rotation_degrees_to_world(rotation_degrees)
                        * primary_rotation.inverse();
                    rotate_selected_models(&selected, &mut models, center, delta);
                } else {
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
            }
            ModelCommand::RotateBy { id, delta_degrees } => {
                let delta = print_rotation_degrees_to_world(delta_degrees);
                if selected.primary() == Some(id)
                    && selected.len() > 1
                    && let Some((center, _)) =
                        selected_center_and_primary_rotation(id, &selected, &mut models)
                {
                    rotate_selected_models(&selected, &mut models, center, delta);
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            let rotation = delta * model_transform.rotation;
                            set_model_rotation(model, &mut model_transform, rotation);
                            break;
                        }
                    }
                }
            }
            ModelCommand::SetScale { id, scale } => {
                if selected.primary() == Some(id)
                    && selected.len() > 1
                    && let Some((center, primary_scale)) =
                        selected_center_and_primary_scale(id, &selected, &mut models)
                {
                    let desired_scale = Vec3::from_array(scale);
                    let factor = Vec3::new(
                        scale_factor_component(desired_scale.x, primary_scale.x),
                        scale_factor_component(desired_scale.y, primary_scale.y),
                        scale_factor_component(desired_scale.z, primary_scale.z),
                    );
                    scale_selected_models(&selected, &mut models, center, factor);
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            set_model_scale(model, &mut model_transform, Vec3::from_array(scale));
                            break;
                        }
                    }
                }
            }
            ModelCommand::CenterOnOrigin(id) => {
                if selected.primary() == Some(id)
                    && selected.len() > 1
                    && let Some((min, max)) = selected_bounds_in_scene(&selected, &mut models)
                {
                    translate_selected_models(&selected, &mut models, -((min + max) * 0.5));
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            center_model_on_origin(model, &mut model_transform);
                            break;
                        }
                    }
                }
            }
            ModelCommand::DropToBuildPlate(id) => {
                if selected.primary() == Some(id) && selected.len() > 1 {
                    for (_, model, mut model_transform, _) in &mut models {
                        if selected.contains(model.id) {
                            drop_model_to_build_plate(model, &mut model_transform);
                        }
                    }
                } else {
                    for (_, model, mut model_transform, _) in &mut models {
                        if model.id == id {
                            drop_model_to_build_plate(model, &mut model_transform);
                            break;
                        }
                    }
                }
            }
            ModelCommand::Cut {
                id,
                plane,
                keep,
                cap,
                new_id,
            } => {
                cut_model_by_plane(
                    &mut commands,
                    &mut meshes,
                    &mut selected,
                    &mut models,
                    id,
                    plane,
                    keep,
                    cap,
                    new_id,
                );
            }
            ModelCommand::SetActiveTool(mode) => set_active_tool(&mut active_tool, mode),
            ModelCommand::Remove(id) => {
                for (entity, model, _, _) in &mut models {
                    if model.id == id {
                        commands.entity(entity).despawn();
                        if selected.contains(id) {
                            selected.remove(id);
                            set_active_tool(&mut active_tool, state::ToolModeSpec::None);
                        }
                        break;
                    }
                }
            }
            ModelCommand::RemoveAll => {
                for (entity, _, _, _) in &mut models {
                    commands.entity(entity).despawn();
                }
                selected.clear();
                set_active_tool(&mut active_tool, state::ToolModeSpec::None);
            }
            ModelCommand::Select(id) => {
                if models.iter_mut().any(|(_, model, _, _)| model.id == id) {
                    selected.set_single(id);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cut_model_by_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    selected: &mut SelectedModel,
    models: &mut Query<(
        Entity,
        &ImportedModel,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    id: u32,
    plane: CutPlane,
    keep: CutKeep,
    cap: bool,
    new_id: Option<u32>,
) {
    for (entity, model, mut transform, material) in models {
        if model.id != id {
            continue;
        }

        let world_mesh = transform_mesh(&model.mesh, &transform);
        let Ok(result) = cut_mesh(&world_mesh, plane, cap) else {
            return;
        };
        if !result.warnings.is_empty() {
            state::record_error(format!(
                "cut completed with warnings: {:?}",
                result.warnings
            ));
        }

        match keep {
            CutKeep::Upper => {
                replace_or_remove_model(
                    commands,
                    meshes,
                    selected,
                    entity,
                    model.id,
                    model.name.clone(),
                    result.upper,
                );
                *transform = Transform::IDENTITY;
            }
            CutKeep::Lower => {
                replace_or_remove_model(
                    commands,
                    meshes,
                    selected,
                    entity,
                    model.id,
                    model.name.clone(),
                    result.lower,
                );
                *transform = Transform::IDENTITY;
            }
            CutKeep::Both => {
                replace_or_remove_model(
                    commands,
                    meshes,
                    selected,
                    entity,
                    model.id,
                    format!("{} lower", model.name),
                    result.lower,
                );
                *transform = Transform::IDENTITY;
                if let (Some(id), Some(upper)) = (new_id, result.upper) {
                    spawn_cut_model(
                        commands,
                        meshes,
                        id,
                        format!("{} upper", model.name),
                        upper,
                        material.clone(),
                    );
                }
            }
        }
        break;
    }
}

fn replace_or_remove_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    selected: &mut SelectedModel,
    entity: Entity,
    id: u32,
    name: String,
    mesh: Option<MeshData>,
) {
    if let Some(mesh) = mesh {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh.clone().into_mesh())),
            Name::new(name.clone()),
            imported_model_metadata(id, name, mesh),
        ));
    } else {
        commands.entity(entity).despawn();
        selected.remove(id);
        state::remember_selected_models(selected.ids().iter().copied());
    }
}

fn transform_mesh(mesh: &MeshData, transform: &Transform) -> MeshData {
    let normal_rotation = transform.rotation;
    MeshData {
        positions: mesh
            .positions
            .iter()
            .map(|position| {
                transform
                    .transform_point(Vec3::from_array(*position))
                    .to_array()
            })
            .collect(),
        normals: mesh
            .normals
            .iter()
            .map(|normal| {
                (normal_rotation * Vec3::from_array(*normal))
                    .normalize_or_zero()
                    .to_array()
            })
            .collect(),
        uvs: mesh.uvs.clone(),
    }
}

fn set_active_tool(active_tool: &mut ActiveTool, mode: state::ToolModeSpec) {
    active_tool.set_mode(mode);
    if mode != state::ToolModeSpec::Cut {
        state::set_cut_preview(state::CutPreviewUpdate {
            visible: false,
            plane: None,
        });
    }
    state::remember_active_tool(mode);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::tool::ActiveTool,
        mesh::{
            MeshBounds, MeshData,
            clip::{CutAxis, CutKeep, CutPlane},
        },
        state::{self, ModelCommand},
    };
    use bevy::ecs::system::RunSystemOnce;

    fn test_model(id: u32, bounds: MeshBounds) -> ImportedModel {
        ImportedModel {
            id,
            name: format!("model-{id}"),
            mesh: Default::default(),
            bounds: Some(bounds),
            triangle_count: 12,
            volume: 1.0,
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            actual.abs_diff_eq(expected, 0.0001),
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn selected_command_app(translations: [Vec3; 2]) -> App {
        let _ = state::drain_commands();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(SelectedModel::default());
        app.insert_resource(ActiveTool::default());

        let material = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial::default())
        };
        let bounds = MeshBounds {
            center: Vec3::ZERO,
            size: Vec3::new(2.0, 2.0, 2.0),
        };

        for (index, translation) in translations.into_iter().enumerate() {
            let id = index as u32 + 1;
            app.world_mut().spawn((
                test_model(id, bounds),
                Transform::from_translation(translation),
                MeshMaterial3d(material.clone()),
            ));
        }

        {
            let mut selected = app.world_mut().resource_mut::<SelectedModel>();
            selected.toggle(1);
            selected.toggle(2);
        }

        app
    }

    fn run_command(app: &mut App, command: ModelCommand) {
        state::push_command(command);
        app.world_mut()
            .run_system_once(apply_model_commands)
            .unwrap();
    }

    fn model_translation(app: &mut App, id: u32) -> Vec3 {
        let world = app.world_mut();
        let mut models = world.query::<(&ImportedModel, &Transform)>();
        models
            .iter(world)
            .find(|(model, _)| model.id == id)
            .map(|(_, transform)| transform.translation)
            .unwrap()
    }

    fn model_scale(app: &mut App, id: u32) -> Vec3 {
        let world = app.world_mut();
        let mut models = world.query::<(&ImportedModel, &Transform)>();
        models
            .iter(world)
            .find(|(model, _)| model.id == id)
            .map(|(_, transform)| transform.scale)
            .unwrap()
    }

    fn model_snapshot(app: &mut App, id: u32) -> (Option<MeshBounds>, f64, Transform) {
        let world = app.world_mut();
        let mut models = world.query::<(&ImportedModel, &Transform)>();
        models
            .iter(world)
            .find(|(model, _)| model.id == id)
            .map(|(model, transform)| (model.bounds, model.volume, *transform))
            .unwrap()
    }

    fn cube_mesh() -> MeshData {
        let p = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mut mesh = MeshData::default();
        for triangle in triangles {
            for index in triangle {
                mesh.positions.push(p[index]);
            }
        }
        mesh
    }

    #[test]
    fn set_translation_moves_selected_group_center() {
        let mut app = selected_command_app([Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)]);

        run_command(
            &mut app,
            ModelCommand::SetTranslation {
                id: 2,
                translation: [10.0, 0.0, 0.0],
            },
        );

        assert_vec3_close(model_translation(&mut app, 1), Vec3::new(8.0, 0.0, 0.0));
        assert_vec3_close(model_translation(&mut app, 2), Vec3::new(12.0, 0.0, 0.0));
    }

    #[test]
    fn rotate_by_rotates_selected_group_around_group_center() {
        let mut app = selected_command_app([Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)]);

        run_command(
            &mut app,
            ModelCommand::RotateBy {
                id: 2,
                delta_degrees: [0.0, 0.0, 90.0],
            },
        );

        let first = model_translation(&mut app, 1);
        let second = model_translation(&mut app, 2);
        assert_vec3_close((first + second) * 0.5, Vec3::new(2.0, 0.0, 0.0));
        assert_close(first.distance(second), 4.0);
    }

    #[test]
    fn set_scale_scales_selected_group_around_group_center() {
        let mut app = selected_command_app([Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)]);

        run_command(
            &mut app,
            ModelCommand::SetScale {
                id: 2,
                scale: [2.0, 2.0, 2.0],
            },
        );

        assert_vec3_close(model_translation(&mut app, 1), Vec3::new(-2.0, 0.0, 0.0));
        assert_vec3_close(model_translation(&mut app, 2), Vec3::new(6.0, 0.0, 0.0));
        assert_vec3_close(model_scale(&mut app, 1), Vec3::splat(2.0));
        assert_vec3_close(model_scale(&mut app, 2), Vec3::splat(2.0));
    }

    #[test]
    fn center_on_origin_moves_selected_group_center_to_origin() {
        let mut app = selected_command_app([Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)]);

        run_command(&mut app, ModelCommand::CenterOnOrigin(2));

        assert_vec3_close(model_translation(&mut app, 1), Vec3::new(-2.0, 0.0, 0.0));
        assert_vec3_close(model_translation(&mut app, 2), Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn drop_to_build_plate_drops_each_selected_model_independently_by_command() {
        let mut app = selected_command_app([Vec3::new(0.0, 5.0, 0.0), Vec3::new(4.0, -3.0, 0.0)]);

        run_command(&mut app, ModelCommand::DropToBuildPlate(2));

        assert_vec3_close(model_translation(&mut app, 1), Vec3::new(0.0, 1.0, 0.0));
        assert_vec3_close(model_translation(&mut app, 2), Vec3::new(4.0, 1.0, 0.0));
    }

    #[test]
    fn cut_command_replaces_model_with_capped_half() {
        let _ = state::drain_commands();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(SelectedModel::default());
        app.insert_resource(ActiveTool::default());

        let material = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial::default())
        };
        let mesh = cube_mesh();
        app.world_mut().spawn((
            test_model(
                1,
                MeshBounds {
                    center: Vec3::splat(0.5),
                    size: Vec3::ONE,
                },
            ),
            Transform::IDENTITY,
            MeshMaterial3d(material),
        ));
        {
            let mut model = app
                .world_mut()
                .query::<&mut ImportedModel>()
                .single_mut(app.world_mut())
                .unwrap();
            model.mesh = mesh;
        }

        run_command(
            &mut app,
            ModelCommand::Cut {
                id: 1,
                plane: CutPlane {
                    axis: CutAxis::Z,
                    position: 0.5,
                },
                keep: CutKeep::Upper,
                cap: true,
                new_id: None,
            },
        );

        let (bounds, volume, transform) = model_snapshot(&mut app, 1);
        assert_vec3_close(bounds.unwrap().size, Vec3::new(1.0, 1.0, 0.5));
        assert_close(volume as f32, 0.5);
        assert_vec3_close(transform.translation, Vec3::ZERO);
    }
}
