use std::{cell::RefCell, collections::HashMap};

use bevy::prelude::*;

use crate::mesh::MeshData;

thread_local! {
    static PENDING_COMMANDS: RefCell<Vec<ModelCommand>> = const { RefCell::new(Vec::new()) };
    static API_STATE: RefCell<ApiState> = RefCell::new(ApiState::default());
}

#[derive(Clone, Debug)]
pub(crate) enum ModelCommand {
    Load {
        id: u32,
        name: String,
        mesh: MeshData,
        transform: TransformSpec,
    },
    SetTransform {
        id: u32,
        transform: TransformSpec,
    },
    CenterOnOrigin(u32),
    DropToBuildPlate(u32),
    SetActiveTool(ToolModeSpec),
    RemoveAll,
    Select(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ToolModeSpec {
    #[default]
    None,
    Move,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TransformSpec {
    pub(crate) translation: [f32; 3],
    pub(crate) rotation_radians: [f32; 3],
    pub(crate) scale: [f32; 3],
}

impl Default for TransformSpec {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_radians: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl TransformSpec {
    pub(crate) fn to_transform(self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                self.rotation_radians[0],
                self.rotation_radians[1],
                self.rotation_radians[2],
            ),
            scale: Vec3::from_array(self.scale),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TransformSnapshot {
    translation: [f32; 3],
    rotation_xyzw: [f32; 4],
    scale: [f32; 3],
}

impl TransformSnapshot {
    pub(crate) fn from_transform(transform: &Transform) -> Self {
        Self {
            translation: transform.translation.to_array(),
            rotation_xyzw: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
        }
    }

    fn to_json(self) -> String {
        format!(
            "{{\"translation\":[{},{},{}],\"rotation_xyzw\":[{},{},{},{}],\"scale\":[{},{},{}]}}",
            self.translation[0],
            self.translation[1],
            self.translation[2],
            self.rotation_xyzw[0],
            self.rotation_xyzw[1],
            self.rotation_xyzw[2],
            self.rotation_xyzw[3],
            self.scale[0],
            self.scale[1],
            self.scale[2]
        )
    }
}

#[derive(Default)]
struct ApiState {
    next_id: u32,
    selected: Option<u32>,
    transforms: HashMap<u32, TransformSnapshot>,
    last_error: String,
}

pub(crate) fn push_command(command: ModelCommand) {
    PENDING_COMMANDS.with(|commands| commands.borrow_mut().push(command));
}

pub(crate) fn drain_commands() -> Vec<ModelCommand> {
    PENDING_COMMANDS.with(|queue| queue.borrow_mut().drain(..).collect())
}

pub(crate) fn allocate_model_id() -> u32 {
    API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.next_id = state.next_id.saturating_add(1).max(1);
        state.next_id
    })
}

pub(crate) fn record_error(error: String) -> String {
    API_STATE.with(|state| state.borrow_mut().last_error = error.clone());
    error
}

pub(crate) fn remember_queued_model_transform(id: u32, transform: Transform) {
    API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state
            .transforms
            .insert(id, TransformSnapshot::from_transform(&transform));
    });
}

pub(crate) fn remember_selection(id: Option<u32>) {
    API_STATE.with(|state| update_selected(&mut state.borrow_mut(), id));
}

pub(crate) fn clear_api_models() {
    API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        update_selected(&mut state, None);
        state.transforms.clear();
    });
}

pub(crate) fn sync_api_state(
    selected: Option<u32>,
    transforms: impl IntoIterator<Item = (u32, TransformSnapshot)>,
) {
    API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        update_selected(&mut state, selected);
        state.transforms.clear();
        state.transforms.extend(transforms);
    });
}

fn update_selected(state: &mut ApiState, selected: Option<u32>) {
    if state.selected == selected {
        return;
    }

    state.selected = selected;
    notify_selection_changed(selected);
}

fn notify_selection_changed(selected: Option<u32>) {
    let selected_id = selected.unwrap_or(0);
    dispatch_selection_changed(selected_id);
}

#[cfg(target_arch = "wasm32")]
fn dispatch_selection_changed(selected_id: u32) {
    browser_events::dispatch_selection_changed(selected_id);
}

#[cfg(not(target_arch = "wasm32"))]
fn dispatch_selection_changed(_selected_id: u32) {}

#[cfg(target_arch = "wasm32")]
mod browser_events {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(
        inline_js = "export function dispatch_selection_changed(selectedModelId) {
            window.dispatchEvent(new CustomEvent('bevy-transform-tools:selection-change', {
                detail: { selectedModelId }
            }));
        }"
    )]
    extern "C" {
        pub(super) fn dispatch_selection_changed(selected_model_id: u32);
    }
}

pub fn transform_json(id: u32) -> String {
    API_STATE.with(|state| {
        state
            .borrow()
            .transforms
            .get(&id)
            .map(|transform| transform.to_json())
            .unwrap_or_else(|| "null".to_string())
    })
}

pub fn selected_model_id() -> u32 {
    API_STATE.with(|state| state.borrow().selected.unwrap_or(0))
}

pub fn last_error() -> String {
    API_STATE.with(|state| state.borrow().last_error.clone())
}
