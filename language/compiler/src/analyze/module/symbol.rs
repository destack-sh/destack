use destack_dir::{GlobalSymbolId, LocalSymbolId, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, Module, ProfileId};

use crate::{BuildRequirementError, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build one typed global symbol id from one local symbol id and symbol table.
    pub(crate) fn typed_global_symbol_id(
        &self,
        module_id: ModuleId,
        symbols: &SymbolTable,
        symbol_id: LocalSymbolId,
    ) -> GlobalSymbolId {
        let symbol_entry = symbols.get_symbol(symbol_id);
        let symbol_id = symbol_id.with_type(symbol_entry.ty);

        GlobalSymbolId::new(module_id, symbol_id)
    }

    /// Provide symbol ctx with exact artifact reads and local reuse.
    pub(crate) fn with_module_symbols_or_local_for_artifact<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        if module_id == module.id {
            return Ok(handle(module, symbols));
        }

        // remote reads require one exact committed artifact
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.as_ref();
        let key = artifact_key(module_id, profile);
        let snapshot = self.require_artifact_dir(key)?;

        Ok(handle(&remote_module, &snapshot.symbols))
    }
}
