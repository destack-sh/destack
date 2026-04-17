use super::constant::StaticCycleDiagnosticMode;
use super::{StaticEvaluationDiagnosticMode, StaticEvaluationMode};
use crate::analyze::StaticMemberSymbolKind;
use crate::analyze::common::{CanonicalSymbolMode, TypeContext};
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_artifact::ArtifactKey;
use destack_dir::{
    BinaryOperator, DependencyItem, EnumFieldValue, Expression, GenericParameterKind,
    GlobalSymbolId, IfCondition, IfKind, LocalNodeId, LocalTypeId, NodeType, Property, Resolution,
    ScalarLiteral, StaticExpression, StaticKey, StaticProperty, Type, UnaryOperator,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use std::collections::{HashMap, HashSet};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn evaluate_static_expression_value(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_EVALUATE);

        let mut visited = HashSet::new();
        let mut inner_ctx = ctx.reborrow();
        self.evaluate_static_expression_value_inner(
            &mut inner_ctx,
            expression_id,
            enum_symbol,
            StaticEvaluationMode::Parametric,
            StaticEvaluationDiagnosticMode::Report,
            None,
            destack_artifact::ArtifactKey::dir_analyzed,
            &mut visited,
        )
    }

    /// Query one static expression value without emitting diagnostics.
    pub(crate) fn query_static_expression_value(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        enum_symbol: Option<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_STATIC_EVALUATE);

        let mut visited = HashSet::new();
        let mut inner_ctx = ctx.reborrow();
        self.evaluate_static_expression_value_inner(
            &mut inner_ctx,
            expression_id,
            enum_symbol,
            StaticEvaluationMode::Parametric,
            StaticEvaluationDiagnosticMode::Suppress,
            None,
            destack_artifact::ArtifactKey::dir_analyzed,
            &mut visited,
        )
    }

    /// Evaluate an expression into a static value expression.
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn evaluate_static_expression_value_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        enum_symbol: Option<GlobalSymbolId>,
        mode: StaticEvaluationMode,
        diagnostic_mode: StaticEvaluationDiagnosticMode,
        substitutions: Option<&HashMap<GlobalSymbolId, LocalTypeId>>,
        remote_dependency_artifact: fn(ModuleId, ProfileId) -> ArtifactKey,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<StaticExpression>> {
        let expression = ctx.tree.get(expression_id);

        let value = match expression {
            Expression::ScalarLiteral { value } => StaticExpression::ScalarLiteral {
                value: value.clone(),
            },
            Expression::TypeLiteral { value } => StaticExpression::TypeLiteral {
                value: value.clone(),
            },
            Expression::Type { resolved_type, .. } => StaticExpression::Type { ty: *resolved_type },
            Expression::Parenthesized { expression } => {
                return self.evaluate_static_expression_value_inner(
                    &mut ctx.reborrow(),
                    *expression,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
                    visited,
                );
            }
            Expression::As {
                expression: value, ..
            }
            | Expression::Satisfies {
                expression: value, ..
            } => {
                return self.evaluate_static_expression_value_inner(
                    &mut ctx.reborrow(),
                    *value,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
                    visited,
                );
            }
            Expression::Unary { operator, right } => {
                let right_value = self.evaluate_static_expression_value_inner(
                    &mut ctx.reborrow(),
                    *right,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
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
                    &mut ctx.reborrow(),
                    *left,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
                    visited,
                )?;
                let right_value = self.evaluate_static_expression_value_inner(
                    &mut ctx.reborrow(),
                    *right,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
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
                            && self.static_expression_may_be_numeric(&left_value, ctx.types)
                            && self.static_expression_may_be_numeric(&right_value, ctx.types)
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
                if let Some((parameter_symbol, kind)) =
                    self.generic_parameter_expression_reference(&mut ctx.reborrow(), expression_id)?
                {
                    if kind == GenericParameterKind::Value {
                        // use substitution values when available
                        if let Some(substitutions) = substitutions
                            && let Some(type_id) = self.substitution_type_id_for_static_parameter(
                                parameter_symbol,
                                substitutions,
                            )
                        {
                            let value =
                                self.static_expression_from_substitution_type(type_id, ctx.types);
                            return Ok(Some(value));
                        }

                        // instantiated evaluation does not permit unresolved static parameters
                        if mode == StaticEvaluationMode::Instantiated {
                            return Ok(None);
                        }

                        let reference_type = Type::Reference {
                            symbol: parameter_symbol,
                            generic_arguments: None,
                        };
                        let ty = ctx.types.insert_type_from(reference_type, expression_id);
                        return Ok(Some(StaticExpression::Type { ty }));
                    }

                    if diagnostic_mode == StaticEvaluationDiagnosticMode::Report {
                        self.error(AnalyzeError::StaticParameterRequiresComptime {
                            node: expression_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                    return Ok(None);
                }

                // unwrap import/export dependency items before canonicalizing
                let mut lookup_symbol = *target_symbol;
                if lookup_symbol.module_id == ctx.module.id {
                    let symbol_entry = ctx.symbols.get_symbol(lookup_symbol.local_id);
                    if let Some(primary_declaration) = symbol_entry.primary_declaration
                        && primary_declaration.local_id.ty == NodeType::DependencyItem
                    {
                        let item_id = primary_declaration.local_id.into_typed::<DependencyItem>();
                        if let DependencyItem::Local { target_symbol, .. }
                        | DependencyItem::Remote { target_symbol, .. } = ctx.tree.get(item_id)
                        {
                            lookup_symbol = *target_symbol;
                        }
                    }
                }

                if let Some(value) = self.resolve_static_constant_reference(
                    &mut ctx.reborrow(),
                    lookup_symbol,
                    mode,
                    substitutions,
                    remote_dependency_artifact,
                    if mode == StaticEvaluationMode::Instantiated {
                        StaticCycleDiagnosticMode::Report
                    } else {
                        StaticCycleDiagnosticMode::Suppress
                    },
                    visited,
                )? {
                    return Ok(Some(value));
                }

                // resolve enum member references eagerly when evaluating enum field values
                if let Some(enum_symbol) = enum_symbol
                    && let Some(value) = self.enum_field_value_for_symbol_reference(
                        &mut ctx.reborrow(),
                        enum_symbol,
                        lookup_symbol,
                    )?
                {
                    return Ok(Some(StaticExpression::ScalarLiteral {
                        value: match value {
                            EnumFieldValue::Int(value) => ScalarLiteral::Integer(value),
                            EnumFieldValue::String(value) => ScalarLiteral::String(value),
                        },
                    }));
                }

                // preserve symbolic static value references for later substitution
                // keep enum fields concrete so enum-member normalization can process them
                if self.symbol_is_symbolic_static_value_reference(ctx.type_view(), lookup_symbol)?
                    && self.query_static_member_symbol_kind_for_symbol(
                        ctx.tree_symbol_view(),
                        lookup_symbol,
                    )? != Some(StaticMemberSymbolKind::EnumField)
                {
                    let reference_type = Type::Reference {
                        symbol: lookup_symbol,
                        generic_arguments: None,
                    };
                    let ty = ctx.types.insert_type_from(reference_type, expression_id);
                    return Ok(Some(StaticExpression::Type { ty }));
                }

                return Ok(None);
            }
            Expression::Member {
                left,
                name,
                generic_arguments: _,
            } => {
                let node_id = expression_id.into_global_any(ctx.module.id);
                let Some(name) = *name else {
                    let error_type_id = ctx
                        .types
                        .insert_type_from_any(Type::Error, expression_id.into_any());
                    return Ok(Some(StaticExpression::Type { ty: error_type_id }));
                };
                let member_key = StaticKey::Name(name);

                // reject cross-module re-entry before projection lookup
                // this keeps static evaluation fail-closed for module-level cycles
                let receiver_symbol = self
                    .reference_symbol_for_expression(ctx.tree_symbol_view(), *left)
                    .or_else(|| ctx.tree.get(*left).target_symbol());
                if let Some(receiver_symbol) = receiver_symbol {
                    let receiver_symbol = self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        receiver_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    let reenters_active_module = receiver_symbol.module_id != ctx.module.id
                        && visited.iter().any(|visited_symbol| {
                            visited_symbol.module_id == receiver_symbol.module_id
                        });
                    if reenters_active_module {
                        if diagnostic_mode == StaticEvaluationDiagnosticMode::Report {
                            self.error(AnalyzeError::CircularStaticArgument {
                                node: expression_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });
                        }

                        let error_type_id = ctx
                            .types
                            .insert_type_from_any(Type::Error, expression_id.into_any());
                        return Ok(Some(StaticExpression::Type { ty: error_type_id }));
                    }
                }

                // resolve projected members in value space for static expressions
                let projection_selection = self.select_associated_projection_member_symbol(
                    &mut ctx.reborrow(),
                    expression_id,
                    *left,
                    member_key,
                    Some(StaticMemberSymbolKind::AssociatedComptimeConst),
                    true,
                    true,
                )?;
                if let Some(selection) = projection_selection {
                    // defer projections whose receiver arguments are not ready yet
                    if mode == StaticEvaluationMode::Parametric
                        && self.receiver_projection_arguments_require_deferral(
                            ctx.type_view(),
                            &selection.receiver_arguments,
                        )
                    {
                        let receiver_arguments = if selection.receiver_arguments.is_empty() {
                            None
                        } else {
                            Some(selection.receiver_arguments.clone())
                        };
                        let reference_type = Type::Reference {
                            symbol: selection.target_symbol,
                            generic_arguments: receiver_arguments,
                        };
                        let ty = ctx.types.insert_type_from(reference_type, expression_id);
                        return Ok(Some(StaticExpression::Type { ty }));
                    }

                    let projection_environment = self.projection_environment_for_member(
                        &mut ctx.reborrow(),
                        expression_id.into_any(),
                        selection.target_symbol,
                        Some(selection.receiver_symbol),
                        &selection.receiver_arguments,
                        None,
                        None,
                        Some(visited),
                    )?;
                    let projection_substitutions = projection_environment.substitutions;

                    // compose projection substitutions with the current evaluation environment
                    let mut merged_substitutions =
                        HashMap::with_capacity(projection_substitutions.len());
                    merged_substitutions.extend(projection_substitutions);
                    if let Some(substitutions) = substitutions {
                        merged_substitutions
                            .extend(substitutions.iter().map(|(key, value)| (*key, *value)));
                    }
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
                        &mut ctx.reborrow(),
                        selection.target_symbol,
                        projected_mode,
                        merged_substitutions.as_ref(),
                        remote_dependency_artifact,
                        if projected_mode == StaticEvaluationMode::Instantiated {
                            StaticCycleDiagnosticMode::Report
                        } else {
                            StaticCycleDiagnosticMode::Suppress
                        },
                        visited,
                    )? {
                        return Ok(Some(value));
                    }

                    // preserve unresolved associated comptime projections as symbolic references
                    let receiver_arguments = if selection.receiver_arguments.is_empty() {
                        None
                    } else {
                        Some(selection.receiver_arguments.clone())
                    };
                    let reference_type = Type::Reference {
                        symbol: selection.target_symbol,
                        generic_arguments: receiver_arguments,
                    };
                    let ty = ctx.types.insert_type_from(reference_type, expression_id);
                    return Ok(Some(StaticExpression::Type { ty }));
                }

                // fall back to resolved static candidates
                if let Some(resolution_id) = ctx.types.get_resolution_for_node(node_id) {
                    let candidate_symbol = match ctx.types.get_resolution(resolution_id) {
                        Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
                        _ => None,
                    };
                    if let Some(candidate_symbol) = candidate_symbol
                        && let Some(value) = self.resolve_static_constant_reference(
                            &mut ctx.reborrow(),
                            candidate_symbol,
                            mode,
                            substitutions,
                            remote_dependency_artifact,
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
                let Some(target_symbol) =
                    self.enum_field_symbol_for_name(ctx.type_view(), enum_symbol, name)?
                else {
                    return Ok(None);
                };
                let Some(value) = self.enum_field_value_for_symbol_reference(
                    &mut ctx.reborrow(),
                    enum_symbol,
                    target_symbol,
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

                let condition_holds = match ctx.tree.get(*condition_expression) {
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
                    &mut ctx.reborrow(),
                    selected,
                    enum_symbol,
                    mode,
                    diagnostic_mode,
                    substitutions,
                    remote_dependency_artifact,
                    visited,
                );
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let element = ctx.tree.get(*element_id);
                    // reject sparse array holes
                    if matches!(ctx.tree.get(element.value()), Expression::Stub) {
                        return Err(AnalyzeError::ArrayLiteralHole {
                            node: element
                                .value()
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                    let value = self.evaluate_static_expression_value_inner(
                        &mut ctx.reborrow(),
                        element.value(),
                        enum_symbol,
                        mode,
                        diagnostic_mode,
                        substitutions,
                        remote_dependency_artifact,
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
                    let element = ctx.tree.get(*element_id);
                    let value = self.evaluate_static_expression_value_inner(
                        &mut ctx.reborrow(),
                        element.value(),
                        enum_symbol,
                        mode,
                        diagnostic_mode,
                        substitutions,
                        remote_dependency_artifact,
                        visited,
                    )?;
                    let Some(value) = value else {
                        return Ok(None);
                    };
                    values.push(value);
                }

                StaticExpression::TupleExpression { elements: values }
            }
            Expression::ObjectExpression { properties, .. } => {
                let mut evaluated_properties = Vec::with_capacity(properties.len());
                for property_id in properties {
                    let property = ctx.tree.get(*property_id).clone();
                    let evaluated_property = match property {
                        Property::Field { key, value, symbol } => {
                            let value = self.evaluate_static_expression_value_inner(
                                &mut ctx.reborrow(),
                                value,
                                enum_symbol,
                                mode,
                                diagnostic_mode,
                                substitutions,
                                remote_dependency_artifact,
                                visited,
                            )?;
                            let Some(value) = value else {
                                return Ok(None);
                            };

                            StaticProperty::Field { key, value, symbol }
                        }
                        Property::Spread { value, symbol } => {
                            let value = self.evaluate_static_expression_value_inner(
                                &mut ctx.reborrow(),
                                value,
                                enum_symbol,
                                mode,
                                diagnostic_mode,
                                substitutions,
                                remote_dependency_artifact,
                                visited,
                            )?;
                            let Some(value) = value else {
                                return Ok(None);
                            };

                            StaticProperty::Spread { value, symbol }
                        }
                        Property::Method { .. } => {
                            return Ok(None);
                        }
                        Property::Error { .. } => {
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
}
