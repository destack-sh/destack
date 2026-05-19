use destack_dir as dir;
use dir::{
    Argument, AssignOperator, AssignPattern, BinaryOperator, CastOperator, CastOrigin, Declarator,
    EnumBackingType, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, MatchCase, NodeType,
    Type, TypeExpression,
};

use super::r#type::{
    has_matching_implicit_value_runtime_family, is_any_type, is_integer_type, is_nullable_union,
    is_object_type, is_pointer_type, is_string_type, is_union_type, is_unknown_type,
    numeric_cast_operator,
};
use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Return one simple expression target from one assignment pattern.
    fn reify_assign_pattern_target_expression(
        &self,
        tree: &dir::Tree,
        mut assign_pattern_id: LocalNodeId<AssignPattern>,
    ) -> Option<LocalNodeId<Expression>> {
        loop {
            let assign_pattern = tree.get(assign_pattern_id);

            // keep direct expression targets
            if let AssignPattern::Expression { value } = assign_pattern {
                return Some(*value);
            }

            // unwrap defaulted targets before checking the base
            if let AssignPattern::Assign { pattern, .. } = assign_pattern {
                assign_pattern_id = *pattern;
                continue;
            }

            // destructuring assignments do not have one simple target type
            return None;
        }
    }

    /// Reify one explicit cast expression.
    pub(super) fn reify_explicit_cast_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
    ) -> ElaborateResult<()> {
        // require both source and target types
        let Some(source_type_id) = self.value_type_id_for_expression(state, value) else {
            return Ok(());
        };
        let Some(target_type_id) = state.type_table().get_declared_or_inferred_type_id(expression_id.into_global_any(state.module_id))
        else {
            return Ok(());
        };

        // resolve the cast operator from analyzed types
        let operator = self.cast_operator_for_types(state, source_type_id, target_type_id);

        // keep `as` as the cast node, but resolve its semantics now
        state.tree.replace(
            expression_id,
            Expression::As {
                operator: Some(operator),
                source: CastOrigin::Explicit,
                expression: value,
                target_type,
            },
        );

        Ok(())
    }

    /// Reify one must expression.
    pub(super) fn reify_must_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // require the inferred must result type
        let Some(target_type_id) = state.type_table().get_declared_or_inferred_type_id(expression_id.into_global_any(state.module_id))
        else {
            return Ok(());
        };

        // wrap the operand when the must narrows its type
        let reified_value_id =
            self.wrap_value_with_cast(state, expression_id, left, target_type_id)?;

        // replace the must wrapper with the reified expression
        state.tree.replace_from(expression_id, reified_value_id);
        state.types_tail.copy_node_relations(
            reified_value_id.into_global_any(state.module_id),
            expression_id.into_global_any(state.module_id),
        );
        state.resolutions_tail.copy_node_relations(
            reified_value_id.into_global_any(state.module_id),
            expression_id.into_global_any(state.module_id),
        );

        Ok(())
    }

    /// Reify implicit casts around one reference expression.
    pub(super) fn reify_implicit_casts_in_reference(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
        _target_symbol: GlobalSymbolId,
    ) -> ElaborateResult<()> {
        todo!("FUGU #Incomplete: elaborate implicit casts in reference")
    }

    /// Reify implicit casts in one binding list.
    pub(super) fn reify_implicit_casts_in_binding(
        &self,
        state: &mut ElaborateState<'_>,
        declarators: &[LocalNodeId<Declarator>],
    ) -> ElaborateResult<()> {
        // visit each declarator
        for declarator_id in declarators {
            let declarator = state.tree.get(*declarator_id).clone();

            // skip declarators without initializers
            let Some(value_id) = declarator.value else {
                continue;
            };

            // require a declared target type on the binding
            let Some(target_type_id) = state.type_table().get_declared_type_id(declarator_id.into_global_any(state.module_id))
            else {
                continue;
            };

            // wrap the initializer when the binding narrows it
            let cast_value_id =
                self.wrap_value_with_cast(state, value_id, value_id, target_type_id)?;
            if cast_value_id == value_id {
                continue;
            }

            // replace the initializer with the wrapped expression
            let updated = Declarator {
                value: Some(cast_value_id),
                ..declarator
            };
            state.tree.replace(*declarator_id, updated);
        }

        Ok(())
    }

    /// Reify implicit casts in one assignment expression.
    pub(super) fn reify_implicit_casts_in_assignment(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<AssignPattern>,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        let Some(left_expression_id) =
            self.reify_assign_pattern_target_expression(state.tree, left)
        else {
            return Ok(());
        };

        // read the target type from the assignment target
        let Some(target_type_id) = self.value_type_id_for_expression(state, left_expression_id)
        else {
            return Ok(());
        };

        // wrap the right hand side when the assignment narrows it
        let cast_right_id =
            self.wrap_value_with_cast(state, expression_id, right, target_type_id)?;
        if cast_right_id == right {
            return Ok(());
        }

        // rewrite the assignment to use the wrapped right hand side
        state.tree.replace(
            expression_id,
            Expression::Assign {
                left,
                right: cast_right_id,
            },
        );

        Ok(())
    }

    /// Reify implicit casts in one compound assignment expression.
    pub(super) fn reify_implicit_casts_in_assign_binary(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // read the target type from the assignment target
        let Some(target_type_id) = self.value_type_id_for_expression(state, left) else {
            return Ok(());
        };

        // wrap the right hand side when the assignment narrows it
        let cast_right_id =
            self.wrap_value_with_cast(state, expression_id, right, target_type_id)?;
        if cast_right_id == right {
            return Ok(());
        }

        // rewrite the assignment to use the wrapped right hand side
        state.tree.replace(
            expression_id,
            Expression::AssignBinary {
                left,
                operator,
                right: cast_right_id,
            },
        );

        Ok(())
    }

    /// Reify implicit casts in one return expression.
    pub(super) fn reify_implicit_casts_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
    ) -> ElaborateResult<()> {
        // skip bare returns
        let Some(value_id) = value else {
            return Ok(());
        };

        // require the surrounding declared return type
        let Some(target_type_id) = self.enclosing_return_type(state, expression_id) else {
            return Ok(());
        };

        // skip returns whose value type is still unresolved
        if self.value_type_id_for_expression(state, value_id).is_none() {
            return Ok(());
        }

        // wrap the returned value when needed
        let cast_value_id =
            self.wrap_value_with_cast(state, expression_id, value_id, target_type_id)?;
        if cast_value_id == value_id {
            return Ok(());
        }

        // rewrite the return to use the wrapped value
        state.tree.replace(
            expression_id,
            Expression::Return {
                value: Some(cast_value_id),
            },
        );

        Ok(())
    }

    /// Reify implicit casts in one call expression.
    pub(super) fn reify_implicit_casts_in_call(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Argument>],
    ) -> ElaborateResult<()> {
        // resolve uniform expected argument types for the call
        let Some(expected_argument_types) =
            self.expected_argument_types_for_call(state, expression_id, arguments)?
        else {
            return Ok(());
        };

        // wrap positional arguments that are narrowed by the call signature
        for (index, argument_id) in arguments.iter().enumerate() {
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
                Argument::Named { name, .. } => Argument::Named {
                    name,
                    value: cast_value_id,
                },
                Argument::Labeled { label, .. } => Argument::Labeled {
                    label,
                    value: cast_value_id,
                },
                Argument::Positional { .. } => Argument::Positional {
                    value: cast_value_id,
                },
                Argument::Spread { label, .. } => Argument::Spread {
                    label,
                    value: cast_value_id,
                },
                Argument::Error { .. } => Argument::Error {
                    value: cast_value_id,
                },
            };
            state.tree.replace(*argument_id, updated);
        }

        Ok(())
    }

    /// Normalize redundant cast structure after reify.
    pub(super) fn normalize_redundant_casts(
        &self,
        _state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        Ok(())
    }

    /// Reify implicit casts in one ternary expression.
    pub(super) fn reify_implicit_casts_in_ternary(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
        _condition: LocalNodeId<Expression>,
        _then_expression: LocalNodeId<Expression>,
        _else_expression: Option<LocalNodeId<Expression>>,
    ) -> ElaborateResult<()> {
        todo!("FUGU #Incomplete: elaborate implicit casts in ternary")
    }

    /// Reify implicit casts in one match expression.
    pub(super) fn reify_implicit_casts_in_match(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
        _cases: &[LocalNodeId<MatchCase>],
    ) -> ElaborateResult<()> {
        todo!("FUGU #Incomplete: elaborate implicit casts in match")
    }

    /// Reify implicit casts in one builtin binary expression.
    pub(super) fn reify_implicit_casts_in_binary(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
        _left: LocalNodeId<Expression>,
        _operator: BinaryOperator,
        _right: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        todo!("FUGU #Incomplete: elaborate implicit casts in binary")
    }

    /// Resolve the value type id for one expression.
    fn value_type_id_for_expression(
        &self,
        state: &mut ElaborateState<'_>,
        value_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        // prefer node-local declared or inferred types
        if let Some(type_id) = state.type_table().get_declared_or_inferred_type_id(value_id.into_global_any(state.module_id))
        {
            return Some(state.type_table().unwrap_form_payload_type_id(type_id));
        }

        // otherwise fall back to reference symbol value types
        let node = value_id.into_global_any(state.module_id);
        let symbol = state.resolution_table().symbol_resolution(node)?;
        let type_id = state.type_table().get_value_type_id(symbol)?;

        Some(state.type_table().unwrap_form_payload_type_id(type_id))
    }

    /// Wrap one expression in an implicit `as` when the target type narrows it.
    fn wrap_value_with_cast(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // require both value and target type ids
        let Some(value_type_id) = self.value_type_id_for_expression(state, value_id) else {
            return Ok(value_id);
        };
        let value_type_id = state.type_table().unwrap_form_payload_type_id(value_type_id);
        let target_type_id = state.type_table().unwrap_form_payload_type_id(target_type_id);

        // skip casts that do not change semantics
        if value_type_id == target_type_id
            || has_matching_implicit_value_runtime_family(
                state.type_table().get_type(value_type_id),
                state.type_table().get_type(target_type_id),
            )
        {
            return Ok(value_id);
        }

        // preserve literal-to-literal identity after value unwrapping
        if matches!(
            (state.type_table().get_type(value_type_id), state.type_table().get_type(target_type_id)),
            (Type::Literal(dir::LiteralType { value: left }), Type::Literal(dir::LiteralType { value: right })) if left == right
        ) {
            return Ok(value_id);
        }

        // classify the inserted cast
        let operator = self.cast_operator_for_types(state, value_type_id, target_type_id);
        if operator == CastOperator::Identity {
            return Ok(value_id);
        }

        // clone the value into the new wrapper scope
        let scope = state.tree.get_scope(origin_id);
        let value_expression = state.tree.get(value_id).clone();
        let value_id =
            self.clone_expression_with_analysis(state, value_id, &value_expression, scope);

        // materialize the target type syntax for the inserted cast
        let target_type =
            self.insert_type_expression_for_type_id(state, origin_id, target_type_id, scope);

        // build the implicit cast expression
        let expression_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let expression_id = state.tree.insert_as_owner(
            expression_id,
            Expression::As {
                operator: Some(operator),
                source: CastOrigin::Implicit,
                expression: value_id,
                target_type,
            },
        );
        state.types_tail.set_inferred_type(
            expression_id.into_global_any(state.module_id),
            target_type_id,
        );

        // parenthesize inserted casts when configured for this module
        if !state.options.parenthesize_casts {
            return Ok(expression_id);
        }

        let parenthesized_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let parenthesized_id = state.tree.insert_as_owner(
            parenthesized_id,
            Expression::Parenthesized {
                expression: expression_id,
            },
        );
        state.types_tail.set_inferred_type(
            parenthesized_id.into_global_any(state.module_id),
            target_type_id,
        );

        Ok(parenthesized_id)
    }

    /// Classify the cast operator for two analyzed types.
    fn cast_operator_for_types(
        &self,
        state: &mut ElaborateState<'_>,
        source_id: LocalTypeId,
        target_id: LocalTypeId,
    ) -> CastOperator {
        // unwrap value wrappers before classification
        let source_id = state.type_table().unwrap_form_payload_type_id(source_id);
        let target_id = state.type_table().unwrap_form_payload_type_id(target_id);

        // fast path for identical types
        if source_id == target_id {
            return CastOperator::Identity;
        }

        // read the source and target types
        let source = state.type_table().get_type(source_id).clone();
        let target = state.type_table().get_type(target_id).clone();

        // any
        if is_any_type(&target) {
            return CastOperator::AnyUpcast;
        }
        if is_any_type(&source) {
            return CastOperator::AnyDowncast;
        }

        // unknown
        if is_unknown_type(&target) {
            return CastOperator::UnknownUpcast;
        }
        if is_unknown_type(&source) {
            return CastOperator::UnknownDowncast;
        }

        // object
        if is_object_type(&target) {
            return CastOperator::ObjectUpcast;
        }
        if is_object_type(&source) {
            return CastOperator::ObjectDowncast;
        }

        // numeric
        if let Some(operator) = numeric_cast_operator(&source, &target) {
            return operator;
        }

        // pointer
        if is_pointer_type(&source) && is_integer_type(&target) {
            return CastOperator::PointerToInt;
        }
        if is_integer_type(&source) && is_pointer_type(&target) {
            return CastOperator::IntToPointer;
        }
        if is_pointer_type(&source) && is_pointer_type(&target) {
            return CastOperator::PointerCast;
        }

        // sized array to slice
        if let (
            Type::FixedArray(dir::FixedArrayType { element, .. }),
            Type::Slice(dir::SliceType {
                element: target_element,
                ..
            }),
        ) = (&source, &target)
        {
            let matches_element = *target_element == *element;
            if matches_element {
                return CastOperator::ArraySizedToSlice;
            }
        }

        // enum
        if let Some(operator) = self.enum_cast_operator(state, &source, &target) {
            return operator;
        }

        // nullable
        if is_nullable_union(&target, &state.type_table()) {
            return CastOperator::NullableUpcast;
        }
        if is_nullable_union(&source, &state.type_table()) {
            return CastOperator::NullableDowncast;
        }

        // union
        if is_union_type(&target) {
            return CastOperator::UnionUpcast;
        }
        if is_union_type(&source) {
            return CastOperator::UnionDowncast;
        }

        // nominal fallback
        if target_id == source_id {
            CastOperator::InstanceUpcast
        } else {
            CastOperator::InstanceDowncast
        }
    }

    /// Classify enum casts between enum and primitive types.
    fn enum_cast_operator(
        &self,
        state: &ElaborateState<'_>,
        source: &Type,
        target: &Type,
    ) -> Option<CastOperator> {
        // read one enum backing type when present
        let backing_for_type = |ty: &Type| -> Option<EnumBackingType> {
            let Type::Named(reference) = ty else {
                return None;
            };

            state.type_table().get_enum_backing_type(reference.symbol)
        };

        // enum to primitive
        if let Some(backing) = backing_for_type(source) {
            match backing {
                EnumBackingType::Integer(_) if is_integer_type(target) => {
                    return Some(CastOperator::EnumToInt);
                }
                EnumBackingType::String if is_string_type(target) => {
                    return Some(CastOperator::EnumToString);
                }
                _ => {}
            }
        }

        // primitive to enum
        if let Some(backing) = backing_for_type(target) {
            match backing {
                EnumBackingType::Integer(_) if is_integer_type(source) => {
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
}
