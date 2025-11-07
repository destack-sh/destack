use std::collections::HashMap;

use crate::{Module, ModuleId};

/// A graph of Modules.
#[derive(Debug, Clone)]
pub struct ModuleGraph {
    /// The modules by id.
    modules_by_id: HashMap<ModuleId, Module>,
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleGraph {
    /// Create a new ModuleGraph.
    pub fn new() -> Self {
        Self {
            modules_by_id: HashMap::new(),
        }
    }

    /// Iterate over the modules in the graph.
    pub fn iter(&self) -> impl Iterator<Item = &Module> {
        self.modules_by_id.values()
    }
}
