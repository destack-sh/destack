use tspp_core::FxIndexMap;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::{ContextError, ModuleContext};

/// Checked DIR state for one program.
#[derive(Debug)]
pub struct ProgramContext {
    /// Checked modules keyed by module id.
    modules: FxIndexMap<ModuleId, ModuleContext>,
}

impl ProgramContext {
    /// Build one program context from checked modules.
    pub fn new(modules: impl IntoIterator<Item = ModuleContext>) -> Result<Self, ContextError> {
        let mut indexed = FxIndexMap::default();

        // require one context per module
        for module in modules {
            let module_id = module.module();
            if indexed.insert(module_id, module).is_some() {
                return Err(ContextError::DuplicateModule(module_id));
            }
        }

        Ok(Self { modules: indexed })
    }

    /// Return one checked module.
    pub fn module(&self, module: ModuleId) -> Result<&ModuleContext, ContextError> {
        self.modules
            .get(&module)
            .ok_or(ContextError::MissingModule(module))
    }

    /// Return one checked type from its owning module.
    pub fn type_by_id(&self, id: dir::GlobalTypeId) -> Result<dir::Type, ContextError> {
        let module = self.module(id.module_id)?;

        module
            .types()
            .get_type_maybe(id.local_id)
            .ok_or(ContextError::MissingType(id))
    }

    /// Return a checked type-id list from its owning module.
    pub fn type_ids(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> Result<&[dir::GlobalTypeId], ContextError> {
        let module = self.module(module)?;

        Ok(module.types().type_ids(list))
    }
}
