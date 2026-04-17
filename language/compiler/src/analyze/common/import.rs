use crate::analyze::common::{TreeSymbolView, TypeContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler, ResolveError};
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, Expression, GlobalSymbolId, ImportSource,
    LocalNodeId, LocalNodeIdAny, LocalTypeId, Path, StaticArgument, StaticKey, StringId,
    SymbolSpaceOrder, Type, TypeExpression,
};
use destack_workspace::workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Query one type import symbol without reporting deferred dependency states as errors.
    pub(crate) fn query_import_type_symbol(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        node: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
    ) -> Option<GlobalSymbolId> {
        match self.resolve_import_type_symbol(revision, module, profile, node, target, qualifier) {
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
        revision: destack_workspace::Revision,
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
            .require_artifact_dir_resolved(revision, module.id, profile)
            .map_err(AnalyzeError::from)?;
        let node = node.into_global(module.id);
        let target = match self.resolve_import_from_resolved_artifact(
            revision,
            module,
            dir.as_ref(),
            profile,
            node,
            ImportSource::ImportStatement,
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
            revision,
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
        generic_arguments: Option<&[StaticArgument]>,
    ) -> Option<LocalTypeId> {
        let symbol = self.query_import_type_symbol(
            ctx.compiler_context.revision(),
            ctx.module,
            ctx.profile,
            source_id,
            target,
            qualifier,
        )?;

        let generic_arguments = generic_arguments.map(|arguments| arguments.to_vec());
        let reference = Type::Reference {
            symbol,
            generic_arguments,
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

    /// Resolve one member through an imported namespace-like binding.
    pub(crate) fn resolve_imported_namespace_member_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<Expression>,
        dependency: &DependencyItem,
        member_key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        let (mode, kind, target_module, target_symbol) = match dependency {
            DependencyItem::Remote {
                mode,
                kind,
                target_module,
                target_symbol,
                ..
            } => (*mode, *kind, Some(*target_module), Some(*target_symbol)),
            DependencyItem::UnresolvedRemote {
                mode,
                kind,
                target_module,
                ..
            } => (*mode, *kind, *target_module, None),
            _ => return None,
        };

        // named imports can still denote one namespace export like `export * as api`
        if mode != DependencyMode::Namespace {
            let target_symbol = target_symbol?;

            return self
                .resolve_symbol_in_namespace(
                    ctx.compiler_context.revision(),
                    expression_id.into_global_any(ctx.module.id),
                    target_symbol,
                    ctx.profile,
                    kind,
                    member_key,
                    None,
                )
                .ok()
                .flatten();
        }

        // namespace imports resolve members through the imported target module
        let target_module = target_module?;
        let target_module = target_module.ty.or(target_module.value)?;
        self.resolve_export_symbol_for_target(
            ctx.compiler_context.revision(),
            ctx.module.id,
            expression_id.into_global_any(ctx.module.id),
            target_module,
            ctx.profile,
            SymbolSpaceOrder::TypeThenValue,
            member_key,
        )
        .ok()
        .flatten()
    }

    /// Resolve one member through an imported namespace-like binding in type syntax.
    pub(crate) fn resolve_imported_namespace_member_symbol_in_type_expression(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        dependency: &DependencyItem,
        member_key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        let (mode, kind, target_module, target_symbol) = match dependency {
            DependencyItem::Remote {
                mode,
                kind,
                target_module,
                target_symbol,
                ..
            } => (*mode, *kind, Some(*target_module), Some(*target_symbol)),
            DependencyItem::UnresolvedRemote {
                mode,
                kind,
                target_module,
                ..
            } => (*mode, *kind, *target_module, None),
            _ => return None,
        };

        // named imports can still denote one namespace export like `export * as api`
        if mode != DependencyMode::Namespace {
            let target_symbol = target_symbol?;

            return self
                .resolve_symbol_in_namespace(
                    ctx.compiler_context.revision(),
                    expression_id.into_global_any(ctx.module.id),
                    target_symbol,
                    ctx.profile,
                    kind,
                    member_key,
                    None,
                )
                .ok()
                .flatten();
        }

        // namespace imports resolve members through the imported target module
        let target_module = target_module?;
        let target_module = target_module.ty.or(target_module.value)?;
        self.resolve_export_symbol_for_target(
            ctx.compiler_context.revision(),
            ctx.module.id,
            expression_id.into_global_any(ctx.module.id),
            target_module,
            ctx.profile,
            match kind {
                DependencyKind::Type => SymbolSpaceOrder::TypeThenValue,
                DependencyKind::Value => SymbolSpaceOrder::ValueThenType,
            },
            member_key,
        )
        .ok()
        .flatten()
    }
}
