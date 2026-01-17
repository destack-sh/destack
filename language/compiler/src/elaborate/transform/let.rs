use destack_dir::{
    Block, Expression, IfCondition, LocalNodeId, LocalScopeId, MatchCase, MatchKind, MatchSelector,
    MatchSource, NodeTree, NodeType, Pattern, SymbolTable, TypeTable,
};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Normalize if let expressions into match expressions.
    pub(super) fn transform_if_let(
        &self,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // collect if let expressions to transform
        let if_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_node_active(tree, symbols, id.into_any()))
            .filter(|id| {
                matches!(
                    tree.get(*id),
                    Expression::If {
                        condition: IfCondition::Let { .. },
                        ..
                    }
                )
            })
            .collect();

        // transform each if let into a match with a default case
        for if_id in if_ids {
            // read the if expression
            let Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } = tree.get(if_id).clone()
            else {
                continue;
            };

            // read the if let condition
            let IfCondition::Let { declarator, .. } = condition else {
                continue;
            };

            // read the declarator and matched value
            let declarator = tree.get(declarator).clone();
            let Some(value_id) = declarator.value else {
                continue;
            };

            // resolve the match metadata for the new expression
            let match_scope = tree.get_scope(if_id);
            let match_symbol = self.match_symbol_for_scope(if_id, match_scope.0, symbols)?;

            // build the then case from the pattern and then expression
            let then_case = self.insert_if_let_case(
                tree,
                if_id,
                declarator.pattern,
                then_expression,
                tree.get_scope(then_expression).0,
            );

            // build the else case with a wildcard pattern
            let else_expression = match else_expression {
                Some(else_expression) => else_expression,
                None => self.insert_empty_block_expression(tree, if_id, match_scope),
            };
            let else_pattern = self.insert_if_let_wildcard(tree, if_id, match_scope);
            let else_case = self.insert_if_let_case(
                tree,
                if_id,
                else_pattern,
                else_expression,
                tree.get_scope(else_expression).0,
            );

            // replace the if let with a match expression
            let match_expression = Expression::Match {
                kind: MatchKind::Match,
                value: value_id,
                cases: vec![then_case, else_case],
                source: MatchSource::Match,
                scope: match_scope.0,
                symbol: match_symbol,
            };
            tree.replace(if_id, match_expression);
        }

        Ok(())
    }

    /// Pick a match symbol from the current scope chain.
    fn match_symbol_for_scope(
        &self,
        if_id: LocalNodeId<Expression>,
        mut scope_id: LocalScopeId,
        symbols: &SymbolTable,
    ) -> ElaborateResult<destack_dir::LocalSymbolId> {
        // walk up scopes until we find an owner symbol
        loop {
            // use the owner symbol when it exists
            let scope = symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner_id {
                return Ok(owner_id);
            }

            // climb to the parent when possible
            if let Some((parent_id, _)) = scope.parent {
                scope_id = parent_id;
                continue;
            }

            // fall back to any symbol in the root scope
            if let Some((_, symbol_id)) = symbols.active_named_symbols(scope).next() {
                return Ok(symbol_id);
            }

            if let Some(symbol_id) = symbols.active_anonymous_symbols(scope).next() {
                return Ok(symbol_id);
            }

            // surface a graceful error when no symbol exists
            return Err(crate::ElaborateError::UnsupportedConstruct {
                node: if_id.into_global_any(symbols.module_id).into_anchored(None),
            });
        }
    }

    /// Insert a wildcard pattern node for the implicit else case.
    fn insert_if_let_wildcard(
        &self,
        tree: &mut NodeTree,
        if_id: LocalNodeId<Expression>,
        scope: (LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Pattern> {
        // allocate and insert the wildcard pattern
        let pattern_id = tree.reserve_from(NodeType::Pattern, if_id.into_any(), scope, None);
        tree.insert(pattern_id, Pattern::Wildcard)
    }

    /// Insert a match case for an if let branch.
    fn insert_if_let_case(
        &self,
        tree: &mut NodeTree,
        if_id: LocalNodeId<Expression>,
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Expression>,
        scope_id: LocalScopeId,
    ) -> LocalNodeId<MatchCase> {
        // allocate and insert the match case
        let scope = tree.get_scope(if_id);
        let case_id = tree.reserve_from(NodeType::MatchCase, if_id.into_any(), scope, None);
        tree.insert(
            case_id,
            MatchCase::Expression {
                selector: MatchSelector::Pattern {
                    pattern,
                    guard: None,
                },
                body,
                scope: scope_id,
            },
        )
    }

    /// Insert an empty block expression for missing else branches.
    fn insert_empty_block_expression(
        &self,
        tree: &mut NodeTree,
        if_id: LocalNodeId<Expression>,
        scope: (LocalScopeId, destack_dir::LocalScopeMark),
    ) -> LocalNodeId<Expression> {
        // allocate the empty block
        let block_id = tree.reserve_from(NodeType::Block, if_id.into_any(), scope, None);
        let block: LocalNodeId<Block> = tree.insert(
            block_id,
            Block {
                scope: scope.0,
                expressions: Vec::new(),
            },
        );

        // wrap the block as an expression
        let block_expr_id = tree.reserve_from(NodeType::Expression, if_id.into_any(), scope, None);
        tree.insert(block_expr_id, Expression::Block { block })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_if_let_literal() {
        // if let literal becomes equality check
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): number {
    if let 1 = x {
        1
    } else {
        0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): number {
    if (x == 1) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_if_let_union() {
        // if let union becomes || checks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): number {
    if let 1 | 2 = x {
        1
    } else {
        0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): number {
    if (x == 1 || x == 2) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_if_let_else_if_chain() {
        // if let else-if chains become nested if expressions
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function classify(x: number): string {
    if let 1 = x {
        "one"
    } else if let 2 = x {
        "two"
    } else {
        "other"
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
    if (x == 1) {
        return "one";
    } else {
        if (x == 2) {
            return "two";
        } else {
            return "other";
        }
    }
}
"#,
        );
    }
}
