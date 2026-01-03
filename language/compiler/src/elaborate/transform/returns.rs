use destack_dir::{Declaration, Expression, IfKind, LocalNodeId, NodeTree, NodeType};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform implicit returns into explicit `return` statements.
    ///
    /// ```ds
    /// function f() { 42 }
    /// ```
    /// ->
    /// ```ds
    /// function f() { return 42; }
    /// ```
    ///
    /// This also handles if/else branches:
    /// ```ds
    /// function f(cond) {
    ///     if (cond) { 1 } else { 2 }
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function f(cond) {
    ///     if (cond) { return 1; } else { return 2; }
    /// }
    /// ```
    pub(super) fn transform_explicit_return(
        &self,
        tree: &mut NodeTree,
        _symbols: &destack_dir::SymbolTable,
    ) -> ElaborateResult<()> {
        // collect all function declarations
        let function_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Declaration>()
            .into_iter()
            .filter(|id| matches!(tree.get(*id), Declaration::Function { .. }))
            .collect();

        for func_id in function_ids {
            let Declaration::Function {
                body: Some(body_id),
                ..
            } = tree.get(func_id).clone()
            else {
                continue;
            };

            let scope = tree.get_scope(body_id);
            self.make_return_explicit(body_id, tree, scope)?;
        }

        Ok(())
    }

    /// Make the implicit return in an expression explicit.
    /// For blocks, this transforms the last expression.
    /// For if/else, this transforms both branches recursively.
    fn make_return_explicit(
        &self,
        expr_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> ElaborateResult<()> {
        let expr = tree.get(expr_id).clone();

        match expr {
            Expression::Block { block } => {
                let block_node = tree.get(block).clone();
                if let Some(last_expr_id) = block_node.expressions.last().copied() {
                    // recursively transform the last expression
                    self.make_return_explicit(last_expr_id, tree, scope)?;
                }
            }

            Expression::If {
                kind: IfKind::If,
                condition: _,
                then_expression,
                else_expression,
            } => {
                // for if-else (not ternary), transform both branches
                self.make_return_explicit(then_expression, tree, scope)?;
                if let Some(else_expr) = else_expression {
                    self.make_return_explicit(else_expr, tree, scope)?;
                }
            }

            Expression::If {
                kind: IfKind::Ternary,
                ..
            } => {
                // for ternary, return the whole expression as a statement
                // reserve a new node for the original expression
                let orig_id =
                    tree.reserve_from(NodeType::Expression, expr_id.into_any(), scope, None);
                let orig_expr_id: LocalNodeId<Expression> = tree.insert(orig_id, expr);

                // create return expression
                let return_id =
                    tree.reserve_from(NodeType::Expression, expr_id.into_any(), scope, None);
                let return_expr: LocalNodeId<Expression> = tree.insert(
                    return_id,
                    Expression::Return {
                        value: Some(orig_expr_id),
                    },
                );

                // replace the original node with a statement wrapping the return
                tree.replace(
                    expr_id,
                    Expression::Statement {
                        statement: return_expr,
                    },
                );
            }

            Expression::Match { .. } => {
                unreachable!("match should be transformed to if/else in transform_match_chain");
            }

            Expression::Return { .. } => {
                // already explicit, do nothing
            }

            Expression::Statement { statement } => {
                // recursively transform the inner statement
                self.make_return_explicit(statement, tree, scope)?;
            }

            Expression::Let { .. } | Expression::Using { .. } => {
                // binding is a statement, don't wrap in return
            }

            _ => {
                // wrap value expression in return statement
                // strategy: create nodes for the original expression and return,
                // then replace expr_id with Statement wrapping the Return

                // reserve a new node for the original expression
                let orig_id =
                    tree.reserve_from(NodeType::Expression, expr_id.into_any(), scope, None);
                let orig_expr_id: LocalNodeId<Expression> = tree.insert(orig_id, expr);

                // create return expression
                let return_id =
                    tree.reserve_from(NodeType::Expression, expr_id.into_any(), scope, None);
                let return_expr: LocalNodeId<Expression> = tree.insert(
                    return_id,
                    Expression::Return {
                        value: Some(orig_expr_id),
                    },
                );

                // replace the original node with a statement wrapping the return
                tree.replace(
                    expr_id,
                    Expression::Statement {
                        statement: return_expr,
                    },
                );
            }
        }

        Ok(())
    }
}
