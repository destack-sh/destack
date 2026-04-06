use destack_artifact::ArtifactKey;
use destack_dir::{GlobalSymbolId, LocalSymbolId, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::{Compiler, CompilerContext, RequirementError};

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
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, RequirementError> {
        if module_id == module.id {
            return Ok(handle(module, symbols));
        }

        // remote reads require one exact committed artifact
        self.with_remote_dir_for_artifact(
            context,
            module_id,
            profile,
            artifact_key,
            |remote_module, _, symbols, _| handle(remote_module, symbols),
        )
    }
}
