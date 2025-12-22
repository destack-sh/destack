use destack_dir::{
    BinaryOperator, Expression, IfKind, LocalNodeId, MatchCase, MatchSelector, MatchSource,
    NodeTree, NodeType, Pattern, PatternField, ScalarLiteral, StringId, SymbolTable,
    TypeBinaryOperator, TypeTable,
};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

// nocheckin TODO #Incomplete: elaborate transform (if let, match, expressions as values)

impl Compiler {
    /// Transform a module with target-independent simplifications:
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

        // 1. if-let → if + binding
        self.transform_if_let(&mut tree, &symbols, &types)?;

        // 2. match → decision trees
        self.transform_match(&mut tree, &symbols, &types)?;

        // 3. expressions as values → temp + assignments
        self.transform_expression_as_value(&mut tree, &symbols, &types)?;

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
    fn transform_if_let(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // #Incomplete: transform if-let expressions
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
            let condition = self.build_equality_check(match_id, value, *pattern_value, tree, scope);
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

        // #Incomplete: remaining pattern types (Range, Union, Array, Maybe, etc.)
        Ok(Some(body))
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
    /// #Incomplete: Currently returns body unchanged.
    /// Proper let binding emission requires creating Expression::Let with
    /// DeclarationDescriptor, Declarator, and Pattern nodes. The pattern
    /// symbols from the original match arm need to be connected to the
    /// emitted let bindings. This is deferred for now; the type checks
    /// and field access logic are the priority.
    fn wrap_with_let_bindings(
        &self,
        _match_id: LocalNodeId<Expression>,
        _bindings: Vec<(
            StringId,
            destack_dir::LocalSymbolId,
            Option<destack_dir::Mutability>,
            LocalNodeId<Expression>,
        )>,
        body: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        // #Incomplete: emit let bindings for pattern destructuring
        // For now, return body unchanged - the pattern symbols are still
        // referenced in the body but won't have proper initialization.
        // Codegen will need to handle this or we need to emit proper Let nodes.
        body
    }

    /// Transform expressions used as values into temporaries and assignments.
    ///
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
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // #Incomplete: transform expressions as values
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

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
        // match on boolean transforms to if-else
        // NOTE #Performance: last case could skip the redundant check for exhaustive matches
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
    if (b == true) 1 else if (b == false) 0
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
        test.compile();
        test.check_clean();
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
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function always(x): number {
    42
}
"#,
        );
    }
}
