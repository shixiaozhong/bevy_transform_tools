use bevy::prelude::*;

use crate::state::ToolModeSpec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ToolMode {
    #[default]
    None,
    Move,
    Rotate,
    Scale,
}

#[derive(Resource, Default)]
pub(super) struct ActiveTool {
    mode: ToolMode,
}

impl ActiveTool {
    pub(super) fn set_mode(&mut self, mode: ToolModeSpec) {
        self.mode = match mode {
            ToolModeSpec::None => ToolMode::None,
            ToolModeSpec::Move => ToolMode::Move,
            ToolModeSpec::Rotate => ToolMode::Rotate,
            ToolModeSpec::Scale => ToolMode::Scale,
        };
    }

    pub(super) fn is_move(&self) -> bool {
        self.mode == ToolMode::Move
    }

    pub(super) fn is_rotate(&self) -> bool {
        self.mode == ToolMode::Rotate
    }

    pub(super) fn is_scale(&self) -> bool {
        self.mode == ToolMode::Scale
    }
}
