use destack_artifact::EmitFormat;
use destack_dir as dir;
use dir::{
    Argument, BinaryOperator, Declarator, Expression, IfCondition, IfForm, LocalNodeId, Mutability,
    NodeType, Property, ScopeKind, SymbolForm, SymbolRole, Type, TypeLiteral,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

/// The synthetic state used to rewrite one nullish coalesce operation.
struct CoalesceBindingState {
    /// The synthetic binding expression for the left operand.
    left_temp_let: LocalNodeId<Expression>,
    /// The synthetic local symbol that stores the left operand.
    left_temp_symbol: dir::GlobalSymbolId,
    /// The local binding name interned for source reconstruction.
    left_temp_name: dir::StringId,
    /// The `left == null || left == undefined` condition expression.
    condition: LocalNodeId<Expression>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize nested coalesce expressions inside one assignment pattern.
    fn normalize_nested_coalesce_in_assign_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        assign_pattern_id: LocalNodeId<dir::AssignPattern>,
    ) -> ElaborateResult<bool> {
        let assign_pattern = state.tree.get(assign_pattern_id).clone();
        let mut modified = false;

        match assign_pattern {
            dir::AssignPattern::Expression { value } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            dir::AssignPattern::Assign { pattern, value } => {
                modified |=
                    self.normalize_nested_coalesce_in_assign_pattern(state, scope, pattern)?;
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Object { fields } => {
                for field_id in fields {
                    modified |= self.normalize_nested_coalesce_in_assign_pattern_field(
                        state, scope, field_id,
                    )?;
                }
            }
        }

        Ok(modified)
    }

    /// Normalize nested coalesce expressions inside one assignment pattern field.
    fn normalize_nested_coalesce_in_assign_pattern_field(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        assign_pattern_field_id: LocalNodeId<dir::AssignPatternField>,
    ) -> ElaborateResult<bool> {
        let assign_pattern_field = state.tree.get(assign_pattern_field_id).clone();
        let mut modified = false;

        match assign_pattern_field {
            dir::AssignPatternField::Named { pattern, .. }
            | dir::AssignPatternField::Spread { pattern } => {
                if let Some(pattern_id) = pattern {
                    modified |=
                        self.normalize_nested_coalesce_in_assign_pattern(state, scope, pattern_id)?;
                }
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, key)?;
                modified |=
                    self.normalize_nested_coalesce_in_assign_pattern(state, scope, pattern)?;
            }
            dir::AssignPatternField::Positional { pattern } => {
                modified |=
                    self.normalize_nested_coalesce_in_assign_pattern(state, scope, pattern)?;
            }
            dir::AssignPatternField::Elision => {}
        }

        Ok(modified)
    }

    /// Normalize nullish coalescing in one let initializer.
    pub(super) fn normalize_coalesce_in_let(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_expr_id: LocalNodeId<Expression>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // keep nullish coalesce syntax in JS and TS outputs
        if self.profile_keeps_nullish_coalesce(state)? {
            return Ok(false);
        }

        // rewrite the original declarator as uninitialized
        state.tree.replace(
            declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: None,
            },
        );
        new_expressions.push(original_expr_id);

        // bind the left operand once before nullish checks
        let Some(binding) =
            self.prepare_coalesce_binding_state(state, scope, left, original_expr_id)?
        else {
            return Ok(false);
        };
        new_expressions.push(binding.left_temp_let);

        // assign the fallback or retained value to the original binding
        let then_target = self.pattern_to_assignment_target(state, scope, declarator.pattern)?;
        let then_assignment = self.wrap_in_assignment(state, scope, then_target, right)?;

        let else_target = self.pattern_to_assignment_target(state, scope, declarator.pattern)?;
        let else_value = self.insert_local_reference_expression_for_symbol(
            state,
            original_expr_id.into_any(),
            scope,
            binding.left_temp_name,
            binding.left_temp_symbol,
        )?;
        let else_assignment = self.wrap_in_assignment(state, scope, else_target, else_value)?;

        // emit the explicit coalesce branch
        let if_id = state.tree.reserve_from(
            NodeType::Expression,
            original_expr_id.into_any(),
            scope,
            None,
        );
        let if_id: LocalNodeId<Expression> = state.tree.insert_as_owner(
            if_id,
            Expression::If {
                form: IfForm::If,
                condition: IfCondition::Expression {
                    condition: binding.condition,
                },
                then_expression: then_assignment,
                else_expression: Some(else_assignment),
            },
        );
        self.set_void_expression_type(state.types_tail, state.module_id, if_id);
        new_expressions.push(if_id);

        Ok(true)
    }

    /// Normalize nullish coalescing in one return expression.
    pub(super) fn normalize_coalesce_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_return_id: LocalNodeId<Expression>,
        original_value_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // keep nullish coalesce syntax in JS and TS outputs
        if self.profile_keeps_nullish_coalesce(state)? {
            return Ok(false);
        }

        // bind the left operand once before nullish checks
        let origin_id = original_return_id;
        let Some(binding) = self.prepare_coalesce_binding_state(state, scope, left, origin_id)?
        else {
            return Ok(false);
        };
        new_expressions.push(binding.left_temp_let);

        // return fallback in nullish case
        let then_return = self.wrap_in_return(state, scope, right)?;

        // return bound left value otherwise
        let else_value = self.insert_local_reference_expression_for_symbol(
            state,
            origin_id.into_any(),
            scope,
            binding.left_temp_name,
            binding.left_temp_symbol,
        )?;
        let else_return = self.wrap_in_return(state, scope, else_value)?;

        // replace the original return with the explicit coalesce branch
        state.tree.replace(
            original_return_id,
            Expression::If {
                form: IfForm::If,
                condition: IfCondition::Expression {
                    condition: binding.condition,
                },
                then_expression: then_return,
                else_expression: Some(else_return),
            },
        );
        state.tree.mark_inactive(original_value_id.into_any());
        self.set_void_expression_type(state.types_tail, state.module_id, original_return_id);
        new_expressions.push(original_return_id);

        Ok(true)
    }

    /// Normalize nested coalesce expressions in-place within expression trees.
    pub(super) fn normalize_nested_coalesce_in_expression(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // keep nullish coalesce syntax in JS and TS outputs
        if self.profile_keeps_nullish_coalesce(state)? {
            return Ok(false);
        }

        let expression = state.tree.get(expression_id).clone();
        let mut modified = false;

        match expression {
            // wrappers and eager single-child forms
            Expression::Parenthesized {
                expression: statement,
            }
            | Expression::Unary {
                right: statement, ..
            }
            | Expression::MoveOf {
                right: statement, ..
            }
            | Expression::BorrowOf {
                right: statement, ..
            }
            | Expression::Maybe { left: statement }
            | Expression::Must { left: statement }
            | Expression::Await {
                expression: statement,
            }
            | Expression::AwaitMaybe {
                expression: statement,
            }
            | Expression::Comptime { body: statement } => {
                modified |=
                    self.normalize_nested_coalesce_in_expression(state, scope, statement)?;
            }

            // eager multi-child forms
            Expression::As {
                expression: value, ..
            }
            | Expression::Satisfies {
                expression: value, ..
            } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Expression::Assign { left, right } => {
                modified |= self.normalize_nested_coalesce_in_assign_pattern(state, scope, left)?;
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, right)?;
            }
            Expression::Index { left, right } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, left)?;
                if let Some(right) = right {
                    modified |=
                        self.normalize_nested_coalesce_in_expression(state, scope, right)?;
                }
            }
            Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, left)?;
            }
            Expression::Call {
                left, arguments, ..
            } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, left)?;
                for argument_id in arguments {
                    modified |=
                        self.normalize_nested_coalesce_in_argument(state, scope, argument_id)?;
                }
            }
            Expression::New { arguments, .. } => {
                for argument_id in arguments {
                    modified |=
                        self.normalize_nested_coalesce_in_argument(state, scope, argument_id)?;
                }
            }
            Expression::ArrayExpression { elements }
            | Expression::TupleExpression { elements }
            | Expression::TaggedTuple { elements, .. } => {
                for argument_id in elements {
                    modified |=
                        self.normalize_nested_coalesce_in_argument(state, scope, argument_id)?;
                }
            }
            Expression::ObjectExpression { properties, .. }
            | Expression::StructExpression { properties, .. } => {
                for property_id in properties {
                    modified |=
                        self.normalize_nested_coalesce_in_property(state, scope, property_id)?;
                }
            }
            Expression::TaggedScalarExpression { ty: _, value } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Expression::SequenceExpression { expressions } => {
                for child_id in expressions {
                    modified |=
                        self.normalize_nested_coalesce_in_expression(state, scope, child_id)?;
                }
            }
            Expression::Return { value: Some(value) } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Expression::Throw { value } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Expression::Let { declarators, .. } | Expression::Using { declarators, .. } => {
                for declarator_id in declarators {
                    let declarator = state.tree.get(declarator_id).clone();
                    if let Some(value) = declarator.value {
                        modified |=
                            self.normalize_nested_coalesce_in_expression(state, scope, value)?;
                    }
                }
            }
            // conditional forms
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                if let IfCondition::Expression { condition } = condition {
                    modified |=
                        self.normalize_nested_coalesce_in_expression(state, scope, condition)?;
                }
                if let IfCondition::Let { declarator, .. } = condition {
                    let declarator = state.tree.get(declarator).clone();
                    if let Some(value) = declarator.value {
                        modified |=
                            self.normalize_nested_coalesce_in_expression(state, scope, value)?;
                    }
                }
                modified |=
                    self.normalize_nested_coalesce_in_expression(state, scope, then_expression)?;
                if let Some(else_expression) = else_expression {
                    modified |= self.normalize_nested_coalesce_in_expression(
                        state,
                        scope,
                        else_expression,
                    )?;
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, left)?;
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, right)?;
                if operator == BinaryOperator::Coalesce {
                    modified |= self.rewrite_coalesce_value_expression(
                        state,
                        scope,
                        expression_id,
                        left,
                        right,
                    )?;
                }
            }

            _ => {}
        }

        Ok(modified)
    }

    /// Normalize coalesce nested inside one argument value expression.
    fn normalize_nested_coalesce_in_argument(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        argument_id: LocalNodeId<Argument>,
    ) -> ElaborateResult<bool> {
        let argument = state.tree.get(argument_id).clone();
        let value_id = argument.value();

        self.normalize_nested_coalesce_in_expression(state, scope, value_id)
    }

    /// Normalize coalesce nested inside one property value expression.
    fn normalize_nested_coalesce_in_property(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        property_id: LocalNodeId<Property>,
    ) -> ElaborateResult<bool> {
        let property = state.tree.get(property_id).clone();
        let mut modified = false;

        match property {
            Property::Field { value, .. } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Property::Method { body, .. } => {
                if let Some(body) = body {
                    modified |= self.normalize_nested_coalesce_in_expression(state, scope, body)?;
                }
            }
            Property::Spread { value, .. } => {
                modified |= self.normalize_nested_coalesce_in_expression(state, scope, value)?;
            }
            Property::Error { .. } => {}
        }

        Ok(modified)
    }

    /// Rewrite one coalesce expression into an explicit local nullish block expression.
    fn rewrite_coalesce_value_expression(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // preserve current expression result type for rewritten nodes
        let result_type_id = state.type_table().get_declared_or_inferred_type_id(expression_id.into_global_any(state.module_id));

        // create one lexical block scope for the local temp
        let block_scope_id = state
            .symbols
            .insert_scope(ScopeKind::Block, Some(scope), None);
        let block_scope_mark = state.symbols.get_scope_mark(block_scope_id);
        let block_scope = (block_scope_id, block_scope_mark);

        // bind the left operand once inside the local block
        let Some(binding) =
            self.prepare_coalesce_binding_state(state, block_scope, left, expression_id)?
        else {
            return Ok(false);
        };

        // keep the bound left value for else branch
        let else_value = self.insert_local_reference_expression_for_symbol(
            state,
            expression_id.into_any(),
            block_scope,
            binding.left_temp_name,
            binding.left_temp_symbol,
        )?;

        // build local if expression: if (t is nullish) right else t
        let if_id = state.tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            block_scope,
            None,
        );
        let if_id: LocalNodeId<Expression> = state.tree.insert_as_owner(
            if_id,
            Expression::If {
                form: IfForm::If,
                condition: IfCondition::Expression {
                    condition: binding.condition,
                },
                then_expression: right,
                else_expression: Some(else_value),
            },
        );

        // build local block value: { let t = left; if (...) ... else ... }
        let block_id = state.tree.reserve_from(
            NodeType::Block,
            expression_id.into_any(),
            block_scope,
            None,
        );
        let block_id = state.tree.insert_as_owner(
            block_id,
            dir::Block {
                context: dir::BlockContext::Expression,
                form: dir::BlockForm::Explicit,
                scope: block_scope_id,
                leading_expressions: vec![binding.left_temp_let],
                tail_expression: Some(if_id),
            },
        );

        // replace the original coalesce with one local block value expression
        state
            .tree
            .replace(expression_id, Expression::Block(block_id));

        // restore result type metadata on rewritten nodes
        if let Some(result_type_id) = result_type_id {
            self.set_expression_type(state.types_tail, state.module_id, if_id, result_type_id);
            state
            .types_tail
            .set_inferred_type(block_id.into_global_any(state.module_id), result_type_id);
            state.types_tail.set_inferred_type(
                expression_id.into_global_any(state.module_id),
                result_type_id,
            );
        }

        Ok(true)
    }

    /// Return true when a coalesce left operand includes one Try type.
    fn coalesce_left_contains_try(
        &self,
        state: &mut ElaborateState<'_>,
        left: LocalNodeId<Expression>,
    ) -> bool {
        let Some(left_type_id) = state.type_table().get_declared_or_inferred_type_id(left.into_global_any(state.module.id))
        else {
            return false;
        };
        let left_type_id = state.type_table().unwrap_form_payload_type_id(left_type_id);

        let mut candidates = Vec::new();
        match state.type_table().get_type(left_type_id) {
            Type::Union(union) => candidates.extend(union.elements.iter().copied()),
            _ => candidates.push(left_type_id),
        }

        for candidate_id in candidates {
            let candidate_id = state.type_table().unwrap_form_payload_type_id(candidate_id);
            let candidate = state.type_table().get_type(candidate_id);
            if matches!(
                candidate,
                Type::Literal(dir::LiteralType {
                    value: TypeLiteral::Null | TypeLiteral::Undefined,
                })
            ) {
                continue;
            }
        }

        false
    }

    /// Insert a synthetic immutable local binding initialized from one value expression.
    fn insert_synthetic_value_binding(
        &self,
        state: &mut ElaborateState<'_>,
        name_prefix: &str,
        value_type_id: dir::LocalTypeId,
        value_expression: LocalNodeId<Expression>,
        origin_id: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> (LocalNodeId<Expression>, dir::GlobalSymbolId, dir::StringId) {
        // create one local symbol for the synthetic binding
        let scope_mark = state.symbols.get_scope_mark(scope.0);
        let (symbol_id, _) = state.symbols.insert_symbol(
            SymbolRole::Local,
            SymbolForm::Variable,
            None,
            (scope.0, scope_mark),
            None,
        );
        let global_symbol = symbol_id.into_global(state.module_id);
        state.types_tail.set_value_type(global_symbol, value_type_id);

        // create one stable synthetic name
        let name_text = format!("{name_prefix}{}", symbol_id.id);
        let name = dir::StringId::for_text(&name_text);

        // create one immutable let expression with the synthetic value
        let let_id = self.insert_single_binding_let_expression(
            state,
            origin_id,
            scope,
            name,
            symbol_id,
            Mutability::Immutable,
            Some(value_expression),
        );

        (let_id, global_symbol, name)
    }

    /// Build a nullish check condition for one local symbol.
    fn build_nullish_condition_for_symbol(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        symbol: dir::GlobalSymbolId,
        name: dir::StringId,
        origin_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // create left == null
        let left_for_null = self.insert_local_reference_expression_for_symbol(
            state,
            origin_id.into_any(),
            scope,
            name,
            symbol,
        )?;
        let null_literal_id =
            self.insert_type_literal_expression(state, origin_id, TypeLiteral::Null, scope);
        let null_eq_id = self.insert_boolean_binary_expression(
            state,
            origin_id,
            left_for_null,
            BinaryOperator::Equal,
            null_literal_id,
            scope,
        );

        // create left == undefined
        let left_for_undefined = self.insert_local_reference_expression_for_symbol(
            state,
            origin_id.into_any(),
            scope,
            name,
            symbol,
        )?;
        let undefined_literal_id =
            self.insert_type_literal_expression(state, origin_id, TypeLiteral::Undefined, scope);
        let undefined_eq_id = self.insert_boolean_binary_expression(
            state,
            origin_id,
            left_for_undefined,
            BinaryOperator::Equal,
            undefined_literal_id,
            scope,
        );

        // combine checks with logical or
        let or_id = self.insert_boolean_binary_expression(
            state,
            origin_id,
            null_eq_id,
            BinaryOperator::Or,
            undefined_eq_id,
            scope,
        );

        Ok(or_id)
    }

    /// Return true when the profile should keep nullish syntax as-is.
    fn profile_keeps_nullish_coalesce(&self, state: &ElaborateState<'_>) -> ElaborateResult<bool> {
        let emit = self
            .profile(state.provider.revision(), state.profile)
            .map_err(|error| ElaborateError::Internal {
                anchor: (state.module_id).into(),
                module: state.module_id,
                message: format!("{error:?}"),
            })?
            .key
            .emit;

        Ok(matches!(emit, EmitFormat::Js | EmitFormat::Ts | EmitFormat::Html))
    }

    /// Build the shared synthetic binding and nullish condition for one left operand.
    fn prepare_coalesce_binding_state(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        left: LocalNodeId<Expression>,
        origin_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<Option<CoalesceBindingState>> {
        // defer Try coalesce to dedicated Try reification
        if self.coalesce_left_contains_try(state, left) {
            return Ok(None);
        }

        // require a known type for the left operand
        let Some(left_type_id) = state.type_table().get_declared_or_inferred_type_id(left.into_global_any(state.module_id))
        else {
            return Ok(None);
        };
        let left_type_id = state.type_table().unwrap_form_payload_type_id(left_type_id);

        // bind the left operand once before nullish checks
        let (left_temp_let, left_temp_symbol, left_temp_name) = self
            .insert_synthetic_value_binding(
                state,
                "__coalesce_left",
                left_type_id,
                left,
                origin_id,
                scope,
            );

        // create the nullish condition for the temporary value
        let condition = self.build_nullish_condition_for_symbol(
            state,
            scope,
            left_temp_symbol,
            left_temp_name,
            origin_id,
        )?;

        Ok(Some(CoalesceBindingState {
            left_temp_let,
            left_temp_symbol,
            left_temp_name,
            condition,
        }))
    }
}
