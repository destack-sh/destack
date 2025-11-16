use parking_lot::Mutex;
use std::collections::HashMap;

use dyst_source::FileId;

use crate::{Module, ModuleId};

/// Inner state for ModuleRegistry. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
struct MutableModuleRegistry {
    /// The modules by id.
    modules_by_id: HashMap<ModuleId, Module>,
    /// The modules by file id.
    modules_by_file_id: HashMap<FileId, ModuleId>,
    /// The next module id.
    next_module_id: u32,
}

/// Graph of Modules (including their underlying Files). THREAD-SAFE.
#[derive(Debug)]
pub struct SharedModuleRegistry {
    state: Mutex<MutableModuleRegistry>,
}

impl Clone for SharedModuleRegistry {
    fn clone(&self) -> Self {
        let state = self.state.lock().clone();
        Self {
            state: Mutex::new(state),
        }
    }
}

impl Default for SharedModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedModuleRegistry {
    /// Create a new ModuleRegistry.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(MutableModuleRegistry {
                modules_by_id: HashMap::new(),
                modules_by_file_id: HashMap::new(),
                next_module_id: 0,
            }),
        }
    }

    /// Get and increment the next module id.
    pub fn next_id(&self) -> ModuleId {
        let mut state = self.state.lock();
        let id = ModuleId::new(state.next_module_id);
        state.next_module_id += 1;
        id
    }

    /// Insert a module into the graph.
    pub fn insert(&self, module: Module) {
        let mut state = self.state.lock();
        state.modules_by_file_id.insert(module.file_id, module.id);
        state.modules_by_id.insert(module.id, module);
    }

    /// Iterate over the modules in the graph.
    ///
    /// NOTE @Robustness: This collects the modules to a Vec for safe iteration due to lock holding.
    pub fn iter(&self) -> impl Iterator<Item = Module> {
        let state = self.state.lock();
        state
            .modules_by_id
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Get a module by module id.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Option<Module> {
        let state = self.state.lock();
        state.modules_by_id.get(&id).cloned()
    }

    /// Get a module by file id.
    #[inline]
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<Module> {
        let state = self.state.lock();
        state
            .modules_by_file_id
            .get(&file_id)
            .and_then(|id| state.modules_by_id.get(id))
            .cloned()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        let state = self.state.lock();
        state.modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let state = self.state.lock();
        state.modules_by_id.is_empty()
    }
}
