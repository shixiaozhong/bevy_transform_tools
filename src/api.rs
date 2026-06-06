use crate::{
    importers::{load_obj_mesh, load_stl_mesh},
    mesh::{
        MeshData,
        clip::{CutAxis, CutKeep, CutPlane},
    },
    state::{
        self, CutPreviewUpdate, ModelCommand, ToolModeSpec, TransformSpec, clear_api_models,
        push_command, record_error, remember_queued_model_transform, remember_selection,
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

pub fn remove_model(id: u32) {
    push_command(ModelCommand::Remove(id));
    state::forget_api_model(id);
}

pub fn center_model_on_origin(id: u32) {
    push_command(ModelCommand::CenterOnOrigin(id));
}

pub fn drop_model_to_build_plate(id: u32) {
    push_command(ModelCommand::DropToBuildPlate(id));
}

pub fn cut_model(id: u32, axis: i32, position: f32, keep: i32, cap: bool) -> Result<u32, String> {
    let axis = parse_cut_axis(axis)?;
    let keep = parse_cut_keep(keep)?;
    let new_id = (keep == CutKeep::Both).then(state::allocate_model_id);
    push_command(ModelCommand::Cut {
        id,
        plane: CutPlane {
            axis,
            position: sanitize_position_component(position),
        },
        keep,
        cap,
        new_id,
    });
    Ok(new_id.unwrap_or(0))
}

pub fn activate_move_tool() {
    push_command(ModelCommand::SetActiveTool(ToolModeSpec::Move));
}

pub fn activate_rotate_tool() {
    push_command(ModelCommand::SetActiveTool(ToolModeSpec::Rotate));
}

pub fn activate_scale_tool() {
    push_command(ModelCommand::SetActiveTool(ToolModeSpec::Scale));
}

pub fn activate_cut_tool() {
    push_command(ModelCommand::SetActiveTool(ToolModeSpec::Cut));
}

pub fn clear_active_tool() {
    push_command(ModelCommand::SetActiveTool(ToolModeSpec::None));
}

pub fn set_cut_preview_plane(axis: i32, position: f32, visible: bool) -> Result<(), String> {
    let axis = parse_cut_axis(axis)?;
    state::set_cut_preview(CutPreviewUpdate {
        visible,
        plane: Some(CutPlane {
            axis,
            position: sanitize_position_component(position),
        }),
    });
    Ok(())
}

pub fn clear_cut_preview_plane() {
    state::set_cut_preview(CutPreviewUpdate {
        visible: false,
        plane: None,
    });
}

pub fn set_model_color(id: u32, red: f32, green: f32, blue: f32) {
    push_command(ModelCommand::SetColor {
        id,
        color: [red, green, blue].map(sanitize_color_channel),
    });
}

pub fn set_model_position(id: u32, x: f32, y: f32, z: f32) {
    push_command(ModelCommand::SetTranslation {
        id,
        translation: [x, y, z].map(sanitize_position_component),
    });
}

pub fn move_model_by(id: u32, x: f32, y: f32, z: f32) {
    push_command(ModelCommand::TranslateBy {
        id,
        delta: [x, y, z].map(sanitize_position_component),
    });
}

pub fn set_model_rotation(id: u32, x_degrees: f32, y_degrees: f32, z_degrees: f32) {
    push_command(ModelCommand::SetRotation {
        id,
        rotation_degrees: [x_degrees, y_degrees, z_degrees].map(sanitize_position_component),
    });
}

pub fn rotate_model_by(id: u32, x_degrees: f32, y_degrees: f32, z_degrees: f32) {
    push_command(ModelCommand::RotateBy {
        id,
        delta_degrees: [x_degrees, y_degrees, z_degrees].map(sanitize_position_component),
    });
}

pub fn set_rotation_focus_axis(axis: i32) {
    state::set_rotation_focus_axis(usize::try_from(axis).ok());
}

pub fn set_model_scale(id: u32, x: f32, y: f32, z: f32) {
    push_command(ModelCommand::SetScale {
        id,
        scale: [x, y, z].map(sanitize_scale_component),
    });
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

pub use state::{
    last_error, model_info_json, selected_bounds_json, selected_model_id, transform_json,
};

fn sanitize_color_channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_position_component(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_scale_component(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.001, 1000.0)
    } else {
        1.0
    }
}

fn parse_cut_axis(axis: i32) -> Result<CutAxis, String> {
    match axis {
        0 => Ok(CutAxis::X),
        1 => Ok(CutAxis::Y),
        2 => Ok(CutAxis::Z),
        _ => Err(record_error(format!("invalid cut axis {axis}"))),
    }
}

fn parse_cut_keep(keep: i32) -> Result<CutKeep, String> {
    match keep {
        0 => Ok(CutKeep::Upper),
        1 => Ok(CutKeep::Lower),
        2 => Ok(CutKeep::Both),
        _ => Err(record_error(format!("invalid cut keep mode {keep}"))),
    }
}

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
    pub fn delete_model(id: u32) {
        remove_model(id);
    }

    #[wasm_bindgen]
    pub fn center_model(id: u32) {
        center_model_on_origin(id);
    }

    #[wasm_bindgen]
    pub fn drop_model_to_platform(id: u32) {
        drop_model_to_build_plate(id);
    }

    #[wasm_bindgen]
    pub fn cut_model_by_plane(
        id: u32,
        axis: i32,
        position: f32,
        keep: i32,
        cap: bool,
    ) -> Result<u32, JsValue> {
        cut_model(id, axis, position, keep, cap).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen]
    pub fn activate_move_tool_mode() {
        activate_move_tool();
    }

    #[wasm_bindgen]
    pub fn activate_rotate_tool_mode() {
        activate_rotate_tool();
    }

    #[wasm_bindgen]
    pub fn activate_scale_tool_mode() {
        activate_scale_tool();
    }

    #[wasm_bindgen]
    pub fn activate_cut_tool_mode() {
        activate_cut_tool();
    }

    #[wasm_bindgen]
    pub fn clear_tool_mode() {
        clear_active_tool();
    }

    #[wasm_bindgen]
    pub fn set_cut_preview_plane_by_axis(
        axis: i32,
        position: f32,
        visible: bool,
    ) -> Result<(), JsValue> {
        set_cut_preview_plane(axis, position, visible).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen]
    pub fn clear_cut_preview() {
        clear_cut_preview_plane();
    }

    #[wasm_bindgen]
    pub fn set_color(id: u32, red: f32, green: f32, blue: f32) {
        set_model_color(id, red, green, blue);
    }

    #[wasm_bindgen]
    pub fn set_position(id: u32, x: f32, y: f32, z: f32) {
        set_model_position(id, x, y, z);
    }

    #[wasm_bindgen]
    pub fn move_by(id: u32, x: f32, y: f32, z: f32) {
        move_model_by(id, x, y, z);
    }

    #[wasm_bindgen]
    pub fn set_rotation(id: u32, x_degrees: f32, y_degrees: f32, z_degrees: f32) {
        set_model_rotation(id, x_degrees, y_degrees, z_degrees);
    }

    #[wasm_bindgen]
    pub fn rotate_by_degrees(id: u32, x_degrees: f32, y_degrees: f32, z_degrees: f32) {
        rotate_model_by(id, x_degrees, y_degrees, z_degrees);
    }

    #[wasm_bindgen]
    pub fn set_rotate_focus_axis(axis: i32) {
        set_rotation_focus_axis(axis);
    }

    #[wasm_bindgen]
    pub fn set_scale(id: u32, x: f32, y: f32, z: f32) {
        set_model_scale(id, x, y, z);
    }

    #[wasm_bindgen]
    pub fn get_selected_model_id() -> u32 {
        selected_model_id()
    }

    #[wasm_bindgen]
    pub fn get_selected_bounds_json() -> String {
        selected_bounds_json()
    }

    #[wasm_bindgen]
    pub fn get_model_transform_json(id: u32) -> String {
        transform_json(id)
    }

    #[wasm_bindgen]
    pub fn get_model_info_json(id: u32) -> String {
        model_info_json(id)
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
