use destack_dir::{
    BinaryOperator, BindingAnchor, Block, DeclarationAbstraction, DeclarationDescriptor,
    DeclarationKind, Declarator, Expression, IfKind, LocalNodeId, LocalSymbolId, MatchCase,
    MatchSelector, MatchSource, Mutability, NodeTree, NodeType, Pattern, PatternField,
    ScalarLiteral, StringId, SymbolTable, TypeBinaryOperator, TypeTable,
};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform a module with target-independent simplifications:
    /// 0. Split multi-declarator lets into individual lets
    /// 1. `if let` → if + explicit binding
    /// 2. `match` → decision trees (if-else chains)
    /// 3. Expressions as values → temp + assignments
    pub(crate) fn elaborate_module_transform(&self, module_id: ModuleId) -> ElaborateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let mut tree = dir.tree.write();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // 0. split multi-declarators into individual lets
        if self.options.elaborate_split_declarators {
            self.transform_split_declarators(&mut tree)?;
        }

        // 1. if-let → if + binding
        self.transform_if_let(&mut tree, &symbols, &types)?;

        // 2. match → decision trees
        self.transform_match(&mut tree, &symbols, &types)?;

        // 3. expressions as values → temp + assignments
        self.transform_expression_as_value(&mut tree, &symbols, &types)?;

        Ok(())
    }

    /// Split multi-declarator let statements into individual let statements.
    ///
    /// ```ds
    /// let a = 1, b = 2;
    /// ```
    /// ->
    /// ```ds
    /// let a = 1;
    /// let b = 2;
    /// ```
    fn transform_split_declarators(&self, tree: &mut NodeTree) -> ElaborateResult<()> {
        // collect all blocks that need transformation
        let block_ids: Vec<_> = tree.iter_node_ids_of_type::<Block>();

        for block_id in block_ids {
            self.split_declarators_in_block(block_id, tree)?;
        }

        Ok(())
    }

    /// Wrap an expression in a block with the given scope.
    /// Returns an Expression::Block containing the expression.
    fn wrap_in_block(
        &self,
        origin_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        tree: &mut NodeTree,
    ) -> LocalNodeId<Expression> {
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

    /// Split multi-declarators in a single block.
    fn split_declarators_in_block(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &mut NodeTree,
    ) -> ElaborateResult<()> {
        let block = tree.get(block_id).clone();
        let mut new_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut modified = false;

        for expr_id in &block.expressions {
            // check if this is a statement wrapping a let
            let (let_expr_id, is_statement) = match tree.get(*expr_id) {
                Expression::Statement { statement } => (*statement, true),
                Expression::Let { .. } => (*expr_id, false),
                _ => {
                    new_expressions.push(*expr_id);
                    continue;
                }
            };

            let Expression::Let {
                descriptor,
                mutability,
                declarators,
            } = tree.get(let_expr_id).clone()
            else {
                new_expressions.push(*expr_id);
                continue;
            };

            // only split if there are multiple declarators
            if declarators.len() <= 1 {
                new_expressions.push(*expr_id);
                continue;
            }

            modified = true;
            let scope = tree.get_scope(let_expr_id);

            // create individual let for each declarator
            for declarator_id in declarators {
                let new_let_id =
                    tree.reserve_from(NodeType::Expression, let_expr_id.into_any(), scope, None);

                // create a new descriptor for this let (reusing the symbol from the declarator's pattern)
                let declarator = tree.get(declarator_id);
                let pattern = tree.get(declarator.pattern);
                let new_symbol = pattern.symbol().unwrap_or(descriptor.symbol);
                let new_descriptor = DeclarationDescriptor {
                    symbol: new_symbol,
                    ..descriptor.clone()
                };

                let new_let: LocalNodeId<Expression> = tree.insert(
                    new_let_id,
                    Expression::Let {
                        descriptor: new_descriptor,
                        mutability,
                        declarators: vec![declarator_id],
                    },
                );

                // wrap in statement if original was wrapped
                let final_expr = if is_statement {
                    let stmt_id =
                        tree.reserve_from(NodeType::Expression, let_expr_id.into_any(), scope, None);
                    tree.insert(stmt_id, Expression::Statement { statement: new_let })
                } else {
                    new_let
                };

                new_expressions.push(final_expr);
            }
        }

        // update block if modified
        if modified {
            let new_block = Block {
                scope: block.scope,
                expressions: new_expressions,
            };
            tree.replace(block_id, new_block);
        }

        Ok(())
    }

    /// Transform `if let` expressions into explicit if + binding.
    ///
    /// ```ds
    /// if let Some(x) = expr { body }
    /// ```
    /// ->
    /// ```ds
    /// { let __m = expr; if (__m is Some) { let x = __m.value; body } }
    /// ```
    ///
    /// NOTE: if-let syntax is parsed as an If expression with a Let expression
    /// as the condition. The transformation requires:
    /// 1. Detecting If nodes where condition is a Let with a refutable pattern
    /// 2. Extracting the pattern and the value expression
    /// 3. Creating the temp binding, type check, and inner bindings
    ///
    /// This is TBD pending full if-let support in the binder and analysis phases.
    fn transform_if_let(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // TODO: if-let transform - needs parser/binder support for if-let patterns
        // The condition of an if-let would be an Expression::Let with a refutable pattern
        // (e.g., x!, Result.ok(v), etc.)
        Ok(())
    }

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
    fn transform_match(
        &self,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        let match_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| {
                matches!(
                    tree.get(*id),
                    Expression::Match {
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

        // wildcard: return the body (it always matches)
        if matches!(pattern, Pattern::Wildcard) && guard.is_none() {
            return Ok(Some(body));
        }

        // binding pattern without further nested pattern: introduces binding, always matches
        if let Pattern::Binding { pattern: None, .. } = &pattern {
            if guard.is_none() {
                // binding with no guard: return the body
                // #Incomplete: need to emit the let binding for the variable
                return Ok(Some(body));
            }

            // binding with guard: guard becomes the condition
            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: guard.unwrap(),
                    then_expression: body,
                    else_expression: else_expr,
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

            // optimization: if this is the last case (else_expr is None) and no guard,
            // skip the condition check - the pattern must match if we reach here
            // (assuming exhaustive match, which is enforced by analysis)
            if else_expr.is_none() && guard.is_none() {
                return Ok(Some(body));
            }

            let condition = self.build_equality_check(match_id, value, *pattern_value, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition,
                    then_expression: body,
                    else_expression: else_expr,
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
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition,
                    then_expression: body_with_bindings,
                    else_expression: else_expr,
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

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition,
                    then_expression: body_with_bindings,
                    else_expression: else_expr,
                },
            );
            return Ok(Some(if_expr));
        }

        // anonymous tuple patterns: no type check, just index accesses
        if let Pattern::Tuple { fields } = &pattern {
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            // if there's a guard, wrap in an if
            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: guard_expr,
                        then_expression: body_with_bindings,
                        else_expression: else_expr,
                    },
                );
                return Ok(Some(if_expr));
            }

            return Ok(Some(body_with_bindings));
        }

        // anonymous object patterns: no type check, just member accesses
        if let Pattern::Object { fields } = &pattern {
            let body_with_bindings =
                self.wrap_with_object_bindings(match_id, value, fields, body, tree, scope);

            // if there's a guard, wrap in an if
            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: guard_expr,
                        then_expression: body_with_bindings,
                        else_expression: else_expr,
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

                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition,
                        then_expression: body,
                        else_expression: else_expr,
                    },
                );
                return Ok(Some(if_expr));
            } else {
                // range with no bounds matches everything
                if guard.is_none() {
                    return Ok(Some(body));
                }
                // guard only
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: guard.unwrap(),
                        then_expression: body,
                        else_expression: else_expr,
                    },
                );
                return Ok(Some(if_expr));
            }
        }

        // union patterns: emit OR check for each alternative
        if let Pattern::Union { patterns } = &pattern {
            let condition =
                self.build_union_check(match_id, value, patterns, tree, scope);
            let condition = self.combine_with_guard(match_id, condition, guard, tree, scope);

            let else_expr =
                self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;

            let if_id = tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let if_expr: LocalNodeId<Expression> = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition,
                    then_expression: body,
                    else_expression: else_expr,
                },
            );
            return Ok(Some(if_expr));
        }

        // array patterns: similar to tuple, use index access
        if let Pattern::Array { fields } = &pattern {
            let body_with_bindings =
                self.wrap_with_tuple_bindings(match_id, value, fields, body, tree, scope);

            if let Some(guard_expr) = guard {
                let else_expr =
                    self.build_match_chain(match_id, value, cases, index + 1, tree, scope)?;
                let if_id =
                    tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
                let if_expr: LocalNodeId<Expression> = tree.insert(
                    if_id,
                    Expression::If {
                        kind: IfKind::If,
                        condition: guard_expr,
                        then_expression: body_with_bindings,
                        else_expression: else_expr,
                    },
                );
                return Ok(Some(if_expr));
            }

            return Ok(Some(body_with_bindings));
        }

        // maybe patterns: check if value is some, then unwrap
        // #Incomplete: Maybe pattern needs proper Option/Result type handling

        // reference/value patterns: unwrap and match inner
        // #Incomplete: ReferenceOf and ValueOf patterns

        // fallback: return body (unhandled pattern type)
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
                        operator: BinaryOperator::GreaterThanOrEqual,
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
                    BinaryOperator::LessThanOrEqual
                } else {
                    BinaryOperator::LessThan
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
                        operator: BinaryOperator::And,
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
            let lit_id =
                tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
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
                    operator: BinaryOperator::Or,
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
                operator: BinaryOperator::Equal,
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
                        operator: BinaryOperator::And,
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
                | PatternField::Elision => {
                    // #Incomplete: handle spread/alias/elision in tuple patterns
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
                | PatternField::Elision => {
                    // #Incomplete: handle positional/spread/elision in object patterns
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
        bindings: Vec<(StringId, LocalSymbolId, Option<Mutability>, LocalNodeId<Expression>)>,
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
            let pattern_id =
                tree.reserve_from(NodeType::Pattern, match_id.into_any(), scope, None);
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
                name: Some(name),
                export: None,
                symbol,
            };

            // create Expression::Let
            let let_id =
                tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let let_expr: LocalNodeId<Expression> = tree.insert(
                let_id,
                Expression::Let {
                    descriptor,
                    mutability: mutability.unwrap_or(Mutability::Immutable),
                    declarators: vec![declarator],
                },
            );

            // wrap in statement
            let stmt_id =
                tree.reserve_from(NodeType::Expression, match_id.into_any(), scope, None);
            let stmt: LocalNodeId<Expression> =
                tree.insert(stmt_id, Expression::Statement { statement: let_expr });

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

    /// Transform expressions used as values into temporaries and assignments.
    /// Also optimizes simple if-else to ternary when enabled.
    ///
    /// For simple if-else (when elaborate_with_ternary is true):
    /// ```ds
    /// if (cond) a else b
    /// ```
    /// ->
    /// ```ds
    /// cond ? a : b
    /// ```
    ///
    /// For complex cases (future):
    /// ```ds
    /// const result = if (cond) { a } else { b };
    /// ```
    /// ->
    /// ```ds
    /// let result;
    /// if (cond) { result = a; } else { result = b; }
    /// ```
    fn transform_expression_as_value(
        &self,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // optimize simple if-else to ternary
        if self.options.elaborate_with_ternary {
            self.transform_if_to_ternary(tree)?;
        }

        // #Incomplete: complex expression-as-value transforms
        // (temp + assignment chains for complex if/match)

        Ok(())
    }

    /// Transform simple if-else expressions to ternary expressions.
    /// Only transforms if both branches are simple (non-block) expressions.
    fn transform_if_to_ternary(&self, tree: &mut NodeTree) -> ElaborateResult<()> {
        let if_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| {
                matches!(
                    tree.get(*id),
                    Expression::If {
                        kind: IfKind::If,
                        ..
                    }
                )
            })
            .collect();

        for if_id in if_ids {
            let Expression::If {
                kind: IfKind::If,
                condition,
                then_expression,
                else_expression: Some(else_expr),
            } = tree.get(if_id).clone()
            else {
                continue;
            };

            // check if both branches are simple (non-block, non-if)
            if !self.is_simple_expression(then_expression, tree) {
                continue;
            }
            if !self.is_simple_expression(else_expr, tree) {
                continue;
            }

            // transform to ternary
            tree.replace(
                if_id,
                Expression::If {
                    kind: IfKind::Ternary,
                    condition,
                    then_expression,
                    else_expression: Some(else_expr),
                },
            );
        }

        Ok(())
    }

    /// Check if an expression is "simple" enough for ternary optimization.
    /// Returns true for literals, references, and simple expressions.
    /// Returns false for blocks, if statements, match, loops, etc.
    fn is_simple_expression(&self, expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
        match tree.get(expr_id) {
            // simple expressions
            Expression::ScalarLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::LocalReference { .. }
            | Expression::ModuleReference { .. }
            | Expression::GlobalReference { .. }
            | Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::ObjectExpression { .. }
            | Expression::TaggedScalarExpression { .. }
            | Expression::TaggedTupleExpression { .. }
            | Expression::TaggedObjectExpression { .. }
            | Expression::Parenthesized { .. }
            | Expression::TemplateExpression { .. } => true,

            // ternary is ok if nested ternaries are ok
            Expression::If {
                kind: IfKind::Ternary,
                ..
            } => true,

            // not simple
            Expression::Block { .. }
            | Expression::If {
                kind: IfKind::If, ..
            }
            | Expression::Match { .. }
            | Expression::Loop { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Try { .. }
            | Expression::Statement { .. }
            | Expression::Let { .. } => false,

            // other expressions: be conservative
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_split_declarators() {
        // multi-declarator let statements are split into individual lets
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1, b = 2, c = 3;
    a + b + c
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    let c = 3;
    a + b + c
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_with_types() {
        // split declarators with type annotations (types are inferred after analysis)
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a: number = 1, b: number = 2;
    a + b
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    a + b
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_single_unchanged() {
        // single declarator let statements are unchanged
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1;
    a
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    a
}
"#,
        );
    }

    #[test]
    fn test_transform_match_literal_patterns() {
        // match on literal values transforms to if-else chain
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
    if (x == 1) "one" else if (x == 2) "two" else "other"
}
"#,
        );
    }

    #[test]
    fn test_transform_match_boolean() {
        // match on boolean transforms to ternary (last case optimized away)
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
        // optimization: last case skips check (exhaustive), then if-else becomes ternary
        test.assert_elaborated(
            module_id,
            r#"
function check(b): number {
    b == true ? 1 : 0
}
"#,
        );
    }

    #[test]
    fn test_transform_match_with_guard() {
        // match with guard clause transforms to nested if
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
    if (n > 0) "positive" else if (n < 0) "negative" else "zero"
}
"#,
        );
    }

    #[test]
    fn test_transform_match_wildcard_only() {
        // match with only wildcard becomes the body directly
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
    42
}
"#,
        );
    }

    #[test]
    fn test_transform_match_range_pattern() {
        // range patterns transform to comparison checks
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
    if (score >= 90 && score < 100) 'A' else if (score >= 80 && score < 90) 'B' else 'C'
}
"#,
        );
    }

    #[test]
    fn test_transform_match_union_pattern() {
        // union patterns transform to OR checks, then to ternary
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
    day == 0 || day == 6 ? true : false
}
"#,
        );
    }

    #[test]
    fn test_transform_if_else_with_blocks() {
        // if-else with block branches stays as if-else (not converted to ternary)
        // because the parser wraps branches in blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function abs(x: number): number {
    if (x < 0) -x else x
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function abs(x): number {
    if (x < 0) { -x } else { x }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_to_ternary() {
        // match with simple branches becomes ternary because match generates
        // simple expressions (not blocks) for the branches
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
    x == 0 ? 0 : 1
}
"#,
        );
    }
}
