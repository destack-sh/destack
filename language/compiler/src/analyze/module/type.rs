use destack_artifact::ArtifactKey;
use destack_dir::{GlobalSymbolId, Lineage, SymbolType, Type, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::{ArtifactRequirementError, Compiler};

impl Compiler {
    /// Read one symbol lineage with exact artifact reads and local reuse.
    pub(crate) fn lineage_for_symbol_or_local_for_artifact(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        types: &TypeTable,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<Option<Lineage>, ArtifactRequirementError> {
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
    ) -> Result<R, ArtifactRequirementError> {
        if module_id == module.id {
            return Ok(handle(module, types));
        }

        // remote reads require one exact committed artifact
        self.with_remote_dir_for_artifact(
            module_id,
            profile,
            artifact_key,
            |remote_module, _, _, types| handle(remote_module, types),
        )
    }

    /// Read one committed remote alias target from declared artifact state.
    pub(crate) fn remote_declared_alias_target(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> Result<
        (
            Option<GlobalSymbolId>,
            Option<(GlobalSymbolId, Type, TypeTable)>,
            Option<GlobalSymbolId>,
        ),
        ArtifactRequirementError,
    > {
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.as_ref();
        let snapshot = self.require_artifact_dir_declared(symbol.module_id, profile)?;
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

        // resolve unevaluated declared aliases before reading the target
        if matches!(
            remote_snapshot.get_type(remote_target_id),
            Type::Unevaluated(_)
        ) {
            let options = self.analyze_context_options_for_module(remote_module.id);
            let mut ctx = TypeContext::new(
                remote_module,
                profile,
                &options,
                &snapshot.tree,
                &snapshot.symbols,
                &mut remote_snapshot,
                AnalyzeIndex::default(),
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
