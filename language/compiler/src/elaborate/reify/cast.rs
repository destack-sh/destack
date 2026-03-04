use destack_dir as dir;
use destack_workspace::ImplicitCollectionConversionPolicy;
use dir::{
    Argument, BinaryOperator, Block, CastOperator, CastSource, Declaration, Declarator, DynamicKey,
    EnumBackingType, Expression, GlobalSymbolId, IfCondition, IfKind, Instance, LocalNodeId,
    LocalTypeId, MatchCase, Member, NodeType, Path, Property, Resolution, ResolutionCandidate,
    ResolvedSignature, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, SymbolSpace,
    SymbolType, Type, TypeBinaryOperator, TypeElement, TypeLiteral, UnaryOperator, WellKnownSymbol,
};

use super::r#type::{
    are_types_semantically_equal, common_numeric_type_id_for_binary, is_any_type, is_integer_type,
    is_nullable_union, is_object_type, is_pointer_type, is_scalar_literal_type, is_string_type,
    is_union_type, is_unknown_type, numeric_cast_operator,
};
use crate::analyze::TypeView;
use crate::analyze::common::TypeContext;
use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult, ElaborateWarning};

/// The resolved record-like target data for reification.
struct RecordLikeTargetInfo {
    /// The key type for the record-like conversion.
    key_type_id: LocalTypeId,
    /// The target map symbol that provides `from`.
    map_symbol: GlobalSymbolId,
    /// The resolved map type reference.
    map_type_id: LocalTypeId,
    /// The entry tuple type for `Map<K, V>`.
    entry_tuple_type_id: LocalTypeId,
    /// The array type for map entries.
    entries_array_type_id: LocalTypeId,
    /// Static arguments for `Map<K, V>`.
    map_static_arguments: Vec<StaticArgument>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify an explicit cast expression into a cast node.
    pub(super) fn reify_explicit_cast_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // read the source and target type ids
        let value_type_id = state
            .types
            .get_declared_or_inferred_type_id(value.into_global_any(state.ctx.module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            })?;
        let target_type_id = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            })?;

        // reify record-like and sized array literals into collection construction
        if let Some(reified) =
            self.reify_record_like_object_literal(state, expression_id, value, target_type_id)?
        {
            let source_id = reified.into_global_any(state.ctx.module_id);
            let target_id = expression_id.into_global_any(state.ctx.module_id);
            state.types.copy_node_analysis(source_id, target_id);
            let expression = state.tree.get(reified).clone();
            state.tree.replace(expression_id, expression);
            return Ok(());
        }
        if let Some(reified) =
            self.reify_array_sized_value(state, expression_id, value, target_type_id)?
        {
            let source_id = reified.into_global_any(state.ctx.module_id);
            let target_id = expression_id.into_global_any(state.ctx.module_id);
            state.types.copy_node_analysis(source_id, target_id);
            let expression = state.tree.get(reified).clone();
            state.tree.replace(expression_id, expression);
            return Ok(());
        }

        // classify the cast
        let operator = self.cast_operator_for_types(state, value_type_id, target_type_id);
        // replace the expression with a cast node
        state.tree.replace(
            expression_id,
            Expression::Cast {
                operator,
                source: CastSource::Explicit,
                value,
                target_type,
            },
        );

        Ok(())
    }

    /// Reify a must expression (`value!`) into an explicit implicit cast.
    ///
    /// This preserves non-null assertion semantics by selecting the
    /// appropriate nullable or union downcast operator.
    pub(super) fn reify_must_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // require the inferred must result type
        let Some(target_type_id) = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };

        // wrap the operand with the required downcast when needed
        let reified_value_id =
            self.wrap_value_with_cast(state, expression_id, left, target_type_id)?;

        // replace the must wrapper with the reified expression
        let reified_expression = state.tree.get(reified_value_id).clone();
        state.tree.replace(expression_id, reified_expression);
        state.types.copy_node_analysis(
            reified_value_id.into_global_any(state.ctx.module_id),
            expression_id.into_global_any(state.ctx.module_id),
        );

        Ok(())
    }

    /// Reify implicit casts for reference expressions when the node type narrows.
    pub(super) fn reify_implicit_casts_in_reference(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> ElaborateResult<()> {
        // skip references that live in type positions
        if self.reference_is_non_value_operand(state, expression_id) {
            return Ok(());
        }

        // skip references used by guard operators
        if self.reference_is_guard_operand(state, expression_id) {
            return Ok(());
        }

        // skip references that resolve in type-only space
        let symbol_space = if target_symbol.module_id == state.ctx.module.id {
            let symbol_entry = state.symbols.get_symbol(target_symbol.local_id);
            symbol_entry.space
        } else {
            let remote_module = self.program.modules.get(target_symbol.module_id);
            let remote_module = remote_module.read();
            let dir = remote_module.dir(state.ctx.profile);
            let remote_symbols = dir.symbols.read();
            let symbol_entry = remote_symbols.get_symbol(target_symbol.local_id);
            symbol_entry.space
        };
        if !matches!(symbol_space, SymbolSpace::Value | SymbolSpace::TypeValue) {
            return Ok(());
        }

        // skip references marked as type expressions
        if let Some(inferred_type_id) = state
            .types
            .get_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
            && matches!(state.types.get_type(inferred_type_id), Type::Value { .. })
        {
            return Ok(());
        }

        // require a declared or inferred node type
        let Some(target_type_id) = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };

        // require a source value type from the referenced symbol
        let Some(source_type_id) = state.types.get_value_type_id(target_symbol) else {
            return Ok(());
        };

        // unwrap type aliases and values
        let target_type_id = self.unwrap_value_type_id(state, target_type_id);
        let source_type_id = self.unwrap_value_type_id(state, source_type_id);

        // skip when the types are semantically identical
        if are_types_semantically_equal(
            state.types.get_type(source_type_id),
            state.types.get_type(target_type_id),
            state.types,
        ) {
            return Ok(());
        }

        // classify the cast for this narrowing
        let operator = self.cast_operator_for_types(state, source_type_id, target_type_id);
        if operator == CastOperator::Identity {
            return Ok(());
        }

        // move the reference into a new value node
        let expression = state.tree.get(expression_id).clone();
        let scope = state.tree.get_scope(expression_id);
        let value_expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
        let value_expression_id = state.tree.insert(value_expression_id, expression);
        state.types.set_inferred_type(
            value_expression_id.into_global_any(state.ctx.module_id),
            source_type_id,
        );

        // build the target type expression
        let target_expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
        let target_expression = match state.types.get_type(target_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: target_type_id,
            },
        };
        let target_expression_id = state.tree.insert(target_expression_id, target_expression);

        // set the inferred type for the target type expression
        let target_type_value = Type::Value {
            value: target_type_id,
        };
        let target_type_value_id = state
            .types
            .insert_type_from(target_type_value, target_expression_id);
        state.types.set_inferred_type(
            target_expression_id.into_global_any(state.ctx.module_id),
            target_type_value_id,
        );

        // replace the reference with the cast expression
        let cast_expression = Expression::Cast {
            operator,
            source: CastSource::Implicit,
            value: value_expression_id,
            target_type: target_expression_id,
        };
        if self.options.elaborate_parenthesize_casts {
            let cast_expression_id = state.tree.reserve_from(
                NodeType::Expression,
                expression_id.into_any(),
                scope,
                None,
            );
            let cast_expression_id = state.tree.insert(cast_expression_id, cast_expression);
            state.types.set_inferred_type(
                cast_expression_id.into_global_any(state.ctx.module_id),
                target_type_id,
            );
            state.tree.replace(
                expression_id,
                Expression::Parenthesized {
                    expression: cast_expression_id,
                },
            );
        } else {
            state.tree.replace(expression_id, cast_expression);
        }
        state.types.set_inferred_type(
            expression_id.into_global_any(state.ctx.module_id),
            target_type_id,
        );

        Ok(())
    }

    /// Return true when a reference expression is used in a known non-value position.
    fn reference_is_non_value_operand(
        &self,
        state: &ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current_id = expression_id;

        loop {
            let Some(parent_id) = state.tree.get_parent(current_id.id) else {
                return false;
            };

            if let Ok(parent_expression_id) = parent_id.try_into_typed::<Expression>() {
                let parent_expression = state.tree.get(parent_expression_id);
                match parent_expression {
                    Expression::Parenthesized { expression } if *expression == current_id => {
                        current_id = parent_expression_id;
                        continue;
                    }
                    Expression::Cast { target_type, .. } => return *target_type == current_id,
                    Expression::TaggedScalarExpression { ty, .. }
                    | Expression::TaggedTupleExpression { ty, .. }
                    | Expression::TaggedObjectExpression { ty, .. } => return *ty == current_id,
                    Expression::TypeBinary {
                        operator, right, ..
                    } => {
                        return match operator {
                            TypeBinaryOperator::Cast => *right == current_id,
                            _ => false,
                        };
                    }
                    _ => return false,
                }
            }

            match parent_id.ty {
                NodeType::Declarator => {
                    let parent_declarator = state.tree.get(parent_id.into_typed::<Declarator>());
                    return parent_declarator.ty == Some(current_id);
                }
                NodeType::Declaration => {
                    let parent_declaration = state.tree.get(parent_id.into_typed::<Declaration>());
                    return match parent_declaration {
                        Declaration::Function { signature, .. } => {
                            signature.return_type == Some(current_id)
                        }
                        _ => false,
                    };
                }
                NodeType::Member => {
                    let parent_member = state.tree.get(parent_id.into_typed::<Member>());
                    return match parent_member {
                        Member::Method { signature, .. } => {
                            signature.return_type == Some(current_id)
                        }
                        _ => false,
                    };
                }
                NodeType::Property => {
                    let parent_property = state.tree.get(parent_id.into_typed::<Property>());
                    return match parent_property {
                        Property::Method { signature, .. } => {
                            signature.return_type == Some(current_id)
                        }
                        _ => false,
                    };
                }
                _ => return false,
            }
        }
    }

    /// Return true when a reference is used by a guard operator.
    fn reference_is_guard_operand(
        &self,
        state: &ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(parent_id) = state.tree.get_parent(expression_id.id) else {
            return false;
        };
        let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
            return false;
        };

        let parent = state.tree.get(parent_id);
        match parent {
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                matches!(
                    operator,
                    TypeBinaryOperator::Is
                        | TypeBinaryOperator::InstanceOf
                        | TypeBinaryOperator::Cast
                ) && (*left == expression_id || *right == expression_id)
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => match operator {
                BinaryOperator::InstanceOf => *left == expression_id || *right == expression_id,
                BinaryOperator::In => *right == expression_id,
                _ => false,
            },
            Expression::Unary { operator, right } => {
                matches!(operator, UnaryOperator::Typeof) && *right == expression_id
            }
            _ => false,
        }
    }

    /// Reify implicit casts in let and using bindings.
    pub(super) fn reify_implicit_casts_in_binding(
        &self,
        state: &mut ElaborateState<'_>,
        declarators: &[LocalNodeId<Declarator>],
    ) -> ElaborateResult<()> {
        // visit each declarator
        for declarator_id in declarators {
            // skip declarators without a value
            let declarator = state.tree.get(*declarator_id).clone();
            let Some(value_id) = declarator.value else {
                continue;
            };
            let Some(target_type_id) = state
                .types
                .get_declared_type_id(declarator_id.into_global_any(state.ctx.module_id))
            else {
                continue;
            };

            // wrap the value with a cast when needed
            let cast_value_id =
                self.wrap_value_with_cast(state, value_id, value_id, target_type_id)?;

            // update the declarator when the value changes
            if cast_value_id != value_id {
                let updated = Declarator {
                    value: Some(cast_value_id),
                    ..declarator
                };
                state.tree.replace(*declarator_id, updated);
            }
        }

        Ok(())
    }

    /// Reify implicit casts in an assignment expression.
    pub(super) fn reify_implicit_casts_in_assignment(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // read the target type from the left hand side
        let target_type_id = self.value_type_id_for_expression(state, left).ok_or(
            ElaborateError::UnsupportedConstruct {
                node: left
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            },
        )?;

        // wrap the right hand side when needed
        let cast_right_id =
            self.wrap_value_with_cast(state, expression_id, right, target_type_id)?;

        // replace the assignment when the value changes
        if cast_right_id != right {
            state.tree.replace(
                expression_id,
                Expression::Assign {
                    left,
                    right: cast_right_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in a return expression.
    pub(super) fn reify_implicit_casts_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
    ) -> ElaborateResult<()> {
        // skip returns without values
        let Some(value_id) = value else {
            return Ok(());
        };

        // read the declared return type
        let Some(target_type_id) = self.enclosing_return_type(state, expression_id) else {
            return Ok(());
        };

        // wrap the return value when needed
        let cast_value_id =
            self.wrap_value_with_cast(state, expression_id, value_id, target_type_id)?;

        // replace the return when the value changes
        if cast_value_id != value_id {
            state.tree.replace(
                expression_id,
                Expression::Return {
                    value: Some(cast_value_id),
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in call arguments.
    pub(super) fn reify_implicit_casts_in_call(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
    ) -> ElaborateResult<()> {
        // resolve expected types for arguments
        let Some(expected_argument_types) =
            self.expected_argument_types_for_call(state, expression_id, dynamic_arguments)?
        else {
            return Ok(());
        };

        // visit each argument and insert casts as needed
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let Some(expected_type_id) = expected_argument_types.get(index).copied().flatten()
            else {
                continue;
            };

            let argument = state.tree.get(*argument_id).clone();
            let value_id = argument.value();

            let cast_value_id =
                self.wrap_value_with_cast(state, expression_id, value_id, expected_type_id)?;

            if cast_value_id == value_id {
                continue;
            }

            let updated = match argument {
                Argument::Named {
                    modifiers, name, ..
                } => Argument::Named {
                    modifiers,
                    name,
                    value: cast_value_id,
                },
                Argument::Labeled {
                    modifiers, label, ..
                } => Argument::Labeled {
                    modifiers,
                    label,
                    value: cast_value_id,
                },
                Argument::Positional { modifiers, .. } => Argument::Positional {
                    modifiers,
                    value: cast_value_id,
                },
                Argument::Spread {
                    modifiers, label, ..
                } => Argument::Spread {
                    modifiers,
                    label,
                    value: cast_value_id,
                },
            };
            state.tree.replace(*argument_id, updated);
        }

        Ok(())
    }

    /// Collapse redundant nested casts to the same target type.
    pub(super) fn normalize_redundant_casts(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        // walk all expressions to find nested casts
        for expression_id in state.tree.iter_node_ids_of_type::<Expression>() {
            // skip inactive expressions
            if !self.is_active_in_state(state, expression_id.into_any()) {
                continue;
            }

            let Expression::Cast {
                value,
                target_type,
                operator,
                source,
            } = state.tree.get(expression_id).clone()
            else {
                continue;
            };

            // require the cast value to be another cast
            let Expression::Cast {
                value: inner_value,
                target_type: inner_target_type,
                ..
            } = state.tree.get(value).clone()
            else {
                continue;
            };

            // resolve target type ids for both casts
            let Some(outer_target_type_id) = self.type_id_for_type_expression(state, target_type)
            else {
                continue;
            };
            let Some(inner_target_type_id) =
                self.type_id_for_type_expression(state, inner_target_type)
            else {
                continue;
            };

            // keep nested casts when targets differ
            if !are_types_semantically_equal(
                state.types.get_type(outer_target_type_id),
                state.types.get_type(inner_target_type_id),
                state.types,
            ) {
                let options = self.analyze_context_options_for_module(state.ctx.module_id);
                let mut ctx = TypeContext::new(
                    state.ctx.module,
                    state.ctx.profile,
                    &options,
                    state.tree,
                    state.symbols,
                    state.types,
                );
                let to_outer = self.is_type_assignable(
                    &mut ctx.reborrow(),
                    outer_target_type_id,
                    inner_target_type_id,
                );
                let to_inner = self.is_type_assignable(
                    &mut ctx.reborrow(),
                    inner_target_type_id,
                    outer_target_type_id,
                );
                if !(to_outer.is_assignable() && to_inner.is_assignable()) {
                    continue;
                }
            }

            // replace with a single cast to the shared target
            state.tree.replace(
                expression_id,
                Expression::Cast {
                    operator,
                    source,
                    value: inner_value,
                    target_type: inner_target_type,
                },
            );
            state.types.set_inferred_type(
                expression_id.into_global_any(state.ctx.module_id),
                outer_target_type_id,
            );
        }

        Ok(())
    }

    /// Reify implicit casts in ternary expressions.
    pub(super) fn reify_implicit_casts_in_ternary(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
    ) -> ElaborateResult<()> {
        // use the expression type as the target for both branches
        let Some(target_type_id) = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };

        // cast the then branch when needed
        let cast_then_id =
            self.wrap_value_with_cast(state, expression_id, then_expression, target_type_id)?;

        // cast the else branch when present
        let cast_else_id = if let Some(else_expression) = else_expression {
            let cast_else_id =
                self.wrap_value_with_cast(state, expression_id, else_expression, target_type_id)?;
            Some(cast_else_id)
        } else {
            None
        };

        // update the ternary expression when any branch changes
        if cast_then_id != then_expression || cast_else_id != else_expression {
            state.tree.replace(
                expression_id,
                Expression::If {
                    kind: IfKind::Ternary,
                    condition: IfCondition::Expression { condition },
                    then_expression: cast_then_id,
                    else_expression: cast_else_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in match case bodies.
    pub(super) fn reify_implicit_casts_in_match(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) -> ElaborateResult<()> {
        // use the match expression type as the target
        let Some(target_type_id) = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };

        // visit each case and cast the produced value
        for case_id in cases {
            let case = state.tree.get(*case_id).clone();
            match case {
                MatchCase::Expression {
                    selector,
                    body,
                    scope,
                } => {
                    let cast_body_id =
                        self.wrap_value_with_cast(state, expression_id, body, target_type_id)?;

                    if cast_body_id != body {
                        state.tree.replace(
                            *case_id,
                            MatchCase::Expression {
                                selector,
                                body: cast_body_id,
                                scope,
                            },
                        );
                    }
                }
                MatchCase::Block {
                    selector: _,
                    body,
                    scope: _,
                } => {
                    let block = state.tree.get(body).clone();
                    let Some(last_expression_id) = block.expressions.last().copied() else {
                        continue;
                    };

                    let cast_last_id = self.wrap_value_with_cast(
                        state,
                        expression_id,
                        last_expression_id,
                        target_type_id,
                    )?;

                    if cast_last_id != last_expression_id {
                        let mut expressions = block.expressions;
                        let Some(last_expression) = expressions.last_mut() else {
                            continue;
                        };
                        *last_expression = cast_last_id;
                        state.tree.replace(
                            body,
                            Block {
                                scope: block.scope,
                                expressions,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Reify implicit casts in builtin binary expressions.
    pub(super) fn reify_implicit_casts_in_binary(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // skip binary expressions without a resolution
        if state
            .types
            .get_resolution_for_node(expression_id.into_global_any(state.ctx.module_id))
            .is_none()
        {
            return Ok(());
        }

        // require numeric operators
        if !self.is_numeric_binary_operator(operator) {
            return Ok(());
        }

        // read operand type ids
        let left_type_id = self.value_type_id_for_expression(state, left).ok_or(
            ElaborateError::UnsupportedConstruct {
                node: left
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            },
        )?;
        let right_type_id = self.value_type_id_for_expression(state, right).ok_or(
            ElaborateError::UnsupportedConstruct {
                node: right
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            },
        )?;

        // compute the common numeric type for both operands
        let Some(target_type_id) = common_numeric_type_id_for_binary(
            left_type_id,
            right_type_id,
            expression_id,
            state.types,
        ) else {
            return Ok(());
        };

        // align the binary expression type with the chosen numeric type
        state.types.set_inferred_type(
            expression_id.into_global_any(state.ctx.module_id),
            target_type_id,
        );

        // wrap both operands when needed
        let cast_left_id = self.wrap_value_with_cast_for_numeric_binary(
            state,
            expression_id,
            left,
            target_type_id,
        )?;
        let cast_right_id = self.wrap_value_with_cast_for_numeric_binary(
            state,
            expression_id,
            right,
            target_type_id,
        )?;

        // update the binary expression when either operand changes
        if cast_left_id != left || cast_right_id != right {
            state.tree.replace(
                expression_id,
                Expression::Binary {
                    left: cast_left_id,
                    operator,
                    right: cast_right_id,
                },
            );
        }

        Ok(())
    }

    /// Wrap a value in a cast when the target type differs.
    fn wrap_value_with_cast(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.wrap_value_with_cast_internal(state, origin_id, value_id, target_type_id, true)
    }

    /// Wrap a value in a cast for numeric binary alignment.
    fn wrap_value_with_cast_for_numeric_binary(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.wrap_value_with_cast_internal(state, origin_id, value_id, target_type_id, false)
    }

    /// Wrap a value in a cast with numeric literal elision.
    fn wrap_value_with_cast_internal(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        allow_assignable_skip: bool,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // read the source type id
        let value_type_id = self.value_type_id_for_expression(state, value_id).ok_or(
            ElaborateError::UnsupportedConstruct {
                node: value_id
                    .into_global_any(state.ctx.module_id)
                    .into_anchored(Some(state.ctx.profile)),
            },
        )?;

        // reify record-like and sized array literals into collection construction
        if let Some(reified) =
            self.reify_record_like_object_literal(state, origin_id, value_id, target_type_id)?
        {
            return Ok(reified);
        }
        if let Some(reified) =
            self.reify_array_sized_value(state, origin_id, value_id, target_type_id)?
        {
            return Ok(reified);
        }

        // resolve unevaluated target types for cast classification
        let options = self.analyze_context_options_for_module(state.ctx.module.id);
        let target_type_source = state.types.get_type_source(target_type_id);
        let target_type_id = {
            let mut ctx = TypeContext::new(
                state.ctx.module,
                state.ctx.profile,
                &options,
                state.tree,
                state.symbols,
                state.types,
            );
            self.ensure_type_evaluated(&mut ctx, target_type_id)
                .map_err(|_| ElaborateError::UnsupportedConstruct {
                    node: target_type_source
                        .into_global(state.ctx.module_id)
                        .into_anchored(Some(state.ctx.profile)),
                })?
        };

        // check casts that change representation despite matching type ids
        let value_type = state.types.get_type(value_type_id).clone();
        let target_type = state.types.get_type(target_type_id).clone();

        let value_is_concrete = self.is_concrete_resolution(state, value_id)
            || self.is_concrete_new_expression(state, value_id)
            || self.is_tagged_expression(state, value_id);
        let value_is_nullish_literal = matches!(
            value_type,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        );
        let types_match = are_types_semantically_equal(&value_type, &target_type, state.types);
        let source_is_interface = self.is_interface_reference_type(state, value_type_id);
        let target_is_interface = self.is_interface_reference_type(state, target_type_id);

        // figure out if we need a representation change cast
        let requires_interface_upcast =
            target_is_interface && (!source_is_interface || value_is_concrete || !types_match);
        let requires_union_upcast =
            is_union_type(&target_type) && (value_is_concrete || value_is_nullish_literal);
        let requires_nullable_upcast = is_nullable_union(&target_type, state.types)
            && (value_is_concrete || value_is_nullish_literal);
        let requires_representation_cast =
            requires_interface_upcast || requires_union_upcast || requires_nullable_upcast;
        if types_match && !requires_representation_cast {
            return Ok(value_id);
        }

        // skip when the types are mutually assignable
        let (to_target, to_source) = {
            let mut ctx = TypeContext::new(
                state.ctx.module,
                state.ctx.profile,
                &options,
                state.tree,
                state.symbols,
                state.types,
            );
            let to_target =
                self.is_type_assignable(&mut ctx.reborrow(), target_type_id, value_type_id);
            let to_source =
                self.is_type_assignable(&mut ctx.reborrow(), value_type_id, target_type_id);
            (to_target, to_source)
        };

        // skip only semantic identity casts
        // true upcasts still need explicit reify so lower can change representation
        if allow_assignable_skip
            && !requires_representation_cast
            && to_target.is_assignable()
            && to_source.is_assignable()
        {
            return Ok(value_id);
        }

        // skip when the value is already cast to an equivalent type
        if let Expression::Cast { target_type, .. } = state.tree.get(value_id)
            && let Some(existing_target_type_id) =
                self.type_id_for_type_expression(state, *target_type)
        {
            let (to_target, to_source) = {
                let mut ctx = TypeContext::new(
                    state.ctx.module,
                    state.ctx.profile,
                    &options,
                    state.tree,
                    state.symbols,
                    state.types,
                );
                let to_target = self.is_type_assignable(
                    &mut ctx.reborrow(),
                    target_type_id,
                    existing_target_type_id,
                );
                let to_source = self.is_type_assignable(
                    &mut ctx.reborrow(),
                    existing_target_type_id,
                    target_type_id,
                );
                (to_target, to_source)
            };
            if to_target.is_assignable() && to_source.is_assignable() {
                return Ok(value_id);
            }
        }

        // classify the cast
        let mut operator = self.cast_operator_for_types(state, value_type_id, target_type_id);
        if requires_interface_upcast && operator == CastOperator::Identity {
            operator = CastOperator::InstanceUpcast;
        }
        if requires_union_upcast && operator == CastOperator::Identity {
            operator = CastOperator::UnionUpcast;
        }
        if requires_nullable_upcast && operator == CastOperator::Identity {
            operator = CastOperator::NullableUpcast;
        }

        // skip redundant union and nullable upcasts
        if matches!(
            operator,
            CastOperator::UnionUpcast | CastOperator::NullableUpcast
        ) {
            let (to_target, to_source) = {
                let mut ctx = TypeContext::new(
                    state.ctx.module,
                    state.ctx.profile,
                    &options,
                    state.tree,
                    state.symbols,
                    state.types,
                );
                let to_target =
                    self.is_type_assignable(&mut ctx.reborrow(), target_type_id, value_type_id);
                let to_source =
                    self.is_type_assignable(&mut ctx.reborrow(), value_type_id, target_type_id);
                (to_target, to_source)
            };
            if to_target.is_assignable() && to_source.is_assignable() {
                return Ok(value_id);
            }
        }

        // skip numeric casts for scalar literals
        let value_type = state.types.get_type(value_type_id);
        if allow_assignable_skip
            && is_scalar_literal_type(value_type)
            && self.is_numeric_cast_operator(operator)
        {
            // align literal types with the selected numeric target
            state.types.set_inferred_type(
                value_id.into_global_any(state.ctx.module_id),
                target_type_id,
            );
            return Ok(value_id);
        }
        if operator == CastOperator::Identity {
            // align equivalent types when numeric binaries need a unified representation
            if !allow_assignable_skip && value_type_id != target_type_id {
                state.types.set_inferred_type(
                    value_id.into_global_any(state.ctx.module_id),
                    target_type_id,
                );
            }
            return Ok(value_id);
        }

        // build the target type expression
        let scope = state.tree.get_scope(origin_id);
        let target_expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let target_expression = match state.types.get_type(target_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: target_type_id,
            },
        };
        let target_expression_id = state.tree.insert(target_expression_id, target_expression);

        // set the inferred type for the target type expression
        let target_type_value = Type::Value {
            value: target_type_id,
        };
        let target_type_value_id = state
            .types
            .insert_type_from(target_type_value, target_expression_id);
        state.types.set_inferred_type(
            target_expression_id.into_global_any(state.ctx.module_id),
            target_type_value_id,
        );

        // insert the cast expression
        let cast_expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let cast_expression_id = state.tree.insert(
            cast_expression_id,
            Expression::Cast {
                operator,
                source: CastSource::Implicit,
                value: value_id,
                target_type: target_expression_id,
            },
        );
        state.types.set_inferred_type(
            cast_expression_id.into_global_any(state.ctx.module_id),
            target_type_id,
        );
        if self.options.elaborate_parenthesize_casts {
            let parenthesized_id =
                state
                    .tree
                    .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
            let parenthesized_id = state.tree.insert(
                parenthesized_id,
                Expression::Parenthesized {
                    expression: cast_expression_id,
                },
            );
            state.types.set_inferred_type(
                parenthesized_id.into_global_any(state.ctx.module_id),
                target_type_id,
            );
            return Ok(parenthesized_id);
        }

        Ok(cast_expression_id)
    }

    /// Resolve the value type id for an expression node.
    fn value_type_id_for_expression(
        &self,
        state: &ElaborateState<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        // prefer declared or inferred types on the node
        if let Some(type_id) = state
            .types
            .get_declared_or_inferred_type_id(value_id.into_global_any(state.ctx.module_id))
        {
            return Some(self.unwrap_value_type_id(state, type_id));
        }

        // read the expression node
        let expression = state.tree.get(value_id);

        // prefer symbol value types when referencing a binding
        let symbol = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };

        // return the symbol value type when available
        if let Some(symbol) = symbol
            && let Some(type_id) = state.types.get_value_type_id(symbol)
        {
            return Some(self.unwrap_value_type_id(state, type_id));
        }

        None
    }

    /// Resolve the type id encoded in a type expression.
    fn type_id_for_type_expression(
        &self,
        state: &ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        // read the type expression node
        let expression = state.tree.get(expression_id);

        // use the explicit type id when available
        if let Expression::Type { value } = expression {
            return Some(*value);
        }

        // fall back to the inferred type value
        let type_id = state
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(state.ctx.module_id))?;
        match state.types.get_type(type_id) {
            Type::Value { value } => Some(*value),
            _ => Some(type_id),
        }
    }

    /// Classify the cast operator for two types.
    fn cast_operator_for_types(
        &self,
        state: &mut ElaborateState<'_>,
        source_id: LocalTypeId,
        target_id: LocalTypeId,
    ) -> CastOperator {
        // fast path for identical types
        if source_id == target_id {
            return CastOperator::Identity;
        }

        // read the source and target types
        let source = state.types.get_type(source_id).clone();
        let target = state.types.get_type(target_id).clone();

        // semantic equality check (handles Foo→Foo, any→any, int32[]→int32[], etc.)
        if are_types_semantically_equal(&source, &target, state.types) {
            return CastOperator::Identity;
        }

        // handle any casts
        if is_any_type(&target) {
            return CastOperator::AnyUpcast;
        }
        if is_any_type(&source) {
            return CastOperator::AnyDowncast;
        }

        // handle unknown casts
        if is_unknown_type(&target) {
            return CastOperator::UnknownUpcast;
        }
        if is_unknown_type(&source) {
            return CastOperator::UnknownDowncast;
        }

        // handle object casts
        if is_object_type(&target) {
            return CastOperator::ObjectUpcast;
        }
        if is_object_type(&source) {
            return CastOperator::ObjectDowncast;
        }

        // handle numeric casts first
        if let Some(operator) = numeric_cast_operator(&source, &target) {
            return operator;
        }

        // handle pointer casts
        if is_pointer_type(&source) && is_integer_type(&target) {
            return CastOperator::PointerToInt;
        }
        if is_integer_type(&source) && is_pointer_type(&target) {
            return CastOperator::IntToPointer;
        }
        if is_pointer_type(&source) && is_pointer_type(&target) {
            return CastOperator::PointerCast;
        }

        // handle sized array to slice casts
        if let (
            Type::ArraySized { element, .. },
            Type::Array {
                element: target_element,
                ..
            },
        ) = (&source, &target)
        {
            let matches_element = target_element
                .map(|target_element| target_element == *element)
                .unwrap_or(true);
            if matches_element {
                return CastOperator::ArraySizedToSlice;
            }
        }

        // handle enum casts
        if let Some(operator) = self.enum_cast_operator(state, &source, &target) {
            return operator;
        }

        // handle nullable casts
        if is_nullable_union(&target, state.types) {
            return CastOperator::NullableUpcast;
        }
        if is_nullable_union(&source, state.types) {
            return CastOperator::NullableDowncast;
        }

        // handle union casts
        if is_union_type(&target) {
            return CastOperator::UnionUpcast;
        }
        if is_union_type(&source) {
            return CastOperator::UnionDowncast;
        }

        // fall back to assignability based instance casts
        let options = self.analyze_context_options_for_module(state.ctx.module_id);
        let mut ctx = TypeContext::new(
            state.ctx.module,
            state.ctx.profile,
            &options,
            state.tree,
            state.symbols,
            state.types,
        );
        let assignable = self.is_type_assignable(&mut ctx.reborrow(), target_id, source_id);
        if assignable.is_assignable() {
            CastOperator::InstanceUpcast
        } else {
            CastOperator::InstanceDowncast
        }
    }

    /// Check whether implicit collection reification is enabled for this profile.
    fn collection_reify_is_enabled(
        &self,
        state: &ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // read the conversion policy from dsconfig
        let policy = self
            .program
            .with_dsconfig_options(state.ctx.module, |ds| {
                ds.compiler.implicit_collection_conversions
            })
            .unwrap_or(ImplicitCollectionConversionPolicy::Allow);

        // skip reify for non-native outputs
        if !self
            .program
            .profile(state.ctx.profile)
            .key
            .output
            .is_native()
        {
            return Ok(false);
        }

        // honor the configured policy for native outputs
        match policy {
            ImplicitCollectionConversionPolicy::Allow => Ok(true),
            ImplicitCollectionConversionPolicy::Warn => {
                self.warning(ElaborateWarning::ImplicitCollectionConversion {
                    node: origin_id
                        .into_global_any(state.ctx.module.id)
                        .into_anchored(Some(state.ctx.profile)),
                });
                Ok(true)
            }
            ImplicitCollectionConversionPolicy::Deny => {
                Err(ElaborateError::ImplicitCollectionConversion {
                    node: origin_id
                        .into_global_any(state.ctx.module.id)
                        .into_anchored(Some(state.ctx.profile)),
                })
            }
        }
    }

    /// Reify record like object literals into map construction.
    fn reify_record_like_object_literal(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // only reify record like literals when enabled for the profile
        if !self.collection_reify_is_enabled(state, origin_id)? {
            return Ok(None);
        }

        // require an object literal value
        let Expression::ObjectExpression { properties } = state.tree.get(value_id).clone() else {
            return Ok(None);
        };

        // resolve the record key/value types
        let Some(record_like) = self.record_like_map_target(state, origin_id, target_type_id)?
        else {
            return Ok(None);
        };

        // resolve Map.from symbol
        let map_from_symbol = self.map_from_symbol(state, origin_id, record_like.map_symbol)?;

        // register a concrete instance for Map.from<K, V>
        let parameter_symbols = self
            .collect_static_parameter_symbols(
                TypeView::new(
                    state.ctx.module,
                    state.ctx.profile,
                    state.tree,
                    state.symbols,
                    state.types,
                ),
                map_from_symbol,
            )
            .unwrap_or_default();
        let map_from_instance = Instance::with_environment(
            map_from_symbol,
            record_like.map_static_arguments.clone(),
            parameter_symbols,
            0,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(state.ctx.module.id)
                .into_anchored(Some(state.ctx.profile)),
        })?;
        let map_from_instance_id =
            if let Some(instance_id) = state.types.find_instance_exact(&map_from_instance) {
                instance_id
            } else {
                state.types.insert_instance(map_from_instance)
            };

        // build tuple entries for each property
        let mut entry_arguments = Vec::with_capacity(properties.len());
        for property_id in properties {
            let property = state.tree.get(property_id);
            let (key, value) = match property {
                dir::Property::Field { key, value, .. } => {
                    let key = key.ok_or(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(state.ctx.module.id)
                            .into_anchored(Some(state.ctx.profile)),
                    })?;
                    let value = value.ok_or(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(state.ctx.module.id)
                            .into_anchored(Some(state.ctx.profile)),
                    })?;
                    (key, value)
                }
                dir::Property::Method { .. } | dir::Property::Spread { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(state.ctx.module.id)
                            .into_anchored(Some(state.ctx.profile)),
                    });
                }
            };

            // resolve the key expression
            let key_expression_id =
                self.record_key_expression(state, origin_id, key, record_like.key_type_id)?;

            // build a tuple expression for the entry
            let entry_tuple_id = self.record_entry_tuple_expression(
                state,
                origin_id,
                key_expression_id,
                value,
                record_like.entry_tuple_type_id,
            )?;

            // wrap tuple entries as array arguments
            let entry_argument_id = state.tree.reserve_from(
                NodeType::Argument,
                origin_id.into_any(),
                state.tree.get_scope(origin_id),
                None,
            );
            let entry_argument_id = state.tree.insert(
                entry_argument_id,
                Argument::Positional {
                    modifiers: None,
                    value: entry_tuple_id,
                },
            );
            entry_arguments.push(entry_argument_id);
        }

        // build the entries array expression
        let entries_array_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let entries_array_id = state.tree.insert(
            entries_array_id,
            Expression::ArrayExpression {
                elements: entry_arguments,
            },
        );
        state.types.set_inferred_type(
            entries_array_id.into_global_any(state.ctx.module.id),
            record_like.entries_array_type_id,
        );

        // build a module reference to Map.from
        let map_name = self
            .program
            .strings
            .intern(WellKnownSymbol::Map.export_name());
        let from_name = self.program.strings.intern("from");
        let left_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let left_id = state.tree.insert(
            left_id,
            Expression::ModuleReference {
                path: Path::from(&[map_name, from_name][..]),
                static_arguments: None,
                target_symbol: map_from_symbol,
            },
        );

        // build the Map.from call expression
        let call_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let entries_argument_id = self.argument_for_value(state, origin_id, entries_array_id);
        let call_id = state.tree.insert(
            call_id,
            Expression::Call {
                left: left_id,
                static_arguments: None,
                dynamic_arguments: vec![entries_argument_id],
            },
        );
        state.types.set_inferred_type(
            call_id.into_global_any(state.ctx.module.id),
            record_like.map_type_id,
        );

        // attach a static resolution for the generated call
        let resolution_id = state.types.insert_resolution(Resolution::Static {
            receiver: None,
            candidate: ResolutionCandidate {
                key: None,
                target_symbol: map_from_symbol,
                instance: Some(map_from_instance_id),
                resolved_signature: Some(ResolvedSignature {
                    dynamic_parameters: vec![record_like.entries_array_type_id],
                    return_type: Some(record_like.map_type_id),
                    static_arguments: record_like.map_static_arguments,
                }),
            },
        });
        state
            .types
            .set_resolution_for_node(call_id.into_global_any(state.ctx.module.id), resolution_id);

        Ok(Some(call_id))
    }

    /// Reify sized arrays into dynamic arrays with Array.fromSized.
    fn reify_array_sized_value(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // only reify array casts when enabled for the profile
        if !self.collection_reify_is_enabled(state, origin_id)? {
            return Ok(None);
        }

        // resolve source and target types
        let value_type_id = self.value_type_id_for_expression(state, value_id).ok_or(
            ElaborateError::UnsupportedConstruct {
                node: value_id
                    .into_global_any(state.ctx.module.id)
                    .into_anchored(Some(state.ctx.profile)),
            },
        )?;
        let value_type_id = self.unwrap_value_type_id(state, value_type_id);
        let target_type_id = self.unwrap_value_type_id(state, target_type_id);
        let value_type = state.types.get_type(value_type_id).clone();
        let target_type = state.types.get_type(target_type_id).clone();

        // resolve the dynamic array target type
        let (element_type_id, target_element_type_id) = match (value_type, target_type) {
            (
                Type::ArraySized {
                    element: value_element,
                    ..
                },
                Type::Array {
                    element: target_element,
                    ..
                },
            ) => (value_element, target_element),
            (
                Type::ArraySized {
                    element: value_element,
                    ..
                },
                Type::Reference {
                    symbol,
                    static_arguments,
                },
            ) => {
                let Some(well_known) = self.well_known_array_kind(state.ctx.profile, symbol) else {
                    return Ok(None);
                };
                let options = self.analyze_context_options_for_module(state.ctx.module.id);
                let mut ctx = TypeContext::new(
                    state.ctx.module,
                    state.ctx.profile,
                    &options,
                    state.tree,
                    state.symbols,
                    state.types,
                );
                let Some(Type::Array { element, .. }) = self.normalize_well_known_type_reference(
                    &mut ctx.reborrow(),
                    origin_id.into_any(),
                    well_known,
                    static_arguments.as_deref(),
                ) else {
                    return Ok(None);
                };
                (value_element, element)
            }
            _ => return Ok(None),
        };

        // ensure element compatibility when the target is explicit
        if let Some(target_element_type_id) = target_element_type_id {
            let options = self.analyze_context_options_for_module(state.ctx.module.id);
            let mut ctx = TypeContext::new(
                state.ctx.module,
                state.ctx.profile,
                &options,
                state.tree,
                state.symbols,
                state.types,
            );
            let to_target = self.is_type_assignable(
                &mut ctx.reborrow(),
                target_element_type_id,
                element_type_id,
            );
            let to_source = self.is_type_assignable(
                &mut ctx.reborrow(),
                element_type_id,
                target_element_type_id,
            );
            if !to_target.is_assignable() || !to_source.is_assignable() {
                return Ok(None);
            }
        }

        // resolve Array.fromSized symbol
        let array_symbol = self
            .get_well_known_type_symbol(state.ctx.profile, WellKnownSymbol::Array)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(state.ctx.module.id)
                    .into_anchored(Some(state.ctx.profile)),
            })?;
        let from_sized_symbol = self.array_from_sized_symbol(state, origin_id, array_symbol)?;

        // register a concrete instance for Array.fromSized<T>
        let array_static_arguments = vec![StaticArgument::value(StaticExpression::Type {
            ty: element_type_id,
        })];
        let parameter_symbols = self
            .collect_static_parameter_symbols(
                TypeView::new(
                    state.ctx.module,
                    state.ctx.profile,
                    state.tree,
                    state.symbols,
                    state.types,
                ),
                from_sized_symbol,
            )
            .unwrap_or_default();
        let from_sized_instance = Instance::with_environment(
            from_sized_symbol,
            array_static_arguments.clone(),
            parameter_symbols,
            0,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(state.ctx.module.id)
                .into_anchored(Some(state.ctx.profile)),
        })?;
        let from_sized_instance_id =
            if let Some(instance_id) = state.types.find_instance_exact(&from_sized_instance) {
                instance_id
            } else {
                state.types.insert_instance(from_sized_instance)
            };

        // build a module reference to Array.fromSized
        let array_name = self
            .program
            .strings
            .intern(WellKnownSymbol::Array.export_name());
        let from_name = self.program.strings.intern("fromSized");
        let left_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let left_id = state.tree.insert(
            left_id,
            Expression::ModuleReference {
                path: Path::from(&[array_name, from_name][..]),
                static_arguments: None,
                target_symbol: from_sized_symbol,
            },
        );

        // build the Array.fromSized call expression
        let call_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let value_argument_id = self.argument_for_value(state, origin_id, value_id);
        let call_id = state.tree.insert(
            call_id,
            Expression::Call {
                left: left_id,
                static_arguments: None,
                dynamic_arguments: vec![value_argument_id],
            },
        );
        state
            .types
            .set_inferred_type(call_id.into_global_any(state.ctx.module.id), target_type_id);

        // attach a static resolution for the generated call
        let resolution_id = state.types.insert_resolution(Resolution::Static {
            receiver: None,
            candidate: ResolutionCandidate {
                key: None,
                target_symbol: from_sized_symbol,
                instance: Some(from_sized_instance_id),
                resolved_signature: Some(ResolvedSignature {
                    dynamic_parameters: vec![value_type_id],
                    return_type: Some(target_type_id),
                    static_arguments: array_static_arguments,
                }),
            },
        });
        state
            .types
            .set_resolution_for_node(call_id.into_global_any(state.ctx.module.id), resolution_id);

        Ok(Some(call_id))
    }

    /// Resolve record like target information for map reification.
    fn record_like_map_target(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<RecordLikeTargetInfo>> {
        // unwrap value wrapper types
        let target_type_id = self.unwrap_value_type_id(state, target_type_id);

        // resolve Map symbol for record like lowering
        let Some(map_symbol) =
            self.get_well_known_type_symbol(state.ctx.profile, WellKnownSymbol::Map)
        else {
            return Ok(None);
        };
        let record_symbol =
            self.get_well_known_type_symbol(state.ctx.profile, WellKnownSymbol::Record);

        // walk aliases until we reach a record like target
        let mut current_type_id = target_type_id;
        let mut visited = Vec::new();
        loop {
            // avoid alias resolution cycles
            if visited.contains(&current_type_id) {
                return Ok(None);
            }
            visited.push(current_type_id);

            // read the current target type
            let target_type = state.types.get_type(current_type_id).clone();

            // use direct object index signatures as map targets
            if let Type::Object {
                index_signatures, ..
            } = &target_type
            {
                if let Some(signature) = index_signatures.first() {
                    let info = self.record_like_target_for_types(
                        state,
                        origin_id,
                        map_symbol,
                        signature.key_type,
                        signature.value_type,
                    )?;
                    return Ok(Some(info));
                }

                return Ok(None);
            }

            // resolve record-like references to Map<K, V>
            if let Type::Reference {
                symbol,
                static_arguments,
            } = &target_type
            {
                // only accept Map and Record symbols
                let is_map_symbol = *symbol == map_symbol
                    || record_symbol.is_some_and(|record_symbol| record_symbol == *symbol);
                if !is_map_symbol {
                    // follow alias targets when present
                    if let Some(alias_target_id) = state.types.get_alias_target_type_id(*symbol) {
                        current_type_id = alias_target_id;
                        continue;
                    }

                    return Ok(None);
                }

                // resolve key and value type ids from static arguments
                let Some(static_arguments) = static_arguments.as_deref() else {
                    return Ok(None);
                };
                if static_arguments.len() != 2 {
                    return Ok(None);
                }

                let key_type_id =
                    self.static_argument_type_id(state, origin_id, &static_arguments[0])?;
                let value_type_id =
                    self.static_argument_type_id(state, origin_id, &static_arguments[1])?;

                let info = self.record_like_target_for_types(
                    state,
                    origin_id,
                    map_symbol,
                    key_type_id,
                    value_type_id,
                )?;
                return Ok(Some(info));
            }

            return Ok(None);
        }
    }

    /// Build record like info for key/value types.
    fn record_like_target_for_types(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        map_symbol: GlobalSymbolId,
        key_type_id: LocalTypeId,
        value_type_id: LocalTypeId,
    ) -> ElaborateResult<RecordLikeTargetInfo> {
        // build the tuple entry type
        let entry_tuple_type = Type::Tuple {
            elements: vec![
                TypeElement::new(key_type_id),
                TypeElement::new(value_type_id),
            ],
            is_readonly: false,
        };
        let entry_tuple_type_id = state.types.insert_type_from(entry_tuple_type, origin_id);

        // build the entries array type
        let entries_array_type = Type::Array {
            element: Some(entry_tuple_type_id),
            is_readonly: false,
        };
        let entries_array_type_id = state.types.insert_type_from(entries_array_type, origin_id);

        // build the Map<K, V> reference type
        let map_static_arguments = vec![
            StaticArgument::value(StaticExpression::Type { ty: key_type_id }),
            StaticArgument::value(StaticExpression::Type { ty: value_type_id }),
        ];
        let map_type_id = state.types.insert_type_from(
            Type::Reference {
                symbol: map_symbol,
                static_arguments: Some(map_static_arguments.clone()),
            },
            origin_id,
        );

        Ok(RecordLikeTargetInfo {
            key_type_id,
            map_symbol,
            map_type_id,
            entry_tuple_type_id,
            entries_array_type_id,
            map_static_arguments,
        })
    }

    /// Resolve a single static argument to a type id.
    fn static_argument_type_id(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        argument: &StaticArgument,
    ) -> ElaborateResult<LocalTypeId> {
        // require an evaluated static argument
        let StaticArgument::Evaluated { value, .. } = argument else {
            return Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(state.ctx.module.id)
                    .into_anchored(Some(state.ctx.profile)),
            });
        };

        // resolve type expressions to concrete ids
        match value {
            StaticExpression::Type { ty } => Ok(*ty),
            StaticExpression::TypeLiteral { value } => {
                let literal_type_id = state.types.insert_type_from(
                    Type::TypeLiteral {
                        value: value.clone(),
                    },
                    origin_id,
                );
                Ok(literal_type_id)
            }
            _ => Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(state.ctx.module.id)
                    .into_anchored(Some(state.ctx.profile)),
            }),
        }
    }

    /// Build a key expression for a record like object property.
    fn record_key_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        key: DynamicKey,
        key_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // require computed keys for non-string index signatures
        let key_type = state.types.get_type(key_type_id);
        let key_is_string = is_string_type(key_type)
            || matches!(
                key_type,
                Type::Reference { symbol, .. }
                    if self.is_well_known_symbol(state.ctx.profile, *symbol, WellKnownSymbol::String)
            );
        let is_static_key = matches!(key, DynamicKey::Name(_) | DynamicKey::Number(_));
        if is_static_key && !key_is_string {
            return Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(state.ctx.module.id)
                    .into_anchored(Some(state.ctx.profile)),
            });
        }

        // use direct expressions for computed keys
        let key_expression_id = match key {
            DynamicKey::Expression(expression) => expression,
            DynamicKey::NamedExpression { key, .. } => key,
            DynamicKey::Name(name) | DynamicKey::Number(name) => {
                // create a string literal for static keys
                let key_expression_id = state.tree.reserve_from(
                    NodeType::Expression,
                    origin_id.into_any(),
                    state.tree.get_scope(origin_id),
                    None,
                );
                let key_expression_id = state.tree.insert(
                    key_expression_id,
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::String(name),
                    },
                );

                // assign the key type for literal values
                state.types.set_inferred_type(
                    key_expression_id.into_global_any(state.ctx.module.id),
                    key_type_id,
                );

                key_expression_id
            }
            DynamicKey::Private(_) => {
                return Err(ElaborateError::UnsupportedConstruct {
                    node: origin_id
                        .into_global_any(state.ctx.module.id)
                        .into_anchored(Some(state.ctx.profile)),
                });
            }
        };

        Ok(key_expression_id)
    }

    /// Build a tuple expression for a record entry.
    fn record_entry_tuple_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        key_expression_id: LocalNodeId<Expression>,
        value_expression_id: LocalNodeId<Expression>,
        tuple_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // allocate tuple argument nodes
        let key_argument_id = state.tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let key_argument_id = state.tree.insert(
            key_argument_id,
            Argument::Positional {
                modifiers: None,
                value: key_expression_id,
            },
        );
        let value_argument_id = state.tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let value_argument_id = state.tree.insert(
            value_argument_id,
            Argument::Positional {
                modifiers: None,
                value: value_expression_id,
            },
        );

        // build the tuple expression
        let tuple_expression_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        let tuple_expression_id = state.tree.insert(
            tuple_expression_id,
            Expression::TupleExpression {
                elements: vec![key_argument_id, value_argument_id],
            },
        );
        state.types.set_inferred_type(
            tuple_expression_id.into_global_any(state.ctx.module.id),
            tuple_type_id,
        );

        Ok(tuple_expression_id)
    }

    /// Resolve the Map.from symbol for record like reification.
    fn map_from_symbol(
        &self,
        state: &ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        map_symbol: GlobalSymbolId,
    ) -> ElaborateResult<GlobalSymbolId> {
        // locate the member key
        let name = self.program.strings.intern("from");
        let member_key = StaticKey::Name(name);

        self.resolve_static_member_symbol(
            state.ctx.module,
            state.ctx.profile,
            origin_id,
            map_symbol,
            member_key,
            state.tree,
            state.symbols,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(state.ctx.module.id)
                .into_anchored(Some(state.ctx.profile)),
        })
    }

    /// Resolve the Array.fromSized symbol for sized array reification.
    fn array_from_sized_symbol(
        &self,
        state: &ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        array_symbol: GlobalSymbolId,
    ) -> ElaborateResult<GlobalSymbolId> {
        // locate the member key
        let name = self.program.strings.intern("fromSized");
        let member_key = StaticKey::Name(name);

        self.resolve_static_member_symbol(
            state.ctx.module,
            state.ctx.profile,
            origin_id,
            array_symbol,
            member_key,
            state.tree,
            state.symbols,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(state.ctx.module.id)
                .into_anchored(Some(state.ctx.profile)),
        })
    }

    /// Build a positional argument for a value expression.
    fn argument_for_value(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Argument> {
        let argument_id = state.tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
        );
        state.tree.insert(
            argument_id,
            Argument::Positional {
                modifiers: None,
                value: value_id,
            },
        )
    }

    /// Check whether a cast operator is numeric.
    fn is_numeric_cast_operator(&self, operator: CastOperator) -> bool {
        // match numeric cast operators
        matches!(
            operator,
            CastOperator::IntWiden
                | CastOperator::IntNarrow
                | CastOperator::IntSignChange
                | CastOperator::FloatWiden
                | CastOperator::FloatNarrow
                | CastOperator::IntToFloat
                | CastOperator::FloatToInt
        )
    }

    /// Classify enum casts between enum and primitive types.
    fn enum_cast_operator(
        &self,
        state: &ElaborateState<'_>,
        source: &Type,
        target: &Type,
    ) -> Option<CastOperator> {
        // shared enum backing lookup
        let backing_for_type = |ty: &Type| -> Option<EnumBackingType> {
            // only enum references have a backing type
            let Type::Reference { symbol, .. } = ty else {
                return None;
            };

            state.types.get_enum_backing_type(*symbol)
        };

        // enum to primitive casts
        if let Some(backing) = backing_for_type(source) {
            match backing {
                EnumBackingType::Int(_) if is_integer_type(target) => {
                    return Some(CastOperator::EnumToInt);
                }
                EnumBackingType::String if is_string_type(target) => {
                    return Some(CastOperator::EnumToString);
                }
                _ => {}
            }
        }

        // primitive to enum casts
        if let Some(backing) = backing_for_type(target) {
            match backing {
                EnumBackingType::Int(_) if is_integer_type(source) => {
                    return Some(CastOperator::IntToEnum);
                }
                EnumBackingType::String if is_string_type(source) => {
                    return Some(CastOperator::StringToEnum);
                }
                _ => {}
            }
        }

        None
    }

    /// Strip value wrapper types to reach the underlying type id.
    fn unwrap_value_type_id(
        &self,
        state: &ElaborateState<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // peel value wrapper types
        let mut current = type_id;
        loop {
            match state.types.get_type(current) {
                Type::Value { value } => {
                    current = *value;
                }
                _ => return current,
            }
        }
    }

    /// Return true when a type id points at an interface reference type.
    fn is_interface_reference_type(
        &self,
        state: &ElaborateState<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        // resolve the underlying type id
        let type_id = self.unwrap_value_type_id(state, type_id);

        // check for interface reference types
        matches!(
            state.types.get_type(type_id),
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Interface
        )
    }

    /// Return true when a new expression targets a nominal class or struct.
    fn is_concrete_new_expression(
        &self,
        state: &ElaborateState<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> bool {
        // require a new expression
        let Expression::New { left, .. } = state.tree.get(value_id) else {
            return false;
        };

        // accept nominal constructor targets
        match state.tree.get(*left) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                matches!(target_symbol.ty(), SymbolType::Class | SymbolType::Struct)
            }
            _ => false,
        }
    }

    /// Return true when a value expression is a tagged constructor literal.
    fn is_tagged_expression(
        &self,
        state: &ElaborateState<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> bool {
        // check tagged constructor expressions
        matches!(
            state.tree.get(value_id),
            Expression::TaggedScalarExpression { .. }
                | Expression::TaggedTupleExpression { .. }
                | Expression::TaggedObjectExpression { .. }
        )
    }

    /// Return true when a value expression resolves to a concrete nominal symbol.
    fn is_concrete_resolution(
        &self,
        state: &ElaborateState<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> bool {
        // resolve the node resolution
        let node_id = value_id.into_global_any(state.ctx.module_id);
        let Some(resolution_id) = state.types.get_resolution_for_node(node_id) else {
            return false;
        };
        let resolution = state.types.get_resolution(resolution_id);

        // accept concrete static resolutions only
        match resolution {
            Resolution::Static { candidate, .. } => {
                self.is_concrete_resolution_candidate(state, candidate)
            }
            Resolution::Dynamic { .. }
            | Resolution::Unresolved { .. }
            | Resolution::Builtin { .. } => false,
        }
    }

    /// Return true when a resolution candidate targets a concrete nominal symbol.
    fn is_concrete_resolution_candidate(
        &self,
        state: &ElaborateState<'_>,
        candidate: &ResolutionCandidate,
    ) -> bool {
        // accept direct nominal symbols
        if matches!(
            candidate.target_symbol.ty(),
            SymbolType::Class | SymbolType::Struct
        ) {
            return true;
        }

        // fall back to the resolved return type
        let Some(resolved_signature) = candidate.resolved_signature.as_ref() else {
            return false;
        };
        let Some(return_type) = resolved_signature.return_type else {
            return false;
        };

        // require a nominal return type
        let return_type = self.unwrap_value_type_id(state, return_type);
        matches!(
            state.types.get_type(return_type),
            Type::Reference { symbol, .. }
                if matches!(symbol.ty(), SymbolType::Class | SymbolType::Struct)
        )
    }

    /// Check whether a binary operator is numeric.
    fn is_numeric_binary_operator(&self, operator: BinaryOperator) -> bool {
        // match numeric binary operators
        matches!(
            operator,
            BinaryOperator::Multiply
                | BinaryOperator::WrappingMultiply
                | BinaryOperator::SaturatingMultiply
                | BinaryOperator::Exponent
                | BinaryOperator::WrappingExponent
                | BinaryOperator::SaturatingExponent
                | BinaryOperator::Divide
                | BinaryOperator::Remainder
                | BinaryOperator::Add
                | BinaryOperator::WrappingAdd
                | BinaryOperator::SaturatingAdd
                | BinaryOperator::Subtract
                | BinaryOperator::WrappingSubtract
                | BinaryOperator::SaturatingSubtract
                | BinaryOperator::ShiftLeft
                | BinaryOperator::SaturatingShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseXor
                | BinaryOperator::ElementwiseOr
                | BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::EqualStrict
                | BinaryOperator::NotEqualStrict
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
        )
    }
}
