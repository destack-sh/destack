use destack_dir::{GlobalSymbolId, LocalSymbolId, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::DirReadBoundary;
use crate::{BuildRequirementError, Compiler};

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

    /// Read one symbol table from a module, reusing a local table when possible.
    fn with_module_symbols_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_symbols: Option<&SymbolTable>,
        source: SymbolReadSource,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // reuse the caller-provided table for local reads
        if module_id == module.id {
            if let Some(local_symbols) = local_symbols {
                return Ok(handle(module, local_symbols));
            }

            match source {
                SymbolReadSource::Profile => {
                    if let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary) {
                        let symbols = dir.symbols.read();
                        return Ok(handle(module, &symbols));
                    }

                    let snapshot =
                        self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
                    return Ok(handle(module, &snapshot.symbols));
                }
                SymbolReadSource::Base => {
                    let snapshot = self.require_artifact_dir_base(module_id)?;
                    return Ok(handle(module, &snapshot.symbols));
                }
            }
        }

        // otherwise read symbols from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        if let SymbolReadSource::Profile = source
            && let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary)
        {
            let symbols = dir.symbols.read();
            return Ok(handle(&remote_module, &symbols));
        }

        let snapshot = match source {
            SymbolReadSource::Profile => {
                self.require_artifact_dir_for_boundary(module_id, profile, boundary)?
            }
            SymbolReadSource::Base => self.require_artifact_dir_base(module_id)?,
        };
        Ok(handle(&remote_module, &snapshot.symbols))
    }

    /// Provide the symbol table for a module with boundary-gated cross-module reads.
    pub(crate) fn with_module_symbols_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_symbols_read(
            module,
            profile,
            module_id,
            None,
            SymbolReadSource::Profile,
            boundary,
            handle,
        )
    }

    /// Provide symbol ctx with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_or_local_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_symbols_read(
            module,
            profile,
            module_id,
            Some(symbols),
            SymbolReadSource::Profile,
            boundary,
            handle,
        )
    }

    /// Provide base symbol ctx for a module with boundary-gated cross-module reads.
    pub(crate) fn with_module_symbols_base_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_symbols_read(
            module,
            profile,
            module_id,
            None,
            SymbolReadSource::Base,
            boundary,
            handle,
        )
    }

    /// Provide base symbol ctx with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_base_or_local_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_symbols_read(
            module,
            profile,
            module_id,
            Some(symbols),
            SymbolReadSource::Base,
            boundary,
            handle,
        )
    }

    /// Provide symbol ctx by module id with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_by_id_at_boundary<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(
            symbols.module_id,
            module_id,
            profile,
            boundary,
        )?;

        if module_id == symbols.module_id {
            return Ok(handle(symbols));
        }

        if let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary) {
            let symbols = dir.symbols.read();
            return Ok(handle(&symbols));
        }

        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&snapshot.symbols))
    }
}
