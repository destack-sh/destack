use super::StaticEvaluationMode;
use super::constant::StaticCycleDiagnosticMode;
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, RelationMode, TypeRewriteCache,
};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler};
use destack_dir::{
    BinaryOperator, DependencyItem, EnumFieldValue, Expression, GlobalSymbolId, IfCondition,
    IfKind, LocalNodeId, LocalTypeId, NodeTree, NodeType, NormalizationMode, Property, Resolution,
    ScalarLiteral, StaticExpression, StaticKey, StaticParameterKind, StaticProperty, SymbolTable,
    Type, TypeBinaryOperator, TypeTable, UnaryOperator,
};
use destack_workspace::{Module, ProfileId};
use std::collections::{HashMap, HashSet};

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
            StaticEvaluationMode::Parametric,
            None,
            AnalyzeDependencyStage::Infer,
            &mut visited,
        )
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]

    pub(super) fn evaluate_static_expression_value_inner(
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
                            && let Some(type_id) = self.substitution_type_id_for_static_parameter(
                                parameter_symbol,
                                substitutions,
                            )
                        {
                            let value =
                                self.static_expression_from_substitution_type(type_id, types);
                            return Ok(Some(value));
                        }

                        // instantiated evaluation does not permit unresolved static parameters
                        if mode == StaticEvaluationMode::Instantiated {
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

                if let Some(value) = self.resolve_static_constant_reference(
                    module,
                    profile,
                    lookup_symbol,
                    tree,
                    symbols,
                    types,
                    mode,
                    substitutions,
                    remote_dependency_stage,
                    if mode == StaticEvaluationMode::Instantiated {
                        StaticCycleDiagnosticMode::Report
                    } else {
                        StaticCycleDiagnosticMode::Suppress
                    },
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
                    let projected_mode = if mode == StaticEvaluationMode::Instantiated
                        || !merged_substitutions.is_empty()
                    {
                        StaticEvaluationMode::Instantiated
                    } else {
                        StaticEvaluationMode::Parametric
                    };
                    let merged_substitutions = if merged_substitutions.is_empty() {
                        None
                    } else {
                        Some(merged_substitutions)
                    };

                    if let Some(value) = self.resolve_static_constant_reference(
                        module,
                        profile,
                        selection.target_symbol,
                        tree,
                        symbols,
                        types,
                        projected_mode,
                        merged_substitutions.as_ref(),
                        remote_dependency_stage,
                        if projected_mode == StaticEvaluationMode::Instantiated {
                            StaticCycleDiagnosticMode::Report
                        } else {
                            StaticCycleDiagnosticMode::Suppress
                        },
                        visited,
                    )? {
                        return Ok(Some(value));
                    }
                }

                // fall back to resolved static candidates
                if let Some(resolution_id) = types.get_resolution_for_node(node_id) {
                    let resolution = types.get_resolution(resolution_id);
                    if let Resolution::Static { candidate, .. } = resolution
                        && let Some(value) = self.resolve_static_constant_reference(
                            module,
                            profile,
                            candidate.target_symbol,
                            tree,
                            symbols,
                            types,
                            mode,
                            substitutions,
                            remote_dependency_stage,
                            if mode == StaticEvaluationMode::Instantiated {
                                StaticCycleDiagnosticMode::Report
                            } else {
                                StaticCycleDiagnosticMode::Suppress
                            },
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
                        let Some(mut left_type_id) = self.resolve_static_conditional_operand_type(
                            module,
                            profile,
                            *left,
                            tree,
                            symbols,
                            types,
                            substitutions,
                            mode,
                        )?
                        else {
                            return Ok(None);
                        };
                        let Some(mut right_type_id) = self
                            .resolve_static_conditional_operand_type(
                                module,
                                profile,
                                *right,
                                tree,
                                symbols,
                                types,
                                substitutions,
                                mode,
                            )?
                        else {
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

                        // unresolved type operands keep conditional evaluation deferred
                        if mode == StaticEvaluationMode::Instantiated
                            && (!self.type_is_converged_for_static_evaluation(
                                module,
                                profile,
                                left_type_id,
                                symbols,
                                types,
                            ) || !self.type_is_converged_for_static_evaluation(
                                module,
                                profile,
                                right_type_id,
                                symbols,
                                types,
                            ))
                        {
                            return Ok(None);
                        }

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
                let Some(mut left_type_id) = self.resolve_static_conditional_operand_type(
                    module,
                    profile,
                    *left,
                    tree,
                    symbols,
                    types,
                    substitutions,
                    mode,
                )?
                else {
                    return Ok(None);
                };
                let Some(mut right_type_id) = self.resolve_static_conditional_operand_type(
                    module,
                    profile,
                    *right,
                    tree,
                    symbols,
                    types,
                    substitutions,
                    mode,
                )?
                else {
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

                // unresolved type operands keep conditional evaluation deferred
                if !self.type_is_converged_for_static_evaluation(
                    module,
                    profile,
                    left_type_id,
                    symbols,
                    types,
                ) || !self.type_is_converged_for_static_evaluation(
                    module,
                    profile,
                    right_type_id,
                    symbols,
                    types,
                ) {
                    return Ok(None);
                }

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

    /// Resolve one conditional operand type for static branch selection.
    fn resolve_static_conditional_operand_type(
        &self,
        module: &Module,
        profile: ProfileId,
        side_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        mode: StaticEvaluationMode,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // prefer caller substitutions for static parameters
        if let Some((parameter_symbol, _)) =
            self.static_parameter_reference(module, profile, side_id, tree, symbols, types)?
        {
            if let Some(substitutions) = substitutions
                && let Some(mapped) =
                    self.substitution_type_id_for_static_parameter(parameter_symbol, substitutions)
            {
                let mapped = types.unwrap_value_type_id(mapped);
                let is_resolved = self.type_is_converged_for_static_evaluation(
                    module, profile, mapped, symbols, types,
                );
                if is_resolved {
                    return Ok(Some(mapped));
                }
            }

            // parametric mode can keep unresolved parameters symbolic
            if mode == StaticEvaluationMode::Parametric {
                let side_type_id = self.resolve_declared_type_expression(
                    module, profile, side_id, tree, symbols, types, true, true,
                )?;
                return Ok(Some(side_type_id));
            }

            // instantiated mode keeps unresolved parameters deferred
            return Ok(None);
        }

        let side_type_id = self.resolve_declared_type_expression(
            module, profile, side_id, tree, symbols, types, true, true,
        )?;
        Ok(Some(side_type_id))
    }
}
