use std::collections::{HashMap, HashSet};

use super::resolve::AssociatedAliasProjectionRewriter;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{AnalyzeDependencyStage, RelationMode, TypeRewriteCache};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeIdAny, Member, NodeTree, NodeType, NormalizationMode,
    StaticArgument, SymbolTable, SymbolType, Type, TypeRewriter, TypeTable,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn materialize_associated_member_projection(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        target_symbol: GlobalSymbolId,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        static_arguments: Option<&[StaticArgument]>,
        member_ty: Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Type> {
        let options = self.analyze_context_options_for_module(module.id);
        let projection_environment = self.projection_environment_for_member(
            module,
            profile,
            source_id,
            target_symbol,
            receiver_symbol,
            receiver_arguments,
            static_arguments,
            Some(&member_ty),
            &options,
            None,
            tree,
            symbols,
            types,
        )?;
        let substitutions = projection_environment.substitutions;
        let owner_symbol = projection_environment.owner_symbol;

        // reject explicit static arguments on non-parameterized members
        if let Some(member_arguments) = static_arguments
            && !member_arguments.is_empty()
        {
            let parameter_count = self
                .with_module_tree_symbols_or_local_at_stage(
                    module,
                    profile,
                    target_symbol.module_id,
                    tree,
                    symbols,
                    AnalyzeDependencyStage::Declare,
                    |owner_module, owner_tree, owner_symbols| {
                        self.collect_static_parameter_symbols(
                            owner_module,
                            target_symbol,
                            profile,
                            owner_tree,
                            owner_symbols,
                            types,
                        )
                        .map(|parameters| parameters.len())
                    },
                )
                .map_err(AnalyzeError::from)?;

            if parameter_count == Some(0) {
                self.error(AnalyzeError::InvalidStaticArgument {
                    node: source_id
                        .into_global(module.id)
                        .into_anchored(Some(profile)),
                    message: "too many static arguments".to_string(),
                });
                return Ok(Type::Error);
            }
        }

        // re-evaluate associated comptime constants with receiver substitutions
        let member_kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
        if matches!(
            member_kind,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst)
        ) {
            let mut visited_symbols = HashSet::new();
            let static_value = match self
                .static_expression_from_constant_reference_instantiated_declared(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                    types,
                    &substitutions,
                    &mut visited_symbols,
                ) {
                Ok(value) => value,
                Err(AnalyzeError::Yield { .. }) => None,
                Err(error) => return Err(error),
            };
            if let Some(static_value) = static_value
                && let Some(value_type_id) =
                    self.static_expression_type_id_for_substitution(source_id, &static_value, types)
            {
                return Ok(types.get_type(value_type_id).clone());
            }

            // keep unresolved associated comptime projections explicit
            // this allows post-convergence obligation reporting to reject unresolved value-space projections
            if let Ok(expression_id) = source_id.try_into_typed::<Expression>() {
                return Ok(Type::Unevaluated(expression_id));
            }

            return Ok(Type::Error);
        }

        // materialize associated type alias targets with merged substitutions
        let mut alias_target_id = if let Some(alias_target_id) = self
            .alias_target_type_id_for_symbol(
                module,
                profile,
                target_symbol,
                source_id,
                symbols,
                types,
            ) {
            alias_target_id
        } else if let Some(alias_target_id) = self.relaxed_alias_target_type_id_for_symbol(
            module,
            profile,
            target_symbol,
            source_id,
            tree,
            symbols,
            types,
        ) {
            alias_target_id
        } else {
            return Ok(member_ty);
        };
        // refresh local alias targets from source expressions once projection context is available
        if target_symbol.module_id == module.id {
            let alias_source = types.get_type_source(alias_target_id);
            if let Ok(alias_expression_id) = alias_source.try_into_typed::<Expression>()
                && tree.has_node_id(alias_expression_id.id)
            {
                alias_target_id = self.resolve_declared_type_expression_fresh(
                    module,
                    profile,
                    alias_expression_id,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
            }
        }

        alias_target_id = self.apply_associated_projection_substitutions(
            module,
            profile,
            target_symbol,
            alias_target_id,
            &substitutions,
            tree,
            symbols,
            types,
        )?;

        let mut materialize_cache = TypeRewriteCache::new();
        let materialized_alias = self.materialize_static_arguments_in_type(
            module,
            profile,
            alias_target_id,
            tree,
            symbols,
            types,
            &mut materialize_cache,
        );

        // substitute after materialization so owner scoped references are concrete first
        let mapped_alias = if substitutions.is_empty() {
            materialized_alias
        } else {
            let mut substitution_cache = HashMap::new();
            self.substitute_static_parameters(
                materialized_alias,
                &substitutions,
                types,
                &mut substitution_cache,
            )
        };

        // rematerialize after substitution to normalize mapped references
        let mapped_alias = self.materialize_static_arguments_in_type(
            module,
            profile,
            mapped_alias,
            tree,
            symbols,
            types,
            &mut materialize_cache,
        );

        let mapped_alias = if let Some(owner_symbol) = owner_symbol {
            let mut rewriter = AssociatedAliasProjectionRewriter::new(
                self,
                module,
                profile,
                source_id,
                owner_symbol,
                &substitutions,
                tree,
                symbols,
            );
            rewriter.rewrite_type_id(types, mapped_alias)
        } else {
            mapped_alias
        };

        let normalized_alias = self.normalize_type_with_relation(
            module,
            profile,
            mapped_alias,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::OBJECT_SHAPE,
        );

        Ok(types.get_type(normalized_alias).clone())
    }

    /// Check whether an associated type alias requires explicit static arguments.
    pub(crate) fn associated_type_requires_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<bool> {
        // only associated type aliases can require projection arguments
        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |_owner_module, owner_tree, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
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
                let Member::Type {
                    static_parameters, ..
                } = owner_tree.get(member_id)
                else {
                    return false;
                };
                let Some(parameters) = static_parameters.as_ref() else {
                    return false;
                };
                if parameters.is_empty() {
                    return false;
                }

                parameters
                    .iter()
                    .any(|parameter_id| !owner_tree.get(*parameter_id).has_default())
            },
        )
        .map_err(AnalyzeError::from)
    }
}
