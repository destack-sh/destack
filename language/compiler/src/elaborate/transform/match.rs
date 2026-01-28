use destack_dir::{
    BindingAnchor, Block, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    Declarator, Expression, IfCondition, IfKind, LocalNodeId, LocalSymbolId, MatchCase, MatchKind,
    MatchSelector, MatchSource, Mutability, Name, NodeTree, NodeType, Pattern, PatternField,
    ScalarLiteral, StringId, TypeBinaryOperator,
};

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Transform `match` expressions into decision trees (if-else chains).
    ///
    /// ```ds
    /// match (x) {
    ///     Some(v) if v > 0 => positive(v)
    ///     Some(v) => nonPositive(v)
    ///     None => zero()
    /// }
    /// ```
    /// ->
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
        _types: &destack_dir::TypeTable,
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
            self.transform_single_match(match_id, tree)?;
        }

        Ok(())
    }

    /// Transform a single match expression into an if-else chain.
    fn transform_single_match(
        &self,
        match_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
    ) -> ElaborateResult<()> {
        let Expression::Match { value, cases, .. } = tree.get(match_id).clone() else {
            return Ok(());
        };
        if cases.is_empty() {
            return Ok(());
        }

        // build the if-else chain from the cases (in reverse order)
        let scope = tree.get_scope(match_id);
        let result = self.build_match_chain(match_id, value, &cases, 0, tree, scope)?;

        // replace the match with the generated if-else chain
        if let Some(replacement) = result {
            let replacement = tree.get(replacement).clone();
            tree.replace(match_id, replacement);
        }

        Ok(())
    }

    /// Build the if-else chain for match cases starting at the given index.
    fn build_match_chain(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        index: usize,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        if index >= cases.len() {
            return Ok(None);
        }

        let case = tree.get(cases[index]).clone();
        let (selector, body) = match &case {
            MatchCase::Expression { selector, body, .. } => (selector.clone(), *body),
            MatchCase::Block { selector, body, .. } => {
                // wrap block in a block expression
                let block_expr_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let block_expr: LocalNodeId<Expression> =
                    tree.insert(block_expr_id, Expression::Block { block: *body });
                (selector.clone(), block_expr)
            }
        };

        // default selector: return the body (it always matches)
        let MatchSelector::Pattern {
            pattern: pattern_id,
            guard,
        } = selector
        else {
            return Ok(Some(body));
        };

        let pattern = tree.get(pattern_id).clone();

        // wildcard: return the body directly (it always matches)
        if matches!(pattern, Pattern::Wildcard) && guard.is_none() {
            return Ok(Some(body));
        }

        // binding pattern without further nested pattern: introduces binding, always matches
        if let Pattern::Binding {
            pattern: None,
            name,
            symbol,
            mutability,
        } = &pattern
        {
            // bind the matched value before evaluating the body
            let bindings = vec![(*name, *symbol, *mutability, value)];

            if guard.is_none() {
                // binding patterns without guards always match
                let body = self.wrap_with_let_bindings(match_id, bindings, body, tree, scope);
                return Ok(Some(body));
            }

            // evaluate the guard with the binding in scope
            let guard_expr = guard.unwrap();
            let guard_condition =
                self.wrap_with_let_bindings(match_id, bindings.clone(), guard_expr, tree, scope);

            // evaluate the then branch with the binding in scope
            let then_body = self.wrap_with_let_bindings(match_id, bindings, body, tree, scope);

            // build the else chain from the remaining cases
            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            // wrap the else branch as needed
            let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

            // build the if expression for the guard
            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression {
                        condition: guard_condition,
                    },
                    then_expression: then_body,
                    else_expression: else_block,
                },
            );
            return Ok(Some(if_expr));
        }

        // expression patterns (literals): emit equality check
        if let Pattern::Expression {
            value: pattern_value,
        } = &pattern
        {
            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            // optimization: if this is the last case and no guard,
            // skip the condition check - the pattern must match if we reach here
            // (assuming exhaustive match, which is enforced by analysis)
            if else_expr.is_none() && guard.is_none() {
                return Ok(Some(self.wrap_in_block(match_id, body, scope, tree)));
            }

            let condition = self.build_equality_check(match_id, value, *pattern_value, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            // wrap then/else in blocks for proper structure
            let then_block = self.wrap_in_block(match_id, body, scope, tree);
            let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression { condition },
                    then_expression: then_block,
                    else_expression: else_block,
                },
            );
            return Ok(Some(if_expr));
        }

        // tagged tuple patterns: emit type check + index accesses
        // e.g., `Point(x, y)` → `if (value is Point) { let x = value[0]; let y = value[1]; body }`
        if let Pattern::TaggedTuple { ty, fields } = &pattern {
            let condition = self.build_type_check(match_id, value, *ty, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            // wrap body with field bindings (index accesses for tuple fields)
            // this already creates a block
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
            let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression { condition },
                    then_expression: body_with_bindings,
                    else_expression: else_block,
                },
            );
            return Ok(Some(if_expr));
        }

        // tagged object patterns: emit type check + member accesses
        // e.g., `Ok { value }` → `if (value is Ok) { let value = value.value; body }`
        if let Pattern::TaggedObject { ty, fields } = &pattern {
            let condition = self.build_type_check(match_id, value, *ty, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            // wrap body with field bindings (member accesses for object fields)
            let body_with_bindings =
                self.wrap_with_object_bindings(match_id, value, fields, body, tree, scope);

            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
            let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression { condition },
                    then_expression: body_with_bindings,
                    else_expression: else_block,
                },
            );
            return Ok(Some(if_expr));
        }

        // anonymous tuple patterns: no type check, just index accesses
        if let Pattern::Tuple { fields } = &pattern {
            // this already creates a block
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            // if there's a guard, wrap in an if
            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: IfCondition::Expression {
                            condition: guard_expr,
                        },
                        then_expression: body_with_bindings,
                        else_expression: else_block,
                    },
                );
                return Ok(Some(if_expr));
            }

            return Ok(Some(body_with_bindings));
        }

        // anonymous object patterns: no type check, just member accesses
        if let Pattern::Object { fields } = &pattern {
            // this already creates a block
            let body_with_bindings =
                self.wrap_with_object_bindings(match_id, value, fields, body, tree, scope);

            // if there's a guard, wrap in an if
            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: IfCondition::Expression {
                            condition: guard_expr,
                        },
                        then_expression: body_with_bindings,
                        else_expression: else_block,
                    },
                );
                return Ok(Some(if_expr));
            }

            return Ok(Some(body_with_bindings));
        }

        // range patterns: emit range check (value >= start && value <= end)
        if let Pattern::Range {
            start,
            end,
            is_inclusive,
        } = &pattern
        {
            let condition = self.build_range_check(
                match_id,
                value,
                start.as_ref().map(|p| tree.get(*p).clone()),
                end.as_ref().map(|p| tree.get(*p).clone()),
                *is_inclusive,
                tree,
                scope,
            );

            if let Some(condition) = condition {
                let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

                let then_block = self.wrap_in_block(match_id, body, scope, tree);
                let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: IfCondition::Expression { condition },
                        then_expression: then_block,
                        else_expression: else_block,
                    },
                );
                return Ok(Some(if_expr));
            } else {
                // range with no bounds matches everything
                if guard.is_none() {
                    return Ok(Some(self.wrap_in_block(match_id, body, scope, tree)));
                }
                // guard only
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let then_block = self.wrap_in_block(match_id, body, scope, tree);
                let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: IfCondition::Expression {
                            condition: guard.unwrap(),
                        },
                        then_expression: then_block,
                        else_expression: else_block,
                    },
                );
                return Ok(Some(if_expr));
            }
        }

        // union patterns: emit OR check for each alternative
        if let Pattern::Union { patterns } = &pattern {
            let condition = self.build_union_check(match_id, value, patterns, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            let then_block = self.wrap_in_block(match_id, body, scope, tree);
            let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression { condition },
                    then_expression: then_block,
                    else_expression: else_block,
                },
            );
            return Ok(Some(if_expr));
        }

        // array patterns: similar to tuple, use index access
        if let Pattern::Array { fields } = &pattern {
            // this already creates a block
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let else_block = self.wrap_else_branch(match_id, else_expr, scope, tree);
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: IfCondition::Expression {
                            condition: guard_expr,
                        },
                        then_expression: body_with_bindings,
                        else_expression: else_block,
                    },
                );
                return Ok(Some(if_expr));
            }

            return Ok(Some(body_with_bindings));
        }

        // must patterns: check if value is non-nullish, then unwrap
        // #Incomplete: must pattern needs proper Result-like type handling

        // reference/value patterns: unwrap and match inner
        // #Incomplete: ReferenceOf and ValueOf patterns

        // fallback: return body wrapped in block (unhandled pattern type)
        Ok(Some(self.wrap_in_block(match_id, body, scope, tree)))
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
    ) -> Option<LocalNodeId<Expression>> {
        let start_check = start.and_then(|p| {
            if let Pattern::Expression { value: start_val } = p {
                // value >= start
                let cmp_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                Some(tree.insert(
                    cmp_id,
                    Expression::Binary {
                        left: value,
                        operator: destack_dir::BinaryOperator::GreaterThanOrEqual,
                        right: start_val,
                    },
                ))
            } else {
                None
            }
        });

        let end_check = end.and_then(|p| {
            if let Pattern::Expression { value: end_val } = p {
                // value <= end (inclusive) or value < end (exclusive)
                let op = if is_inclusive {
                    destack_dir::BinaryOperator::LessThanOrEqual
                } else {
                    destack_dir::BinaryOperator::LessThan
                };
                let cmp_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                Some(tree.insert(
                    cmp_id,
                    Expression::Binary {
                        left: value,
                        operator: op,
                        right: end_val,
                    },
                ))
            } else {
                None
            }
        });

        match (start_check, end_check) {
            (Some(start), Some(end)) => {
                // combine with &&
                let and_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                Some(tree.insert(
                    and_id,
                    Expression::Binary {
                        left: start,
                        operator: destack_dir::BinaryOperator::And,
                        right: end,
                    },
                ))
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
    ) -> LocalNodeId<Expression> {
        let mut checks: Vec<LocalNodeId<Expression>> = Vec::new();

        for pattern_id in patterns {
            let pattern = tree.get(*pattern_id).clone();

            let check = match pattern {
                Pattern::Expression { value: pat_val } => {
                    // equality check
                    self.build_equality_check(match_id, value, pat_val, tree, scope)
                }
                Pattern::TaggedTuple { ty, .. } | Pattern::TaggedObject { ty, .. } => {
                    // type check
                    self.build_type_check(match_id, value, ty, tree, scope)
                }
                Pattern::Wildcard => {
                    // always matches, emit true literal
                    let lit_id =
                        tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                    tree.insert(
                        lit_id,
                        Expression::ScalarLiteral {
                            value: ScalarLiteral::Boolean(true),
                        },
                    )
                }
                _ => {
                    // #Incomplete: other pattern types in union
                    let lit_id =
                        tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                    tree.insert(
                        lit_id,
                        Expression::ScalarLiteral {
                            value: ScalarLiteral::Boolean(true),
                        },
                    )
                }
            };

            checks.push(check);
        }

        // combine all checks with ||
        if checks.is_empty() {
            // empty union: emit false (shouldn't happen in practice)
            let lit_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            return tree.insert(
                lit_id,
                Expression::ScalarLiteral {
                    value: ScalarLiteral::Boolean(false),
                },
            );
        }

        let mut result = checks[0];
        for check in checks.into_iter().skip(1) {
            let or_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            result = tree.insert(
                or_id,
                Expression::Binary {
                    left: result,
                    operator: destack_dir::BinaryOperator::Or,
                    right: check,
                },
            );
        }

        result
    }

    /// Build equality check expression: `value == pattern_value`.
    fn build_equality_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        pattern_value: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        let eq_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        tree.insert(
            eq_id,
            Expression::Binary {
                left: value,
                operator: destack_dir::BinaryOperator::Equal,
                right: pattern_value,
            },
        )
    }

    /// Build type check expression: `value is Type`.
    fn build_type_check(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        ty: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        let is_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        tree.insert(
            is_id,
            Expression::TypeBinary {
                left: value,
                operator: TypeBinaryOperator::Is,
                right: ty,
            },
        )
    }

    /// Combine a condition with an optional guard using `&&`.
    fn combine_with_guard(
        &self,
        match_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        guard: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        match guard {
            Some(guard_expression) => {
                let and_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                tree.insert(
                    and_id,
                    Expression::Binary {
                        left: condition,
                        operator: destack_dir::BinaryOperator::And,
                        right: guard_expression,
                    },
                )
            }
            None => condition,
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
    ) -> LocalNodeId<Expression> {
        // create index literal
        let index_lit_id =
            tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        let index_expr: LocalNodeId<Expression> = tree.insert(
            index_lit_id,
            Expression::ScalarLiteral {
                value: ScalarLiteral::Integer(index as i64),
            },
        );

        // create index expression: value[index]
        let idx_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        tree.insert(
            idx_id,
            Expression::Index {
                left: value,
                right: Some(index_expr),
            },
        )
    }

    /// Build a member access expression: `value.name`.
    fn build_member_access(
        &self,
        match_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        name: StringId,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        let member_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
        tree.insert(
            member_id,
            Expression::Member {
                left: value,
                name,
                static_arguments: None,
            },
        )
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
    ) -> LocalNodeId<Expression> {
        // collect bindings to emit
        let mut bindings = Vec::new();

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
                        let access = self.build_index_access(match_id, value, i, tree, scope);
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
                    // named field in tuple position - use index access
                    let access = self.build_index_access(match_id, value, i, tree, scope);
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

        self.wrap_with_let_bindings(match_id, bindings, body, tree, scope)
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
    ) -> LocalNodeId<Expression> {
        // collect bindings to emit
        let mut bindings = Vec::new();

        for field_id in fields.iter() {
            let field = tree.get(*field_id).clone();
            match field {
                PatternField::Named {
                    name,
                    symbol,
                    mutability,
                    ..
                } => {
                    let access = self.build_member_access(match_id, value, name, tree, scope);
                    bindings.push((name, symbol, mutability, access));
                }
                PatternField::Alias {
                    name,
                    alias,
                    symbol,
                    mutability,
                    ..
                } => {
                    // `field: binding` - access by field name, bind to alias
                    let access = self.build_member_access(match_id, value, name, tree, scope);
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

        self.wrap_with_let_bindings(match_id, bindings, body, tree, scope)
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
    ) -> LocalNodeId<Expression> {
        // if no bindings, just wrap body in a block
        if bindings.is_empty() {
            return self.wrap_in_block(match_id, body, scope, tree);
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
        tree.insert(block_expr_id, Expression::Block { block })
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
    ) -> LocalNodeId<Expression> {
        // don't double-wrap if already a block
        if matches!(tree.get(body), Expression::Block { .. }) {
            return body;
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
        tree.insert(block_expr_id, Expression::Block { block })
    }

    /// Wrap an else branch, but don't wrap if it's an If expression (creates else-if chains).
    fn wrap_else_branch(
        &self,
        origin_id: LocalNodeId<Expression>,
        else_expr: Option<LocalNodeId<Expression>>,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        tree: &mut NodeTree,
    ) -> Option<LocalNodeId<Expression>> {
        else_expr.map(|e| {
            // don't wrap if it's an If expression (creates flat else-if chain)
            if matches!(tree.get(e), Expression::If { .. }) {
                e
            } else {
                self.wrap_in_block(origin_id, e, scope, tree)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_match_literal_patterns() {
        // match on literal values transforms to nested if-else with proper blocks
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
        // match with guard clause transforms to nested if-else
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
        // range patterns transform to nested if-else
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
        // match generates if-else with proper blocks
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
