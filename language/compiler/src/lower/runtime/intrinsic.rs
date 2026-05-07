use destack_artifact::WellKnownIntrinsics;
use destack_core::StringId;
use destack_dir as dir;

use crate::CompilerResult;
use crate::lower::{FunctionLowerer, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StringId>> {
        // ensure the target module is checked before resolving intrinsics
        self.require_checked_module(target_symbol.module_id)?;

        Ok(resolve_intrinsic_binding_name_id(
            self.well_known_intrinsics.as_ref(),
            target_symbol,
        ))
    }
}

impl FunctionLowerer<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StringId>> {
        Ok(resolve_intrinsic_binding_name_id(
            self.context.well_known_intrinsics,
            target_symbol,
        ))
    }
}

/// Resolve the intrinsic binding name for a symbol.
fn resolve_intrinsic_binding_name_id(
    well_known_intrinsics: Option<&WellKnownIntrinsics>,
    target_symbol: dir::GlobalSymbolId,
) -> Option<dir::StringId> {
    // skip when no intrinsic registry is available
    let Some(well_known_intrinsics) = well_known_intrinsics else {
        return None;
    };

    well_known_intrinsics
        .name_for_symbol(target_symbol)
        .map(StringId::for_text)
}
