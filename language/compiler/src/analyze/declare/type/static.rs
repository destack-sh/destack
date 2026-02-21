use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, RelationMode, TypeRewriteCache,
};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler};
use destack_dir::{
    Argument, BinaryOperator, DependencyItem, EnumFieldValue, Expression, GlobalSymbolId,
    IfCondition, IfKind, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, Mutability, NodeTree,
    NodeType, NormalizationMode, PrimitiveType, Property, Resolution, ScalarLiteral,
    StaticArgument, StaticExpression, StaticKey, StaticParameterKind, StaticProperty, SymbolTable,
    Type, TypeBinaryOperator, TypeLiteral, TypeTable, UnaryOperator,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

/// The evaluation mode for static expression folding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticEvaluationMode {
    /// Evaluate without receiver specialization.
    Generic,
    /// Evaluate with receiver specialization.
    Specialized,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn evaluate_integer_static_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<i64>> {
        // prefer existing type facts before re-evaluating the expression tree
        let expression_global = expression_id.into_global_any(module.id);
        if let Some(type_id) = types
            .get_inferred_type_id(expression_global)
            .or_else(|| types.get_declared_type_id(expression_global))
        {
            if let Some(value) = self.integer_literal_value_for_type_id(type_id, types) {
                return Ok(Some(value));
            }
        }

        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);
        let value = self.evaluate_static_expression_value(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            None,
        )?;
        let literal = match value {
            Some(StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            }) => Some(value),
            Some(StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            }) => Some(value),
            Some(StaticExpression::Type { ty }) => match types.get_type(ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => Some(*value),
                _ => None,
            },
            _ => None,
        };
        Ok(literal)
    }

    /// Convert one substituted static parameter type into a static expression.

    pub(crate) fn static_expression_from_substitution_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> StaticExpression {
        let type_id = types.unwrap_value_type_id(type_id);
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Type::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            _ => StaticExpression::Type { ty: type_id },
        }
    }

    /// Return true when one binary operator can be deferred for symbolic static evaluation.

    pub(crate) fn binary_operator_supports_symbolic_static_evaluation(
        &self,
        operator: BinaryOperator,
    ) -> bool {
        matches!(
            operator,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Remainder
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseOr
                | BinaryOperator::ElementwiseXor
        )
    }

    /// Return true when one static expression can participate in symbolic numeric evaluation.

    pub(crate) fn static_expression_may_be_numeric(
        &self,
        value: &StaticExpression,
        types: &TypeTable,
    ) -> bool {
        if self.static_expression_integer_value(value, types).is_some() {
            return true;
        }

        match value {
            StaticExpression::Type { ty } => {
                let mut current = *ty;
                for _ in 0..8 {
                    match types.get_type(current) {
                        Type::Value { value } => current = *value,
                        Type::Reference { .. } | Type::Unevaluated(_) => return true,
                        Type::TypeLiteral {
                            value:
                                TypeLiteral::Primitive(PrimitiveType::Number | PrimitiveType::Int(_)),
                        } => return true,
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        } => return true,
                        _ => return false,
                    }
                }
                false
            }
            StaticExpression::Unevaluated { .. } => true,
            _ => false,
        }
    }

    /// Extract one integer literal value from a static expression when possible.

    pub(crate) fn static_expression_integer_value(
        &self,
        value: &StaticExpression,
        types: &TypeTable,
    ) -> Option<i64> {
        match value {
            StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => Some(*value),
            StaticExpression::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            } => Some(*value),
            StaticExpression::Type { ty } => match types.get_type(*ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                } => Some(*value),
                Type::Value { value } => match types.get_type(*value) {
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
                    } => Some(*value),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    /// Validate that array sizes only use comptime static parameters.

    pub(crate) fn static_parameter_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, StaticParameterKind)>> {
        // only treat references as static parameters in Destack modules
        if !module.language_type.is_destack() {
            return Ok(None);
        }

        // unwrap explicit comptime wrappers to reach the reference
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            target_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                self.static_parameter_reference_in_symbols(
                    owner_module,
                    profile,
                    *target_symbol,
                    owner_tree,
                    owner_symbols,
                    types,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the static parameter kind for a reference expression.

    pub(crate) fn static_parameter_reference_kind(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticParameterKind>> {
        Ok(self
            .static_parameter_reference(module, profile, expression_id, tree, symbols, types)?
            .map(|(_, kind)| kind))
    }

    /// Resolve the static parameter symbol and kind for a symbol within a symbol table.

    pub(crate) fn static_parameter_reference_in_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(GlobalSymbolId, StaticParameterKind)> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            let kind = self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            );
            return Some((target_symbol, kind));
        }

        // fall back to a same-scope static parameter with the same key
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let key = symbol_entry.key?;
        let mut scope_cursor = Some(symbol_entry.scope);
        while let Some((scope_id, mark)) = scope_cursor {
            let scope = symbols.get_scope_by_id(scope_id);
            let limit = mark.0 as usize;
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().take(limit).rev()
            {
                if *candidate_key != key {
                    continue;
                }
                let candidate_symbol = symbols.get_symbol(*candidate_symbol_id);
                if !candidate_symbol.is_active || !candidate_symbol.is_static_parameter() {
                    continue;
                }
                let candidate_global = candidate_symbol_id.into_global(module.id);
                let kind = self.static_parameter_kind_for_symbol(
                    module,
                    profile,
                    candidate_global,
                    tree,
                    symbols,
                    types,
                );
                return Some((candidate_global, kind));
            }

            scope_cursor = scope.parent;
        }

        None
    }

    /// Look up one substitution type for a static parameter symbol.

    pub(crate) fn substitution_type_id_for_static_parameter_symbol(
        &self,
        symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        if let Some(type_id) = substitutions.get(&symbol) {
            return Some(*type_id);
        }

        substitutions.iter().find_map(|(candidate, type_id)| {
            let matches_symbol = candidate.module_id == symbol.module_id
                && candidate.local_id.id == symbol.local_id.id;
            if matches_symbol { Some(*type_id) } else { None }
        })
    }

    /// Try to evaluate an Expression as a Type id.
    /// Set validate_static_argument_bounds to false to defer bound checks.
    /// Set enforce_implicit_managed to false to skip noImplicitManaged enforcement.

    pub(crate) fn evaluate_static_arguments(
        &self,
        module: &Module,
        _profile: ProfileId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
    ) -> AnalyzeResult<Option<Vec<StaticArgument>>> {
        // skip when there are no static arguments
        let Some(static_arguments) = static_arguments else {
            return Ok(None);
        };

        let mut evaluated_arguments = Vec::with_capacity(static_arguments.len());

        // defer static argument evaluation until parameter kinds are known
        for argument_id in static_arguments {
            evaluated_arguments.push(StaticArgument::Unevaluated {
                node: argument_id.into_global_any(module.id),
            });
        }

        Ok(Some(evaluated_arguments))
    }

    /// Evaluate a template literal span expression into a type id.

    pub(crate) fn evaluate_static_expression_value(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_EVALUATE);

        let mut visited = HashSet::new();
        self.evaluate_static_expression_value_inner(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            enum_symbol,
            StaticEvaluationMode::Generic,
            None,
            AnalyzeDependencyStage::Infer,
            &mut visited,
        )
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]

    fn evaluate_static_expression_value_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        enum_symbol: Option<GlobalSymbolId>,
        mode: StaticEvaluationMode,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        remote_dependency_stage: AnalyzeDependencyStage,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let expression = tree.get(expression_id);

        let value = match expression {
            Expression::ScalarLiteral { value } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Expression::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            Expression::Type { value } => StaticExpression::Type { ty: *value },
            Expression::Parenthesized { expression } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *expression,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                );
            }
            Expression::Cast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                );
            }
            Expression::OwnershipCast { value, .. } => {
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *value,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                );
            }
            Expression::Unary { operator, right } => {
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?;
                let Some(StaticExpression::ScalarLiteral { value }) = right_value else {
                    return Ok(None);
                };
                let ScalarLiteral::Integer(value) = value else {
                    return Ok(None);
                };
                let value = match operator {
                    UnaryOperator::Plus => value,
                    UnaryOperator::Negate => -value,
                    UnaryOperator::ElementwiseNot => !value,
                    _ => return Ok(None),
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *left,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?;
                let right_value = self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )?;
                let (left_value, right_value) = match (left_value, right_value) {
                    (
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(left_value),
                        }),
                        Some(StaticExpression::ScalarLiteral {
                            value: ScalarLiteral::Integer(right_value),
                        }),
                    ) => (left_value, right_value),
                    (Some(left_value), Some(right_value)) => {
                        // defer symbolic static arithmetic until substitutions are available
                        if self.binary_operator_supports_symbolic_static_evaluation(*operator)
                            && self.static_expression_may_be_numeric(&left_value, types)
                            && self.static_expression_may_be_numeric(&right_value, types)
                        {
                            return Ok(Some(StaticExpression::Unevaluated {
                                node: expression_id,
                            }));
                        }

                        return Ok(None);
                    }
                    _ => return Ok(None),
                };

                let value = match operator {
                    BinaryOperator::Add => left_value.checked_add(right_value),
                    BinaryOperator::Subtract => left_value.checked_sub(right_value),
                    BinaryOperator::Multiply => left_value.checked_mul(right_value),
                    BinaryOperator::Divide => {
                        if right_value == 0 {
                            None
                        } else {
                            let remainder = left_value % right_value;
                            if remainder != 0 {
                                None
                            } else {
                                left_value.checked_div(right_value)
                            }
                        }
                    }
                    BinaryOperator::Remainder => left_value.checked_rem(right_value),
                    BinaryOperator::ShiftLeft => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shl(shift)),
                    BinaryOperator::ShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| left_value.checked_shr(shift)),
                    BinaryOperator::UnsignedShiftRight => u32::try_from(right_value)
                        .ok()
                        .and_then(|shift| (left_value as u64).checked_shr(shift))
                        .and_then(|shifted| i64::try_from(shifted).ok()),
                    BinaryOperator::ElementwiseAnd => Some(left_value & right_value),
                    BinaryOperator::ElementwiseOr => Some(left_value | right_value),
                    BinaryOperator::ElementwiseXor => Some(left_value ^ right_value),
                    _ => None,
                };

                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(value),
                }
            }
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                // static parameter references
                if let Some((parameter_symbol, kind)) = self.static_parameter_reference(
                    module,
                    profile,
                    expression_id,
                    tree,
                    symbols,
                    types,
                )? {
                    if kind == StaticParameterKind::Value {
                        // use substitution values when available
                        if let Some(substitutions) = substitutions
                            && let Some(type_id) = self
                                .substitution_type_id_for_static_parameter_symbol(
                                    parameter_symbol,
                                    substitutions,
                                )
                        {
                            let value =
                                self.static_expression_from_substitution_type(type_id, types);
                            return Ok(Some(value));
                        }

                        // specialized evaluation does not permit unresolved static parameters
                        if mode == StaticEvaluationMode::Specialized {
                            return Ok(None);
                        }

                        let reference_type = Type::Reference {
                            symbol: parameter_symbol,
                            static_arguments: None,
                        };
                        let ty = types.insert_type_from(reference_type, expression_id);
                        return Ok(Some(StaticExpression::Type { ty }));
                    }

                    self.error(AnalyzeError::StaticParameterRequiresComptime {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                    return Ok(None);
                }

                // unwrap import/export dependency items before canonicalizing
                let mut lookup_symbol = *target_symbol;
                if lookup_symbol.module_id == module.id {
                    let symbol_entry = symbols.get_symbol(lookup_symbol.local_id);
                    if let Some(primary_declaration) = symbol_entry.primary_declaration
                        && primary_declaration.local_id.ty == NodeType::DependencyItem
                    {
                        let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                        if let DependencyItem::Local { target_symbol, .. }
                        | DependencyItem::Remote { target_symbol, .. } = tree.get(item_id)
                        {
                            lookup_symbol = *target_symbol;
                        }
                    }
                }

                if let Some(value) = self.static_expression_from_constant_reference_inner(
                    module,
                    profile,
                    lookup_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                )? {
                    return Ok(Some(value));
                }

                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };
                let value = self.enum_field_value_for_symbol_reference(
                    module,
                    profile,
                    enum_symbol,
                    *target_symbol,
                    symbols,
                    types,
                )?;
                let Some(value) = value else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::Member {
                left,
                name,
                static_arguments: _,
            } => {
                let node_id = expression_id.into_global_any(module.id);
                let member_key = StaticKey::Name(*name);

                // reject cross-module re-entry before projection lookup
                // this keeps static evaluation fail-closed for module-level cycles
                let receiver_symbol = self
                    .reference_symbol_for_expression(module, *left, profile, tree, symbols)
                    .or_else(|| tree.get(*left).target_symbol());
                if let Some(receiver_symbol) = receiver_symbol {
                    let receiver_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        receiver_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    let reenters_active_module = receiver_symbol.module_id != module.id
                        && visited.iter().any(|visited_symbol| {
                            visited_symbol.module_id == receiver_symbol.module_id
                        });
                    if reenters_active_module {
                        self.error(AnalyzeError::CircularStaticArgument {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });

                        let error_type_id =
                            types.insert_type_from_any(Type::Error, expression_id.into_any());
                        return Ok(Some(StaticExpression::Type { ty: error_type_id }));
                    }
                }

                // resolve projected members in value space for static expressions
                if let Some(selection) = self.select_associated_projection_member_symbol(
                    module,
                    profile,
                    expression_id,
                    *left,
                    member_key,
                    Some(StaticMemberSymbolKind::AssociatedComptimeConst),
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )? {
                    let options = self.analyze_context_options_for_module(module.id);
                    let projection_environment = self.projection_environment_for_member(
                        module,
                        profile,
                        expression_id.into_any(),
                        selection.target_symbol,
                        Some(selection.receiver_symbol),
                        &selection.receiver_arguments,
                        None,
                        None,
                        &options,
                        Some(visited),
                        tree,
                        symbols,
                        types,
                    )?;
                    let projection_substitutions = projection_environment.substitutions;

                    // compose projection substitutions with the current evaluation environment
                    let mut merged_substitutions =
                        HashMap::with_capacity(projection_substitutions.len());
                    if let Some(substitutions) = substitutions {
                        merged_substitutions
                            .extend(substitutions.iter().map(|(key, value)| (*key, *value)));
                    }
                    merged_substitutions.extend(projection_substitutions);
                    let projected_mode = if mode == StaticEvaluationMode::Specialized
                        || !merged_substitutions.is_empty()
                    {
                        StaticEvaluationMode::Specialized
                    } else {
                        StaticEvaluationMode::Generic
                    };
                    let merged_substitutions = if merged_substitutions.is_empty() {
                        None
                    } else {
                        Some(merged_substitutions)
                    };

                    if let Some(value) = self.static_expression_from_constant_reference_inner(
                        module,
                        profile,
                        selection.target_symbol,
                        tree,
                        symbols,
                        types,
                        projected_mode,
                        merged_substitutions.as_ref(),
                        remote_dependency_stage,
                        visited,
                    )? {
                        return Ok(Some(value));
                    }
                }

                // fall back to resolved static candidates
                if let Some(resolution_id) = types.get_resolution_for_node(node_id) {
                    let resolution = types.get_resolution(resolution_id);
                    if let Resolution::Static { candidate, .. } = resolution
                        && let Some(value) = self.static_expression_from_constant_reference_inner(
                            module,
                            profile,
                            candidate.target_symbol,
                            tree,
                            symbols,
                            types,
                            mode,
                            substitutions,
                            remote_dependency_stage,
                            visited,
                        )?
                    {
                        return Ok(Some(value));
                    }
                }

                // keep enum member literals available in static contexts
                let Some(enum_symbol) = enum_symbol else {
                    return Ok(None);
                };
                let Some(target_symbol) = self.enum_field_symbol_for_name(
                    module,
                    profile,
                    enum_symbol,
                    *name,
                    tree,
                    symbols,
                )?
                else {
                    return Ok(None);
                };
                let Some(value) = self.enum_field_value_for_symbol_reference(
                    module,
                    profile,
                    enum_symbol,
                    target_symbol,
                    symbols,
                    types,
                )?
                else {
                    return Ok(None);
                };
                StaticExpression::ScalarLiteral {
                    value: match value {
                        EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                        EnumFieldValue::String(value) => ScalarLiteral::String(value),
                    },
                }
            }
            Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } if *kind == IfKind::Ternary => {
                let Some(else_expression) = else_expression else {
                    return Ok(None);
                };
                let IfCondition::Expression {
                    condition: condition_expression,
                } = condition
                else {
                    return Ok(None);
                };

                let condition_holds = match tree.get(*condition_expression) {
                    Expression::TypeBinary {
                        left,
                        operator: TypeBinaryOperator::Extends,
                        right,
                    } => {
                        // resolve both sides for extends checks
                        let mut evaluate_side =
                            |side_id: LocalNodeId<Expression>| -> AnalyzeResult<Option<LocalTypeId>> {
                                // resolve static parameters from specialization substitutions
                                if let Some((parameter_symbol, _)) = self
                                    .static_parameter_reference(
                                        module, profile, side_id, tree, symbols, types,
                                    )?
                                {
                                    if let Some(substitutions) = substitutions
                                        && let Some(mapped) = self
                                            .substitution_type_id_for_static_parameter_symbol(
                                                parameter_symbol,
                                                substitutions,
                                            )
                                    {
                                        return Ok(Some(types.unwrap_value_type_id(mapped)));
                                    }

                                    // specialized evaluation must not fold with unresolved parameters
                                    if mode == StaticEvaluationMode::Specialized {
                                        return Ok(None);
                                    }
                                }

                                let side_type_id = self.resolve_declared_type_expression(
                                    module, profile, side_id, tree, symbols, types, true, true,
                                )?;
                                Ok(Some(side_type_id))
                            };
                        let Some(mut left_type_id) = evaluate_side(*left)? else {
                            return Ok(None);
                        };
                        let Some(mut right_type_id) = evaluate_side(*right)? else {
                            return Ok(None);
                        };

                        if let Some(substitutions) = substitutions
                            && !substitutions.is_empty()
                        {
                            let mut substitution_cache = HashMap::new();
                            left_type_id = self.substitute_static_parameters(
                                left_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                            right_type_id = self.substitute_static_parameters(
                                right_type_id,
                                substitutions,
                                types,
                                &mut substitution_cache,
                            );
                        }

                        let mut materialize_cache = TypeRewriteCache::new();
                        left_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            left_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );
                        right_type_id = self.materialize_static_arguments_in_type(
                            module,
                            profile,
                            right_type_id,
                            tree,
                            symbols,
                            types,
                            &mut materialize_cache,
                        );
                        left_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            left_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::STATIC_EVAL,
                        );
                        right_type_id = self.normalize_type_with_relation(
                            module,
                            profile,
                            right_type_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::STATIC_EVAL,
                        );

                        let options = self.analyze_context_options_for_module(module.id);
                        self.is_type_assignable(
                            module,
                            profile,
                            symbols,
                            right_type_id,
                            left_type_id,
                            types,
                            &options,
                        ) != Assignability::NotAssignable
                    }
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::Boolean(value),
                    } => *value,
                    _ => return Ok(None),
                };

                let selected = if condition_holds {
                    *then_expression
                } else {
                    *else_expression
                };
                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    selected,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                );
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                // evaluate both sides as types before selecting one branch
                let mut evaluate_side =
                    |side_id: LocalNodeId<Expression>| -> AnalyzeResult<Option<LocalTypeId>> {
                        // prefer caller substitutions for static parameters
                        if let Some((parameter_symbol, _)) = self.static_parameter_reference(
                            module, profile, side_id, tree, symbols, types,
                        )? {
                            if let Some(substitutions) = substitutions
                                && let Some(mapped) = self
                                    .substitution_type_id_for_static_parameter_symbol(
                                        parameter_symbol,
                                        substitutions,
                                    )
                            {
                                return Ok(Some(types.unwrap_value_type_id(mapped)));
                            }

                            // specialized evaluation must not fold with unresolved parameters
                            if mode == StaticEvaluationMode::Specialized {
                                return Ok(None);
                            }
                        }

                        let side_type_id = self.resolve_declared_type_expression(
                            module, profile, side_id, tree, symbols, types, true, true,
                        )?;
                        Ok(Some(side_type_id))
                    };
                let Some(mut left_type_id) = evaluate_side(*left)? else {
                    return Ok(None);
                };
                let Some(mut right_type_id) = evaluate_side(*right)? else {
                    return Ok(None);
                };

                // apply caller substitutions before relation checks
                if let Some(substitutions) = substitutions
                    && !substitutions.is_empty()
                {
                    let mut substitution_cache = HashMap::new();
                    left_type_id = self.substitute_static_parameters(
                        left_type_id,
                        substitutions,
                        types,
                        &mut substitution_cache,
                    );
                    right_type_id = self.substitute_static_parameters(
                        right_type_id,
                        substitutions,
                        types,
                        &mut substitution_cache,
                    );
                }

                // materialize and normalize both sides in type-op relation mode
                let mut materialize_cache = TypeRewriteCache::new();
                left_type_id = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    left_type_id,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                );
                right_type_id = self.materialize_static_arguments_in_type(
                    module,
                    profile,
                    right_type_id,
                    tree,
                    symbols,
                    types,
                    &mut materialize_cache,
                );
                left_type_id = self.normalize_type_with_relation(
                    module,
                    profile,
                    left_type_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::STATIC_EVAL,
                );
                right_type_id = self.normalize_type_with_relation(
                    module,
                    profile,
                    right_type_id,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::STATIC_EVAL,
                );

                // choose the branch using extends assignability semantics
                let options = self.analyze_context_options_for_module(module.id);
                let is_assignable = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    right_type_id,
                    left_type_id,
                    types,
                    &options,
                );
                let selected = if is_assignable == Assignability::NotAssignable {
                    *else_type
                } else {
                    *then_type
                };

                return self.evaluate_static_expression_value_inner(
                    module,
                    profile,
                    selected,
                    tree,
                    symbols,
                    types,
                    enum_symbol,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    visited,
                );
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    // reject sparse array holes
                    if matches!(tree.get(element.value()), Expression::Stub) {
                        return Err(AnalyzeError::ArrayLiteralHole {
                            node: element
                                .value()
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                        mode,
                        substitutions,
                        remote_dependency_stage,
                        visited,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::ArrayExpression { elements: values }
            }
            Expression::TupleExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value_inner(
                        module,
                        profile,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        enum_symbol,
                        mode,
                        substitutions,
                        remote_dependency_stage,
                        visited,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::TupleExpression { elements: values }
            }
            Expression::ObjectExpression { properties } => {
                let mut evaluated_properties = Vec::with_capacity(properties.len());
                for property_id in properties {
                    let property = tree.get(*property_id).clone();
                    let evaluated_property = match property {
                        Property::Field {
                            modifiers,
                            key,
                            value,
                            default,
                            symbol,
                        } => {
                            let Some(value_id) = value else {
                                return Ok(None);
                            };
                            let value = self.evaluate_static_expression_value_inner(
                                module,
                                profile,
                                value_id,
                                tree,
                                symbols,
                                types,
                                enum_symbol,
                                mode,
                                substitutions,
                                remote_dependency_stage,
                                visited,
                            )?;
                            let Some(value) = value else {
                                return Ok(None);
                            };
                            let default = if let Some(default_id) = default {
                                let default_value = self.evaluate_static_expression_value_inner(
                                    module,
                                    profile,
                                    default_id,
                                    tree,
                                    symbols,
                                    types,
                                    enum_symbol,
                                    mode,
                                    substitutions,
                                    remote_dependency_stage,
                                    visited,
                                )?;
                                let Some(default_value) = default_value else {
                                    return Ok(None);
                                };
                                Some(default_value)
                            } else {
                                None
                            };

                            StaticProperty::Field {
                                modifiers,
                                key,
                                value,
                                default,
                                symbol,
                            }
                        }
                        Property::Method { .. } | Property::Spread { .. } => {
                            return Ok(None);
                        }
                    };
                    evaluated_properties.push(evaluated_property);
                }

                StaticExpression::ObjectExpression {
                    properties: evaluated_properties,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(value))
    }

    /// Resolve one constant reference in generic evaluation mode.

    pub(crate) fn static_expression_from_constant_reference_generic(
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
            StaticEvaluationMode::Generic,
            None,
            AnalyzeDependencyStage::Infer,
            visited,
        )
    }

    /// Resolve one constant reference in specialized evaluation mode.

    pub(crate) fn static_expression_from_constant_reference_specialized(
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
            StaticEvaluationMode::Specialized,
            Some(substitutions),
            AnalyzeDependencyStage::Infer,
            visited,
        )
    }

    /// Resolve one constant reference in specialized mode using declared dependency ownership.
    pub(crate) fn static_expression_from_constant_reference_specialized_declared(
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
            StaticEvaluationMode::Specialized,
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

    fn static_expression_from_constant_reference_inner(
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
            let is_constant_cycle_candidate = if mode == StaticEvaluationMode::Specialized {
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
        if symbol.module_id == module.id && mode == StaticEvaluationMode::Generic {
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
                        if mode == StaticEvaluationMode::Specialized {
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
                // generic mode may reuse inferred literal value snapshots
                if mode == StaticEvaluationMode::Generic
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
