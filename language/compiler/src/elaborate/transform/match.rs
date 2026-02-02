use destack_dir::{
    BindingAnchor, Block, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    Declarator, Expression, IfCondition, IfKind, LocalNodeId, LocalSymbolId, LocalTypeId,
    MatchCase, MatchKind, MatchSelector, MatchSource, Mutability, Name, NodeTree, NodeType,
    Pattern, PatternField, ScalarLiteral, StringId, TypeBinaryOperator, TypeTable,
};
use destack_source::ModuleId;

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
    pub(super) fn transform_match(
        &self,
        tree: &mut NodeTree,
        symbols: &destack_dir::SymbolTable,
        types: &mut destack_dir::TypeTable,
    ) -> ElaborateResult<()> {
        let match_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_node_active(tree, symbols, id.into_any()))
            .filter(|id| {
                matches!(
                    tree.get(*id),
                    Expression::Match {
                        kind: MatchKind::Match,
                        source: MatchSource::Match,
                        ..
                    }
                )
            })
            .collect();

        for match_id in match_ids {
            self.transform_single_match(match_id, tree, types)?;
        }

        Ok(())
    }

    /// Transform a single match expression into an if else chain.
    fn transform_single_match(
        &self,
        match_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        let Expression::Match { value, cases, .. } = tree.get(match_id).clone() else {
            return Ok(());
        };
        if cases.is_empty() {
            return Ok(());
        }

        let match_type_id = self.match_expression_type_id(match_id, tree, types)?;

        // build the if else chain from the cases, in reverse order
        let scope = tree.get_scope(match_id);
        let result = self.build_match_chain(
            match_id,
            value,
            &cases,
            0,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        // replace the match with the generated if else chain
        if let Some(replacement) = result {
            let replacement = tree.get(replacement).clone();
            tree.replace(match_id, replacement);
        }

        Ok(())
    }

    /// Resolve or derive the match expression type id.
    fn match_expression_type_id(
        &self,
        match_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(match_id.into_global_any(tree.module_id))
        {
            return Ok(type_id);
        }

        let Expression::Match { cases, .. } = tree.get(match_id) else {
            return Err(ElaborateError::UnsupportedConstruct {
                node: match_id.into_global_any(tree.module_id).into_anchored(None),
            });
        };

        let mut case_type_id = None;
        for case_id in cases {
            let body_id = match tree.get(*case_id) {
                MatchCase::Expression { body, .. } => body.into_global_any(tree.module_id),
                MatchCase::Block { body, .. } => body.into_global_any(tree.module_id),
            };

            let Some(body_type_id) = types.get_declared_or_inferred_type_id(body_id) else {
                return Err(ElaborateError::UnsupportedConstruct {
                    node: body_id.into_anchored(None),
                });
            };

            if let Some(expected) = case_type_id {
                if expected != body_type_id {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: match_id.into_global_any(tree.module_id).into_anchored(None),
                    });
                }
            } else {
                case_type_id = Some(body_type_id);
            }
        }

        let Some(case_type_id) = case_type_id else {
            return Err(ElaborateError::UnsupportedConstruct {
                node: match_id.into_global_any(tree.module_id).into_anchored(None),
            });
        };

        types.set_inferred_type(match_id.into_global_any(tree.module_id), case_type_id);

        Ok(case_type_id)
    }

    /// Build the if else chain for match cases starting at the given index.
    fn build_match_chain(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // stop when the case list is exhausted
        if index >= cases.len() {
            return Ok(None);
        }

        // resolve the current case selector and body
        let case = tree.get(cases[index]).clone();
        let (selector, body) = match &case {
            MatchCase::Expression { selector, body, .. } => (selector.clone(), *body),
            MatchCase::Block { selector, body, .. } => {
                // wrap block in a block expression
                let block_expr_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let block_expr: LocalNodeId<Expression> =
                    tree.insert(block_expr_id, Expression::Block { block: *body });
                let block_type_id = types
                    .get_declared_or_inferred_type_id(body.into_global_any(tree.module_id))
                    .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                        node: body.into_global_any(tree.module_id).into_anchored(None),
                    })?;
                self.set_expression_type(types, tree.module_id, block_expr, block_type_id);
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

        // load the pattern node
        let pattern = tree.get(pattern_id).clone();

        // wildcard: return the body directly
        if matches!(pattern, Pattern::Wildcard) && guard.is_none() {
            return Ok(Some(body));
        }
        // binding pattern without a nested pattern
        else if let Pattern::Binding {
            pattern: None,
            name,
            symbol,
            mutability,
        } = &pattern
        {
            return self.handle_binding_pattern(
                match_id,
                value,
                body,
                *name,
                *symbol,
                *mutability,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // expression patterns: emit equality check
        else if let Pattern::Expression {
            value: pattern_value,
        } = &pattern
        {
            return self.handle_expression_pattern(
                match_id,
                value,
                body,
                *pattern_value,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // tagged tuple patterns: emit type check and index access
        else if let Pattern::TaggedTuple { ty, fields } = &pattern {
            return self.handle_tagged_tuple_pattern(
                match_id,
                value,
                body,
                *ty,
                fields,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // tagged object patterns: emit type check and member access
        else if let Pattern::TaggedObject { ty, fields } = &pattern {
            return self.handle_tagged_object_pattern(
                match_id,
                value,
                body,
                *ty,
                fields,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // anonymous tuple patterns: use index access
        else if let Pattern::Tuple { fields } = &pattern {
            return self.handle_tuple_pattern(
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // anonymous object patterns: use member access
        else if let Pattern::Object { fields } = &pattern {
            return self.handle_object_pattern(
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // range patterns: emit range check
        else if let Pattern::Range {
            start,
            end,
            is_inclusive,
        } = &pattern
        {
            return self.handle_range_pattern(
                match_id,
                value,
                body,
                start,
                end,
                *is_inclusive,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // union patterns: emit or check
        else if let Pattern::Union { patterns } = &pattern {
            return self.handle_union_pattern(
                match_id,
                value,
                body,
                patterns,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }
        // array patterns: use index access
        else if let Pattern::Array { fields } = &pattern {
            return self.handle_array_pattern(
                match_id,
                value,
                body,
                fields,
                guard,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            );
        }

        // fallback to block wrapped body
        self.handle_fallback_pattern(match_id, body, tree, scope, types)
    }

    /// Build an if expression for a match case.
    fn build_if_expression(
        &self,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> LocalNodeId<Expression> {
        // allocate the if expression node
        let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);

        // insert the if expression
        let if_expr: LocalNodeId<Expression> = tree.insert(
            if_id,
            Expression::If {
                kind: IfKind::If,
                condition: IfCondition::Expression { condition },
                then_expression,
                else_expression,
            },
        );

        // record the match type on the expression
        self.set_expression_type(types, tree.module_id, if_expr, match_type_id);

        if_expr
    }

    /// Build an if expression that falls through to the next match case.
    fn build_case_if(
        &self,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // build the else branch from remaining cases
        let else_expr = self.build_match_chain(
            match_id,
            value,
            cases,
            index + 1,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        // wrap the else branch as needed
        let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree, types)?;

        // build the if expression
        let if_expr = self.build_if_expression(
            match_id,
            condition,
            then_expression,
            else_block,
            tree,
            scope,
            types,
            match_type_id,
        );

        Ok(if_expr)
    }

    /// Handle a binding pattern without a nested pattern.
    #[allow(clippy::too_many_arguments)]
    fn handle_binding_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        name: StringId,
        symbol: LocalSymbolId,
        mutability: Option<Mutability>,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // bind the matched value before evaluating the body
        let bindings = vec![(name, symbol, mutability, value)];

        // handle bindings without a guard
        if guard.is_none() {
            let body = self.wrap_with_let_bindings(match_id, bindings, body, tree, scope, types)?;
            return Ok(Some(body));
        }

        // evaluate the guard with the binding in scope
        let guard_expr = guard.unwrap();
        let guard_condition = self.wrap_with_let_bindings(
            match_id,
            bindings.clone(),
            guard_expr,
            tree,
            scope,
            types,
        )?;

        // evaluate the then branch with the binding in scope
        let then_body =
            self.wrap_with_let_bindings(match_id, bindings, body, tree, scope, types)?;

        // build the if expression for the guard
        let if_expr = self.build_case_if(
            match_id,
            guard_condition,
            then_body,
            value,
            cases,
            index,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle an expression pattern by emitting an equality check.
    #[allow(clippy::too_many_arguments)]
    fn handle_expression_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        pattern_value: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // build the else chain from the remaining cases
        let else_expr = self.build_match_chain(
            match_id,
            value,
            cases,
            index + 1,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        // skip the condition when this is the last case and has no guard
        if else_expr.is_none() && guard.is_none() {
            let body = self.wrap_in_block(match_id, body, scope, tree, types)?;
            return Ok(Some(body));
        }

        // build the equality check
        let condition =
            self.build_equality_check(match_id, value, pattern_value, tree, scope, types)?;

        // combine the condition with the guard
        let condition = self.combine_with_guard(match_id, condition, guard, tree, scope, types)?;

        // wrap then and else branches
        let then_block = self.wrap_in_block(match_id, body, scope, tree, types)?;
        let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree, types)?;

        // build the if expression
        let if_expr = self.build_if_expression(
            match_id,
            condition,
            then_block,
            else_block,
            tree,
            scope,
            types,
            match_type_id,
        );

        Ok(Some(if_expr))
    }

    /// Handle a tagged tuple pattern by emitting type and field checks.
    #[allow(clippy::too_many_arguments)]
    fn handle_tagged_tuple_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        ty: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // start with the type check for the tag
        let mut condition = self.build_type_check(match_id, value, ty, tree, scope, types)?;

        // refine the condition with field checks
        for (index, field_id) in fields.iter().enumerate() {
            // resolve the field pattern id
            let field = tree.get(*field_id).clone();
            let pattern_id = match field {
                PatternField::Positional { pattern } => Some(pattern),
                PatternField::Named { pattern, .. } => pattern,
                PatternField::Alias { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: field_id.into_global_any(tree.module_id).into_anchored(None),
                    });
                }
                PatternField::Computed { .. }
                | PatternField::Spread { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: field_id.into_global_any(tree.module_id).into_anchored(None),
                    });
                }
            };

            // skip fields without a pattern
            let Some(pattern_id) = pattern_id else {
                continue;
            };

            // evaluate the field pattern
            let field_pattern = tree.get(pattern_id).clone();
            match field_pattern {
                Pattern::Wildcard => {}
                Pattern::Binding { pattern, .. } => {
                    // reject nested binding patterns
                    if pattern.is_some() {
                        return Err(ElaborateError::UnsupportedConstruct {
                            node: pattern_id
                                .into_global_any(tree.module_id)
                                .into_anchored(None),
                        });
                    }
                }
                Pattern::Expression {
                    value: pattern_value,
                } => {
                    // check equality for literal patterns
                    let access =
                        self.build_index_access(match_id, value, index, tree, scope, types)?;
                    let check = self.build_equality_check(
                        match_id,
                        access,
                        pattern_value,
                        tree,
                        scope,
                        types,
                    )?;
                    condition = self.combine_with_guard(
                        match_id,
                        condition,
                        Some(check),
                        tree,
                        scope,
                        types,
                    )?;
                }
                Pattern::Range {
                    start,
                    end,
                    is_inclusive,
                } => {
                    // check range bounds for range patterns
                    let access =
                        self.build_index_access(match_id, value, index, tree, scope, types)?;
                    let range_condition = self.build_range_check(
                        match_id,
                        access,
                        start
                            .as_ref()
                            .map(|pattern_id| tree.get(*pattern_id).clone()),
                        end.as_ref().map(|pattern_id| tree.get(*pattern_id).clone()),
                        is_inclusive,
                        tree,
                        scope,
                        types,
                    );
                    if let Some(range_condition) = range_condition {
                        condition = self.combine_with_guard(
                            match_id,
                            condition,
                            Some(range_condition),
                            tree,
                            scope,
                            types,
                        )?;
                    }
                }
                _ => {
                    // reject unsupported field patterns
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: pattern_id
                            .into_global_any(tree.module_id)
                            .into_anchored(None),
                    });
                }
            }
        }

        // apply the guard if present
        let condition = self.combine_with_guard(match_id, condition, guard, tree, scope, types)?;

        // wrap body with field bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope, types)?;

        // build the if expression
        let if_expr = self.build_case_if(
            match_id,
            condition,
            body_with_bindings,
            value,
            cases,
            index,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a tagged object pattern by emitting type and field checks.
    #[allow(clippy::too_many_arguments)]
    fn handle_tagged_object_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        ty: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // start with the type check for the tag
        let mut condition = self.build_type_check(match_id, value, ty, tree, scope, types)?;

        // refine the condition with field checks
        for field_id in fields.iter() {
            // resolve the field name and pattern
            let field = tree.get(*field_id).clone();
            let (field_name, pattern_id) = match field {
                PatternField::Named { name, pattern, .. } => (Some(name), pattern),
                PatternField::Alias { name, .. } => (Some(name), None),
                PatternField::Positional { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: field_id.into_global_any(tree.module_id).into_anchored(None),
                    });
                }
                PatternField::Computed { .. }
                | PatternField::Spread { .. }
                | PatternField::Elision => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: field_id.into_global_any(tree.module_id).into_anchored(None),
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
            let field_pattern = tree.get(pattern_id).clone();
            match field_pattern {
                Pattern::Wildcard => {}
                Pattern::Binding { pattern, .. } => {
                    // reject nested binding patterns
                    if pattern.is_some() {
                        return Err(ElaborateError::UnsupportedConstruct {
                            node: pattern_id
                                .into_global_any(tree.module_id)
                                .into_anchored(None),
                        });
                    }
                }
                Pattern::Expression {
                    value: pattern_value,
                } => {
                    // check equality for literal patterns
                    let access =
                        self.build_member_access(match_id, value, field_name, tree, scope, types)?;
                    let check = self.build_equality_check(
                        match_id,
                        access,
                        pattern_value,
                        tree,
                        scope,
                        types,
                    )?;
                    condition = self.combine_with_guard(
                        match_id,
                        condition,
                        Some(check),
                        tree,
                        scope,
                        types,
                    )?;
                }
                Pattern::Range {
                    start,
                    end,
                    is_inclusive,
                } => {
                    // check range bounds for range patterns
                    let access =
                        self.build_member_access(match_id, value, field_name, tree, scope, types)?;
                    let range_condition = self.build_range_check(
                        match_id,
                        access,
                        start
                            .as_ref()
                            .map(|pattern_id| tree.get(*pattern_id).clone()),
                        end.as_ref().map(|pattern_id| tree.get(*pattern_id).clone()),
                        is_inclusive,
                        tree,
                        scope,
                        types,
                    );
                    if let Some(range_condition) = range_condition {
                        condition = self.combine_with_guard(
                            match_id,
                            condition,
                            Some(range_condition),
                            tree,
                            scope,
                            types,
                        )?;
                    }
                }
                _ => {
                    // reject unsupported field patterns
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: pattern_id
                            .into_global_any(tree.module_id)
                            .into_anchored(None),
                    });
                }
            }
        }

        // apply the guard if present
        let condition = self.combine_with_guard(match_id, condition, guard, tree, scope, types)?;

        // wrap body with field bindings
        let body_with_bindings =
            self.wrap_with_object_bindings(match_id, value, fields, body, tree, scope, types)?;

        // build the if expression
        let if_expr = self.build_case_if(
            match_id,
            condition,
            body_with_bindings,
            value,
            cases,
            index,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a tuple pattern by using index access bindings.
    #[allow(clippy::too_many_arguments)]
    fn handle_tuple_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with tuple bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope, types)?;

        // handle guard if present
        if let Some(guard_expr) = guard {
            let if_expr = self.build_case_if(
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle an object pattern by using member access bindings.
    #[allow(clippy::too_many_arguments)]
    fn handle_object_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with object bindings
        let body_with_bindings =
            self.wrap_with_object_bindings(match_id, value, fields, body, tree, scope, types)?;

        // handle guard if present
        if let Some(guard_expr) = guard {
            let if_expr = self.build_case_if(
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle a range pattern by emitting bound checks.
    #[allow(clippy::too_many_arguments)]
    fn handle_range_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        start: &Option<LocalNodeId<Pattern>>,
        end: &Option<LocalNodeId<Pattern>>,
        is_inclusive: bool,
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // build the range condition
        let condition = self.build_range_check(
            match_id,
            value,
            start
                .as_ref()
                .map(|pattern_id| tree.get(*pattern_id).clone()),
            end.as_ref().map(|pattern_id| tree.get(*pattern_id).clone()),
            is_inclusive,
            tree,
            scope,
            types,
        );

        // handle bounded ranges with a condition
        if let Some(condition) = condition {
            let condition =
                self.combine_with_guard(match_id, condition, guard, tree, scope, types)?;
            let then_block = self.wrap_in_block(match_id, body, scope, tree, types)?;
            let if_expr = self.build_case_if(
                match_id,
                condition,
                then_block,
                value,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        // handle unbounded ranges
        if guard.is_none() {
            let body = self.wrap_in_block(match_id, body, scope, tree, types)?;
            return Ok(Some(body));
        }

        // handle guard only ranges
        let then_block = self.wrap_in_block(match_id, body, scope, tree, types)?;
        let if_expr = self.build_case_if(
            match_id,
            guard.unwrap(),
            then_block,
            value,
            cases,
            index,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle a union pattern by ORing the individual checks.
    #[allow(clippy::too_many_arguments)]
    fn handle_union_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        patterns: &[LocalNodeId<Pattern>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // build the union check
        let condition = self.build_union_check(match_id, value, patterns, tree, scope, types)?;

        // combine the union check with the guard
        let condition = self.combine_with_guard(match_id, condition, guard, tree, scope, types)?;

        // wrap the body for the then branch
        let then_block = self.wrap_in_block(match_id, body, scope, tree, types)?;

        // build the if expression
        let if_expr = self.build_case_if(
            match_id,
            condition,
            then_block,
            value,
            cases,
            index,
            tree,
            scope,
            types,
            match_type_id,
        )?;

        Ok(Some(if_expr))
    }

    /// Handle an array pattern by using index access bindings.
    #[allow(clippy::too_many_arguments)]
    fn handle_array_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        guard: Option<LocalNodeId<Expression>>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
        match_type_id: LocalTypeId,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body with array bindings
        let body_with_bindings =
            self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope, types)?;

        // handle guard if present
        if let Some(guard_expr) = guard {
            let if_expr = self.build_case_if(
                match_id,
                guard_expr,
                body_with_bindings,
                value,
                cases,
                index,
                tree,
                scope,
                types,
                match_type_id,
            )?;
            return Ok(Some(if_expr));
        }

        Ok(Some(body_with_bindings))
    }

    /// Handle unimplemented patterns by returning the body in a block.
    fn handle_fallback_pattern(
        &self,
        match_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // wrap the body in a block expression
        let body = self.wrap_in_block(match_id, body, scope, tree, types)?;

        Ok(Some(body))
    }

    /// Build a range check: value >= start && value <= end (or < for exclusive).
    fn build_range_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        start: Option<Pattern>,
        end: Option<Pattern>,
        is_inclusive: bool,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> Option<LocalNodeId<Expression>> {
        // build the start bound check
        let start_check = start.and_then(|p| {
            if let Pattern::Expression { value: start_val } = p {
                // value >= start
                let cmp_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let expr_id = tree.insert(
                    cmp_id,
                    Expression::Binary {
                        left: value,
                        operator: destack_dir::BinaryOperator::GreaterThanOrEqual,
                        right: start_val,
                    },
                );
                self.set_boolean_expression_type(types, tree.module_id, expr_id);
                Some(expr_id)
            } else {
                None
            }
        });

        // build the end bound check
        let end_check = end.and_then(|p| {
            if let Pattern::Expression { value: end_val } = p {
                // value <= end, or value < end for exclusive ranges
                let op = if is_inclusive {
                    destack_dir::BinaryOperator::LessThanOrEqual
                } else {
                    destack_dir::BinaryOperator::LessThan
                };
                let cmp_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let expr_id = tree.insert(
                    cmp_id,
                    Expression::Binary {
                        left: value,
                        operator: op,
                        right: end_val,
                    },
                );
                self.set_boolean_expression_type(types, tree.module_id, expr_id);
                Some(expr_id)
            } else {
                None
            }
        });

        // combine the bound checks
        match (start_check, end_check) {
            (Some(start), Some(end)) => {
                // combine with and
                let and_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let expr_id = tree.insert(
                    and_id,
                    Expression::Binary {
                        left: start,
                        operator: destack_dir::BinaryOperator::And,
                        right: end,
                    },
                );
                self.set_boolean_expression_type(types, tree.module_id, expr_id);
                Some(expr_id)
            }
            (Some(check), None) | (None, Some(check)) => Some(check),
            (None, None) => None,
        }
    }

    /// Build a union check: matches any of the patterns.
    fn build_union_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        patterns: &[LocalNodeId<Pattern>],
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect checks for each union pattern
        let mut checks: Vec<LocalNodeId<Expression>> = Vec::new();

        // build a check for each pattern
        for pattern_id in patterns {
            let pattern = tree.get(*pattern_id).clone();

            let check = match pattern {
                Pattern::Expression { value: pat_val } => {
                    // equality check
                    self.build_equality_check(match_id, value, pat_val, tree, scope, types)?
                }
                Pattern::TaggedTuple { ty, .. } | Pattern::TaggedObject { ty, .. } => {
                    // type check
                    self.build_type_check(match_id, value, ty, tree, scope, types)?
                }
                Pattern::Wildcard => {
                    // always matches, emit true literal
                    let lit_id =
                        tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                    let expr_id = tree.insert(
                        lit_id,
                        Expression::ScalarLiteral {
                            value: ScalarLiteral::Boolean(true),
                        },
                    );
                    self.set_scalar_literal_type(
                        types,
                        tree.module_id,
                        expr_id,
                        ScalarLiteral::Boolean(true),
                    );
                    expr_id
                }
                _ => {
                    // #Incomplete: other pattern types in union
                    let lit_id =
                        tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                    let expr_id = tree.insert(
                        lit_id,
                        Expression::ScalarLiteral {
                            value: ScalarLiteral::Boolean(true),
                        },
                    );
                    self.set_scalar_literal_type(
                        types,
                        tree.module_id,
                        expr_id,
                        ScalarLiteral::Boolean(true),
                    );
                    expr_id
                }
            };

            checks.push(check);
        }

        // combine all checks with or
        if checks.is_empty() {
            // empty union: emit false, should not happen in practice
            let lit_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let expr_id = tree.insert(
                lit_id,
                Expression::ScalarLiteral {
                    value: ScalarLiteral::Boolean(false),
                },
            );
            self.set_scalar_literal_type(
                types,
                tree.module_id,
                expr_id,
                ScalarLiteral::Boolean(false),
            );
            return Ok(expr_id);
        }

        // fold the checks into a single disjunction
        let mut result = checks[0];
        for check in checks.into_iter().skip(1) {
            // build the or node for the next check
            let or_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            result = tree.insert(
                or_id,
                Expression::Binary {
                    left: result,
                    operator: destack_dir::BinaryOperator::Or,
                    right: check,
                },
            );
            self.set_boolean_expression_type(types, tree.module_id, result);
        }

        Ok(result)
    }

    /// Build equality check expression: `value == pattern_value`.
    fn build_equality_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        pattern_value: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // reserve the equality node
        let eq_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);

        // insert the equality expression
        let expr_id = tree.insert(
            eq_id,
            Expression::Binary {
                left: value,
                operator: destack_dir::BinaryOperator::Equal,
                right: pattern_value,
            },
        );

        // assign the boolean result type
        self.set_boolean_expression_type(types, tree.module_id, expr_id);
        Ok(expr_id)
    }

    /// Build type check expression: `value is Type`.
    fn build_type_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        ty: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // reserve the type check node
        let is_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);

        // insert the type check expression
        let expr_id = tree.insert(
            is_id,
            Expression::TypeBinary {
                left: value,
                operator: TypeBinaryOperator::Is,
                right: ty,
            },
        );

        // assign the boolean result type
        self.set_boolean_expression_type(types, tree.module_id, expr_id);
        Ok(expr_id)
    }

    /// Combine a condition with an optional guard using `&&`.
    fn combine_with_guard(
        &self,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // merge the guard into the condition when present
        match guard {
            Some(guard_expression) => {
                // allocate the and node
                let and_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);

                // insert the and expression
                let expr_id = tree.insert(
                    and_id,
                    Expression::Binary {
                        left: condition,
                        operator: destack_dir::BinaryOperator::And,
                        right: guard_expression,
                    },
                );

                // assign the boolean result type
                self.set_boolean_expression_type(types, tree.module_id, expr_id);
                Ok(expr_id)
            }
            None => Ok(condition),
        }
    }

    /// Build an index access expression: `value[index]`.
    fn build_index_access(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // create index literal
        let index_lit_id =
            tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        let index_expr: LocalNodeId<Expression> = tree.insert(
            index_lit_id,
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(index as i64),
            },
        );
        self.set_scalar_literal_type(
            types,
            tree.module_id,
            index_expr,
            ScalarLiteral::Integer(index as i64),
        );

        // create index expression: value[index]
        let idx_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        let expr_id = tree.insert(
            idx_id,
            Expression::Index {
                left: value,
                right: Some(index_expr),
            },
        );

        // resolve the element type
        let element_type_id = self
            .index_access_type_id(types, tree.module_id, value, index)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: expr_id.into_global_any(tree.module_id).into_anchored(None),
            })?;

        // assign the element type
        self.set_expression_type(types, tree.module_id, expr_id, element_type_id);
        Ok(expr_id)
    }

    /// Build a member access expression: `value.name`.
    fn build_member_access(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        name: StringId,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // reserve the member access node
        let member_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);

        // insert the member access expression
        let expr_id = tree.insert(
            member_id,
            Expression::Member {
                left: value,
                name,
                static_arguments: None,
            },
        );

        // resolve the field type
        let field_type_id = self
            .member_access_type_id(types, tree.module_id, value, name)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: expr_id.into_global_any(tree.module_id).into_anchored(None),
            })?;

        // assign the field type
        self.set_expression_type(types, tree.module_id, expr_id, field_type_id);
        Ok(expr_id)
    }

    /// Wrap body with let bindings for tuple field extractions.
    ///
    /// For each field in the pattern, creates `let <binding> = value[i]`.
    /// Returns a block containing the bindings followed by the body.
    fn wrap_with_tuple_bindings(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        body: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect bindings to emit
        let mut bindings = Vec::new();

        // walk each field in order
        for (i, field_id) in fields.iter().enumerate() {
            let field = tree.get(*field_id).clone();
            match field {
                PatternField::Positional { pattern } => {
                    // check if the pattern introduces a binding
                    let pat = tree.get(pattern).clone();
                    if let Pattern::Binding {
                        name,
                        symbol,
                        mutability,
                        ..
                    } = pat
                    {
                        let access =
                            self.build_index_access(match_id, value, i, tree, scope, types)?;
                        bindings.push((name, symbol, mutability, access));
                    }
                    // #Incomplete: handle nested patterns in positional fields
                }
                PatternField::Named {
                    name,
                    symbol,
                    mutability,
                    ..
                } => {
                    // named field in tuple position, use index access
                    let access = self.build_index_access(match_id, value, i, tree, scope, types)?;
                    bindings.push((name, symbol, mutability, access));
                }
                PatternField::Spread { .. }
                | PatternField::Alias { .. }
                | PatternField::Computed { .. }
                | PatternField::Elision => {
                    // #Incomplete: handle spread, alias, computed, elision in tuple patterns
                }
            }
        }

        self.wrap_with_let_bindings(match_id, bindings, body, tree, scope, types)
    }

    /// Wrap body with let bindings for object field extractions.
    ///
    /// For each field in the pattern, creates `let <binding> = value.<field>`.
    /// Returns a block containing the bindings followed by the body.
    fn wrap_with_object_bindings(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        body: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // collect bindings to emit
        let mut bindings = Vec::new();

        // walk each field in order
        for field_id in fields.iter() {
            let field = tree.get(*field_id).clone();
            match field {
                PatternField::Named {
                    name,
                    symbol,
                    mutability,
                    ..
                } => {
                    let access =
                        self.build_member_access(match_id, value, name, tree, scope, types)?;
                    bindings.push((name, symbol, mutability, access));
                }
                PatternField::Alias {
                    name,
                    alias,
                    symbol,
                    mutability,
                    ..
                } => {
                    // `field: binding`, access by field name, bind to alias
                    let access =
                        self.build_member_access(match_id, value, name, tree, scope, types)?;
                    bindings.push((alias, symbol, mutability, access));
                }
                PatternField::Positional { .. }
                | PatternField::Spread { .. }
                | PatternField::Computed { .. }
                | PatternField::Elision => {
                    // #Incomplete: handle positional, spread, computed, elision in object patterns
                }
            }
        }

        self.wrap_with_let_bindings(match_id, bindings, body, tree, scope, types)
    }

    /// Wrap body in a block with the given let bindings.
    ///
    /// Creates a block containing:
    /// 1. Let expressions for each binding (e.g., `let x = value[0];`)
    /// 2. The original body expression
    fn wrap_with_let_bindings(
        &self,
        match_id: LocalNodeId<Expression>,
        bindings: Vec<(
            StringId,
            LocalSymbolId,
            Option<Mutability>,
            LocalNodeId<Expression>,
        )>,
        body: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // if no bindings, just wrap body in a block
        if bindings.is_empty() {
            return self.wrap_in_block(match_id, body, scope, tree, types);
        }

        // build let expressions for each binding
        let mut expressions: Vec<LocalNodeId<Expression>> = Vec::new();

        for (name, symbol, mutability, value) in bindings {
            // create Pattern::Binding for the declarator
            let pattern_id = tree.reserve_from(NodeType::Pattern, match_id.into_any(), scope, None);
            let pattern: LocalNodeId<Pattern> = tree.insert(
                pattern_id,
                Pattern::Binding {
                    mutability,
                    name,
                    pattern: None,
                    symbol,
                },
            );

            // create Declarator with the pattern and value
            let declarator_id =
                tree.reserve_from(NodeType::Declarator, match_id.into_any(), scope, None);
            let declarator: LocalNodeId<Declarator> = tree.insert(
                declarator_id,
                Declarator {
                    pattern,
                    ty: None,
                    value: Some(value),
                },
            );

            // create DeclarationDescriptor for the let
            let descriptor = DeclarationDescriptor {
                kind: DeclarationKind::Definition,
                abstraction: DeclarationAbstraction::Concrete,
                anchor: BindingAnchor::Instance,
                name: Some(Name::Identifier(name)),
                export: None,
                symbol,
            };

            // create Expression::Let
            let let_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let let_expr: LocalNodeId<Expression> = tree.insert(
                let_id,
                Expression::Let {
                    descriptor,
                    mutability: mutability.unwrap_or(Mutability::Immutable),
                    declarators: vec![declarator],
                },
            );

            // wrap in statement
            let stmt_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let stmt: LocalNodeId<Expression> = tree.insert(
                stmt_id,
                Expression::Statement {
                    statement: let_expr,
                },
            );

            expressions.push(stmt);
        }

        // add body as final expression
        expressions.push(body);

        // create Block containing all expressions
        let block_id = tree.reserve_from(NodeType::Block, match_id.into_any(), scope, None);
        let block: LocalNodeId<Block> = tree.insert(
            block_id,
            Block {
                scope: scope.0,
                expressions,
            },
        );

        // wrap in Expression::Block
        let block_expr_id =
            tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        let expr_id = tree.insert(block_expr_id, Expression::Block { block });
        let body_type_id = types
            .get_declared_or_inferred_type_id(body.into_global_any(tree.module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: body.into_global_any(tree.module_id).into_anchored(None),
            })?;
        types.set_inferred_type(block.into_global_any(tree.module_id), body_type_id);
        self.set_expression_type(types, tree.module_id, expr_id, body_type_id);
        Ok(expr_id)
    }

    /// Wrap an expression in a block with the given scope.
    /// Returns an Expression::Block containing the expression.
    /// If the expression is already a block, returns it unchanged.
    fn wrap_in_block(
        &self,
        origin_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        tree: &mut NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // don't double wrap if already a block
        if let Expression::Block { block } = tree.get(body) {
            let body_type_id =
                types.get_declared_or_inferred_type_id(body.into_global_any(tree.module_id));
            if let Some(_body_type_id) = body_type_id {
                return Ok(body);
            }

            let block_type_id = types
                .get_declared_or_inferred_type_id(block.into_global_any(tree.module_id))
                .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                    node: block.into_global_any(tree.module_id).into_anchored(None),
                })?;
            self.set_expression_type(types, tree.module_id, body, block_type_id);

            return Ok(body);
        }

        // create the Block node
        let block_id = tree.reserve_from(NodeType::Block, origin_id.into_any(), scope, None);
        let block: LocalNodeId<Block> = tree.insert(
            block_id,
            Block {
                scope: scope.0,
                expressions: vec![body],
            },
        );

        // create the Expression::Block wrapper
        let block_expr_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let expr_id = tree.insert(block_expr_id, Expression::Block { block });
        let body_type_id = types
            .get_declared_or_inferred_type_id(body.into_global_any(tree.module_id))
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: body.into_global_any(tree.module_id).into_anchored(None),
            })?;
        types.set_inferred_type(block.into_global_any(tree.module_id), body_type_id);
        self.set_expression_type(types, tree.module_id, expr_id, body_type_id);
        Ok(expr_id)
    }

    /// Wrap an else branch, but don't wrap if it's an If expression.
    /// This keeps else if chains flat.
    fn wrap_else_branch(
        &self,
        origin_id: LocalNodeId<Expression>,
        else_expr: Option<LocalNodeId<Expression>>,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        tree: &mut NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        let else_expr = else_expr.map(|e| {
            // don't wrap if it's an If expression, creates flat else if chain
            if matches!(tree.get(e), Expression::If { .. }) {
                Ok(e)
            } else {
                self.wrap_in_block(origin_id, e, scope, tree, types)
            }
        });
        else_expr.transpose()
    }

    /// Resolve the inferred type for a synthesized index access.
    fn index_access_type_id(
        &self,
        types: &TypeTable,
        module_id: ModuleId,
        value: LocalNodeId<Expression>,
        index: usize,
    ) -> Option<LocalTypeId> {
        let value_type_id =
            types.get_declared_or_inferred_type_id(value.into_global_any(module_id))?;
        let value_type_id = types.unwrap_value_type_id(value_type_id);
        types.get_index_access_type(value_type_id, index)
    }

    /// Resolve the inferred type for a synthesized member access.
    fn member_access_type_id(
        &self,
        types: &TypeTable,
        module_id: ModuleId,
        value: LocalNodeId<Expression>,
        name: StringId,
    ) -> Option<LocalTypeId> {
        let value_type_id =
            types.get_declared_or_inferred_type_id(value.into_global_any(module_id))?;
        let value_type_id = types.unwrap_value_type_id(value_type_id);
        types.get_member_access_type(value_type_id, name)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_match_literal_patterns() {
        // match on literal values transforms to nested if else with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): string {
    match (x) {
        1 => "one"
        2 => "two"
        _ => "other"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): string {
    if (x == 1) {
        return "one";
    } else if (x == 2) {
        return "two";
    } else {
        return "other";
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_boolean() {
        // match on boolean: exhaustive optimization skips last check
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(b: boolean): number {
    match (b) {
        true => 1
        false => 0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(b): number {
    if (b == true) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_with_guard() {
        // match with guard clause transforms to nested if else
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function classify(x: number): string {
    match (x) {
        n if n > 0 => "positive"
        n if n < 0 => "negative"
        _ => "zero"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function classify(x): string {
    if ({
        const n = x;
        n > 0 as number
    }) {
        const n = x;
        return "positive";
    } else if ({
        const n = x;
        n < 0 as number
    }) {
        const n = x;
        return "negative";
    } else {
        return "zero";
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_wildcard_only() {
        // match with only wildcard becomes the body in a block with explicit return
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function always(x: number): number {
    match (x) {
        _ => 42
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function always(x): number {
    return 42;
}
"#,
        );
    }

    #[test]
    fn test_transform_match_range_pattern() {
        // range patterns transform to nested if else
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function grade(score: number): string {
    match (score) {
        90..100 => "A"
        80..90 => "B"
        _ => "C"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function grade(score): string {
    if (score >= 90 && score < 100) {
        return 'A';
    } else if (score >= 80 && score < 90) {
        return 'B';
    } else {
        return 'C';
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_union_pattern() {
        // union patterns transform to OR checks with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function isWeekend(day: number): boolean {
    match (day) {
        0 | 6 => true
        _ => false
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function isWeekend(day): boolean {
    if (day == 0 || day == 6) {
        return true;
    } else {
        return false;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_to_if_else() {
        // match generates if else with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function sign(x: number): number {
    match (x) {
        0 => 0
        _ => 1
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function sign(x): number {
    if (x == 0) {
        return 0;
    } else {
        return 1;
    }
}
"#,
        );
    }
}
