use super::StaticEvaluationMode;
use crate::analyze::common::{AnalyzeDependencyStage, RelationMode, TypeRewriteCache};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DependencyItem, Expression, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Member, Mutability,
    NodeTree, NodeType, NormalizationMode, StaticExpression, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn static_expression_from_constant_reference_parametric(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.static_expression_from_constant_reference_inner(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Parametric,
            None,
            AnalyzeDependencyStage::Infer,
            visited,
        )
    }

    /// Resolve one constant reference in instantiated evaluation mode.

    pub(crate) fn static_expression_from_constant_reference_instantiated(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.static_expression_from_constant_reference_inner(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Instantiated,
            Some(substitutions),
            AnalyzeDependencyStage::Infer,
            visited,
        )
    }

    /// Resolve one constant reference in instantiated mode using declared dependency ownership.
    pub(crate) fn static_expression_from_constant_reference_instantiated_declared(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.static_expression_from_constant_reference_inner(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Instantiated,
            Some(substitutions),
            AnalyzeDependencyStage::Declare,
            visited,
        )
    }

    /// Report a circular static-argument diagnostic and return one local error expression.
    fn static_cycle_error_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        source_node: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> StaticExpression {
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let node = symbol_entry
            .primary_declaration
            .map(|primary_declaration| primary_declaration.local_id)
            .unwrap_or(module.dir(profile).anchor_node);
        self.error(AnalyzeError::CircularStaticArgument {
            node: node.into_global(module.id).into_anchored(Some(profile)),
        });

        let error_type_id = types.insert_type_from_any(Type::Error, source_node);
        StaticExpression::Type { ty: error_type_id }
    }

    /// Resolve constant bindings into static expressions when possible.

    pub(super) fn static_expression_from_constant_reference_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: StaticEvaluationMode,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        remote_dependency_stage: AnalyzeDependencyStage,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.static_expression_from_constant_reference_inner_with_previsited(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            mode,
            substitutions,
            remote_dependency_stage,
            visited,
            None,
        )
    }

    /// Resolve constant bindings into static expressions when possible.
    #[allow(clippy::too_many_arguments)]
    fn static_expression_from_constant_reference_inner_with_previsited(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: StaticEvaluationMode,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        remote_dependency_stage: AnalyzeDependencyStage,
        visited: &mut HashSet<GlobalSymbolId>,
        previsited_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        // avoid recursive constant evaluation
        let is_symbol_equivalent = |left: GlobalSymbolId, right: GlobalSymbolId| {
            left.module_id == right.module_id && left.local_id.id == right.local_id.id
        };
        let has_existing_visit = visited
            .iter()
            .any(|visited_symbol| is_symbol_equivalent(*visited_symbol, symbol));
        let owns_visit_marker = if !has_existing_visit {
            visited.insert(symbol);
            true
        } else if previsited_symbol
            .is_some_and(|previsited| is_symbol_equivalent(previsited, symbol))
        {
            false
        } else {
            let is_immutable_binding_cycle_candidate = self
                .direct_binding_declarator_for_symbol(module, symbol, tree, symbols)
                .is_some_and(|declarator_id| {
                    let Some(parent_id) = tree.get_parent(declarator_id.id) else {
                        return false;
                    };
                    if parent_id.ty != NodeType::Expression {
                        return false;
                    }
                    matches!(
                        tree.get(parent_id.into_typed::<Expression>()),
                        Expression::Let {
                            mutability: Mutability::Immutable,
                            ..
                        }
                    )
                });
            let is_constant_cycle_candidate = if mode == StaticEvaluationMode::Instantiated {
                true
            } else if is_immutable_binding_cycle_candidate {
                true
            } else {
                let symbol_entry = symbols.get_symbol(symbol.local_id);
                symbol_entry
                    .primary_declaration
                    .is_some_and(|primary_declaration| {
                        if primary_declaration.local_id.ty != NodeType::Member {
                            return false;
                        }
                        let member_id = primary_declaration.local_id.into_typed::<Member>();
                        matches!(
                            tree.get(member_id),
                            Member::ComptimeConst { value: Some(_), .. }
                        )
                    })
            };
            if !is_constant_cycle_candidate {
                return Ok(None);
            }
            let error = self.static_cycle_error_expression(
                module,
                profile,
                symbol,
                symbols,
                module.dir(profile).anchor_node,
                types,
            );
            return Ok(Some(error));
        };

        // reject cross-module re-entry for a different symbol in an active module
        // this avoids deadlock and models module-level static cycles as circular arguments
        let has_active_module_reentry = symbol.module_id != module.id
            && visited.iter().any(|visited_symbol| {
                visited_symbol.module_id == symbol.module_id
                    && visited_symbol.local_id.id != symbol.local_id.id
            });
        if has_active_module_reentry {
            let error = self.static_cycle_error_expression(
                module,
                profile,
                symbol,
                symbols,
                module.dir(profile).anchor_node,
                types,
            );
            if owns_visit_marker {
                visited.remove(&symbol);
            }
            return Ok(Some(error));
        }

        // evaluate using the owning module context
        if symbol.module_id != module.id {
            let source_node = module.dir(profile).anchor_node;
            let remote_value = self
                .with_module_tree_symbols_at_stage(
                    module,
                    profile,
                    symbol.module_id,
                    remote_dependency_stage,
                    |owner_module, owner_tree, owner_symbols| {
                        let Some(mut owner_types) = owner_module.dir(profile).types.try_write()
                        else {
                            let error = self.static_cycle_error_expression(
                                owner_module,
                                profile,
                                symbol,
                                owner_symbols,
                                source_node,
                                types,
                            );
                            return Ok(Some(error));
                        };
                        let mapped_substitutions = substitutions.map(|substitutions| {
                            let mut mapped = HashMap::new();
                            for (substitution_symbol, substitution_type_id) in substitutions {
                                let substitution_ty = types.get_type(*substitution_type_id);
                                let substitution_source =
                                    types.get_type_source(*substitution_type_id);
                                let mapped_type_id = self.import_type_from_remote_for_node(
                                    substitution_source,
                                    substitution_ty,
                                    types,
                                    *substitution_symbol,
                                    &mut owner_types,
                                );
                                mapped.insert(*substitution_symbol, mapped_type_id);
                            }
                            mapped
                        });
                        let remote_value = self
                            .static_expression_from_constant_reference_inner_with_previsited(
                                owner_module,
                                profile,
                                symbol,
                                owner_tree,
                                owner_symbols,
                                &mut owner_types,
                                mode,
                                mapped_substitutions.as_ref(),
                                remote_dependency_stage,
                                visited,
                                Some(symbol),
                            )?;

                        let local_value = remote_value.map(|value| {
                            self.import_static_expression_from_remote_for_node(
                                source_node,
                                &value,
                                &owner_types,
                                symbol,
                                types,
                            )
                        });
                        Ok(local_value)
                    },
                )
                .map_err(AnalyzeError::from)?;
            if owns_visit_marker {
                visited.remove(&symbol);
            }
            return remote_value;
        }

        // ensure dependency items are resolved before evaluating local constants
        if symbol.module_id == module.id && mode == StaticEvaluationMode::Parametric {
            self.require_resolve_module_direct(module.id, profile)
                .map_err(AnalyzeError::from)?;
        }

        let value = if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(module, symbol, tree, symbols)
        {
            let declarator = tree.get(declarator_id);

            // require immutable bindings for static arguments
            let parent_id = tree.get_parent(declarator_id.id);
            let Some(parent_id) = parent_id else {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            };
            if parent_id.ty != NodeType::Expression {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            }

            let expression_id = parent_id.into_typed::<Expression>();
            let Expression::Let { mutability, .. } = tree.get(expression_id) else {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            };
            if *mutability != Mutability::Immutable {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            }

            // evaluate the initializer as a static expression
            let Some(value_id) = declarator.value else {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            };
            self.evaluate_static_expression_value_inner(
                module,
                profile,
                value_id,
                tree,
                symbols,
                types,
                None,
                mode,
                substitutions,
                remote_dependency_stage,
                visited,
            )?
        } else {
            let symbol_entry = symbols.get_symbol(symbol.local_id);

            // evaluate associated comptime member initializers
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::Member
            {
                let member_id = primary_declaration.local_id.into_typed::<Member>();
                if let Member::ComptimeConst {
                    value: Some(value_expression_id),
                    ..
                } = tree.get(member_id)
                {
                    // evaluate literal/static forms directly first
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        *value_expression_id,
                        tree,
                        symbols,
                        types,
                        None,
                        mode,
                        substitutions,
                        remote_dependency_stage,
                        visited,
                    )?;
                    let Some(value) = value else {
                        if mode == StaticEvaluationMode::Instantiated {
                            if owns_visit_marker {
                                visited.remove(&symbol);
                            }
                            return Ok(None);
                        }

                        // evaluate type-level forms through substitution and normalization
                        let mut value_type_id = self.resolve_declared_type_expression(
                            module,
                            profile,
                            *value_expression_id,
                            tree,
                            symbols,
                            types,
                            true,
                            true,
                        )?;

                        if let Some(substitutions) = substitutions
                            && !substitutions.is_empty()
                        {
                            let mut substitution_cache = HashMap::new();
                            value_type_id = self.substitute_static_parameters(
                                value_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        value_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            value_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );

                        value_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            value_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::STATIC_EVAL,
                        );

                        let projected_value = match types.get_type(value_type_id) {
                            Type::TypeLiteral {
                                value: TypeLiteral::ScalarLiteral(value),
                            } => Some(StaticExpression::ScalarLiteral {
                                value: value.clone(),
                            }),
                            Type::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                                value: value.clone(),
                            }),
                            _ => None,
                        };

                        if owns_visit_marker {
                            visited.remove(&symbol);
                        }
                        return Ok(projected_value);
                    };
                    if owns_visit_marker {
                        visited.remove(&symbol);
                    }
                    return Ok(Some(value));
                }
            }

            // follow export/import dependency targets when available
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
                && let item_id = primary_declaration.local_id.into_typed::<DependencyItem>()
                && let DependencyItem::Local { target_symbol, .. }
                | DependencyItem::Remote { target_symbol, .. } = tree.get(item_id)
                && let Some(value) = self.static_expression_from_constant_reference_inner(
                    module,
                    profile,
                    *target_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?
            {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(Some(value));
            }

            // follow export expressions that wrap dependency items
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::Expression
            {
                let expression_id = primary_declaration.local_id.into_typed::<Expression>();
                if let Expression::Export { items, .. } = tree.get(expression_id) {
                    for item_id in items {
                        match tree.get(*item_id) {
                            DependencyItem::Local {
                                symbol: Some(local_symbol),
                                target_symbol,
                                ..
                            }
                            | DependencyItem::Remote {
                                symbol: Some(local_symbol),
                                target_symbol,
                                ..
                            } if *local_symbol == symbol.local_id => {
                                if let Some(value) = self
                                    .static_expression_from_constant_reference_inner(
                                        module,
                                        profile,
                                        *target_symbol,
                                        tree,
                                        symbols,
                                        types,
                                        mode,
                                        substitutions,
                                        remote_dependency_stage,
                                        visited,
                                    )?
                                {
                                    if owns_visit_marker {
                                        visited.remove(&symbol);
                                    }
                                    return Ok(Some(value));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some(target_symbol) = symbol_entry.target_symbol {
                self.static_expression_from_constant_reference_inner(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?
            } else if let Some(canonical_symbol) = symbol_entry.canonical_symbol {
                self.static_expression_from_constant_reference_inner(
                    module,
                    profile,
                    canonical_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?
            } else {
                // parametric mode may reuse inferred literal value snapshots
                if mode == StaticEvaluationMode::Parametric
                    && let Some(value_type_id) = types.get_value_type_id(symbol)
                    && let Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    } = types.get_type(value_type_id)
                {
                    Some(StaticExpression::ScalarLiteral {
                        value: value.clone(),
                    })
                } else {
                    None
                }
            }
        };

        if owns_visit_marker {
            visited.remove(&symbol);
        }
        Ok(value)
    }
}
