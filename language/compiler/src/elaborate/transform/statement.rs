use destack_dir::{
    Block, DeclarationDescriptor, Declarator, Expression, IfCondition, IfKind, LocalNodeId,
    Mutability, NodeTree, NodeType, Path, SymbolTable, TypeTable,
};
use destack_source::ModuleId;
use smallvec::smallvec;

use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize value-position control flow into explicit statements.
    ///
    /// Transforms control flow expressions used as values into explicit
    /// assignments or returns without changing evaluation order.
    pub(crate) fn transform_normalize_value_expressions(
        &self,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // collect all block ids to process (we modify the tree, so collect first)
        let block_ids: Vec<_> = tree.iter_node_ids_of_type::<Block>().into_iter().collect();

        // process each block
        for block_id in block_ids {
            if !self.is_node_active(tree, symbols, block_id.into_any()) {
                continue;
            }
            self.normalize_block_expressions(block_id, tree, types, module_id)?;
        }

        Ok(())
    }

    /// Normalize value expressions within a single block.
    fn normalize_block_expressions(
        &self,
        block_id: LocalNodeId<Block>,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // read the block and scope
        let block = tree.get(block_id).clone();
        let scope = tree.get_scope(block_id);

        // process each expression, collecting replacements
        let mut new_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut modified = false;

        for expr_id in block.expressions.iter().copied() {
            let expr = tree.get(expr_id).clone();

            // unwrap Statement wrapper if present
            let inner_expr = match &expr {
                Expression::Statement { statement } => tree.get(*statement).clone(),
                _ => expr.clone(),
            };

            match &inner_expr {
                // let x = if (c) { a } else { b }
                Expression::Let {
                    descriptor,
                    mutability,
                    declarators,
                } => {
                    if declarators.len() != 1 {
                        new_expressions.push(expr_id);
                        continue;
                    }
                    let declarator_id = declarators[0];
                    let declarator = tree.get(declarator_id).clone();

                    // skip declarators without values
                    let Some(value_id) = declarator.value else {
                        new_expressions.push(expr_id);
                        continue;
                    };

                    let value = tree.get(value_id).clone();
                    match value {
                        // let x = if (c) { a } else { b }
                        Expression::If {
                            kind: IfKind::If,
                            condition,
                            then_expression,
                            else_expression: Some(else_expr),
                        } => {
                            self.normalize_if_in_let(
                                tree,
                                types,
                                scope,
                                &mut new_expressions,
                                expr_id,
                                declarator_id,
                                &declarator,
                                condition,
                                then_expression,
                                else_expr,
                                module_id,
                            )?;
                            modified = true;
                            continue;
                        }

                        // let x = (a, b, c)
                        Expression::SequenceExpression { expressions } => {
                            self.normalize_sequence_in_let(
                                tree,
                                types,
                                scope,
                                &mut new_expressions,
                                expr_id,
                                declarator_id,
                                &declarator,
                                descriptor.clone(),
                                *mutability,
                                &expressions,
                                module_id,
                            )?;
                            modified = true;
                            continue;
                        }

                        // let x = { ... }
                        Expression::Block { block: inner_block } => {
                            self.normalize_block_in_let(
                                tree,
                                types,
                                scope,
                                &mut new_expressions,
                                declarator_id,
                                &declarator,
                                descriptor.clone(),
                                *mutability,
                                inner_block,
                                module_id,
                            )?;
                            modified = true;
                            continue;
                        }

                        _ => {}
                    }
                }

                // return if (c) { a } else { b }
                Expression::Return {
                    value: Some(value_id),
                } => {
                    let value = tree.get(*value_id).clone();

                    if let Expression::If {
                        kind: IfKind::If,
                        condition,
                        then_expression,
                        else_expression: Some(else_expr),
                    } = value
                    {
                        self.normalize_if_in_return(
                            tree,
                            types,
                            scope,
                            &mut new_expressions,
                            expr_id,
                            condition,
                            then_expression,
                            else_expr,
                            module_id,
                        )?;
                        modified = true;
                        continue;
                    }
                }

                _ => {}
            }

            // no transformation, keep the original
            new_expressions.push(expr_id);
        }

        // update the block when modified
        if modified {
            let block_mut = tree.get_mut(block_id);
            block_mut.expressions = new_expressions;

            self.reinfer_block_type(block_id, tree, types, module_id)?;
        }

        Ok(())
    }

    /// Normalize `let x = if (c) { a } else { b }` into statement form.
    ///
    /// Produces `let x; if (c) { x = a } else { x = b }`.
    /// Replaces nodes in-place to avoid orphaned nodes in the tree.
    fn normalize_if_in_let(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_expr_id: LocalNodeId<Expression>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        condition: IfCondition,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // replace the declarator in place to have no value (uninit)
        tree.replace(
            declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: None,
            },
        );

        // the original let expression now has an uninit declarator
        // keep the original expression (Statement wrapping Let) in the block
        new_expressions.push(original_expr_id);

        // create assignment targets for each branch
        let target_id =
            self.pattern_to_assignment_target(tree, types, scope, declarator.pattern, module_id)?;
        let target_id2 =
            self.pattern_to_assignment_target(tree, types, scope, declarator.pattern, module_id)?;

        // wrap the value-producing part of each branch in assignment
        // (for blocks, this replaces the last expression; for simple expressions, wraps the whole thing)
        let then_transformed = self.wrap_branch_value_in_assignment(
            tree,
            types,
            scope,
            target_id,
            then_expression,
            module_id,
        )?;
        let else_transformed = self.wrap_branch_value_in_assignment(
            tree,
            types,
            scope,
            target_id2,
            else_expression,
            module_id,
        )?;

        // create the new if expression
        let new_if_id =
            tree.reserve_from(NodeType::Expression, declarator_id.into_any(), scope, None);
        let new_if: LocalNodeId<Expression> = tree.insert(
            new_if_id,
            Expression::If {
                kind: IfKind::If,
                condition,
                then_expression: then_transformed,
                else_expression: Some(else_transformed),
            },
        );
        self.set_void_expression_type(types, module_id, new_if);
        new_expressions.push(new_if);

        Ok(())
    }

    /// Normalize `let x = (a, b, c)` into statement form.
    ///
    /// Produces `a; b; let x = c;`.
    #[allow(clippy::too_many_arguments)]
    fn normalize_sequence_in_let(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_let_id: LocalNodeId<Expression>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        seq_expressions: &[LocalNodeId<Expression>],
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // skip empty sequences
        if seq_expressions.is_empty() {
            return Ok(());
        }

        // add all but the last expression as statements
        for &expr_id in &seq_expressions[..seq_expressions.len() - 1] {
            let stmt_id = tree.reserve_from(
                NodeType::Expression,
                original_let_id.into_any(),
                scope,
                None,
            );
            let stmt: LocalNodeId<Expression> =
                tree.insert(stmt_id, Expression::Statement { statement: expr_id });
            self.set_void_expression_type(types, module_id, stmt);
            new_expressions.push(stmt);
        }

        // create let with the last expression as value
        let last_expr = seq_expressions[seq_expressions.len() - 1];

        let new_declarator_id =
            tree.reserve_from(NodeType::Declarator, declarator_id.into_any(), scope, None);
        let new_declarator: LocalNodeId<Declarator> = tree.insert(
            new_declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: Some(last_expr),
            },
        );

        let new_let_id = tree.reserve_from(
            NodeType::Expression,
            original_let_id.into_any(),
            scope,
            None,
        );
        let new_let: LocalNodeId<Expression> = tree.insert(
            new_let_id,
            Expression::Let {
                descriptor,
                mutability,
                declarators: vec![new_declarator],
            },
        );
        self.set_void_expression_type(types, module_id, new_let);
        new_expressions.push(new_let);

        Ok(())
    }

    /// Normalize `let x = { ... last }` into statement form.
    ///
    /// Produces `let x; { ... x = last }`.
    fn normalize_block_in_let(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        inner_block_id: LocalNodeId<Block>,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // skip empty blocks
        let inner_block = tree.get(inner_block_id).clone();
        if inner_block.expressions.is_empty() {
            return Ok(());
        }

        // create uninitialized declarator
        let uninit_declarator_id =
            tree.reserve_from(NodeType::Declarator, declarator_id.into_any(), scope, None);
        let uninit_declarator: LocalNodeId<Declarator> = tree.insert(
            uninit_declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: None,
            },
        );

        // create the uninitialized let
        let uninit_let_id =
            tree.reserve_from(NodeType::Expression, declarator_id.into_any(), scope, None);
        let uninit_let: LocalNodeId<Expression> = tree.insert(
            uninit_let_id,
            Expression::Let {
                descriptor,
                mutability,
                declarators: vec![uninit_declarator],
            },
        );
        self.set_void_expression_type(types, module_id, uninit_let);
        new_expressions.push(uninit_let);

        // replace the last expression with an assignment
        let last_expr_id = inner_block.expressions[inner_block.expressions.len() - 1];
        let target_id =
            self.pattern_to_assignment_target(tree, types, scope, declarator.pattern, module_id)?;
        let assign_expr =
            self.wrap_in_assignment(tree, types, scope, target_id, last_expr_id, module_id)?;

        // update the block
        let block_mut = tree.get_mut(inner_block_id);
        if let Some(last) = block_mut.expressions.last_mut() {
            *last = assign_expr;
        }

        // add the block expression
        let block_expr_id =
            tree.reserve_from(NodeType::Expression, declarator_id.into_any(), scope, None);
        let block_expr: LocalNodeId<Expression> = tree.insert(
            block_expr_id,
            Expression::Block {
                block: inner_block_id,
            },
        );
        self.set_void_block_expression_type(types, module_id, inner_block_id, block_expr);
        new_expressions.push(block_expr);

        Ok(())
    }

    /// Normalize `return if (c) { a } else { b }` into statement form.
    ///
    /// Produces `if (c) { return a } else { return b }`.
    fn normalize_if_in_return(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_return_id: LocalNodeId<Expression>,
        condition: IfCondition,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<()> {
        // wrap the value producing part of each branch in return
        let then_transformed =
            self.wrap_branch_value_in_return(tree, types, scope, then_expression, module_id)?;
        let else_expr = tree.get(else_expression).clone();
        let else_transformed = match else_expr {
            Expression::If {
                kind: IfKind::If, ..
            } => {
                // normalize nested if for else if chains
                self.normalize_if_expression_for_return(
                    tree,
                    types,
                    scope,
                    else_expression,
                    module_id,
                )?
            }
            _ => {
                self.wrap_branch_value_in_return(tree, types, scope, else_expression, module_id)?
            }
        };

        // create the new if expression
        let new_if_id = tree.reserve_from(
            NodeType::Expression,
            original_return_id.into_any(),
            scope,
            None,
        );
        let new_if: LocalNodeId<Expression> = tree.insert(
            new_if_id,
            Expression::If {
                kind: IfKind::If,
                condition,
                then_expression: then_transformed,
                else_expression: Some(else_transformed),
            },
        );
        self.set_void_expression_type(types, module_id, new_if);
        new_expressions.push(new_if);

        Ok(())
    }

    /// Normalize nested if expressions when they appear in return branches.
    /// Preserves else if chains by converting each branch to return form.
    fn normalize_if_expression_for_return(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        if_id: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let Expression::If {
            kind: IfKind::If,
            condition,
            then_expression,
            else_expression,
        } = tree.get(if_id).clone()
        else {
            return Ok(if_id);
        };

        // wrap the value producing part of each branch in return
        let then_transformed =
            self.wrap_branch_value_in_return(tree, types, scope, then_expression, module_id)?;
        let else_transformed = match else_expression {
            Some(else_expression) => {
                let else_expr = tree.get(else_expression).clone();
                match else_expr {
                    Expression::If {
                        kind: IfKind::If, ..
                    } => {
                        // normalize nested else if
                        Some(self.normalize_if_expression_for_return(
                            tree,
                            types,
                            scope,
                            else_expression,
                            module_id,
                        )?)
                    }
                    _ => Some(self.wrap_branch_value_in_return(
                        tree,
                        types,
                        scope,
                        else_expression,
                        module_id,
                    )?),
                }
            }
            None => None,
        };

        tree.replace(
            if_id,
            Expression::If {
                kind: IfKind::If,
                condition,
                then_expression: then_transformed,
                else_expression: else_transformed,
            },
        );
        self.set_void_expression_type(types, module_id, if_id);

        Ok(if_id)
    }

    /// Convert a pattern to an assignment target expression.
    fn pattern_to_assignment_target(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        pattern_id: LocalNodeId<destack_dir::Pattern>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        use destack_dir::Pattern;

        let pattern = tree.get(pattern_id).clone();

        match pattern {
            // simple binding creates a local reference
            Pattern::Binding { name, symbol, .. } => {
                let path = Path {
                    segments: smallvec![name],
                };
                let global_symbol = symbol.into_global(module_id);
                let value_type_id = self.value_type_id_or_error(
                    module_id,
                    global_symbol,
                    pattern_id.into_any(),
                    types,
                )?;

                let ref_id =
                    tree.reserve_from(NodeType::Expression, pattern_id.into_any(), scope, None);
                let ref_expr: LocalNodeId<Expression> = tree.insert(
                    ref_id,
                    Expression::LocalReference {
                        path,
                        static_arguments: None,
                        target_symbol: global_symbol,
                    },
                );
                self.set_expression_type(types, module_id, ref_expr, value_type_id);

                Ok(ref_expr)
            }

            // complex patterns not yet supported
            _ => Err(ElaborateError::UnsupportedConstruct {
                node: pattern_id.into_global_any(module_id).into_anchored(None),
            }),
        }
    }

    /// Wrap the value-producing part of a branch expression in assignment.
    ///
    /// For blocks, this replaces the last expression with an assignment.
    /// For simple expressions, wraps the whole thing in assignment.
    fn wrap_branch_value_in_assignment(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        target: LocalNodeId<Expression>,
        branch: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let branch_expr = tree.get(branch).clone();
        match branch_expr {
            // for blocks, wrap the last expression in assignment
            Expression::Block { block: block_id } => {
                let block = tree.get(block_id).clone();
                if block.expressions.is_empty() {
                    // empty block: just return the branch unchanged
                    return Ok(branch);
                }

                // get the last expression
                let last_idx = block.expressions.len() - 1;
                let last_expr_id = block.expressions[last_idx];

                // wrap the last expression in assignment
                let assign =
                    self.wrap_in_assignment(tree, types, scope, target, last_expr_id, module_id)?;

                // wrap the assignment in a Statement for proper semicolon
                let stmt_id =
                    tree.reserve_from(NodeType::Expression, last_expr_id.into_any(), scope, None);
                let stmt: LocalNodeId<Expression> =
                    tree.insert(stmt_id, Expression::Statement { statement: assign });
                self.set_void_expression_type(types, module_id, stmt);

                // update the block with the statement as the last expression
                let mut new_expressions = block.expressions.clone();
                new_expressions[last_idx] = stmt;

                let block_mut = tree.get_mut(block_id);
                block_mut.expressions = new_expressions;
                self.set_void_block_expression_type(types, module_id, block_id, branch);

                // return the original branch (now modified)
                Ok(branch)
            }

            // for simple expressions, wrap the whole thing
            _ => self.wrap_in_assignment(tree, types, scope, target, branch, module_id),
        }
    }

    /// Wrap the value-producing part of a branch expression in return.
    ///
    /// For blocks, this replaces the last expression with a return.
    /// For simple expressions, wraps the whole thing in return.
    fn wrap_branch_value_in_return(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        branch: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let branch_expr = tree.get(branch).clone();

        match branch_expr {
            // for blocks, wrap the last expression in return
            Expression::Block { block: block_id } => {
                let block = tree.get(block_id).clone();
                if block.expressions.is_empty() {
                    // empty block: just return the branch unchanged
                    return Ok(branch);
                }

                // get the last expression
                let last_idx = block.expressions.len() - 1;
                let last_expr_id = block.expressions[last_idx];

                // wrap the last expression in return
                let return_expr =
                    self.wrap_in_return(tree, types, scope, last_expr_id, module_id)?;

                // wrap the return in a Statement for proper semicolon
                let stmt_id =
                    tree.reserve_from(NodeType::Expression, last_expr_id.into_any(), scope, None);
                let stmt: LocalNodeId<Expression> = tree.insert(
                    stmt_id,
                    Expression::Statement {
                        statement: return_expr,
                    },
                );
                self.set_void_expression_type(types, module_id, stmt);

                // update the block with the statement as the last expression
                let mut new_expressions = block.expressions.clone();
                new_expressions[last_idx] = stmt;

                let block_mut = tree.get_mut(block_id);
                block_mut.expressions = new_expressions;
                self.set_void_block_expression_type(types, module_id, block_id, branch);

                // return the original branch (now modified)
                Ok(branch)
            }

            // for simple expressions, wrap the whole thing
            _ => {
                // wrap the return in a statement inside a block
                let return_expr = self.wrap_in_return(tree, types, scope, branch, module_id)?;

                let stmt_id =
                    tree.reserve_from(NodeType::Expression, branch.into_any(), scope, None);
                let stmt: LocalNodeId<Expression> = tree.insert(
                    stmt_id,
                    Expression::Statement {
                        statement: return_expr,
                    },
                );
                self.set_void_expression_type(types, module_id, stmt);

                let block_id = tree.reserve_from(NodeType::Block, branch.into_any(), scope, None);
                let block: LocalNodeId<Block> = tree.insert(
                    block_id,
                    Block {
                        scope: scope.0,
                        expressions: vec![stmt],
                    },
                );

                let block_expr_id =
                    tree.reserve_from(NodeType::Expression, branch.into_any(), scope, None);
                let block_expr: LocalNodeId<Expression> =
                    tree.insert(block_expr_id, Expression::Block { block });
                self.set_void_block_expression_type(types, module_id, block, block_expr);

                Ok(block_expr)
            }
        }
    }

    /// Wrap an expression in an assignment to a target.
    fn wrap_in_assignment(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        target: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let assign_id = tree.reserve_from(NodeType::Expression, value.into_any(), scope, None);
        let assign: LocalNodeId<Expression> = tree.insert(
            assign_id,
            Expression::Assign {
                left: target,
                right: value,
            },
        );
        self.set_void_expression_type(types, module_id, assign);

        Ok(assign)
    }

    /// Wrap an expression in a return statement.
    fn wrap_in_return(
        &self,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
        value: LocalNodeId<Expression>,
        module_id: ModuleId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let return_id = tree.reserve_from(NodeType::Expression, value.into_any(), scope, None);
        let return_expr: LocalNodeId<Expression> =
            tree.insert(return_id, Expression::Return { value: Some(value) });
        self.set_never_expression_type(types, module_id, return_expr);

        Ok(return_expr)
    }
}
