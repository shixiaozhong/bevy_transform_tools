use bevy::prelude::*;

use crate::state::ToolModeSpec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ToolMode {
    #[default]
    None,
    Move,
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
        };
    }

    pub(super) fn is_move(&self) -> bool {
        self.mode == ToolMode::Move
    }
}
