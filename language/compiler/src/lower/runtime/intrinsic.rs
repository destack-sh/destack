use destack_artifact::LanguageIntrinsics;
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
            self.language_intrinsics.as_ref(),
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
            self.context.language_intrinsics,
            target_symbol,
        ))
    }
}

/// Resolve the intrinsic binding name for a symbol.
fn resolve_intrinsic_binding_name_id(
    language_intrinsics: Option<&LanguageIntrinsics>,
    target_symbol: dir::GlobalSymbolId,
) -> Option<dir::StringId> {
    // skip when no intrinsic registry is available
    let Some(language_intrinsics) = language_intrinsics else {
        return None;
    };

    language_intrinsics
        .name_for_symbol(target_symbol)
        .map(StringId::for_text)
}
