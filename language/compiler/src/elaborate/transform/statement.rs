use destack_dir as dir;
use dir::{
    BinaryOperator, Block, BlockContext, BlockForm, Declarator, ExportKind, Expression,
    IfCondition, IfForm, LocalNodeId, Mutability, NodeType,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize value-position control flow into explicit statements.
    ///
    /// Transforms control flow expressions used as values into explicit
    /// assignments or returns without changing evaluation order.
    pub(crate) fn transform_normalize_value_expressions(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        // collect all block ids to process (we modify the tree, so collect first)
        let block_ids: Vec<_> = state
            .tree
            .iter_node_ids_of_type::<Block>()
            .into_iter()
            .collect();

        // process each block
        for block_id in block_ids {
            if !self.is_active_in_state(state, block_id.into_any()) {
                continue;
            }
            self.normalize_block_expressions(state, block_id)?;
        }

        Ok(())
    }

    /// Normalize value expressions within a single block.
    fn normalize_block_expressions(
        &self,
        state: &mut ElaborateState<'_>,
        block_id: LocalNodeId<Block>,
    ) -> ElaborateResult<()> {
        // read the block and scope
        let block = state.tree.get(block_id).clone();
        let scope = state.tree.get_scope(block_id);

        // process leading expressions in effect-position
        let mut new_leading_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut new_tail_expression = block.tail_expression;
        let mut modified = false;

        for expr_id in block.leading_expressions.iter().copied() {
            let expr = state.tree.get(expr_id).clone();
            match &expr {
                // let x = if (c) { a } else { b }
                Expression::Let {
                    export,
                    is_ambient,
                    mutability,
                    declarators,
                } => {
                    if declarators.len() != 1 {
                        new_leading_expressions.push(expr_id);
                        continue;
                    }
                    let declarator_id = declarators[0];
                    let declarator = state.tree.get(declarator_id).clone();

                    // skip declarators without values
                    let Some(value_id) = declarator.value else {
                        new_leading_expressions.push(expr_id);
                        continue;
                    };

                    let value = state.tree.get(value_id).clone();
                    match value {
                        // let x = if (c) { a } else { b }
                        Expression::If {
                            form: IfForm::If,
                            condition,
                            then_expression,
                            else_expression: Some(else_expr),
                        } => {
                            self.normalize_if_in_let(
                                state,
                                scope,
                                &mut new_leading_expressions,
                                expr_id,
                                declarator_id,
                                &declarator,
                                condition,
                                then_expression,
                                else_expr,
                            )?;
                            modified = true;
                            continue;
                        }

                        // let x = (a, b, c)
                        Expression::SequenceExpression { expressions } => {
                            let did_normalize = self.normalize_sequence_in_let(
                                state,
                                scope,
                                &mut new_leading_expressions,
                                expr_id,
                                declarator_id,
                                &declarator,
                                *export,
                                *is_ambient,
                                *mutability,
                                &expressions,
                            )?;
                            if did_normalize {
                                modified = true;
                                continue;
                            }
                        }

                        // let x = { ... }
                        Expression::Block(inner_block) => {
                            let did_normalize = self.normalize_block_in_let(
                                state,
                                scope,
                                &mut new_leading_expressions,
                                declarator_id,
                                &declarator,
                                *export,
                                *is_ambient,
                                *mutability,
                                inner_block,
                            )?;
                            if did_normalize {
                                modified = true;
                                continue;
                            }
                        }

                        // let x = left ?? right
                        Expression::Binary {
                            left,
                            operator: BinaryOperator::Coalesce,
                            right,
                        } => {
                            let did_normalize = self.normalize_coalesce_in_let(
                                state,
                                scope,
                                &mut new_leading_expressions,
                                expr_id,
                                declarator_id,
                                &declarator,
                                left,
                                right,
                            )?;
                            if did_normalize {
                                modified = true;
                                continue;
                            }
                        }

                        _ => {}
                    }
                }

                // return if (c) { a } else { b }
                Expression::Return {
                    value: Some(value_id),
                } => {
                    let value = state.tree.get(*value_id).clone();

                    if let Expression::If {
                        form: IfForm::If,
                        condition,
                        then_expression,
                        else_expression: Some(else_expr),
                    } = value.clone()
                    {
                        self.normalize_if_in_return(
                            state,
                            scope,
                            &mut new_leading_expressions,
                            expr_id,
                            *value_id,
                            condition,
                            then_expression,
                            else_expr,
                        )?;
                        modified = true;
                        continue;
                    }

                    if let Expression::Binary {
                        left,
                        operator: BinaryOperator::Coalesce,
                        right,
                    } = value.clone()
                    {
                        let did_normalize = self.normalize_coalesce_in_return(
                            state,
                            scope,
                            &mut new_leading_expressions,
                            expr_id,
                            *value_id,
                            left,
                            right,
                        )?;
                        if did_normalize {
                            modified = true;
                            continue;
                        }
                    }
                }

                _ => {}
            }

            // normalize nested coalesce in eager value contexts
            let did_normalize_nested =
                self.normalize_nested_coalesce_in_expression(state, scope, expr_id)?;
            if did_normalize_nested {
                modified = true;
                new_leading_expressions.push(expr_id);
                continue;
            }

            // no transformation, keep the original
            new_leading_expressions.push(expr_id);
        }

        // process the tail expression in value-position
        if let Some(expr_id) = block.tail_expression {
            let expr = state.tree.get(expr_id).clone();
            let mut consumed_tail = false;

            match &expr {
                // let x = if (c) { a } else { b }
                Expression::Let {
                    export,
                    is_ambient,
                    mutability,
                    declarators,
                } => {
                    if declarators.len() == 1 {
                        let declarator_id = declarators[0];
                        let declarator = state.tree.get(declarator_id).clone();

                        if let Some(value_id) = declarator.value {
                            let value = state.tree.get(value_id).clone();
                            match value {
                                Expression::If {
                                    form: IfForm::If,
                                    condition,
                                    then_expression,
                                    else_expression: Some(else_expr),
                                } => {
                                    self.normalize_if_in_let(
                                        state,
                                        scope,
                                        &mut new_leading_expressions,
                                        expr_id,
                                        declarator_id,
                                        &declarator,
                                        condition,
                                        then_expression,
                                        else_expr,
                                    )?;
                                    modified = true;
                                    consumed_tail = true;
                                }

                                Expression::SequenceExpression { expressions } => {
                                    let did_normalize = self.normalize_sequence_in_let(
                                        state,
                                        scope,
                                        &mut new_leading_expressions,
                                        expr_id,
                                        declarator_id,
                                        &declarator,
                                        *export,
                                        *is_ambient,
                                        *mutability,
                                        &expressions,
                                    )?;
                                    if did_normalize {
                                        modified = true;
                                        consumed_tail = true;
                                    }
                                }

                                Expression::Block(inner_block) => {
                                    let did_normalize = self.normalize_block_in_let(
                                        state,
                                        scope,
                                        &mut new_leading_expressions,
                                        declarator_id,
                                        &declarator,
                                        *export,
                                        *is_ambient,
                                        *mutability,
                                        inner_block,
                                    )?;
                                    if did_normalize {
                                        modified = true;
                                        consumed_tail = true;
                                    }
                                }

                                Expression::Binary {
                                    left,
                                    operator: BinaryOperator::Coalesce,
                                    right,
                                } => {
                                    let did_normalize = self.normalize_coalesce_in_let(
                                        state,
                                        scope,
                                        &mut new_leading_expressions,
                                        expr_id,
                                        declarator_id,
                                        &declarator,
                                        left,
                                        right,
                                    )?;
                                    if did_normalize {
                                        modified = true;
                                        consumed_tail = true;
                                    }
                                }

                                _ => {}
                            }
                        }
                    }
                }

                // return if (c) { a } else { b }
                Expression::Return {
                    value: Some(value_id),
                } => {
                    let value = state.tree.get(*value_id).clone();

                    if let Expression::If {
                        form: IfForm::If,
                        condition,
                        then_expression,
                        else_expression: Some(else_expr),
                    } = value.clone()
                    {
                        self.normalize_if_in_return(
                            state,
                            scope,
                            &mut new_leading_expressions,
                            expr_id,
                            *value_id,
                            condition,
                            then_expression,
                            else_expr,
                        )?;
                        modified = true;
                        consumed_tail = true;
                    }

                    if !consumed_tail
                        && let Expression::Binary {
                            left,
                            operator: BinaryOperator::Coalesce,
                            right,
                        } = value
                    {
                        let did_normalize = self.normalize_coalesce_in_return(
                            state,
                            scope,
                            &mut new_leading_expressions,
                            expr_id,
                            *value_id,
                            left,
                            right,
                        )?;
                        if did_normalize {
                            modified = true;
                            consumed_tail = true;
                        }
                    }
                }

                _ => {}
            }

            // normalize nested coalesce in eager value contexts
            if !consumed_tail {
                let did_normalize_nested =
                    self.normalize_nested_coalesce_in_expression(state, scope, expr_id)?;
                if did_normalize_nested {
                    modified = true;
                }

                new_tail_expression = Some(expr_id);
            }
            // the tail was normalized into explicit leading expressions
            else {
                new_tail_expression = None;
            }
        }

        // update the block when modified
        if modified {
            let updated_block = Block {
                leading_expressions: new_leading_expressions,
                tail_expression: new_tail_expression,
                ..block
            };
            state.tree.replace(block_id, updated_block);

            self.reinfer_block_type_in_state(state, block_id)?;
        }

        Ok(())
    }

    /// Normalize `let x = if (c) { a } else { b }` into statement form.
    ///
    /// Produces `let x; if (c) { x = a } else { x = b }`.
    /// Replaces nodes in-place to avoid orphaned nodes in the tree.
    fn normalize_if_in_let(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_expr_id: LocalNodeId<Expression>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        condition: IfCondition,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // replace the declarator in place to have no value (uninit)
        state.tree.replace(
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
        let target_id = self.pattern_to_assignment_target(state, scope, declarator.pattern)?;
        let target_id2 = self.pattern_to_assignment_target(state, scope, declarator.pattern)?;

        // wrap the value-producing part of each branch in assignment
        // (for blocks, this replaces the last expression; for simple expressions, wraps the whole thing)
        let then_transformed =
            self.wrap_branch_value_in_assignment(state, scope, target_id, then_expression)?;
        let else_transformed =
            self.wrap_branch_value_in_assignment(state, scope, target_id2, else_expression)?;

        // create the new if expression
        let new_if_id = state.tree.reserve_from(
            NodeType::Expression,
            declarator_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let new_if: LocalNodeId<Expression> = state.tree.insert_as_owner(
            new_if_id,
            Expression::If {
                form: IfForm::If,
                condition,
                then_expression: then_transformed,
                else_expression: Some(else_transformed),
            },
        );
        self.set_void_expression_type(state.types, state.module_id, new_if);
        new_expressions.push(new_if);

        Ok(())
    }

    /// Normalize `let x = (a, b, c)` into statement form.
    ///
    /// Produces `a; b; let x = c;`.
    fn normalize_sequence_in_let(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_let_id: LocalNodeId<Expression>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        export: Option<ExportKind>,
        is_ambient: bool,
        mutability: Mutability,
        seq_expressions: &[LocalNodeId<Expression>],
    ) -> ElaborateResult<bool> {
        // skip empty sequences
        if seq_expressions.is_empty() {
            return Ok(false);
        }

        // add all but the last expression as statements
        for &expr_id in &seq_expressions[..seq_expressions.len() - 1] {
            let stmt = self.insert_effect_expression(state, original_let_id, expr_id, scope);
            new_expressions.push(stmt);
        }

        // create let with the last expression as value
        let last_expr = seq_expressions[seq_expressions.len() - 1];

        let new_declarator_id = state.tree.reserve_from(
            NodeType::Declarator,
            declarator_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let new_declarator: LocalNodeId<Declarator> = state.tree.insert_as_owner(
            new_declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: Some(last_expr),
            },
        );

        let new_let_id = state.tree.reserve_from(
            NodeType::Expression,
            original_let_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let new_let: LocalNodeId<Expression> = state.tree.insert_as_owner(
            new_let_id,
            Expression::Let {
                export,
                is_ambient,
                mutability,
                declarators: vec![new_declarator],
            },
        );
        self.set_void_expression_type(state.types, state.module_id, new_let);
        new_expressions.push(new_let);

        Ok(true)
    }

    /// Normalize `let x = { ... last }` into statement form.
    ///
    /// Produces `let x; { ... x = last }`.
    fn normalize_block_in_let(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        declarator_id: LocalNodeId<Declarator>,
        declarator: &Declarator,
        export: Option<ExportKind>,
        is_ambient: bool,
        mutability: Mutability,
        inner_block_id: LocalNodeId<Block>,
    ) -> ElaborateResult<bool> {
        // skip empty blocks
        let inner_block = state.tree.get(inner_block_id).clone();
        let Some(last_expr_id) = inner_block.tail_expression else {
            return Ok(false);
        };

        // create uninitialized declarator
        let uninit_declarator_id = state.tree.reserve_from(
            NodeType::Declarator,
            declarator_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let uninit_declarator: LocalNodeId<Declarator> = state.tree.insert_as_owner(
            uninit_declarator_id,
            Declarator {
                pattern: declarator.pattern,
                ty: declarator.ty,
                value: None,
            },
        );

        // create the uninitialized let
        let uninit_let_id = state.tree.reserve_from(
            NodeType::Expression,
            declarator_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let uninit_let: LocalNodeId<Expression> = state.tree.insert_as_owner(
            uninit_let_id,
            Expression::Let {
                export,
                is_ambient,
                mutability,
                declarators: vec![uninit_declarator],
            },
        );
        self.set_void_expression_type(state.types, state.module_id, uninit_let);
        new_expressions.push(uninit_let);

        // replace the tail expression with an assignment
        let target_id = self.pattern_to_assignment_target(state, scope, declarator.pattern)?;
        let assign_expr = self.wrap_in_assignment(state, scope, target_id, last_expr_id)?;

        // update the block
        let updated_block = Block {
            tail_expression: Some(assign_expr),
            ..inner_block
        };
        state.tree.replace(inner_block_id, updated_block);

        // add the block expression
        let block_expr_id = state.tree.reserve_from(
            NodeType::Expression,
            declarator_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let block_expr: LocalNodeId<Expression> = state
            .tree
            .insert_as_owner(block_expr_id, Expression::Block(inner_block_id));
        self.set_void_block_expression_type(
            state.types,
            state.module_id,
            inner_block_id,
            block_expr,
        );
        new_expressions.push(block_expr);

        Ok(true)
    }

    /// Normalize `return if (c) { a } else { b }` into statement form.
    ///
    /// Produces `if (c) { return a } else { return b }`.
    fn normalize_if_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        new_expressions: &mut Vec<LocalNodeId<Expression>>,
        original_return_id: LocalNodeId<Expression>,
        original_value_id: LocalNodeId<Expression>,
        condition: IfCondition,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // wrap the value producing part of each branch in return
        let then_transformed = self.wrap_branch_value_in_return(state, scope, then_expression)?;
        let else_expr = state.tree.get(else_expression).clone();
        let else_transformed = match else_expr {
            Expression::If {
                form: IfForm::If, ..
            } => {
                // normalize nested if for else if chains
                self.normalize_if_expression_for_return(state, scope, else_expression)?
            }
            _ => self.wrap_branch_value_in_return(state, scope, else_expression)?,
        };

        // replace the original return with the normalized if
        state.tree.replace(
            original_return_id,
            Expression::If {
                form: IfForm::If,
                condition,
                then_expression: then_transformed,
                else_expression: Some(else_transformed),
            },
        );
        state.tree.mark_inactive(original_value_id.into_any());
        self.set_void_expression_type(state.types, state.module_id, original_return_id);
        new_expressions.push(original_return_id);

        Ok(())
    }

    /// Normalize nested if expressions when they appear in return branches.
    /// Preserves else if chains by converting each branch to return form.
    fn normalize_if_expression_for_return(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        if_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let Expression::If {
            form: IfForm::If,
            condition,
            then_expression,
            else_expression,
        } = state.tree.get(if_id).clone()
        else {
            return Ok(if_id);
        };

        // wrap the value producing part of each branch in return
        let then_transformed = self.wrap_branch_value_in_return(state, scope, then_expression)?;
        let else_transformed = match else_expression {
            Some(else_expression) => {
                let else_expr = state.tree.get(else_expression).clone();
                match else_expr {
                    Expression::If {
                        form: IfForm::If, ..
                    } => {
                        // normalize nested else if
                        Some(self.normalize_if_expression_for_return(
                            state,
                            scope,
                            else_expression,
                        )?)
                    }
                    _ => Some(self.wrap_branch_value_in_return(state, scope, else_expression)?),
                }
            }
            None => None,
        };

        state.tree.replace(
            if_id,
            Expression::If {
                form: IfForm::If,
                condition,
                then_expression: then_transformed,
                else_expression: else_transformed,
            },
        );
        self.set_void_expression_type(state.types, state.module_id, if_id);

        Ok(if_id)
    }

    /// Convert a pattern to an assignment target expression.
    pub(super) fn pattern_to_assignment_target(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        pattern_id: LocalNodeId<dir::Pattern>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        use dir::Pattern;

        let pattern = state.tree.get(pattern_id).clone();

        match pattern {
            // simple binding creates a local reference
            Pattern::Binding { name, symbol, .. } => {
                let global_symbol = symbol.into_global(state.module_id);
                self.insert_local_reference_expression_for_symbol(
                    state,
                    pattern_id.into_any(),
                    scope,
                    name,
                    global_symbol,
                )
            }

            // complex patterns not yet supported
            _ => Err(ElaborateError::UnsupportedConstruct {
                anchor: state.module_id.into(),
            }),
        }
    }

    /// Wrap the value-producing part of a branch expression in assignment.
    ///
    /// For blocks, this replaces the last expression with an assignment.
    /// For simple expressions, wraps the whole thing in assignment.
    fn wrap_branch_value_in_assignment(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        target: LocalNodeId<Expression>,
        branch: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let branch_expr = state.tree.get(branch).clone();
        match branch_expr {
            // for blocks, wrap the last expression in assignment
            Expression::Block(block_id) => {
                let block = state.tree.get(block_id).clone();
                let Some(last_expr_id) = block.tail_expression else {
                    // empty block: just return the branch unchanged
                    return Ok(branch);
                };

                // wrap the last expression in assignment
                let assign = self.wrap_in_assignment(state, scope, target, last_expr_id)?;

                let updated_block = Block {
                    tail_expression: Some(assign),
                    ..block
                };
                state.tree.replace(block_id, updated_block);
                self.set_void_block_expression_type(state.types, state.module_id, block_id, branch);

                // return the original branch (now modified)
                Ok(branch)
            }

            // for simple expressions, wrap the whole thing
            _ => self.wrap_in_assignment(state, scope, target, branch),
        }
    }

    /// Wrap the value-producing part of a branch expression in return.
    ///
    /// For blocks, this replaces the last expression with a return.
    /// For simple expressions, wraps the whole thing in return.
    fn wrap_branch_value_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        branch: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let branch_expr = state.tree.get(branch).clone();

        match branch_expr {
            // for blocks, wrap the last expression in return
            Expression::Block(block_id) => {
                let block = state.tree.get(block_id).clone();
                let Some(last_expr_id) = block.tail_expression else {
                    // empty block: just return the branch unchanged
                    return Ok(branch);
                };

                // wrap the last expression in return
                let return_expr = self.wrap_in_return(state, scope, last_expr_id)?;

                let updated_block = Block {
                    tail_expression: Some(return_expr),
                    ..block
                };
                state.tree.replace(block_id, updated_block);
                self.set_void_block_expression_type(state.types, state.module_id, block_id, branch);

                // return the original branch (now modified)
                Ok(branch)
            }

            // for simple expressions, wrap the whole thing
            _ => {
                let return_expr = self.wrap_in_return(state, scope, branch)?;

                Ok(self.wrap_expression_in_block(state, branch, return_expr, scope))
            }
        }
    }

    /// Wrap one expression in a block expression.
    fn wrap_expression_in_block(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        body: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // don't double wrap blocks
        if matches!(state.tree.get(body), Expression::Block(_)) {
            return body;
        }

        // create the block node
        let block_id = state.tree.reserve_from(
            NodeType::Block,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let block: LocalNodeId<Block> = state.tree.insert_as_owner(
            block_id,
            Block {
                context: BlockContext::Expression,
                form: BlockForm::Explicit,
                scope: scope.0,
                leading_expressions: Vec::new(),
                tail_expression: Some(body),
            },
        );

        // wrap the block as an expression
        let block_expr_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let block_expr_id = state
            .tree
            .insert_as_owner(block_expr_id, Expression::Block(block));
        self.set_void_block_expression_type(state.types, state.module_id, block, block_expr_id);

        block_expr_id
    }

    /// Wrap an expression in an assignment to a target.
    pub(super) fn wrap_in_assignment(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        target: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.replace_expression_with_explicit_assignment(state, value, target, scope);

        Ok(value)
    }

    /// Wrap an expression in a return statement.
    pub(super) fn wrap_in_return(
        &self,
        state: &mut ElaborateState<'_>,
        scope: dir::LocalScope,
        value: LocalNodeId<Expression>,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.replace_expression_with_explicit_return(state, value, scope);

        Ok(value)
    }
}
