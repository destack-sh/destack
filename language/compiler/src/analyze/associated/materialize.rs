use std::collections::{HashMap, HashSet};

use super::resolve::AssociatedAliasProjectionRewriter;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{RelationMode, TreeSymbolView, TypeContext, TypeRewriteCache};
use crate::analyze::declare::StaticConstantResolutionMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    GenericParameter, GlobalSymbolId, LocalNodeIdAny, Member, NodeType, NormalizationMode,
    StaticArgument, SymbolType, Type, TypeExpression, TypeRewriter,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn materialize_associated_member_projection(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        static_arguments: Option<&[StaticArgument]>,
        member_ty: Type,
    ) -> AnalyzeResult<Type> {
        // suppress cascading projection diagnostics when the receiver already failed
        if let Some(receiver_symbol) = receiver_symbol {
            let (receiver_symbol, _) = self.normalize_projection_receiver_reference(
                &mut ctx.reborrow(),
                source_id,
                receiver_symbol,
                receiver_arguments,
            )?;
            if self.symbol_has_missing_associated_requirements(ctx.type_view(), receiver_symbol)? {
                return Ok(Type::Error);
            }
        }

        let projection_environment = self.projection_environment_for_member(
            &mut ctx.reborrow(),
            source_id,
            target_symbol,
            receiver_symbol,
            receiver_arguments,
            static_arguments,
            Some(&member_ty),
            None,
        )?;
        let substitutions = projection_environment.substitutions;
        let owner_symbol = projection_environment.owner_symbol;
        let projection_receiver_symbol = projection_environment.receiver_symbol;
        let projection_receiver_arguments = projection_environment.receiver_arguments;
        // reject explicit static arguments on non-parameterized members
        if let Some(member_arguments) = static_arguments
            && !member_arguments.is_empty()
        {
            let parameter_count = self
                .with_module_tree_symbol_view_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    target_symbol.module_id,
                    ctx.tree,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |view| {
                        self.collect_static_parameter_symbols(
                            view.type_view(ctx.types),
                            target_symbol,
                        )
                        .map(|parameters| parameters.len())
                    },
                )
                .map_err(AnalyzeError::from)?;

            if parameter_count == Some(0) {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: source_id
                        .into_global(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                    message: "too many static arguments".to_string(),
                });
                return Ok(Type::Error);
            }
        }

        // re-evaluate associated comptime constants with receiver substitutions
        let member_kind =
            self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), target_symbol)?;
        if matches!(
            member_kind,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst)
        ) {
            let mut visited_symbols = HashSet::new();
            let static_value = self.resolve_static_constant_reference_for_mode(
                &mut ctx.reborrow(),
                target_symbol,
                Some(&substitutions),
                &mut visited_symbols,
                StaticConstantResolutionMode::InstantiatedDeclare,
            )?;
            if let Some(static_value) = static_value
                && let Some(value_type_id) = self.static_expression_type_id_for_substitution(
                    source_id,
                    &static_value,
                    ctx.types,
                )
            {
                return Ok(ctx.types.get_type(value_type_id).clone());
            }

            // keep unresolved associated comptime projections explicit
            // this allows post-convergence obligation reporting to reject unresolved value-space projections
            if let Ok(expression_id) = source_id.try_into_typed::<TypeExpression>()
                && ctx.tree.has_node_id(expression_id.id)
            {
                return Ok(Type::Unevaluated(expression_id));
            }

            return Ok(member_ty);
        }

        // materialize associated type alias targets with merged substitutions
        let Some(mut alias_target_id) = self.require_alias_target_type_id_for_symbol(
            &mut ctx.reborrow(),
            target_symbol,
            source_id,
        )?
        else {
            return Ok(member_ty);
        };

        alias_target_id = self.apply_associated_projection_substitutions(
            &mut ctx.reborrow(),
            target_symbol,
            alias_target_id,
            &substitutions,
        )?;
        let alias_target_id = if let Some(owner_symbol) = owner_symbol {
            let mut rewriter = AssociatedAliasProjectionRewriter::new(
                self,
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                source_id,
                owner_symbol,
                projection_receiver_symbol,
                &projection_receiver_arguments,
                &substitutions,
                ctx.tree,
                ctx.symbols,
            );
            rewriter.rewrite_type_id(ctx.types, alias_target_id)
        } else {
            alias_target_id
        };
        // apply projection substitutions before materialization so unresolved defaults stay symbolic
        let mapped_alias = if substitutions.is_empty() {
            alias_target_id
        } else {
            let mut substitution_cache = HashMap::new();
            self.substitute_static_parameters(
                alias_target_id,
                &substitutions,
                ctx.types,
                &mut substitution_cache,
            )
        };

        // materialize after substitution to resolve alias references deterministically
        let mut materialize_cache = TypeRewriteCache::new();
        let mapped_alias = self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            mapped_alias,
            &mut materialize_cache,
        );
        let normalized_alias = self.normalize_type_with_relation(
            &mut ctx.reborrow(),
            mapped_alias,
            NormalizationMode::Assign,
            RelationMode::OBJECT_SHAPE,
        );
        Ok(ctx.types.get_type(normalized_alias).clone())
    }

    /// Check whether an associated type alias requires explicit static arguments.
    pub(crate) fn associated_type_requires_static_arguments(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<bool> {
        // only associated type aliases can require projection arguments
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let symbol_entry = view.symbols.get_symbol(symbol.local_id);
                if symbol_entry.ty != SymbolType::TypeAlias {
                    return false;
                }

                // skip aliases that are not declared as members
                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return false;
                };
                if primary_declaration.local_id.ty != NodeType::Member {
                    return false;
                }

                // require arguments when any parameter has no default
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                let Member::AssociatedType {
                    generic_parameters, ..
                } = view.tree.get(member_id)
                else {
                    return false;
                };
                if generic_parameters.is_empty() {
                    return false;
                }
                let parameters = generic_parameters;
                if parameters.is_empty() {
                    return false;
                }

                parameters.iter().any(|parameter_id| {
                    matches!(
                        view.tree.get(*parameter_id),
                        GenericParameter::Type { default: None, .. }
                            | GenericParameter::Value { default: None, .. }
                            | GenericParameter::Error { .. }
                    )
                })
            },
        )
        .map_err(AnalyzeError::from)
    }
}
