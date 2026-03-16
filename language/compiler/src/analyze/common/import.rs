use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, AnalyzeResult, Compiler, ResolveError};
use destack_dir::{
    DependencyKind, DependencySource, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Path,
    StaticArgument, StaticKey, StringId, SymbolSpaceOrder, Type,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Query one type import symbol without reporting deferred dependency states as errors.
    pub(crate) fn query_import_type_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
    ) -> Option<GlobalSymbolId> {
        match self.resolve_import_type_symbol(module, profile, node, target, qualifier) {
            Ok(symbol) => symbol,
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Resolve a type import into a concrete exported symbol.
    pub(crate) fn resolve_import_type_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // reject empty or nested qualifiers for now
        let Some(member_key) = self.import_type_member_key(qualifier) else {
            return Ok(None);
        };

        // resolve the module target from the import specifier
        let dir = self
            .require_artifact_dir_resolved(module.id, profile)
            .map_err(AnalyzeError::from)?;
        let node = node.into_global(module.id);
        let target = match self.resolve_import_from_artifact(
            module,
            dir.as_ref(),
            profile,
            node,
            DependencySource::ImportStatement,
            target,
            DependencyKind::Type,
        ) {
            Ok(target) => target,
            Err(error) => {
                if matches!(
                    error,
                    ResolveError::Yield { .. } | ResolveError::UnsatisfiedRequirement { .. }
                ) {
                    return Err(AnalyzeError::from(error));
                }

                self.error(error);
                return Ok(None);
            }
        };

        // resolve the exported symbol from the target module
        let symbol = match self.resolve_export_symbol_for_target(
            module.id,
            node,
            target,
            profile,
            SymbolSpaceOrder::TypeOnly,
            member_key,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                if matches!(
                    error,
                    ResolveError::Yield { .. } | ResolveError::UnsatisfiedRequirement { .. }
                ) {
                    return Err(AnalyzeError::from(error));
                }

                self.error(error);
                return Ok(None);
            }
        };
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        Ok(Some(symbol))
    }

    /// Query a type import reference in non-AnalyzeResult paths.
    pub(crate) fn query_import_type_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
        static_arguments: Option<&[StaticArgument]>,
    ) -> Option<LocalTypeId> {
        let symbol =
            self.query_import_type_symbol(ctx.module, ctx.profile, source_id, target, qualifier)?;

        let static_arguments = static_arguments.map(|arguments| arguments.to_vec());
        let reference = Type::Reference {
            symbol,
            static_arguments,
        };
        Some(ctx.types.insert_type_from_any(reference, source_id))
    }

    /// Convert an import qualifier path into a static key.
    fn import_type_member_key(&self, qualifier: Option<&Path>) -> Option<StaticKey> {
        let qualifier = qualifier?;
        if qualifier.segments.len() != 1 {
            return None;
        }
        qualifier.last_segment().map(StaticKey::Name)
    }
}
