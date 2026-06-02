mod commands;
mod info;
mod spawn;
mod transform;

use bevy::prelude::*;

use crate::mesh::MeshBounds;

pub(super) use commands::apply_model_commands;
pub(super) use info::update_api_state_from_scene;
pub(super) use transform::{model_visual_center, model_world_bounds, selected_world_bounds};

#[derive(Component)]
pub(super) struct ImportedModel {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) bounds: Option<MeshBounds>,
    pub(super) triangle_count: usize,
    pub(super) volume: f64,
}

#[derive(Resource, Default)]
pub(super) struct SelectedModel {
    ids: Vec<u32>,
}

impl SelectedModel {
    pub(super) fn ids(&self) -> &[u32] {
        &self.ids
    }

    pub(super) fn primary(&self) -> Option<u32> {
        self.ids.last().copied()
    }

    pub(super) fn contains(&self, id: u32) -> bool {
        self.ids.contains(&id)
    }

    pub(super) fn len(&self) -> usize {
        self.ids.len()
    }

    pub(super) fn set_single(&mut self, id: u32) {
        self.ids.clear();
        self.ids.push(id);
    }

    pub(super) fn toggle(&mut self, id: u32) {
        if let Some(index) = self.ids.iter().position(|selected| *selected == id) {
            self.ids.remove(index);
        } else {
            self.ids.push(id);
        }
    }

    pub(super) fn remove(&mut self, id: u32) {
        self.ids.retain(|selected| *selected != id);
    }

    pub(super) fn clear(&mut self) {
        self.ids.clear();
    }
}

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
