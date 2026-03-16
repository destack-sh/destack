use destack_dir::{GlobalSymbolId, Lineage, SymbolType, Type, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, Module, ProfileId};

use crate::analyze::common::TypeContext;
use crate::{BuildRequirementError, Compiler};

impl Compiler {
    /// Read one symbol lineage with exact artifact reads and local reuse.
    pub(crate) fn lineage_for_symbol_or_local_for_artifact(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        types: &TypeTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<Option<Lineage>, BuildRequirementError> {
        if let Some(lineage) = types.get_lineage_for_symbol(symbol) {
            return Ok(Some(lineage.clone()));
        }

        if symbol.module_id == module.id {
            return Ok(None);
        }

        self.with_module_types_or_local_for_artifact(
            module,
            profile,
            symbol.module_id,
            types,
            artifact_key,
            |_, owner_types| owner_types.get_lineage_for_symbol(symbol).cloned(),
        )
    }

    /// Provide type ctx with exact artifact reads and local reuse.
    pub(crate) fn with_module_types_or_local_for_artifact<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &TypeTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        if module_id == module.id {
            return Ok(handle(module, types));
        }

        // remote reads require one exact committed artifact
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.as_ref();
        let key = artifact_key(module_id, profile);
        let snapshot = self.require_artifact_dir(key)?;

        Ok(handle(&remote_module, &snapshot.types))
    }

    /// Read one committed remote alias target from one exact artifact family.
    pub(crate) fn remote_alias_target_for_artifact(
        &self,
        _module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<
        (
            Option<GlobalSymbolId>,
            Option<(GlobalSymbolId, Type, TypeTable)>,
            Option<GlobalSymbolId>,
        ),
        BuildRequirementError,
    > {
        // remote reads require one exact committed artifact
        let key = artifact_key(symbol.module_id, profile);
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.as_ref();
        let snapshot = self.require_artifact_dir(key)?;
        let symbol_entry = snapshot.symbols.get_symbol(symbol.local_id);
        if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((None, None, target_symbol));
        }

        let typed_symbol =
            GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_entry.ty));
        let Some(remote_target_id) = snapshot.types.get_alias_target_type_id(typed_symbol) else {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((Some(typed_symbol), None, target_symbol));
        };
        let mut remote_snapshot = snapshot.types.as_ref().clone();
        if matches!(
            remote_snapshot.get_type(remote_target_id),
            Type::Unevaluated(_)
        ) {
            let options = self.analyze_context_options_for_module(remote_module.id);
            let mut ctx = TypeContext::new(
                &remote_module,
                profile,
                &options,
                &snapshot.tree,
                &snapshot.symbols,
                &mut remote_snapshot,
            );

            if let Err(error) = self.resolve_declared_type(&mut ctx, remote_target_id) {
                self.error(error);
            }
        }

        if matches!(
            remote_snapshot.get_type(remote_target_id),
            Type::Unevaluated(_)
        ) {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((Some(typed_symbol), None, target_symbol));
        }

        let remote_target_ty = remote_snapshot.get_type(remote_target_id).clone();
        Ok((
            Some(typed_symbol),
            Some((typed_symbol, remote_target_ty, remote_snapshot)),
            None,
        ))
    }
}
