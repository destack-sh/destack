use std::collections::HashMap;

use dyst_source::{File, FileId};

use crate::{Module, ModuleId};

/// A graph of Modules (including their underlying Files).
#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: HashMap<ModuleId, Module>,
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
        }
    }

    /// Insert a module into the graph.
    pub fn insert(&mut self, module: Module) {
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
        self.modules_by_id.get(&ModuleId::new(file_id))
    }

    /// Get a file by module id.
    #[inline]
    pub fn get_file(&self, id: ModuleId) -> Option<&File> {
        self.get(id).map(|module| &module.file)
    }

    /// Get a file by file id.
    #[inline]
    pub fn get_file_by_file_id(&self, file_id: FileId) -> Option<&File> {
        self.get_by_file_id(file_id).map(|module| &module.file)
    }
}
