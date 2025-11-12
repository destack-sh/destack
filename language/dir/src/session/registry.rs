use std::collections::HashMap;

use dyst_source::FileId;

use crate::{Module, ModuleId};

/// Graph of Modules (including their underlying Files).
#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: HashMap<ModuleId, Module>,
    /// The modules by file id.
    modules_by_file_id: HashMap<FileId, ModuleId>,
    /// The next module id.
    next_module_id: u32 = 0,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Create a new ModuleRegistry.
    pub fn new() -> Self {
        Self {
            modules_by_id: HashMap::new(),
            modules_by_file_id: HashMap::new(),
            next_module_id: 0,
        }
    }

    /// Get and increment the next module id.
    pub fn next_id(&mut self) -> ModuleId {
        let id = ModuleId::new(self.next_module_id);
        self.next_module_id += 1;
        id
    }

    /// Insert a module into the graph.
    pub fn insert(&mut self, module: Module) {
        self.modules_by_file_id.insert(module.file_id, module.id);
        self.modules_by_id.insert(module.id, module);
    }

    /// Iterate over the modules in the graph.
    pub fn iter(&self) -> impl Iterator<Item = &Module> {
        self.modules_by_id.values()
    }

    /// Get a module by module id.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules_by_id.get(&id)
    }

    /// Get a module by file id.
    #[inline]
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<&Module> {
        self.modules_by_file_id
            .get(&file_id)
            .and_then(|id| self.modules_by_id.get(id))
    }
}
