use destack_artifact::WellKnownIntrinsics;
use destack_dir as dir;
use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};

use crate::lower::{FunctionLowerer, ModuleLowerer};
use crate::{Compiler, LowerError, LowerResult, RequirementError};

impl ModuleLowerer<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<dir::StringId>> {
        // ensure the target module is analyzed before resolving intrinsics
        self.require_analyzed_module(target_symbol.module_id)?;

        // resolve the intrinsic name through the registry
        resolve_intrinsic_binding_name_id(
            self.module_id,
            self.profile,
            self.context.revision(),
            self.compiler,
            self.symbols,
            self.well_known_intrinsics.as_ref(),
            target_symbol,
        )
    }
}

impl FunctionLowerer<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<dir::StringId>> {
        // resolve the intrinsic name through the registry
        resolve_intrinsic_binding_name_id(
            self.context.module_id,
            self.context.profile,
            self.context.revision,
            self.context.compiler,
            self.context.symbols,
            self.context.well_known_intrinsics,
            target_symbol,
        )
    }
}

/// Resolve the intrinsic binding name for a symbol.
fn resolve_intrinsic_binding_name_id(
    module_id: ModuleId,
    profile: ProfileId,
    revision: Revision,
    compiler: &Compiler,
    local_symbols: &dir::SymbolTable,
    well_known_intrinsics: Option<&WellKnownIntrinsics>,
    target_symbol: GlobalSymbolId,
) -> LowerResult<Option<dir::StringId>> {
    // skip when no intrinsic registry is available
    let Some(well_known_intrinsics) = well_known_intrinsics else {
        return Ok(None);
    };

    // resolve canonical symbol for intrinsic lookup
    let canonical_symbol = resolve_canonical_symbol(
        module_id,
        profile,
        revision,
        compiler,
        local_symbols,
        target_symbol,
    )?;
    if let Some(name) = well_known_intrinsics.name_for_symbol(canonical_symbol) {
        return Ok(Some(compiler.repository.strings.intern(name)));
    }

    Ok(None)
}

/// Resolve the canonical symbol used for intrinsic lookup.
fn resolve_canonical_symbol(
    module_id: ModuleId,
    profile: ProfileId,
    revision: Revision,
    compiler: &Compiler,
    local_symbols: &dir::SymbolTable,
    symbol_id: GlobalSymbolId,
) -> LowerResult<GlobalSymbolId> {
    let (canonical_symbol, target_symbol) = if symbol_id.module_id == module_id {
        let symbol = local_symbols.get_symbol(symbol_id.local_id);
        (symbol.canonical_symbol, symbol.target_symbol)
    } else {
        let snapshot =
            compiler.require_artifact_dir_analyzed(revision, symbol_id.module_id, profile);
        let snapshot = match snapshot {
            Ok(snapshot) => snapshot,
            Err(RequirementError::NotReady { requirement }) => {
                return Err(LowerError::Yield { requirement });
            }
            Err(RequirementError::Failed { requirement }) => {
                return Err(LowerError::UnsatisfiedRequirement { requirement });
            }
        };
        let symbol = snapshot.symbols.get_symbol(symbol_id.local_id);
        (symbol.canonical_symbol, symbol.target_symbol)
    };

    if target_symbol.is_some() && canonical_symbol.is_none() {
        return Err(LowerError::Internal {
            module: module_id,
            message: format!("missing canonical symbol for intrinsic target {symbol_id:?}"),
        });
    }

    Ok(canonical_symbol.unwrap_or(symbol_id))
}
