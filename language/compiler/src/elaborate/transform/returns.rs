use destack_dir::{
    Declaration, Expression, IfKind, LocalNodeId, Member, NodeTree, NodeType, TypeTable,
};

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
        symbols: &destack_dir::SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        // collect function and method bodies to rewrite
        let mut body_ids = Vec::new();
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            if !self.is_node_active(tree, symbols, declaration_id.into_any()) {
                continue;
            }

            let declaration = tree.get(declaration_id).clone();

            // function declarations
            if let Declaration::Function {
                body: Some(body_id),
                ..
            } = declaration
            {
                body_ids.push(body_id);
            }

            // method bodies on structured declarations
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    let Member::Method {
                        body: Some(body_id),
                        ..
                    } = tree.get(*member_id)
                    else {
                        continue;
                    };

                    body_ids.push(*body_id);
                }
            }
        }

        // rewrite implicit returns in each body
        for body_id in body_ids {
            let scope = tree.get_scope(body_id);
            self.make_return_explicit(body_id, tree, scope, types)?;
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
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        let expr = tree.get(expr_id).clone();

        match expr {
            Expression::Block { block } => {
                let block_node = tree.get(block).clone();
                if let Some(last_expr_id) = block_node.expressions.last().copied() {
                    // recursively transform the last expression
                    self.make_return_explicit(last_expr_id, tree, scope, types)?;
                }
            }

            Expression::If {
                kind: IfKind::If,
                condition: _,
                then_expression,
                else_expression,
            } => {
                // for if else (not ternary), transform both branches
                self.make_return_explicit(then_expression, tree, scope, types)?;
                if let Some(else_expr) = else_expression {
                    self.make_return_explicit(else_expr, tree, scope, types)?;
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

                // preserve analysis metadata for the cloned expression
                let module_id = types.module_id;
                types.copy_node_analysis(
                    expr_id.into_global_any(module_id),
                    orig_expr_id.into_global_any(module_id),
                );

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
                self.make_return_explicit(statement, tree, scope, types)?;
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

                // preserve analysis metadata for the cloned expression
                let module_id = types.module_id;
                types.copy_node_analysis(
                    expr_id.into_global_any(module_id),
                    orig_expr_id.into_global_any(module_id),
                );

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
