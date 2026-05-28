use destack_dir as dir;
use dir::{
    BinaryOperator, Block, Expression, IfCondition, IfForm, LocalNodeId, LocalSymbolId,
    LocalTypeId, MatchCase, MatchForm, MatchOrigin, MatchSelector, Mutability, NodeType, Pattern,
    PatternField, ScalarLiteral, StringId, TypeExpression,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Transform `match` expressions into decision trees (if else chains).
    ///
    /// ```ds
    /// match (x) {
    ///     Some(v) if v > 0 => positive(v)
    ///     Some(v) => nonPositive(v)
    ///     None => zero()
    /// }
    /// ```
    /// becomes:
    /// ```ds
    /// if (x is Some) {
    ///     let v = x.value;
    ///     if (v > 0) { positive(v) }
    ///     else { nonPositive(v) }
    /// } else { zero() }
    /// ```
    pub(super) fn transform_match(&self, state: &mut ElaborateState<'_>) -> ElaborateResult<()> {
        let match_ids: Vec<_> = state
            .tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_active_in_state(state, id.into_any()))
            .filter(|id| {
                matches!(
                    state.tree.get(*id),
                    Expression::Match {
                        form: MatchForm::Match,
                        source: MatchOrigin::Match,
                        ..
                    }
                )
            })
            .collect();

        for match_id in match_ids {
            self.transform_single_match(state, match_id)?;
        }

        Ok(())
    }

    /// Transform a single match expression into an if else chain.
    fn transform_single_match(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        let Expression::Match { value, cases, .. } = state.tree.get(match_id).clone() else {
            return Ok(());
        };
        if cases.is_empty() {
            return Ok(());
        }

        let match_type_id = self.match_expression_type_id(state, match_id)?;

        // build the if else chain from the cases, in reverse order
        let scope = state.tree.get_scope(match_id);
        let result =
            self.build_match_chain(state, match_id, value, &cases, 0, scope, match_type_id)?;

        // replace the match with the generated if else chain
        if let Some(replacement) = result {
            state.tree.replace_from(match_id, replacement);
        }

        Ok(())
    }

    /// Resolve or derive the match expression type id.
    fn match_expression_type_id(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalTypeId> {
        if let Some(type_id) = state.type_table().get_declared_or_inferred_type_id(match_id.into_global_any(state.tree.module_id))
        {
            return Ok(type_id);
        }

        let Expression::Match { cases, .. } = state.tree.get(match_id) else {
            return Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            });
        };

        let mut case_type_id = None;
        for case_id in cases {
            let body_id = match state.tree.get(*case_id) {
                MatchCase::Expression { body, .. } => body.into_global_any(state.tree.module_id),
                MatchCase::Block { body, .. } => body.into_global_any(state.tree.module_id),
            };

            let Some(body_type_id) = state.type_table().get_declared_or_inferred_type_id(body_id) else {
                return Err(ElaborateError::UnsupportedConstruct {
                    anchor: state.module_id.into(),
                });
            };

            if let Some(expected) = case_type_id {
                if expected != body_type_id {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            } else {
                case_type_id = Some(body_type_id);
            }
        }

        let Some(case_type_id) = case_type_id else {
            return Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            });
        };

        state
            .types_tail
            .set_inferred_type(match_id.into_global_any(state.tree.module_id), case_type_id);

        Ok(case_type_id)
    }

    /// Build the if else chain for match cases starting at the given index.
    fn build_match_chain(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // stop when the case list is exhausted
        if index >= cases.len() {
            return Ok(None);
        }

        // resolve the current case selector and body
        let case = state.tree.get(cases[index]).clone();
        let (selector, body) = match &case {
            MatchCase::Expression { selector, body, .. } => (selector.clone(), *body),
            MatchCase::Block { selector, body, .. } => {
                // wrap block in a block expression
                let block_expr_id = state.tree.reserve_from(
                    NodeType::Expression,
                    match_id.into_any(),
                    scope,
                    None,
                );
                let block_expr: LocalNodeId<Expression> = state
                    .tree
                    .insert_as_owner(block_expr_id, Expression::Block(*body));
                let block_type_id = state.type_table().get_declared_or_inferred_type_id(body.into_global_any(state.tree.module_id))
                    .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    })?;
                self.set_expression_type(
                    state.types_tail,
                    state.tree.module_id,
                    block_expr,
                    block_type_id,
                );
                (selector.clone(), block_expr)
            }
        };

        // resolve the selector pattern or return the body
        let MatchSelector::Pattern {
            pattern: pattern_id,
            guard,
        } = selector
        else {
            return Ok(Some(body));
        };

        // unwrap transparent pattern wrappers before lowering
        let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);

        // load the pattern node
        let pattern = state.tree.get(pattern_id).clone();

        // wildcard: return the body directly unless there is a guard
        if matches!(pattern, Pattern::Wildcard) {
            if guard.is_none() {
                return Ok(Some(body));
            }

            let guard_expr = guard.unwrap();
            let if_expr = self.build_case_if(
                state,
                match_id,
                guard_expr,
                body,
                value,
                cases,
                index,
                scope,
                match_type_id,
            )?;

            return Ok(Some(if_expr));
        }
        // binding pattern without a nested pattern
        else if let Pattern::Binding {
            pattern: None,
            name,
            symbol,
        } = &pattern
        {
            return self.handle_binding_pattern(
                state,
                match_id,
                value,
                body,
                *name,
                *symbol,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // expression patterns: emit equality check
        else if let Pattern::Expression {
            value: pattern_value,
        } = &pattern
        {
            return self.handle_expression_pattern(
                state,
                match_id,
                value,
                body,
                *pattern_value,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // range patterns: emit ordered bound checks
        else if let Pattern::Range {
            start,
            end,
            end_kind,
        } = &pattern
        {
            return self.handle_range_pattern(
                state,
                match_id,
                value,
                body,
                *start,
                *end,
                *end_kind,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // newtype patterns: emit type check and index access
        else if let Pattern::Newtype { ty, fields } = &pattern {
            return self.handle_newtype_pattern(
                state,
                match_id,
                value,
                body,
                *ty,
                fields,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // nominal object patterns: emit type check and member access
        else if let Pattern::NominalObject { ty, fields } = &pattern {
            return self.handle_nominal_object_pattern(
                state,
                match_id,
                value,
                body,
                *ty,
                fields,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // anonymous tuple patterns: use index access
        else if let Pattern::Tuple { fields } = &pattern {
            return self.handle_tuple_pattern(
                state,
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // anonymous object patterns: use member access
        else if let Pattern::Object { fields } = &pattern {
            return self.handle_object_pattern(
                state,
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // union patterns: emit or check
        else if let Pattern::Union { patterns } = &pattern {
            return self.handle_union_pattern(
                state,
                match_id,
                value,
                body,
                patterns,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }
        // sequence patterns: use index access
        else if let Pattern::Sequence { fields } = &pattern {
            return self.handle_sequence_pattern(
                state,
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                scope,
                match_type_id,
            );
        }

        // fallback to block wrapped body
        self.handle_fallback_pattern(state, match_id, body, scope)
    }

    /// Build an if expression for a match case.
    fn build_if_expression(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> LocalNodeId<Expression> {
        // allocate the if expression node
        let if_id = state.tree.reserve_from(
            NodeType::Expression,
            match_id.into_any(),
            scope,
            None,
        );

        // insert the if expression
        let if_expr: LocalNodeId<Expression> = state.tree.insert_as_owner(
            if_id,
            Expression::If {
                form: IfForm::If,
                condition: IfCondition::Expression { condition },
                then_expression,
                else_expression,
            },
        );

        // record the match type on the expression
        self.set_expression_type(state.types_tail, state.tree.module_id, if_expr, match_type_id);

        if_expr
    }

    /// Build an if expression that falls through to the next match case.
    fn build_case_if(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // build the else branch from remaining cases
        let else_expr = self.build_match_chain(
            state,
            match_id,
            value,
            cases,
            index + 1,
            scope,
            match_type_id,
        )?;

        // wrap the else branch as needed
        let else_block = self.wrap_else_branch(state, match_id, else_expr, scope)?;

        // build the if expression
        let if_expr = self.build_if_expression(
            state,
            match_id,
            condition,
            then_expression,
            else_block,
            scope,
            match_type_id,
        );

        Ok(if_expr)
    }

    /// Handle a binding pattern without a nested pattern.
    fn handle_binding_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        name: StringId,
        symbol: LocalSymbolId,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // bind the matched value before evaluating the body
        let bindings = vec![(name, symbol, value)];

        // handle bindings without a guard
        if guard.is_none() {
            let body = self.wrap_with_let_bindings(state, match_id, bindings, body, scope)?;
            return Ok(Some(body));
        }

        // evaluate the guard with the binding in scope
        let guard_expr = guard.unwrap();
        let guard_condition =
            self.wrap_with_let_bindings(state, match_id, bindings.clone(), guard_expr, scope)?;

        // evaluate the then branch with the binding in scope
        let then_body = self.wrap_with_let_bindings(state, match_id, bindings, body, scope)?;

        // build the if expression for the guard
        let if_expr = self.build_case_if(
            state,
            match_id,
            guard_condition,
            then_body,
            value,
            cases,
            index,
            scope,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle an expression pattern by emitting an equality check.
    fn handle_expression_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        pattern_value: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // build the else chain from the remaining cases
        let else_expr = self.build_match_chain(
            state,
            match_id,
            value,
            cases,
            index + 1,
            scope,
            match_type_id,
        )?;

        // skip the condition when this is the last case and has no guard
        if else_expr.is_none() && guard.is_none() {
            let body = self.wrap_in_block(state, match_id, body, scope)?;
            return Ok(Some(body));
        }

        // build the equality check
        let condition = self.build_equality_check(state, match_id, value, pattern_value, scope)?;

        // combine the condition with the guard
        let condition = self.combine_with_guard(state, match_id, condition, guard, scope)?;

        // wrap then and else branches
        let then_block = self.wrap_in_block(state, match_id, body, scope)?;
        let else_block = self.wrap_else_branch(state, match_id, else_expr, scope)?;

        // build the if expression
        let if_expr = self.build_if_expression(
            state,
            match_id,
            condition,
            then_block,
            else_block,
            scope,
            match_type_id,
        );

        Ok(Some(if_expr))
    }

    /// Handle a range pattern by emitting ordered bound checks.
    fn handle_range_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        start: Option<LocalNodeId<Expression>>,
        end: Option<LocalNodeId<Expression>>,
        end_kind: dir::RangeEnd,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        let else_expr = self.build_match_chain(
            state,
            match_id,
            value,
            cases,
            index + 1,
            scope,
            match_type_id,
        )?;

        let condition =
            self.build_range_check(state, match_id, value, start, end, end_kind, scope)?;
        let condition = self.combine_with_guard(state, match_id, condition, guard, scope)?;

        let then_block = self.wrap_in_block(state, match_id, body, scope)?;
        let else_block = self.wrap_else_branch(state, match_id, else_expr, scope)?;

        let if_expr = self.build_if_expression(
            state,
            match_id,
            condition,
            then_block,
            else_block,
            scope,
            match_type_id,
        );

        Ok(Some(if_expr))
    }

    /// Handle a newtype pattern by emitting type and field checks.
    fn handle_newtype_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        ty: LocalNodeId<TypeExpression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // start with the type check for the tag
        let mut condition = self.build_type_guard(state, match_id, value, ty, scope)?;

        // extend the condition with field checks
        for (index, field_id) in fields.iter().enumerate() {
            // resolve the field pattern id
            let field = state.tree.get(*field_id).clone();
            let pattern_id = match field {
                PatternField::Positional { pattern, .. } => Some(pattern),
                PatternField::Named { pattern, .. } => pattern,
                PatternField::Computed { .. }
                | PatternField::Spread { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            };

            // skip fields without a pattern
            let Some(pattern_id) = pattern_id else {
                continue;
            };

            // evaluate the field pattern
            let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
            let field_pattern = state.tree.get(pattern_id).clone();
            match field_pattern {
                Pattern::Wildcard => {}
                Pattern::Binding { pattern, .. } => {
                    // reject nested binding patterns
                    if pattern.is_some() {
                        return Err(ElaborateError::UnsupportedConstruct {
                            anchor: state.module_id.into(),
                        });
                    }
                }
                Pattern::Expression {
                    value: pattern_value,
                } => {
                    // check equality for literal patterns
                    let access = self.build_index_access(state, match_id, value, index, scope)?;
                    let check =
                        self.build_equality_check(state, match_id, access, pattern_value, scope)?;
                    condition =
                        self.combine_with_guard(state, match_id, condition, Some(check), scope)?;
                }
                _ => {
                    // reject unsupported field patterns
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            }
        }

        // wrap body with field bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(state, match_id, value, fields, body, scope)?;

        // apply the guard with tuple bindings in scope
        let guard = match guard {
            Some(guard_expression) => Some(self.wrap_with_tuple_bindings(
                state,
                match_id,
                value,
                fields,
                guard_expression,
                scope,
            )?),
            None => None,
        };
        let condition = self.combine_with_guard(state, match_id, condition, guard, scope)?;

        // build the if expression
        let if_expr = self.build_case_if(
            state,
            match_id,
            condition,
            body_with_bindings,
            value,
            cases,
            index,
            scope,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a nominal object pattern by emitting type and field checks.
    fn handle_nominal_object_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        ty: LocalNodeId<TypeExpression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // start with the type check for the tag
        let mut condition = self.build_type_guard(state, match_id, value, ty, scope)?;

        // extend the condition with field checks
        for field_id in fields.iter() {
            // resolve the field name and pattern
            let field = state.tree.get(*field_id).clone();
            let (field_name, pattern_id) = match field {
                PatternField::Named { name, pattern, .. } => (Some(name), pattern),
                PatternField::Positional { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
                PatternField::Computed { .. }
                | PatternField::Spread { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            };

            // skip fields without a pattern
            let Some(pattern_id) = pattern_id else {
                continue;
            };

            // skip fields without a name
            let Some(field_name) = field_name else {
                continue;
            };

            // evaluate the field pattern
            let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
            let field_pattern = state.tree.get(pattern_id).clone();
            match field_pattern {
                Pattern::Wildcard => {}
                Pattern::Binding { pattern, .. } => {
                    // reject nested binding patterns
                    if pattern.is_some() {
                        return Err(ElaborateError::UnsupportedConstruct {
                            anchor: state.module_id.into(),
                        });
                    }
                }
                Pattern::Expression {
                    value: pattern_value,
                } => {
                    // check equality for literal patterns
                    let access =
                        self.build_member_access(state, match_id, value, field_name, scope)?;
                    let check =
                        self.build_equality_check(state, match_id, access, pattern_value, scope)?;
                    condition =
                        self.combine_with_guard(state, match_id, condition, Some(check), scope)?;
                }
                _ => {
                    // reject unsupported field patterns
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            }
        }

        // wrap body with field bindings
        let body_with_bindings =
            self.wrap_with_object_bindings(state, match_id, value, fields, body, scope)?;

        // apply the guard with object bindings in scope
        let guard = match guard {
            Some(guard_expression) => Some(self.wrap_with_object_bindings(
                state,
                match_id,
                value,
                fields,
                guard_expression,
                scope,
            )?),
            None => None,
        };
        let condition = self.combine_with_guard(state, match_id, condition, guard, scope)?;

        // build the if expression
        let if_expr = self.build_case_if(
            state,
            match_id,
            condition,
            body_with_bindings,
            value,
            cases,
            index,
            scope,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a tuple pattern by using index access bindings.
    fn handle_tuple_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with tuple bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(state, match_id, value, fields, body, scope)?;

        // handle guard with tuple bindings in scope
        if let Some(guard_expr) = guard {
            let guard_expr =
                self.wrap_with_tuple_bindings(state, match_id, value, fields, guard_expr, scope)?;
            let if_expr = self.build_case_if(
                state,
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                scope,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle an object pattern by using member access bindings.
    fn handle_object_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with object bindings
        let body_with_bindings =
            self.wrap_with_object_bindings(state, match_id, value, fields, body, scope)?;

        // handle guard with object bindings in scope
        if let Some(guard_expr) = guard {
            let guard_expr =
                self.wrap_with_object_bindings(state, match_id, value, fields, guard_expr, scope)?;
            let if_expr = self.build_case_if(
                state,
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                scope,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle a union pattern by ORing the individual checks.
    fn handle_union_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        patterns: &[LocalNodeId<Pattern>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // build the union check
        let condition = self.build_union_check(state, match_id, value, patterns, scope)?;

        // combine the union check with the guard
        let condition = self.combine_with_guard(state, match_id, condition, guard, scope)?;

        // wrap the body for the then branch
        let then_block = self.wrap_in_block(state, match_id, body, scope)?;

        // build the if expression
        let if_expr = self.build_case_if(
            state,
            match_id,
            condition,
            then_block,
            value,
            cases,
            index,
            scope,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a sequence pattern by using index access bindings.
    fn handle_sequence_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        scope: dir::LocalScope,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with array bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(state, match_id, value, fields, body, scope)?;

        // handle guard with array bindings in scope
        if let Some(guard_expr) = guard {
            let guard_expr =
                self.wrap_with_tuple_bindings(state, match_id, value, fields, guard_expr, scope)?;
            let if_expr = self.build_case_if(
                state,
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                scope,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle unimplemented patterns by returning the body in a block.
    fn handle_fallback_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body in a block expression
        let body = self.wrap_in_block(state, match_id, body, scope)?;

        Ok(Some(body))
    }

    /// Build a union check: matches any of the patterns.
    fn build_union_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        patterns: &[LocalNodeId<Pattern>],
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect checks for each union pattern
        let mut checks: Vec<LocalNodeId<Expression>> = Vec::new();

        // build a check for each pattern
        for pattern_id in patterns {
            let check = self.build_pattern_check(state, match_id, value, *pattern_id, scope)?;

            checks.push(check);
        }

        // combine all checks with or
        if checks.is_empty() {
            // empty union: emit false, should not happen in practice
            return Ok(self.insert_boolean_literal_expression(state, match_id, false, scope));
        }

        // fold the checks into a single disjunction
        let mut result = checks[0];
        for check in checks.into_iter().skip(1) {
            // combine with one additional or check
            result = self.insert_boolean_binary_expression(
                state,
                match_id,
                result,
                BinaryOperator::Or,
                check,
                scope,
            );
        }

        Ok(result)
    }

    /// Build a runtime boolean check for one pattern against a value.
    fn build_pattern_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        pattern_id: LocalNodeId<Pattern>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
        let pattern = state.tree.get(pattern_id).clone();
        match pattern {
            // wildcard always matches
            Pattern::Wildcard => {
                Ok(self.insert_boolean_literal_expression(state, match_id, true, scope))
            }

            // plain bindings are irrefutable
            Pattern::Binding { pattern: None, .. } => {
                Ok(self.insert_boolean_literal_expression(state, match_id, true, scope))
            }

            // defaulting patterns defer to their wrapped pattern
            Pattern::Assign { pattern, .. } => {
                self.build_pattern_check(state, match_id, value, pattern, scope)
            }

            // nested bindings defer to their inner pattern
            Pattern::Binding {
                pattern: Some(inner),
                ..
            } => self.build_pattern_check(state, match_id, value, inner, scope),

            // literal/path expression patterns use equality
            Pattern::Expression {
                value: pattern_value,
            } => self.build_equality_check(state, match_id, value, pattern_value, scope),

            // range patterns use ordered bound checks
            Pattern::Range {
                start,
                end,
                end_kind,
            } => self.build_range_check(state, match_id, value, start, end, end_kind, scope),

            // type-space patterns lower to runtime type checks
            Pattern::TypeExpression { value: target_type } => {
                self.build_type_guard(state, match_id, value, target_type, scope)
            }

            // newtype: type check plus constrained slot checks
            Pattern::Newtype { ty, fields } => {
                let type_check = self.build_type_guard(state, match_id, value, ty, scope)?;
                self.extend_sequence_pattern_check(
                    state,
                    match_id,
                    value,
                    &fields,
                    Some(type_check),
                    scope,
                )
            }

            // nominal object: type check plus constrained field checks
            Pattern::NominalObject { ty, fields } => {
                let type_check = self.build_type_guard(state, match_id, value, ty, scope)?;
                self.extend_object_pattern_check(
                    state,
                    match_id,
                    value,
                    &fields,
                    Some(type_check),
                    scope,
                )
            }

            // untagged sequence patterns use constrained slot checks
            Pattern::Tuple { fields } | Pattern::Sequence { fields } => {
                self.extend_sequence_pattern_check(state, match_id, value, &fields, None, scope)
            }

            // untagged object patterns use constrained field checks
            Pattern::Object { fields } => {
                self.extend_object_pattern_check(state, match_id, value, &fields, None, scope)
            }

            // nested union patterns fold recursively
            Pattern::Union { patterns } => {
                self.build_union_check(state, match_id, value, &patterns, scope)
            }

            // stack access patterns need ownership-aware lowering
            Pattern::DereferenceOf { .. } => Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            }),

            // transparent wrappers recurse to their inner pattern
            Pattern::Must(inner)
            | Pattern::BorrowOf { right: inner, .. }
            | Pattern::MoveOf { right: inner, .. } => {
                self.build_pattern_check(state, match_id, value, inner, scope)
            }
        }
    }

    /// Extend one condition with tuple or array field checks.
    fn extend_sequence_pattern_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        condition: Option<LocalNodeId<Expression>>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let mut condition = condition;

        // combine constrained field checks in order
        for (index, field_id) in fields.iter().enumerate() {
            let field = state.tree.get(*field_id).clone();
            let pattern_id = match field {
                PatternField::Positional { pattern, .. } => Some(pattern),
                PatternField::Named { pattern, .. } => pattern,
                PatternField::Elision => None,
                PatternField::Computed { .. } | PatternField::Spread { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            };

            let Some(pattern_id) = pattern_id else {
                continue;
            };
            let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
            let nested = state.tree.get(pattern_id).clone();
            if matches!(
                nested,
                Pattern::Wildcard | Pattern::Binding { pattern: None, .. }
            ) {
                continue;
            }

            let access = self.build_index_access(state, match_id, value, index, scope)?;
            let nested_check =
                self.build_pattern_check(state, match_id, access, pattern_id, scope)?;

            condition = Some(self.merge_condition_with_check(
                state,
                match_id,
                condition,
                nested_check,
                scope,
            )?);
        }

        if let Some(condition) = condition {
            Ok(condition)
        } else {
            Ok(self.insert_boolean_literal_expression(state, match_id, true, scope))
        }
    }

    /// Extend one condition with object field checks.
    fn extend_object_pattern_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        condition: Option<LocalNodeId<Expression>>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let mut condition = condition;

        // combine constrained field checks in declaration order
        for field_id in fields {
            let field = state.tree.get(*field_id).clone();
            let (field_name, pattern_id) = match field {
                PatternField::Named { name, pattern, .. } => (Some(name), pattern),
                PatternField::Positional { .. }
                | PatternField::Computed { .. }
                | PatternField::Spread { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            };

            let Some(pattern_id) = pattern_id else {
                continue;
            };
            let Some(field_name) = field_name else {
                continue;
            };
            let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
            let nested = state.tree.get(pattern_id).clone();
            if matches!(
                nested,
                Pattern::Wildcard | Pattern::Binding { pattern: None, .. }
            ) {
                continue;
            }

            let access = self.build_member_access(state, match_id, value, field_name, scope)?;
            let nested_check =
                self.build_pattern_check(state, match_id, access, pattern_id, scope)?;

            condition = Some(self.merge_condition_with_check(
                state,
                match_id,
                condition,
                nested_check,
                scope,
            )?);
        }

        if let Some(condition) = condition {
            Ok(condition)
        } else {
            Ok(self.insert_boolean_literal_expression(state, match_id, true, scope))
        }
    }

    /// Build equality check expression: `value == pattern_value`.
    fn build_equality_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        pattern_value: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // build `value == pattern_value`
        let expression_id = self.insert_boolean_binary_expression(
            state,
            match_id,
            value,
            BinaryOperator::Equal,
            pattern_value,
            scope,
        );

        Ok(expression_id)
    }

    /// Build one ordered range pattern check.
    fn build_range_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        start: Option<LocalNodeId<Expression>>,
        end: Option<LocalNodeId<Expression>>,
        end_kind: dir::RangeEnd,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let mut condition = None;

        // include the start bound
        if let Some(start) = start {
            let start_check = self.insert_boolean_binary_expression(
                state,
                match_id,
                start,
                BinaryOperator::LessThanOrEqual,
                value,
                scope,
            );
            condition = Some(start_check);
        }

        // apply the end bound
        if let Some(end) = end {
            let operator = match end_kind {
                dir::RangeEnd::Open => BinaryOperator::LessThan,
                dir::RangeEnd::Inclusive => BinaryOperator::LessThanOrEqual,
            };
            let end_check =
                self.insert_boolean_binary_expression(state, match_id, value, operator, end, scope);
            condition = Some(
                self.merge_condition_with_check(state, match_id, condition, end_check, scope)?,
            );
        }

        if let Some(condition) = condition {
            Ok(condition)
        } else {
            Ok(self.insert_boolean_literal_expression(state, match_id, true, scope))
        }
    }

    /// Build one runtime type guard: `value is Type`.
    fn build_type_guard(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        ty: LocalNodeId<TypeExpression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // build `value is ty`
        let expr_id = self.insert_is_type_check_expression(state, match_id, value, ty, scope);

        // resolve the value type for runtime checks
        let Some(value_type_id) = state.type_table().get_declared_or_inferred_type_id(value.into_global_any(state.tree.module_id))
        else {
            return Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            });
        };

        // resolve the target type for runtime checks
        let Some(target_type_id) = state.type_table().get_declared_or_inferred_type_id(ty.into_global_any(state.tree.module_id))
        else {
            return Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            });
        };

        // derive and record the runtime check kind
        let guard_entry =
            self.guard_entry_for_relation(&state.type_table(), value_type_id, target_type_id);
        let Some(guard_entry) = guard_entry else {
            return Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            });
        };
        state
            .guards
            .set_entry(expr_id.into_global_any(state.tree.module_id), guard_entry);
        Ok(expr_id)
    }

    /// Combine a condition with an optional guard using `&&`.
    fn combine_with_guard(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // merge the guard into the condition when present
        match guard {
            Some(guard_expression) => {
                // build `condition && guard_expression`
                let expr_id = self.insert_boolean_binary_expression(
                    state,
                    match_id,
                    condition,
                    BinaryOperator::And,
                    guard_expression,
                    scope,
                );
                Ok(expr_id)
            }
            None => Ok(condition),
        }
    }

    /// Merge one optional condition with a required check.
    fn merge_condition_with_check(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        condition: Option<LocalNodeId<Expression>>,
        check: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        match condition {
            Some(condition) => {
                self.combine_with_guard(state, match_id, condition, Some(check), scope)
            }
            None => Ok(check),
        }
    }

    /// Build an index access expression: `value[index]`.
    fn build_index_access(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        index: usize,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // create index literal
        let index_lit_id = state.tree.reserve_from(
            NodeType::Expression,
            match_id.into_any(),
            scope,
            None,
        );
        let index_expr: LocalNodeId<Expression> = state.tree.insert_as_owner(
            index_lit_id,
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(index as i64),
            },
        );
        self.set_scalar_literal_type(
            state.types_tail,
            state.tree.module_id,
            index_expr,
            ScalarLiteral::Integer(index as i64),
        );

        // create index expression: value[index]
        let idx_id = state.tree.reserve_from(
            NodeType::Expression,
            match_id.into_any(),
            scope,
            None,
        );
        let expr_id = state.tree.insert_as_owner(
            idx_id,
            Expression::Index {
                left: value,
                right: Some(index_expr),
            },
        );

        // resolve the element type
        let element_type_id = self
            .index_access_type_id(state, value, index)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            })?;

        // assign the element type
        self.set_expression_type(state.types_tail, state.tree.module_id, expr_id, element_type_id);
        Ok(expr_id)
    }

    /// Build a member access expression: `value.name`.
    fn build_member_access(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        name: StringId,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // reserve the member access node
        let member_id = state.tree.reserve_from(
            NodeType::Expression,
            match_id.into_any(),
            scope,
            None,
        );

        // insert the member access expression
        let expr_id = state.tree.insert_as_owner(
            member_id,
            Expression::Member {
                left: value,
                name: Some(name),
            },
        );

        // resolve the field type
        let field_type_id = self
            .member_access_type_id(state, value, name)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            })?;

        // assign the field type
        self.set_expression_type(state.types_tail, state.tree.module_id, expr_id, field_type_id);
        Ok(expr_id)
    }

    /// Resolve a direct binding tuple or object field pattern.
    fn direct_binding_from_pattern(
        &self,
        state: &ElaborateState<'_>,
        pattern_id: LocalNodeId<Pattern>,
    ) -> Option<(StringId, LocalSymbolId)> {
        let pattern_id = self.unwrap_pattern_wrappers(state, pattern_id);
        let pattern = state.tree.get(pattern_id);

        // extract direct binding patterns without nested sub-patterns
        if let Pattern::Binding {
            name,
            pattern,
            symbol,
        } = pattern
            && pattern.is_none()
        {
            return Some((*name, *symbol));
        }

        None
    }

    /// Wrap body with let bindings for tuple field extractions.
    ///
    /// For each field in the pattern, creates `let <binding> = value[i]`.
    /// Returns a block containing the bindings followed by the body.
    fn wrap_with_tuple_bindings(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect bindings to emit
        let mut bindings = Vec::new();

        // walk each field in order
        for (i, field_id) in fields.iter().enumerate() {
            let field = state.tree.get(*field_id).clone();
            match field {
                PatternField::Positional { pattern, .. } => {
                    if let Some((name, symbol)) = self.direct_binding_from_pattern(state, pattern) {
                        let access = self.build_index_access(state, match_id, value, i, scope)?;
                        bindings.push((name, symbol, access));
                        continue;
                    }

                    let pattern_id = self.unwrap_pattern_wrappers(state, pattern);
                    if let Pattern::Binding {
                        name: _,
                        symbol: _,
                        pattern: Some(_),
                    } = state.tree.get(pattern_id)
                    {
                        return Err(ElaborateError::UnsupportedConstruct {
                            anchor: state.module_id.into(),
                        });
                    }
                }
                PatternField::Named { pattern, .. } => {
                    // named field in tuple position, use index access
                    let access = self.build_index_access(state, match_id, value, i, scope)?;

                    // bind only direct nested binding patterns
                    if let Some(pattern_id) = pattern
                        && let Some((name, symbol)) =
                            self.direct_binding_from_pattern(state, pattern_id)
                    {
                        bindings.push((name, symbol, access));
                    }
                }
                PatternField::Spread { .. }
                | PatternField::Computed { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            }
        }

        self.wrap_with_let_bindings(state, match_id, bindings, body, scope)
    }

    /// Unwrap transparent pattern wrappers and return the innermost pattern.
    fn unwrap_pattern_wrappers(
        &self,
        state: &ElaborateState<'_>,
        pattern_id: LocalNodeId<Pattern>,
    ) -> LocalNodeId<Pattern> {
        let mut current = pattern_id;

        loop {
            match state.tree.get(current) {
                Pattern::Assign { pattern: inner, .. }
                | Pattern::Must(inner)
                | Pattern::BorrowOf { right: inner, .. }
                | Pattern::MoveOf { right: inner, .. } => {
                    current = *inner;
                }
                _ => return current,
            }
        }
    }

    /// Wrap body with let bindings for object field extractions.
    ///
    /// For each field in the pattern, creates `let <binding> = value.<field>`.
    /// Returns a block containing the bindings followed by the body.
    fn wrap_with_object_bindings(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect bindings to emit
        let mut bindings = Vec::new();

        // walk each field in order
        for field_id in fields.iter() {
            let field = state.tree.get(*field_id).clone();
            match field {
                PatternField::Named {
                    name,
                    symbol,
                    is_shorthand,
                    pattern,
                } => {
                    let access = self.build_member_access(state, match_id, value, name, scope)?;

                    // bind either nested bindings or shorthand field bindings
                    if let Some(pattern_id) = pattern {
                        if let Some((binding_name, symbol)) =
                            self.direct_binding_from_pattern(state, pattern_id)
                        {
                            bindings.push((binding_name, symbol, access));
                        }
                    } else if is_shorthand && let Some(symbol) = symbol {
                        bindings.push((name, symbol, access));
                    }
                }
                PatternField::Positional { .. }
                | PatternField::Spread { .. }
                | PatternField::Computed { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                }
            }
        }

        self.wrap_with_let_bindings(state, match_id, bindings, body, scope)
    }

    /// Wrap body in a block with the given let bindings.
    ///
    /// Creates a block containing:
    /// 1. Let expressions for each binding (e.g., `let x = value[0];`)
    /// 2. The original body expression
    fn wrap_with_let_bindings(
        &self,
        state: &mut ElaborateState<'_>,
        match_id: LocalNodeId<Expression>,
        bindings: Vec<(StringId, LocalSymbolId, LocalNodeId<Expression>)>,
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // if no bindings, just wrap body in a block
        if bindings.is_empty() {
            return self.wrap_in_block(state, match_id, body, scope);
        }

        // build let expressions for each binding
        let mut leading_expressions: Vec<LocalNodeId<Expression>> = Vec::new();

        for (name, symbol, value) in bindings {
            // create one local let binding for this extracted value
            let let_expr = self.insert_single_binding_let_expression(
                state,
                match_id,
                scope,
                name,
                symbol,
                Mutability::Immutable,
                Some(value),
            );

            // wrap in statement
            let stmt = self.insert_effect_expression(state, match_id, let_expr, scope);

            leading_expressions.push(stmt);
        }

        // create Block containing all expressions
        let block_id = state.tree.reserve_from(
            NodeType::Block,
            match_id.into_any(),
            scope,
            None,
        );
        let block: LocalNodeId<Block> = state.tree.insert_as_owner(
            block_id,
            Block {
                context: dir::BlockContext::Expression,
                form: dir::BlockForm::Explicit,
                scope: scope.0,
                leading_expressions,
                tail_expression: Some(body),
            },
        );

        // wrap in Expression::Block
        let block_expr_id = state.tree.reserve_from(
            NodeType::Expression,
            match_id.into_any(),
            scope,
            None,
        );
        let expr_id = state
            .tree
            .insert_as_owner(block_expr_id, Expression::Block(block));
        let body_type_id = state.type_table().get_declared_or_inferred_type_id(body.into_global_any(state.tree.module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            })?;
        state
            .types_tail
            .set_inferred_type(block.into_global_any(state.tree.module_id), body_type_id);
        self.set_expression_type(state.types_tail, state.tree.module_id, expr_id, body_type_id);
        Ok(expr_id)
    }

    /// Wrap an expression in a block with the given scope.
    /// Returns an Expression::Block containing the expression.
    /// If the expression is already a block, returns it unchanged.
    fn wrap_in_block(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // don't double wrap if already a block
        if let Expression::Block(block) = state.tree.get(body) {
            let body_type_id = state.type_table().get_declared_or_inferred_type_id(body.into_global_any(state.tree.module_id));
            if let Some(_body_type_id) = body_type_id {
                return Ok(body);
            }

            let block_type_id = state.type_table().get_declared_or_inferred_type_id(block.into_global_any(state.tree.module_id))
                .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                    anchor: state.module_id.into(),
                })?;
            self.set_expression_type(state.types_tail, state.tree.module_id, body, block_type_id);

            return Ok(body);
        }

        // create the Block node
        let block_id = state.tree.reserve_from(
            NodeType::Block,
            origin_id.into_any(),
            scope,
            None,
        );
        let block: LocalNodeId<Block> = state.tree.insert_as_owner(
            block_id,
            Block {
                context: dir::BlockContext::Expression,
                form: dir::BlockForm::Explicit,
                scope: scope.0,
                leading_expressions: Vec::new(),
                tail_expression: Some(body),
            },
        );

        // create the Expression::Block wrapper
        let block_expr_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let expr_id = state
            .tree
            .insert_as_owner(block_expr_id, Expression::Block(block));
        let body_type_id = state.type_table().get_declared_or_inferred_type_id(body.into_global_any(state.tree.module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            })?;
        state
            .types_tail
            .set_inferred_type(block.into_global_any(state.tree.module_id), body_type_id);
        self.set_expression_type(state.types_tail, state.tree.module_id, expr_id, body_type_id);
        Ok(expr_id)
    }

    /// Wrap an else branch, but don't wrap if it's an If expression.
    /// This keeps else if chains flat.
    fn wrap_else_branch(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        else_expr: Option<LocalNodeId<Expression>>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        let else_expr = else_expr.map(|e| {
            // don't wrap if it's an If expression, creates flat else if chain
            if matches!(state.tree.get(e), Expression::If { .. }) {
                Ok(e)
            } else {
                self.wrap_in_block(state, origin_id, e, scope)
            }
        });
        else_expr.transpose()
    }

    /// Resolve the inferred type for a synthesized index access.
    fn index_access_type_id(
        &self,
        state: &ElaborateState<'_>,
        value: LocalNodeId<Expression>,
        index: usize,
    ) -> Option<LocalTypeId> {
        let value_type_id = state.type_table().get_declared_or_inferred_type_id(value.into_global_any(state.tree.module_id))?;
        let value_type_id = state.type_table().unwrap_form_payload_type_id(value_type_id);
        let value_type = state.type_table().get_type(value_type_id);

        match value_type {
            dir::Type::Tuple(tuple) => tuple.elements.get(index).map(|element| element.ty),
            dir::Type::FixedArray(array) => Some(array.element),
            dir::Type::Slice(slice) => slice.element,
            _ => None,
        }
    }

    /// Resolve the inferred type for a synthesized member access.
    fn member_access_type_id(
        &self,
        state: &ElaborateState<'_>,
        value: LocalNodeId<Expression>,
        name: StringId,
    ) -> Option<LocalTypeId> {
        let value_type_id = state.type_table().get_declared_or_inferred_type_id(value.into_global_any(state.tree.module_id))?;
        let value_type_id = state.type_table().unwrap_form_payload_type_id(value_type_id);
        let value_type = state.type_table().get_type(value_type_id);

        match value_type {
            dir::Type::Shape(object) => object
                .fields
                .iter()
                .find_map(|field| (field.key == dir::StaticKey::Name(name)).then_some(field.ty)),
            _ => None,
        }
    }
}
