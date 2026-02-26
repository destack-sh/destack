use destack_dir::SymbolTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

/// Select which symbol table source to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolReadSource {
    /// Read symbols from the profile directory.
    Profile,
    /// Read symbols from the base directory.
    Base,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Read one symbol table from a module, reusing a local table when possible.
    fn with_module_symbols_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_symbols: Option<&SymbolTable>,
        source: SymbolReadSource,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // reuse the caller-provided table for local reads
        if module_id == module.id {
            if let Some(local_symbols) = local_symbols {
                return handle(module, local_symbols);
            }

            let symbols = match source {
                SymbolReadSource::Profile => module.dir(profile).symbols.read(),
                SymbolReadSource::Base => module.dir_base().symbols.read(),
            };
            return handle(module, &symbols);
        }

        // otherwise read symbols from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let symbols = match source {
            SymbolReadSource::Profile => remote_module.dir(profile).symbols.read(),
            SymbolReadSource::Base => remote_module.dir_base().symbols.read(),
        };
        handle(&remote_module, &symbols)
    }

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

        Ok(self.with_module_symbols_read(
            module,
            profile,
            module_id,
            None,
            SymbolReadSource::Profile,
            handle,
        ))
    }

    /// Provide symbol ctx with stage-gated cross-module reads and local reuse.
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

        Ok(self.with_module_symbols_read(
            module,
            profile,
            module_id,
            Some(symbols),
            SymbolReadSource::Profile,
            handle,
        ))
    }

    /// Provide base symbol ctx for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_symbols_base_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_symbols_read(
            module,
            profile,
            module_id,
            None,
            SymbolReadSource::Base,
            handle,
        ))
    }

    /// Provide base symbol ctx with stage-gated cross-module reads and local reuse.
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

        Ok(self.with_module_symbols_read(
            module,
            profile,
            module_id,
            Some(symbols),
            SymbolReadSource::Base,
            handle,
        ))
    }

    /// Provide symbol ctx by module id with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_by_id_at_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(symbols.module_id, module_id, profile, stage)?;

        if module_id == symbols.module_id {
            return Ok(handle(symbols));
        }

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        Ok(handle(&remote_symbols))
    }
}
