use destack_dir::SymbolTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Provide the symbol table for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_symbols_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_symbols_unchecked(module, profile, module_id, handle))
    }

    /// Provide symbol tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_or_local_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(
            self.with_module_symbols_or_local_unchecked(
                module, profile, module_id, symbols, handle,
            ),
        )
    }

    /// Provide base symbol tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_symbols_base_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_symbols_base_unchecked(module, module_id, handle))
    }

    /// Provide base symbol tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_base_or_local_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_symbols_base_or_local_unchecked(module, module_id, symbols, handle))
    }

    /// Provide symbol tables by module id with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_by_id_at_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(symbols.module_id, module_id, profile, stage)?;

        Ok(self.with_module_symbols_by_id_unchecked(profile, module_id, symbols, handle))
    }

    /// Provide the symbol table for a module in the given profile.
    fn with_module_symbols_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let symbols = module.dir(profile).symbols.read();
            return handle(module, &symbols);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        handle(&remote_module, &remote_symbols)
    }

    /// Provide a symbol table, reusing local references when possible.
    fn with_module_symbols_or_local_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, symbols);
        }

        self.with_module_symbols_unchecked(module, profile, module_id, handle)
    }

    /// Provide the symbol table from the base directory for a module.
    fn with_module_symbols_base_unchecked<R>(
        &self,
        module: &Module,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let symbols = module.dir_base().symbols.read();
            return handle(module, &symbols);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir_base().symbols.read();
        handle(&remote_module, &remote_symbols)
    }

    /// Provide a base symbol table, reusing local references when possible.
    fn with_module_symbols_base_or_local_unchecked<R>(
        &self,
        module: &Module,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, symbols);
        }

        self.with_module_symbols_base_unchecked(module, module_id, handle)
    }

    /// Provide a symbol table by module id, reusing local symbols when possible.
    fn with_module_symbols_by_id_unchecked<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == symbols.module_id {
            return handle(symbols);
        }

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        handle(&remote_symbols)
    }
}
