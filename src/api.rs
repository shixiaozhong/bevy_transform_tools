use crate::{
    importers::{load_obj_mesh, load_stl_mesh},
    mesh::MeshData,
    state::{
        self, ModelCommand, TransformSpec, clear_api_models, push_command, record_error,
        remember_queued_model_transform, remember_selection,
    },
};

pub fn queue_obj_model(name: impl Into<String>, source: &str) -> Result<u32, String> {
    let mesh = load_obj_mesh(source).map_err(record_error)?;
    queue_mesh_model(name.into(), mesh)
}

pub fn queue_stl_model(name: impl Into<String>, bytes: &[u8]) -> Result<u32, String> {
    let mesh = load_stl_mesh(bytes).map_err(record_error)?;
    queue_mesh_model(name.into(), mesh)
}

pub fn select_model(id: u32) {
    push_command(ModelCommand::Select(id));
    remember_selection(Some(id));
}

pub fn clear_models() {
    push_command(ModelCommand::RemoveAll);
    clear_api_models();
}

#[allow(clippy::too_many_arguments)]
pub fn set_model_transform(
    id: u32,
    tx: f32,
    ty: f32,
    tz: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    sx: f32,
    sy: f32,
    sz: f32,
) {
    let transform = TransformSpec {
        translation: [tx, ty, tz],
        rotation_radians: [rx, ry, rz],
        scale: [sx, sy, sz],
    };

    push_command(ModelCommand::SetTransform { id, transform });
}

pub use state::{last_error, selected_model_id, transform_json};

fn queue_mesh_model(name: String, mesh: MeshData) -> Result<u32, String> {
    let id = state::allocate_model_id();
    let transform = TransformSpec::default();
    push_command(ModelCommand::Load {
        id,
        name,
        mesh,
        transform,
    });
    remember_queued_model_transform(id, transform.to_transform());
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use crate::run_app;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() {
        run_app();
    }

    #[wasm_bindgen]
    pub fn load_obj_model(name: &str, source: &str) -> Result<u32, JsValue> {
        queue_obj_model(name, source).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen]
    pub fn load_stl_model(name: &str, bytes: &[u8]) -> Result<u32, JsValue> {
        queue_stl_model(name, bytes).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen]
    pub fn select_model_by_id(id: u32) {
        select_model(id);
    }

    #[wasm_bindgen]
    pub fn clear_scene_models() {
        clear_models();
    }

    #[wasm_bindgen]
    pub fn get_selected_model_id() -> u32 {
        selected_model_id()
    }

    #[wasm_bindgen]
    pub fn get_model_transform_json(id: u32) -> String {
        transform_json(id)
    }

    #[wasm_bindgen]
    pub fn get_last_error() -> String {
        last_error()
    }

    #[wasm_bindgen]
    #[allow(clippy::too_many_arguments)]
    pub fn set_transform(
        id: u32,
        tx: f32,
        ty: f32,
        tz: f32,
        rx: f32,
        ry: f32,
        rz: f32,
        sx: f32,
        sy: f32,
        sz: f32,
    ) {
        set_model_transform(id, tx, ty, tz, rx, ry, rz, sx, sy, sz);
    }
}
