use std::{cell::RefCell, collections::HashMap};

use bevy::prelude::*;

use crate::mesh::MeshData;

thread_local! {
    static PENDING_COMMANDS: RefCell<Vec<ModelCommand>> = const { RefCell::new(Vec::new()) };
    static API_STATE: RefCell<ApiState> = RefCell::new(ApiState::default());
    static ROTATION_FOCUS_AXIS: RefCell<Option<usize>> = const { RefCell::new(None) };
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
    SetColor {
        id: u32,
        color: [f32; 3],
    },
    SetTranslation {
        id: u32,
        translation: [f32; 3],
    },
    TranslateBy {
        id: u32,
        delta: [f32; 3],
    },
    SetRotation {
        id: u32,
        rotation_degrees: [f32; 3],
    },
    RotateBy {
        id: u32,
        delta_degrees: [f32; 3],
    },
    SetScale {
        id: u32,
        scale: [f32; 3],
    },
    CenterOnOrigin(u32),
    DropToBuildPlate(u32),
    SetActiveTool(ToolModeSpec),
    Remove(u32),
    RemoveAll,
    Select(u32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ToolModeSpec {
    #[default]
    None,
    Move,
    Rotate,
    Scale,
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
    visual_position: [f32; 3],
    rotation_degrees: [f32; 3],
    rotation_xyzw: [f32; 4],
    scale: [f32; 3],
}

impl TransformSnapshot {
    pub(crate) fn from_transform(transform: &Transform) -> Self {
        Self::from_transform_with_visual_position(transform, transform.translation)
    }

    pub(crate) fn from_transform_with_visual_position(
        transform: &Transform,
        visual_position: Vec3,
    ) -> Self {
        let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
        let rotation_degrees = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
        Self::from_transform_with_visual_position_and_rotation(
            transform,
            visual_position,
            rotation_degrees,
        )
    }

    pub(crate) fn from_transform_with_visual_position_and_rotation(
        transform: &Transform,
        visual_position: Vec3,
        rotation_degrees: [f32; 3],
    ) -> Self {
        Self {
            translation: transform.translation.to_array(),
            visual_position: visual_position.to_array(),
            rotation_degrees,
            rotation_xyzw: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
        }
    }

    fn to_json(self) -> String {
        format!(
            "{{\"translation\":[{},{},{}],\"visual_position\":[{},{},{}],\"rotation_degrees\":[{},{},{}],\"rotation_xyzw\":[{},{},{},{}],\"scale\":[{},{},{}]}}",
            self.translation[0],
            self.translation[1],
            self.translation[2],
            self.visual_position[0],
            self.visual_position[1],
            self.visual_position[2],
            self.rotation_degrees[0],
            self.rotation_degrees[1],
            self.rotation_degrees[2],
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

#[derive(Clone, Debug)]
pub(crate) struct ModelInfoSnapshot {
    pub(crate) name: String,
    pub(crate) size: [f32; 3],
    pub(crate) volume: f64,
    pub(crate) triangle_count: usize,
}

impl ModelInfoSnapshot {
    fn to_json(&self) -> String {
        format!(
            "{{\"name\":{},\"size\":[{},{},{}],\"volume\":{},\"triangle_count\":{}}}",
            json_string(&self.name),
            self.size[0],
            self.size[1],
            self.size[2],
            self.volume,
            self.triangle_count
        )
    }
}

#[derive(Default)]
struct ApiState {
    next_id: u32,
    selected: Option<u32>,
    transforms: HashMap<u32, TransformSnapshot>,
    model_infos: HashMap<u32, ModelInfoSnapshot>,
    last_error: String,
}

pub(crate) fn push_command(command: ModelCommand) {
    PENDING_COMMANDS.with(|commands| commands.borrow_mut().push(command));
}

pub(crate) fn drain_commands() -> Vec<ModelCommand> {
    PENDING_COMMANDS.with(|queue| queue.borrow_mut().drain(..).collect())
}

pub(crate) fn set_rotation_focus_axis(axis: Option<usize>) {
    ROTATION_FOCUS_AXIS.with(|focus| *focus.borrow_mut() = axis.filter(|axis| *axis < 3));
}

pub(crate) fn rotation_focus_axis() -> Option<usize> {
    ROTATION_FOCUS_AXIS.with(|focus| *focus.borrow())
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
    let changed = API_STATE.with(|state| update_selected(&mut state.borrow_mut(), id));
    if changed {
        notify_selection_changed(id);
    }
}

pub(crate) fn clear_api_models() {
    let changed = API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let changed = update_selected(&mut state, None);
        state.transforms.clear();
        state.model_infos.clear();
        changed
    });
    if changed {
        notify_selection_changed(None);
    }
}

pub(crate) fn forget_api_model(id: u32) {
    let changed = API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.transforms.remove(&id);
        state.model_infos.remove(&id);
        if state.selected == Some(id) {
            update_selected(&mut state, None)
        } else {
            false
        }
    });
    if changed {
        notify_selection_changed(None);
    }
}

pub(crate) fn sync_api_state(
    selected: Option<u32>,
    models: impl IntoIterator<Item = (u32, TransformSnapshot, ModelInfoSnapshot)>,
) {
    let changed = API_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.transforms.clear();
        state.model_infos.clear();
        for (id, transform, info) in models {
            state.transforms.insert(id, transform);
            state.model_infos.insert(id, info);
        }
        update_selected(&mut state, selected)
    });
    if changed {
        notify_selection_changed(selected);
    }
}

fn update_selected(state: &mut ApiState, selected: Option<u32>) -> bool {
    if state.selected == selected {
        return false;
    }

    state.selected = selected;
    true
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

pub fn model_info_json(id: u32) -> String {
    API_STATE.with(|state| {
        state
            .borrow()
            .model_infos
            .get(&id)
            .map(ModelInfoSnapshot::to_json)
            .unwrap_or_else(|| "null".to_string())
    })
}

pub fn selected_model_id() -> u32 {
    API_STATE.with(|state| state.borrow().selected.unwrap_or(0))
}

pub fn last_error() -> String {
    API_STATE.with(|state| state.borrow().last_error.clone())
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
