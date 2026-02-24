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

/// One declared static-constant lookup result for one module-local symbol graph walk.
enum PublishedStaticConstantLookup {
    /// One concrete published value found in the current module table walk.
    Found {
        symbol: GlobalSymbolId,
        value: StaticExpression,
    },
    /// One set of forwarded symbols that must be queried in other modules.
    Forward { symbols: Vec<GlobalSymbolId> },
}

/// Control whether static-cycle diagnostics are emitted while evaluating constants.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum StaticCycleDiagnosticMode {
    /// Emit cycle diagnostics.
    Report,
    /// Suppress cycle diagnostics.
    Suppress,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Publish declared static constant values for all active symbols in one module.
    pub(crate) fn publish_declared_static_constant_values_for_module(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // declare publishes static constant commitments for static-capable declaration symbols
        for local_symbol_id in symbols.active_symbol_ids() {
            let symbol_id = local_symbol_id.into_global(module.id);
            if !self.symbol_can_be_static_constant(module, symbol_id, tree, symbols) {
                continue;
            }

            let mut visited = HashSet::new();
            let value = self.resolve_static_constant_reference(
                module,
                profile,
                symbol_id,
                tree,
                symbols,
                types,
                StaticEvaluationMode::Parametric,
                None,
                AnalyzeDependencyStage::Declare,
                StaticCycleDiagnosticMode::Suppress,
                &mut visited,
            );
            let Some(value) = (match value {
                Ok(value) => value,

                // keep declare publication local only: skip remote unresolved values here
                Err(AnalyzeError::Yield { .. }) => None,
                Err(error) => return Err(error),
            }) else {
                continue;
            };

            // publish one declared static constant commitment per symbol
            types.publish_static_constant_value(symbol_id, value);
        }

        Ok(())
    }

    /// Return whether one symbol can participate in static constant publication.
    fn symbol_can_be_static_constant(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> bool {
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return false;
        };

        // associated comptime constants are publishable
        if primary_declaration.local_id.ty == NodeType::Member {
            let member_id = primary_declaration.local_id.into_typed::<Member>();
            if matches!(
                tree.get(member_id),
                Member::ComptimeConst { value: Some(_), .. }
            ) {
                return true;
            }
        }

        // export or import dependency aliases may publish through target symbol commitments
        if primary_declaration.local_id.ty == NodeType::DependencyItem
            || primary_declaration.local_id.ty == NodeType::Expression
        {
            return true;
        }

        // immutable direct bindings with initializers can publish
        if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(module, symbol, tree, symbols)
            && let Some(parent_id) = tree.get_parent(declarator_id.id)
            && parent_id.ty == NodeType::Expression
            && let Expression::Let { mutability, .. } = tree.get(parent_id.into_typed())
        {
            return *mutability == Mutability::Immutable;
        }

        false
    }

    /// Query one declared static constant lookup in one module-local symbol graph.
    fn query_published_static_constant_lookup_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<PublishedStaticConstantLookup> {
        let mut pending_symbols = vec![symbol];
        let mut visited_symbols = HashSet::new();
        let mut forwarded_symbols = Vec::new();

        while let Some(candidate_symbol) = pending_symbols.pop() {
            if !visited_symbols.insert(candidate_symbol) {
                continue;
            }

            if let Some(value) = types.query_published_static_constant_value(candidate_symbol) {
                return Some(PublishedStaticConstantLookup::Found {
                    symbol: candidate_symbol,
                    value,
                });
            }

            if candidate_symbol.module_id != symbols.module_id {
                forwarded_symbols.push(candidate_symbol);
                continue;
            }

            let symbol_entry = symbols.get_symbol(candidate_symbol.local_id);
            let normalized_symbol = GlobalSymbolId::new(
                symbols.module_id,
                candidate_symbol.local_id.with_type(symbol_entry.ty),
            );
            if let Some(value) = types.query_published_static_constant_value(normalized_symbol) {
                return Some(PublishedStaticConstantLookup::Found {
                    symbol: normalized_symbol,
                    value,
                });
            }

            if let Some(target_symbol) = symbol_entry.target_symbol {
                pending_symbols.push(target_symbol);
            }

            if let Some(canonical_symbol) = symbol_entry.canonical_symbol {
                pending_symbols.push(canonical_symbol);
            }

            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                continue;
            };

            if primary_declaration.local_id.ty == NodeType::DependencyItem {
                let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                if let DependencyItem::Local { target_symbol, .. }
                | DependencyItem::Remote { target_symbol, .. } = tree.get(dependency_id)
                {
                    pending_symbols.push(*target_symbol);
                }
            }

            if primary_declaration.local_id.ty == NodeType::Expression {
                let expression_id = primary_declaration.local_id.into_typed::<Expression>();
                if let Expression::Export { items, .. } = tree.get(expression_id) {
                    for item_id in items {
                        if let DependencyItem::Local {
                            symbol: Some(local_symbol),
                            target_symbol,
                            ..
                        }
                        | DependencyItem::Remote {
                            symbol: Some(local_symbol),
                            target_symbol,
                            ..
                        } = tree.get(*item_id)
                            && *local_symbol == candidate_symbol.local_id
                        {
                            pending_symbols.push(*target_symbol);
                        }
                    }
                }
            }
        }

        if forwarded_symbols.is_empty() {
            return None;
        }

        Some(PublishedStaticConstantLookup::Forward {
            symbols: forwarded_symbols,
        })
    }

    /// Resolve one constant reference in parametric evaluation mode.
    pub(crate) fn resolve_static_constant_reference_parametric(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.resolve_static_constant_reference(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Parametric,
            None,
            AnalyzeDependencyStage::Infer,
            StaticCycleDiagnosticMode::Suppress,
            visited,
        )
    }

    /// Resolve one constant reference in instantiated evaluation mode.

    pub(crate) fn resolve_static_constant_reference_instantiated(
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
        self.resolve_static_constant_reference(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Instantiated,
            Some(substitutions),
            AnalyzeDependencyStage::Infer,
            StaticCycleDiagnosticMode::Report,
            visited,
        )
    }

    /// Resolve one constant reference in instantiated mode using declared dependency ownership.
    pub(crate) fn resolve_static_constant_reference_instantiated_declared(
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
        self.resolve_static_constant_reference(
            module,
            profile,
            symbol,
            tree,
            symbols,
            types,
            StaticEvaluationMode::Instantiated,
            Some(substitutions),
            AnalyzeDependencyStage::Declare,
            StaticCycleDiagnosticMode::Report,
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
        cycle_diagnostic_mode: StaticCycleDiagnosticMode,
    ) -> StaticExpression {
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let node = symbol_entry
            .primary_declaration
            .map(|primary_declaration| primary_declaration.local_id)
            .unwrap_or(module.dir(profile).anchor_node);
        if cycle_diagnostic_mode == StaticCycleDiagnosticMode::Report {
            self.error(AnalyzeError::CircularStaticArgument {
                node: node.into_global(module.id).into_anchored(Some(profile)),
            });
        }

        let error_type_id = types.insert_type_from_any(Type::Error, source_node);
        StaticExpression::Type { ty: error_type_id }
    }

    /// Collect substitution entries as concrete local types for remote static evaluation.
    fn collect_remote_static_substitution_entries(
        &self,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        types: &TypeTable,
    ) -> Option<Vec<(GlobalSymbolId, Type)>> {
        substitutions.map(|substitutions| {
            substitutions
                .iter()
                .map(|(symbol, type_id)| (*symbol, types.get_type(*type_id).clone()))
                .collect::<Vec<_>>()
        })
    }

    /// Resolve constant bindings into static expressions when possible.

    pub(super) fn resolve_static_constant_reference(
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
        cycle_diagnostic_mode: StaticCycleDiagnosticMode,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        self.resolve_static_constant_reference_with_previsited(
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
            cycle_diagnostic_mode,
        )
    }

    /// Resolve constant bindings into static expressions when possible.
    fn resolve_static_constant_reference_with_previsited(
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
        cycle_diagnostic_mode: StaticCycleDiagnosticMode,
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
            let is_constant_cycle_candidate = if mode == StaticEvaluationMode::Instantiated {
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
                cycle_diagnostic_mode,
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
            let is_parametric_immutable_binding = mode == StaticEvaluationMode::Parametric
                && self
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
            if is_parametric_immutable_binding {
                if owns_visit_marker {
                    visited.remove(&symbol);
                }
                return Ok(None);
            }

            let error = self.static_cycle_error_expression(
                module,
                profile,
                symbol,
                symbols,
                module.dir(profile).anchor_node,
                types,
                cycle_diagnostic_mode,
            );
            if owns_visit_marker {
                visited.remove(&symbol);
            }
            return Ok(Some(error));
        }

        // evaluate cross-module constants from declare-published static constant values
        if symbol.module_id != module.id {
            let source_node = module.dir(profile).anchor_node;
            let mut pending_symbols = vec![symbol];
            let mut visited_symbols = HashSet::new();
            let mut local_value = None;
            let can_use_published_lookup =
                substitutions.is_none_or(|substitutions| substitutions.is_empty());
            let substitution_entries =
                self.collect_remote_static_substitution_entries(substitutions, types);

            if can_use_published_lookup {
                while let Some(candidate_symbol) = pending_symbols.pop() {
                    if !visited_symbols.insert(candidate_symbol) {
                        continue;
                    }

                    let (found_value, forwarded_symbols) = self
                        .with_module_tree_symbols_types_by_id_at_stage(
                            profile,
                            candidate_symbol.module_id,
                            tree,
                            symbols,
                            types,
                            remote_dependency_stage,
                            |owner_tree, owner_symbols, owner_types| match self
                                .query_published_static_constant_lookup_for_symbol(
                                    candidate_symbol,
                                    owner_tree,
                                    owner_symbols,
                                    owner_types,
                                ) {
                                Some(PublishedStaticConstantLookup::Found {
                                    symbol: resolved_symbol,
                                    value,
                                }) => {
                                    let snapshot = owner_types.clone();
                                    (Some((resolved_symbol, value, snapshot)), Vec::new())
                                }
                                Some(PublishedStaticConstantLookup::Forward {
                                    symbols: forwarded_symbols,
                                }) => (None, forwarded_symbols),
                                None => (None, Vec::new()),
                            },
                        )
                        .map_err(AnalyzeError::from)?;

                    if let Some((_resolved_symbol, value, remote_snapshot)) = found_value {
                        local_value = Some(self.import_remote_static_expression_for_node(
                            source_node,
                            &value,
                            &remote_snapshot,
                            types,
                        ));
                        break;
                    }

                    for forwarded_symbol in forwarded_symbols {
                        if !visited_symbols.contains(&forwarded_symbol) {
                            pending_symbols.push(forwarded_symbol);
                        }
                    }
                }
            }

            // evaluate unresolved remote constants on a cloned remote snapshot when publication is absent
            if local_value.is_none() {
                let evaluated_remote_value = self
                    .with_module_tree_symbols_at_stage(
                        module,
                        profile,
                        symbol.module_id,
                        remote_dependency_stage,
                        |remote_module,
                         remote_tree,
                         remote_symbols|
                         -> AnalyzeResult<Option<(StaticExpression, TypeTable)>> {
                            let remote_types = remote_module.dir(profile).types.read();
                            let mut remote_snapshot = remote_types.clone();
                            let mut remote_visited = visited.clone();
                            let remote_substitutions =
                                substitution_entries.as_ref().map(|entries| {
                                    let mut mapped = HashMap::with_capacity(entries.len());
                                    for (parameter_symbol, local_type) in entries {
                                        let remote_type_id = self.import_remote_type_for_node(
                                            source_node,
                                            local_type,
                                            types,
                                            &mut remote_snapshot,
                                        );
                                    mapped.insert(*parameter_symbol, remote_type_id);
                                    }
                                    mapped
                                });

                            let value = self
                                .resolve_static_constant_reference_with_previsited(
                                    remote_module,
                                    profile,
                                    symbol,
                                    remote_tree,
                                    remote_symbols,
                                    &mut remote_snapshot,
                                    mode,
                                    remote_substitutions.as_ref(),
                                    remote_dependency_stage,
                                    &mut remote_visited,
                                    Some(symbol),
                                    cycle_diagnostic_mode,
                                )?;

                            Ok(value.map(|value| (value, remote_snapshot)))
                        },
                    )
                    .map_err(AnalyzeError::from)??;

                local_value = evaluated_remote_value.map(|(value, remote_snapshot)| {
                    self.import_remote_static_expression_for_node(
                        source_node,
                        &value,
                        &remote_snapshot,
                        types,
                    )
                });
            }

            if owns_visit_marker {
                visited.remove(&symbol);
            }
            return Ok(local_value);
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
                && let Some(value) = self.resolve_static_constant_reference(
                    module,
                    profile,
                    *target_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    cycle_diagnostic_mode,
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
                                if let Some(value) = self.resolve_static_constant_reference(
                                    module,
                                    profile,
                                    *target_symbol,
                                    tree,
                                    symbols,
                                    types,
                                    mode,
                                    substitutions,
                                    remote_dependency_stage,
                                    cycle_diagnostic_mode,
                                    visited,
                                )? {
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
                self.resolve_static_constant_reference(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    cycle_diagnostic_mode,
                    visited,
                )?
            } else if let Some(canonical_symbol) = symbol_entry.canonical_symbol {
                self.resolve_static_constant_reference(
                    module,
                    profile,
                    canonical_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    cycle_diagnostic_mode,
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
