use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, RelationMode, TypeTablesContext,
};
use crate::analyze::{AssociatedProjectionSelection, StaticMemberSymbolKind};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DependencyItem, DependencyMode, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, NodeTree, NodeType, NormalizationMode, ScalarLiteral, StaticArgument,
    StaticExpression, StaticKey, StaticParameterKind, SymbolKind, SymbolSpace, SymbolSpaceOrder,
    SymbolTable, Type, TypeLiteral, TypeTable, TypeUnaryOperator, UnaryOperator,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

/// Selected member target for type evaluation.
#[derive(Clone, Debug)]
pub(crate) enum TypeMemberResolution {
    /// A namespace import member.
    Namespace {
        /// The selected member symbol.
        target_symbol: GlobalSymbolId,
    },
    /// An associated projection member.
    Associated(AssociatedProjectionSelection),
}

/// Classification for `TypeIndex` disambiguation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeIndexResolutionKind {
    /// Interpret `T[K]` as indexed access.
    IndexAccess,
    /// Interpret `T[N]` as fixed-size array construction.
    ArraySized,
}

/// The receiver normalization result for type-index queries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeIndexReceiverState {
    /// A concrete receiver type id after alias, instance, and constraint resolution.
    Resolved(LocalTypeId),
    /// An unconstrained static type parameter receiver.
    UnconstrainedStaticParameter,
    /// A receiver that could not be resolved for index queries.
    Unresolved,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn set_integer_literal_type(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        value: i64,
        types: &mut TypeTable,
    ) {
        let literal_type = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
        };
        let literal_type_id = types.insert_type_from(literal_type, expression_id);
        types.set_inferred_type(expression_id.into_global_any(module_id), literal_type_id);
    }

    /// Resolve one array-size count type id for an expression.

    pub(crate) fn array_sized_count_type_id_for_expression(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        {
            return types.unwrap_value_type_id(type_id);
        }

        let unknown_type_id = types.insert_type_from(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            expression_id,
        );
        types.set_inferred_type(expression_id.into_global_any(module_id), unknown_type_id);
        unknown_type_id
    }

    /// Resolve the inferred type for a static value parameter used as an array size.

    pub(crate) fn resolve_array_size_parameter_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip comptime size validation outside destack modules
        if !tables.module.language_type.is_destack() {
            return Ok(None);
        }
        let (candidate_expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(expression_id, tables.tree);

        // skip expressions that are not static value references
        let kind = if let Some(kind) =
            self.static_parameter_reference_kind(&mut tables.reborrow(), candidate_expression_id)?
        {
            kind
        } else {
            let index_ty_id = self.resolve_declared_type_expression(
                &mut tables.reborrow(),
                candidate_expression_id,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )?;
            let symbol = self.unwrap_type_value_symbol(tables.types, index_ty_id);
            let selection = if symbol.is_none() {
                self.associated_comptime_selection_from_expression(
                    &mut tables.reborrow(),
                    candidate_expression_id,
                )?
            } else {
                None
            };
            let symbol = if let Some(symbol) = symbol {
                Some(symbol)
            } else {
                selection.as_ref().map(|selection| selection.target_symbol)
            };
            let Some(symbol) = symbol else {
                if is_explicit_comptime {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                    });
                }
                return Ok(None);
            };
            let is_static_parameter = self.symbol_is_static_parameter(
                tables.module,
                tables.profile,
                symbol,
                tables.symbols,
                tables.types,
            );
            let is_associated_comptime = matches!(
                self.query_static_member_symbol_kind_for_symbol(
                    tables.module,
                    tables.profile,
                    symbol,
                    tables.tree,
                    tables.symbols,
                )?,
                Some(StaticMemberSymbolKind::AssociatedComptimeConst)
            );
            if !is_static_parameter && !is_associated_comptime {
                if is_explicit_comptime {
                    self.error(AnalyzeError::InvalidComptimeExpression {
                        node: expression_id
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                    });
                }
                return Ok(None);
            }
            if is_associated_comptime {
                // non-this projections must already fold to a concrete integer count
                if !self
                    .associated_projection_receiver_is_this(candidate_expression_id, tables.tree)
                {
                    let has_unresolved_receiver_arguments =
                        selection.as_ref().is_some_and(|selection| {
                            self.receiver_projection_arguments_require_deferral(
                                tables.module,
                                tables.profile,
                                &selection.receiver_arguments,
                                tables.symbols,
                                tables.types,
                            )
                        });
                    if has_unresolved_receiver_arguments {
                        if is_explicit_comptime {
                            self.error(AnalyzeError::InvalidComptimeExpression {
                                node: expression_id
                                    .into_global_any(tables.module.id)
                                    .into_anchored(Some(tables.profile)),
                            });
                            return Ok(None);
                        }

                        let inferred_id = tables.types.unwrap_value_type_id(index_ty_id);
                        tables.types.set_inferred_type(
                            expression_id.into_global_any(tables.module.id),
                            inferred_id,
                        );
                        return Ok(Some(inferred_id));
                    }

                    let has_concrete_count = self
                        .evaluate_integer_static_literal(
                            &mut tables.reborrow(),
                            candidate_expression_id,
                        )?
                        .is_some();
                    if !has_concrete_count {
                        self.error(AnalyzeError::InvalidComptimeExpression {
                            node: expression_id
                                .into_global_any(tables.module.id)
                                .into_anchored(Some(tables.profile)),
                        });
                        return Ok(None);
                    }
                }

                // keep associated comptime references as symbols so substitution can resolve counts
                let reference_type = Type::Reference {
                    symbol,
                    static_arguments: None,
                };
                let reference_type_id = tables
                    .types
                    .insert_type_from_any(reference_type, expression_id.into_any());
                tables.types.set_inferred_type(
                    expression_id.into_global_any(tables.module.id),
                    reference_type_id,
                );
                return Ok(Some(reference_type_id));
            }
            self.static_parameter_kind_for_symbol(&mut tables.reborrow(), symbol)
        };

        // require comptime for value usage
        if kind != StaticParameterKind::Value {
            self.error(AnalyzeError::StaticParameterRequiresComptime {
                node: expression_id
                    .into_global_any(tables.module.id)
                    .into_anchored(Some(tables.profile)),
            });
            return Ok(None);
        }

        // resolve the parameter type and cache it as the inferred type
        let index_id = self.resolve_declared_type_expression(
            &mut tables.reborrow(),
            candidate_expression_id,
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?;
        let inferred_id = if let Type::Value { value } = tables.types.get_type(index_id) {
            *value
        } else {
            index_id
        };
        tables
            .types
            .set_inferred_type(expression_id.into_global_any(tables.module.id), inferred_id);
        Ok(Some(inferred_id))
    }

    /// Check whether an expression can be used as an array size candidate.

    pub(crate) fn expression_is_array_size_candidate(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        let (expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(expression_id, tables.tree);
        if is_explicit_comptime {
            return Ok(true);
        }

        if let Some(symbol) =
            self.associated_comptime_symbol_from_expression(&mut tables.reborrow(), expression_id)?
        {
            if matches!(
                self.query_static_member_symbol_kind_for_symbol(
                    tables.module,
                    tables.profile,
                    symbol,
                    tables.tree,
                    tables.symbols,
                )?,
                Some(StaticMemberSymbolKind::AssociatedComptimeConst)
            ) {
                return Ok(true);
            }
        }

        let target_symbol =
            if let Some(target_symbol) = tables.tree.get(expression_id).target_symbol() {
                Some(target_symbol)
            } else {
                let index_type_id = self.resolve_declared_type_expression(
                    &mut tables.reborrow(),
                    expression_id,
                    true,
                    true,
                )?;
                self.unwrap_type_value_symbol(tables.types, index_type_id)
            };

        let Some(target_symbol) = target_symbol else {
            return Ok(false);
        };

        if self.symbol_is_static_parameter(
            tables.module,
            tables.profile,
            target_symbol,
            tables.symbols,
            tables.types,
        ) {
            let kind = self.static_parameter_kind_for_symbol(&mut tables.reborrow(), target_symbol);
            return Ok(matches!(kind, StaticParameterKind::Value));
        }

        let is_associated_comptime = matches!(
            self.query_static_member_symbol_kind_for_symbol(
                tables.module,
                tables.profile,
                target_symbol,
                tables.tree,
                tables.symbols,
            )?,
            Some(StaticMemberSymbolKind::AssociatedComptimeConst)
        );

        Ok(is_associated_comptime)
    }

    /// Resolve an associated comptime member symbol from one projection expression.
    pub(crate) fn associated_comptime_selection_from_expression(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<AssociatedProjectionSelection>> {
        let Expression::Member { left, name, .. } = tables.tree.get(expression_id) else {
            return Ok(None);
        };

        self.select_associated_projection_member_symbol(
            &mut tables.reborrow(),
            expression_id,
            *left,
            StaticKey::Name(*name),
            Some(StaticMemberSymbolKind::AssociatedComptimeConst),
            true,
            true,
        )
    }

    /// Resolve an associated comptime member symbol from one projection expression.
    pub(crate) fn associated_comptime_symbol_from_expression(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let selection = self
            .associated_comptime_selection_from_expression(&mut tables.reborrow(), expression_id)?;
        Ok(selection.map(|selection| selection.target_symbol))
    }

    /// Return true when one associated projection receiver is `this`.
    fn associated_projection_receiver_is_this(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let Expression::Member { left, .. } = tree.get(expression_id) else {
            return false;
        };

        let left = self.unwrap_parenthesized_expression(*left, tree);
        matches!(tree.get(left), Expression::This)
    }

    /// Classify one `TypeIndex` expression as index-access or fixed-array construction.

    pub(crate) fn type_index_interpretation(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<TypeIndexResolutionKind> {
        let (_, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(index_expression_id, tables.tree);
        if tables.module.language_type.is_declaration() {
            return Ok(if is_explicit_comptime {
                TypeIndexResolutionKind::ArraySized
            } else {
                TypeIndexResolutionKind::IndexAccess
            });
        }
        let index_is_array_size_candidate =
            self.expression_is_array_size_candidate(&mut tables.reborrow(), index_expression_id)?;
        let should_evaluate_static_integer = index_is_array_size_candidate
            || self.type_index_is_integer_literal(index_expression_id, tables.tree);
        let index_static_integer = if should_evaluate_static_integer {
            self.evaluate_integer_static_literal(&mut tables.reborrow(), index_expression_id)?
        } else {
            None
        };
        let left_is_array_sized =
            matches!(tables.types.get_type(left_type_id), Type::ArraySized { .. });
        let supports_index_access =
            self.type_supports_index_access(&mut tables.reborrow(), left_type_id, true)?;

        if !is_explicit_comptime
            && self.type_index_is_ambiguous_without_comptime(
                &mut tables.reborrow(),
                left_type_id,
                index_expression_id,
            )?
        {
            self.error(AnalyzeError::InvalidStaticArgument {
                node: index_expression_id
                    .into_global_any(tables.module.id)
                    .into_anchored(Some(tables.profile)),
                message: "ambiguous type index: use `as comptime` for array sizes or constrain the index for type access".to_string(),
            });
            return Ok(TypeIndexResolutionKind::IndexAccess);
        }

        let is_index_access = self.type_index_should_use_index_access(
            &mut tables.reborrow(),
            left_type_id,
            index_expression_id,
            supports_index_access,
            left_is_array_sized,
            index_is_array_size_candidate,
            index_static_integer.is_some(),
        )?;

        Ok(if is_index_access {
            TypeIndexResolutionKind::IndexAccess
        } else {
            TypeIndexResolutionKind::ArraySized
        })
    }

    /// Decide whether one type-index expression should preserve indexed-access semantics.

    pub(crate) fn type_index_should_use_index_access(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
        supports_index_access: bool,
        left_is_array_sized: bool,
        index_is_array_size_candidate: bool,
        index_is_static_integer: bool,
    ) -> AnalyzeResult<bool> {
        // receivers that are primitive literals cannot use indexed-access type semantics
        let is_primitive_literal_receiver =
            self.type_is_primitive_literal(left_type_id, tables.types);
        if !supports_index_access || is_primitive_literal_receiver {
            return Ok(false);
        }

        // resolve strict index admissibility for the current receiver and key
        let index_access_is_admissible = self.type_index_is_index_access_admissible(
            &mut tables.reborrow(),
            left_type_id,
            index_expression_id,
        )?;

        // detect explicit fixed-array intent
        let force_array_from_value_candidate = index_is_array_size_candidate;
        let force_array_from_nested_sized = left_is_array_sized
            && self.type_index_is_integer_literal(index_expression_id, tables.tree);

        // plain integer literals select fixed arrays only when indexed access is inadmissible
        let force_array_from_static_integer =
            index_is_static_integer && !index_access_is_admissible;

        Ok(!(force_array_from_value_candidate
            || force_array_from_nested_sized
            || force_array_from_static_integer))
    }

    /// Check whether one type-index expression has admissible indexed-access semantics.

    pub(crate) fn type_index_is_index_access_admissible(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        // normalize the receiver before index admissibility checks
        let receiver_resolution =
            self.resolve_type_index_receiver_type(&mut tables.reborrow(), left_type_id)?;
        let receiver_type_id = match receiver_resolution {
            TypeIndexReceiverState::Resolved(receiver_type_id) => receiver_type_id,
            TypeIndexReceiverState::UnconstrainedStaticParameter => return Ok(true),
            TypeIndexReceiverState::Unresolved => return Ok(false),
        };

        // resolve the index operand type before compatibility checks
        let index_type_id = self.resolve_declared_type_expression(
            &mut tables.reborrow(),
            index_expression_id,
            true,
            true,
        )?;
        let index_type_id = tables.types.unwrap_value_type_id(index_type_id);

        // resolve indexed-access types and collect missing literal keys
        let mut visited = Vec::new();
        let resolution = self.resolve_index_access_types(
            &mut tables.reborrow(),
            index_expression_id.into_any(),
            receiver_type_id,
            index_type_id,
            NormalizationMode::Flow,
            RelationMode::INDEX_ACCESS,
            &mut visited,
        );

        Ok(resolution.missing_keys.is_empty())
    }

    /// Resolve one type-index receiver through constraints, aliases, and instance targets.

    fn resolve_type_index_receiver_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        receiver_type_id: LocalTypeId,
    ) -> AnalyzeResult<TypeIndexReceiverState> {
        // track visited types to avoid loops in alias or constraint chains
        let mut visited = HashSet::new();
        let mut receiver_type_id = receiver_type_id;

        loop {
            // stop on cycles
            if !visited.insert(receiver_type_id) {
                return Ok(TypeIndexReceiverState::Unresolved);
            }

            match tables.types.get_type(receiver_type_id).clone() {
                Type::Reference { symbol, .. } => {
                    // follow static parameter constraints when available
                    if self.symbol_is_static_parameter(
                        tables.module,
                        tables.profile,
                        symbol,
                        tables.symbols,
                        tables.types,
                    ) {
                        let source_id = tables.types.get_type_source(receiver_type_id);
                        if let Some(constraint_type_id) = self.static_parameter_constraint_type(
                            &mut tables.reborrow(),
                            symbol,
                            source_id,
                        ) {
                            receiver_type_id = constraint_type_id;
                            continue;
                        }

                        return Ok(TypeIndexReceiverState::UnconstrainedStaticParameter);
                    }

                    // follow alias targets when available
                    if let Some(alias_target_type_id) =
                        tables.types.get_alias_target_type_id(symbol)
                    {
                        receiver_type_id = alias_target_type_id;
                        continue;
                    }

                    // follow instance types when available
                    if let Some(instance_type_id) = tables.types.get_instance_type_id(symbol) {
                        receiver_type_id = instance_type_id;
                        continue;
                    }

                    return Ok(TypeIndexReceiverState::Unresolved);
                }
                Type::Unevaluated(_) => {
                    // evaluate before resolving index-query semantics
                    self.resolve_declared_type(&mut tables.reborrow(), receiver_type_id)?;
                    continue;
                }
                _ => return Ok(TypeIndexReceiverState::Resolved(receiver_type_id)),
            }
        }
    }

    /// Check whether one type index is ambiguous without explicit comptime intent.

    pub(crate) fn type_index_is_ambiguous_without_comptime(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        let (candidate_expression_id, is_explicit_comptime) =
            self.unwrap_as_comptime_expression(index_expression_id, tables.tree);
        if is_explicit_comptime {
            return Ok(false);
        }

        let Some(target_symbol) = tables.tree.get(candidate_expression_id).target_symbol() else {
            return Ok(false);
        };

        // mapped key parameters are always type-space index operands
        if self.type_index_symbol_is_mapped_parameter(
            tables.module,
            target_symbol,
            candidate_expression_id,
            tables.tree,
        ) {
            return Ok(false);
        }

        if !self.symbol_is_static_parameter(
            tables.module,
            tables.profile,
            target_symbol,
            tables.symbols,
            tables.types,
        ) {
            return Ok(false);
        }
        if self.static_parameter_kind_for_symbol(&mut tables.reborrow(), target_symbol)
            != StaticParameterKind::Type
        {
            return Ok(false);
        }

        if self.type_index_constraint_matches_left_keyof(
            &mut tables.reborrow(),
            target_symbol,
            left_type_id,
            candidate_expression_id.into_any(),
        ) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Return whether one type-index symbol comes from an enclosing mapped parameter.
    fn type_index_symbol_is_mapped_parameter(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        // mapped-parameter membership is local syntax context
        if symbol.module_id != module.id {
            return false;
        }

        let mut cursor = Some(expression_id.into_any());
        while let Some(node_id) = cursor {
            if node_id.ty == NodeType::Expression {
                let parent_expression_id = node_id.into_typed::<Expression>();
                if let Expression::TypeMapped { parameter, .. } = tree.get(parent_expression_id)
                    && parameter.symbol == symbol.local_id
                {
                    return true;
                }
            }

            cursor = tree.get_parent(node_id.id);
        }

        false
    }

    /// Check whether one type index parameter is constrained by `keyof` over the receiver type.

    pub(crate) fn type_index_constraint_matches_left_keyof(
        &self,
        tables: &mut TypeTablesContext<'_>,
        parameter_symbol: GlobalSymbolId,
        left_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> bool {
        let mut visited_symbols = HashSet::new();
        self.type_index_constraint_matches_left_keyof_inner(
            &mut tables.reborrow(),
            parameter_symbol,
            left_type_id,
            source_id,
            &mut visited_symbols,
        )
    }

    /// Check whether one type index parameter constraint chain reaches `keyof` on the receiver type.

    pub(crate) fn type_index_constraint_matches_left_keyof_inner(
        &self,
        tables: &mut TypeTablesContext<'_>,
        parameter_symbol: GlobalSymbolId,
        left_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        if !visited_symbols.insert(parameter_symbol) {
            return false;
        }

        let Some(constraint_id) = self.static_parameter_constraint_type(
            &mut tables.reborrow(),
            parameter_symbol,
            source_id,
        ) else {
            return false;
        };

        let constraint_id = tables.types.unwrap_value_type_id(constraint_id);
        let constraint_ty = tables.types.get_type(constraint_id).clone();

        if let Type::Reference { symbol, .. } = constraint_ty
            && self.symbol_is_static_parameter(
                tables.module,
                tables.profile,
                symbol,
                tables.symbols,
                tables.types,
            )
        {
            return self.type_index_constraint_matches_left_keyof_inner(
                &mut tables.reborrow(),
                symbol,
                left_type_id,
                source_id,
                visited_symbols,
            );
        }

        let Type::Unary {
            operator: TypeUnaryOperator::Keyof,
            right,
        } = constraint_ty
        else {
            return false;
        };

        let left_type_id = tables.types.unwrap_value_type_id(left_type_id);
        let right_type_id = tables.types.unwrap_value_type_id(right);
        if left_type_id == right_type_id {
            return true;
        }

        let (
            Type::Reference {
                symbol: left_symbol,
                ..
            },
            Type::Reference {
                symbol: right_symbol,
                ..
            },
        ) = (
            tables.types.get_type(left_type_id),
            tables.types.get_type(right_type_id),
        )
        else {
            return false;
        };

        self.resolve_type_reference_symbol(tables, *left_symbol)
            == self.resolve_type_reference_symbol(tables, *right_symbol)
    }

    /// Decide whether one `TypeIndex` expression should use index-access semantics.

    pub(crate) fn type_index_uses_index_access(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left_type_id: LocalTypeId,
        index_expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        Ok(self.type_index_interpretation(
            &mut tables.reborrow(),
            left_type_id,
            index_expression_id,
        )? == TypeIndexResolutionKind::IndexAccess)
    }

    /// Check whether a type-index expression is a plain integer literal.

    pub(crate) fn type_index_is_integer_literal(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);

        if matches!(
            tree.get(expression_id),
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(_),
            }
        ) {
            return true;
        }

        if let Expression::Unary {
            operator: UnaryOperator::Negate,
            right,
        } = tree.get(expression_id)
        {
            return matches!(
                tree.get(*right),
                Expression::ScalarLiteral {
                    value: ScalarLiteral::Integer(_),
                }
            );
        }

        false
    }

    /// Unwrap parenthesized expressions and explicit `as comptime` markers.

    pub(crate) fn unwrap_as_comptime_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> (LocalNodeId<Expression>, bool) {
        let mut expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        let mut is_explicit_comptime = false;

        while let Expression::TypeUnary {
            operator: TypeUnaryOperator::AsComptime,
            right,
        } = tree.get(expression_id)
        {
            is_explicit_comptime = true;
            expression_id = self.unwrap_parenthesized_expression(*right, tree);
        }

        (expression_id, is_explicit_comptime)
    }

    /// Check whether a type supports indexed access in a type expression.

    pub(crate) fn type_supports_index_access(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        treat_unknown_as_indexable: bool,
    ) -> AnalyzeResult<bool> {
        // normalize the receiver before checking index support
        let receiver_resolution =
            self.resolve_type_index_receiver_type(&mut tables.reborrow(), type_id)?;
        let receiver_type_id = match receiver_resolution {
            TypeIndexReceiverState::Resolved(receiver_type_id) => receiver_type_id,
            TypeIndexReceiverState::UnconstrainedStaticParameter => {
                return Ok(treat_unknown_as_indexable);
            }
            TypeIndexReceiverState::Unresolved => return Ok(false),
        };

        // classify one normalized receiver shape
        let receiver_type = tables.types.get_type(receiver_type_id).clone();
        match receiver_type {
            Type::Tuple { .. } | Type::Array { .. } | Type::ArraySized { .. } => Ok(true),
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => {
                // keep unknown or any indexability configurable per disambiguation context
                Ok(treat_unknown_as_indexable)
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => Ok(!fields.is_empty()
                || !call_signatures.is_empty()
                || !construct_signatures.is_empty()
                || !index_signatures.is_empty()),
            _ => Ok(false),
        }
    }

    /// Check whether a type resolves to a primitive literal.

    pub(crate) fn type_is_primitive_literal(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(_),
            } => true,
            Type::Reference { symbol, .. } => {
                if let Some(alias_id) = types.get_alias_target_type_id(*symbol) {
                    matches!(
                        types.get_type(alias_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(_),
                        }
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Resolve the static parameter symbol and kind for a reference expression.

    pub(crate) fn resolve_namespace_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // require a reference expression on the left side
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(left)
        else {
            return None;
        };

        // namespace imports are local dependency items
        if target_symbol.module_id != module.id {
            return None;
        }

        // require a dependency declaration
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::DependencyItem {
            return None;
        }

        // select dependency mode and target module
        let dependency_id = primary_declaration.local_id.into_typed::<DependencyItem>();
        let dependency = tree.get(dependency_id);
        let (mode, target_module) = match dependency {
            DependencyItem::Remote {
                mode,
                target_module,
                ..
            } => (*mode, Some(*target_module)),
            DependencyItem::UnresolvedRemote {
                mode,
                target_module,
                ..
            } => (*mode, *target_module),
            _ => (DependencyMode::Item, None),
        };

        // only namespace imports can project members
        if mode != DependencyMode::Namespace {
            return None;
        }

        // select type space first, then value space
        let target_module = target_module?;
        let target_module = target_module.ty.or(target_module.value)?;
        self.resolve_export_symbol_for_target(
            module.id,
            expression_id.into_global_any(module.id),
            target_module,
            profile,
            SymbolSpaceOrder::TypeThenValue,
            member_key,
        )
        .ok()
        .flatten()
    }

    /// Select a type member symbol for one member expression.

    pub(crate) fn resolve_type_member_symbol(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<TypeMemberResolution>> {
        // select namespace imports before projection lookups
        if let Some(namespace_symbol) = self.resolve_namespace_member_symbol(
            tables.module,
            tables.profile,
            expression_id,
            left,
            member_key,
            tables.tree,
            tables.symbols,
        ) {
            return Ok(Some(TypeMemberResolution::Namespace {
                target_symbol: namespace_symbol,
            }));
        }

        // select projected members through nominal receivers
        let Some(selection) = self.select_associated_projection_member_symbol(
            &mut tables.reborrow(),
            expression_id,
            left,
            member_key,
            Some(StaticMemberSymbolKind::AssociatedType),
            validate_static_argument_bounds,
            enforce_implicit_managed,
        )?
        else {
            if let Some(enum_member_symbol) = self.resolve_enum_member_symbol(
                tables,
                left,
                member_key,
                validate_static_argument_bounds,
                enforce_implicit_managed,
            )? {
                return Ok(Some(TypeMemberResolution::Namespace {
                    target_symbol: enum_member_symbol,
                }));
            }
            if let Some(projected_symbol) = tables.tree.get(expression_id).target_symbol() {
                let is_enum_field = self
                    .with_module_tree_symbols_or_local_at_stage(
                        tables.module,
                        tables.profile,
                        projected_symbol.module_id,
                        tables.tree,
                        tables.symbols,
                        AnalyzeDependencyStage::Declare,
                        |_owner_module, _owner_tree, owner_symbols| {
                            let symbol_entry = owner_symbols.get_symbol(projected_symbol.local_id);
                            let Some(primary_declaration) = symbol_entry.primary_declaration else {
                                return false;
                            };
                            primary_declaration.local_id.ty == NodeType::EnumField
                        },
                    )
                    .map_err(AnalyzeError::from)?;
                if is_enum_field {
                    return Ok(Some(TypeMemberResolution::Namespace {
                        target_symbol: projected_symbol,
                    }));
                }
            }

            return Ok(None);
        };

        let selection_kind = self.query_static_member_symbol_kind_for_symbol(
            tables.module,
            tables.profile,
            selection.target_symbol,
            tables.tree,
            tables.symbols,
        )?;

        match selection_kind {
            Some(StaticMemberSymbolKind::AssociatedType)
            | Some(StaticMemberSymbolKind::AssociatedComptimeConst) => {
                Ok(Some(TypeMemberResolution::Associated(selection)))
            }
            Some(StaticMemberSymbolKind::EnumField) => Ok(Some(TypeMemberResolution::Namespace {
                target_symbol: selection.target_symbol,
            })),
            _ => Ok(None),
        }
    }

    /// Select one enum member symbol for a type-position member expression.

    pub(crate) fn resolve_enum_member_symbol(
        &self,
        tables: &mut TypeTablesContext<'_>,
        left: LocalNodeId<Expression>,
        member_key: StaticKey,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let left_target_symbol = self
            .reference_symbol_for_expression(
                tables.module,
                left,
                tables.profile,
                tables.tree,
                tables.symbols,
            )
            .or_else(|| tables.tree.get(left).target_symbol())
            .map(|symbol| self.resolve_type_reference_symbol(tables, symbol))
            .or_else(|| {
                let left_ty_id = self
                    .resolve_declared_type_expression(
                        &mut tables.reborrow(),
                        left,
                        validate_static_argument_bounds,
                        enforce_implicit_managed,
                    )
                    .ok()?;
                let left_ty = tables.types.get_type(left_ty_id);
                self.enum_symbol_for_type(left_ty, tables.types)
                    .map(|symbol| self.resolve_type_reference_symbol(tables, symbol))
            });
        let Some(left_target_symbol) = left_target_symbol else {
            return Ok(None);
        };

        self.enum_field_symbol_for_member_key(
            tables.module,
            left_target_symbol,
            &member_key,
            tables.profile,
            tables.tree,
            tables.symbols,
        )
    }

    /// Evaluate an Expression into a Type with validation controls.

    pub(crate) fn resolve_type_reference_symbol(
        &self,
        tables: &TypeTablesContext<'_>,
        target_symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        // follow import dependency items before normalization
        let mut resolved_symbol = target_symbol;
        if resolved_symbol.module_id == tables.module.id {
            let symbol_entry = tables.symbols.get_symbol(resolved_symbol.local_id);
            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
            {
                let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                if let DependencyItem::Local {
                    target_symbol: dependency_target,
                    ..
                }
                | DependencyItem::Remote {
                    target_symbol: dependency_target,
                    ..
                } = tables.tree.get(item_id)
                {
                    resolved_symbol = *dependency_target;
                }
            }
        }

        // keep simple local type-space symbols in fast path form
        if resolved_symbol.module_id == tables.module.id {
            let symbol_entry = tables.symbols.get_symbol(resolved_symbol.local_id);
            if symbol_entry.is_static_parameter() {
                return GlobalSymbolId::new(
                    tables.module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }

            let is_simple = symbol_entry.target_symbol.is_none()
                && symbol_entry.canonical_symbol.is_none()
                && symbol_entry.merge_group.is_none()
                && symbol_entry.kind != SymbolKind::Namespace;
            let is_type_space = matches!(
                symbol_entry.space,
                SymbolSpace::Type | SymbolSpace::TypeValue
            );
            if is_simple && is_type_space {
                return GlobalSymbolId::new(
                    tables.module.id,
                    resolved_symbol.local_id.with_type(symbol_entry.ty),
                );
            }
        }

        // normalize and canonicalize for all non-fast-path cases
        let normalized =
            self.normalize_reference_symbol_id(tables.module, tables.profile, resolved_symbol);
        let canonical = self.canonical_symbol_id(
            tables.module,
            tables.symbols,
            tables.profile,
            normalized,
            CanonicalSymbolMode::PreserveAliases,
        );

        self.merged_type_symbol_id(tables.module, tables.symbols, tables.profile, canonical)
    }

    /// Evaluate a reference to a nominal symbol into a Type.

    pub(crate) fn resolve_type_reference_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
        resolve_static_arguments: bool,
        validate_static_argument_bounds: bool,
        enforce_implicit_managed: bool,
    ) -> AnalyzeResult<Type> {
        // check cached reference types first
        let reference_cache_key = self.type_reference_cache_key(
            target_symbol,
            static_arguments.as_deref(),
            validate_static_argument_bounds,
            enforce_implicit_managed,
            resolve_static_arguments,
        );
        let cached_reference = reference_cache_key
            .and_then(|cache_key| tables.types.get_type_reference_cache(cache_key).cloned());
        if let Some(cached) = cached_reference {
            return Ok(cached);
        }

        // defer static argument resolution when requested
        if !resolve_static_arguments {
            let ty = Type::Reference {
                symbol: target_symbol,
                static_arguments,
            };
            self.cache_type_reference_maybe(reference_cache_key, &ty, tables.types);
            return Ok(ty);
        }

        // resolve static arguments against declared bounds
        let has_explicit_arguments = static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty());
        let parameter_symbols = self.collect_static_parameter_symbols(
            tables.module,
            target_symbol,
            tables.profile,
            tables.tree,
            tables.symbols,
            tables.types,
        );
        let parameters_known = parameter_symbols.is_some();
        let has_parameters = parameter_symbols.is_some_and(|parameters| !parameters.is_empty());
        let resolved_arguments = if !has_explicit_arguments && parameters_known && !has_parameters {
            None
        } else {
            let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_ARGUMENTS);
            self.resolve_type_reference_static_arguments(
                &mut tables.reborrow(),
                expression_id.into_any(),
                target_symbol,
                static_arguments.as_deref(),
                validate_static_argument_bounds,
            )?
        };
        let static_arguments = resolved_arguments.or(static_arguments);

        // return errors directly when static arguments failed to resolve
        let has_error_argument = static_arguments.as_deref().is_some_and(|arguments| {
            arguments.iter().any(|argument| match argument {
                StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } => tables.types.get_type(*ty).is_error(),
                _ => false,
            })
        });
        if has_error_argument {
            return Ok(Type::Error);
        }

        // fold value-space constant references used in type positions
        let symbol_space = self.query_symbol_space_for_reference_if_declared(
            tables.module,
            tables.profile,
            target_symbol,
            tables.symbols,
        );
        if symbol_space == Some(SymbolSpace::Value) {
            let mut visited = HashSet::new();
            if let Some(static_value) = self.resolve_static_constant_reference_parametric(
                &mut tables.reborrow(),
                target_symbol,
                &mut visited,
            )? {
                let constant_type = match static_value {
                    StaticExpression::ScalarLiteral { value } => Some(Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(value),
                    }),
                    StaticExpression::TypeLiteral { value } => Some(Type::TypeLiteral { value }),
                    StaticExpression::Type { ty } => Some(tables.types.get_type(ty).clone()),
                    _ => None,
                };

                if let Some(constant_type) = constant_type {
                    self.cache_type_reference_maybe(
                        reference_cache_key,
                        &constant_type,
                        tables.types,
                    );
                    return Ok(constant_type);
                }
            }

            return Ok(Type::Unevaluated(expression_id));
        }

        // normalize well known references into canonical structural types
        let normalized =
            if let Some(well_known) = self.well_known_array_kind(tables.profile, target_symbol) {
                let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_REFERENCE_WELL_KNOWN);
                self.normalize_well_known_type_reference(
                    tables.module,
                    tables.symbols,
                    tables.profile,
                    expression_id.into_any(),
                    target_symbol,
                    well_known,
                    static_arguments.as_deref(),
                    tables.types,
                )
            } else {
                None
            };
        if let Some(normalized) = normalized {
            self.cache_type_reference_maybe(reference_cache_key, &normalized, tables.types);
            return Ok(normalized);
        }

        // fall back to a nominal reference
        let ty = Type::Reference {
            symbol: target_symbol,
            static_arguments,
        };
        self.cache_type_reference_maybe(reference_cache_key, &ty, tables.types);
        Ok(ty)
    }

    /// Resolve symbol space for one type-reference target when declare commitments are available.
    pub(crate) fn query_symbol_space_for_reference_if_declared(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<SymbolSpace> {
        self.with_module_symbols_or_local_at_stage(
            module,
            profile,
            target_symbol.module_id,
            symbols,
            AnalyzeDependencyStage::Declare,
            |_owner_module, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(target_symbol.local_id);
                Some(symbol_entry.space)
            },
        )
        .ok()
        .flatten()
    }

    /// Evaluate a typeof type expression into a Type.

    pub(crate) fn resolve_typeof_expression(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Type> {
        let _timing = self.timing_scope(tags::ANALYZE_TYPES_EVALUATE_TYPEOF);

        // unwrap parenthesized targets
        let mut target_id = right_id;
        loop {
            let Expression::Parenthesized { expression } = tables.tree.get(target_id) else {
                break;
            };
            target_id = *expression;
        }

        // resolve the target symbol for a typeof reference
        let Some(target_symbol) = tables.tree.get(target_id).target_symbol() else {
            return Ok(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            });
        };

        // resolve the value type for the target symbol
        if let Some(value_ty_id) = tables.types.get_value_type_id(target_symbol) {
            return Ok(tables.types.get_type(value_ty_id).clone());
        }

        if target_symbol.module_id != tables.module.id {
            let value_ty_id = self.resolve_remote_symbol_value_type_for_surface(
                tables.module,
                tables.profile,
                expression_id.into_any(),
                target_symbol,
                tables.types,
            )?;
            return Ok(tables.types.get_type(value_ty_id).clone());
        }

        // fall back to local declaration types when available
        if let Some(declarator_id) = self.direct_binding_declarator_for_symbol(
            tables.module,
            target_symbol,
            tables.tree,
            tables.symbols,
        ) {
            let declared_ty_id = tables
                .types
                .get_declared_type_id(declarator_id.into_global_any(tables.module.id));
            if let Some(declared_ty_id) = declared_ty_id {
                self.resolve_declared_type(&mut tables.reborrow(), declared_ty_id)?;
                return Ok(tables.types.get_type(declared_ty_id).clone());
            }

            let declarator = tables.tree.get(declarator_id);
            if let Some(value_id) = declarator.value {
                let ty = self.resolve_declared_type_expression_value(
                    &mut tables.reborrow(),
                    value_id,
                    true,
                    true,
                    true,
                    true,
                    true,
                )?;
                if !matches!(ty, Type::Unevaluated(_)) {
                    return Ok(ty);
                }
            }
        }

        Ok(Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        })
    }
}
