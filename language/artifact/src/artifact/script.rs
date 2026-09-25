use serde::{Deserialize, Serialize};
use tspp_js::Module;
use tspp_serde::Reflect;

use crate::SourceMap;

/// One structured script linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Script {
    /// The structured JavaScript module.
    pub module: Module,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
    /// Whether this script has top level side effects.
    pub has_top_level_side_effects: bool,
}

impl Script {
    /// Return the structured JavaScript module.
    pub const fn module(&self) -> &Module {
        &self.module
    }

    /// Consume this script into its structured JavaScript module.
    pub fn into_module(self) -> Module {
        self.module
    }

    /// Replace the structured JavaScript module.
    pub fn replace_module(&mut self, module: Module) {
        self.module = module;
    }
}
