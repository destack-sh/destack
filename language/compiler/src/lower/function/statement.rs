use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use crate::lower::lower_mutability;

use super::FunctionLowerer;
use crate::lower::{BreakContext, LocalBinding, LoopContext, Terminates};

impl FunctionLowerer<'_> {
    /// Resolve an explicit control transfer target.
    fn control_target_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let node_id = expression_id.into_global_any(self.context.module_id);

        match self.context.types.control_resolution(node_id) {
            Some(dir::ControlResolution::Label(symbol)) => Some(symbol),
            _ => None,
        }
    }

    /// Lower a statement expression.
    pub(crate) fn lower_statement_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Terminates> {
        match self.context.dir_tree.get(expression_id) {
            dir::Expression::Labelled { body, symbol, .. } => {
                self.lower_labelled_statement(expression_id, *symbol, *body)
            }
            dir::Expression::Block(block_id) => {
                let block = self.context.dir_tree.get(*block_id);
                for expr_id in block.iter_expressions() {
                    let terminated = self.lower_statement_expression(expr_id)?;
                    if terminated.is_yes() {
                        return Ok(Terminates::Yes);
                    }
                }
                Ok(Terminates::No)
            }
            dir::Expression::Let {
                mutability,
                declarators,
                ..
            } => self.lower_let_expression(expression_id, *mutability, declarators),
            dir::Expression::Return { value } => {
                // handle constructor returns separately
                if self.state.constructor_state.is_some() {
                    if value.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "constructor cannot return a value".to_string(),
                        }
                        .into());
                    }

                    let node = expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile));
                    self.return_constructor_value(node)?;
                    return Ok(Terminates::Yes);
                }

                // lower standard returns
                let return_value = if let Some(value_id) = value {
                    if self.context.return_type == self.context.type_lowerer.ty_void {
                        return Err(LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "return value not allowed for void function".to_string(),
                        }
                        .into());
                    }

                    let value = self
                        .lower_value_for_target(
                            expression_id,
                            *value_id,
                            self.context.return_type_id,
                            self.context.return_type,
                        )?
                        .0;
                    Some(value)
                } else {
                    None
                };
                self.state.builder.return_(return_value);
                Ok(Terminates::Yes)
            }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => match condition {
                dir::IfCondition::Expression { condition } => {
                    self.lower_if_statement(*condition, *then_expression, *else_expression)
                }
                dir::IfCondition::Let { .. } => {
                    // if let should be elaborated before lowering
                    Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported if-let condition".to_string(),
                    }
                    .into())
                }
            },

            dir::Expression::Loop {
                kind,
                condition,
                body,
                symbol,
                ..
            } => self.lower_loop_statement(*kind, *condition, *body, *symbol, None),

            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
                symbol,
                ..
            } => self.lower_for_statement(
                *initialization,
                *condition,
                *increment,
                *body,
                *symbol,
                None,
            ),

            dir::Expression::Break { .. } => {
                let target_symbol = self.control_target_symbol(expression_id);

                self.lower_break_statement(expression_id, target_symbol)
            }

            dir::Expression::Continue { .. } => {
                let target_symbol = self.control_target_symbol(expression_id);

                self.lower_continue_statement(expression_id, target_symbol)
            }

            dir::Expression::Match {
                form,
                value,
                cases,
                symbol,
                ..
            } => self.lower_match_statement(expression_id, *form, *value, cases, *symbol),

            dir::Expression::Call {
                left,
                arguments,
                generic_arguments,
            } => {
                self.lower_call_statement(expression_id, left, arguments, generic_arguments)?;
                Ok(Terminates::No)
            }

            _ => {
                self.lower_value_expression(expression_id)?;
                Ok(Terminates::No)
            }
        }
    }

    /// Lower a let binding statement into locals.
    fn lower_let_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<Terminates> {
        // lower each declarator in order
        for declarator_id in declarators {
            let declarator = self.context.dir_tree.get(*declarator_id);

            let pattern_id = declarator.pattern;
            if let Some(value_id) = declarator.value {
                // lower initialized declarators directly
                let (value, value_type) = self.lower_value_expression(value_id)?;
                self.bind_pattern_value(pattern_id, value, value_type, Some(mutability))?;
            } else {
                // create uninitialized locals for transformed value statements
                self.bind_uninitialized_pattern(
                    expression_id,
                    pattern_id,
                    declarator.ty,
                    Some(mutability),
                )?;
            }
        }

        Ok(Terminates::No)
    }

    /// Lower a labelled statement with an explicit break target.
    /// TODO #Architecture: should we elaborate labelled statements away..?
    fn lower_labelled_statement(
        &mut self,
        _expression_id: dir::LocalNodeId<dir::Expression>,
        label_symbol: dir::LocalSymbolId,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Terminates> {
        let label_symbol = label_symbol.into_global(self.context.module_id);
        let body = self.context.dir_tree.get(body_id);

        match body {
            dir::Expression::Loop {
                kind,
                condition,
                body,
                symbol,
                ..
            } => self.lower_loop_statement(*kind, *condition, *body, *symbol, Some(label_symbol)),
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
                symbol,
                ..
            } => self.lower_for_statement(
                *initialization,
                *condition,
                *increment,
                *body,
                *symbol,
                Some(label_symbol),
            ),
            _ => {
                let break_block = self.state.builder.block();
                let label_context = BreakContext { break_block };
                self.state
                    .control
                    .labels_by_symbol
                    .insert(label_symbol, label_context);

                let terminated = self.lower_statement_expression(body_id)?;
                if !terminated.is_yes() {
                    self.state.builder.jump(break_block);
                }

                self.state.control.labels_by_symbol.remove(&label_symbol);
                self.state.builder.switch_to_block(break_block);
                Ok(Terminates::No)
            }
        }
    }

    /// Lower an if statement into blocks and branches.
    fn lower_if_statement(
        &mut self,
        condition_id: dir::LocalNodeId<dir::Expression>,
        then_id: dir::LocalNodeId<dir::Expression>,
        else_id: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Terminates> {
        let then_block = self.state.builder.block();
        let else_block = self.state.builder.block();
        let join_block = self.state.builder.block();

        // branch on the condition (with union tag checks when possible)
        let did_check = self.lower_union_tag_check(condition_id, then_block, else_block)?;
        if !did_check {
            // lower the condition value
            let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
            self.check_type_is_bool(condition_id, condition_type, "if condition")?;

            // branch
            self.state
                .builder
                .branch(condition_value, then_block, else_block);
        }

        // then branch
        self.state.builder.switch_to_block(then_block);
        let then_terminated = self.lower_statement_expression(then_id)?;
        if !then_terminated.is_yes() {
            self.state.builder.jump(join_block);
        }

        // else branch
        self.state.builder.switch_to_block(else_block);
        let else_terminated = if let Some(else_id) = else_id {
            self.lower_statement_expression(else_id)?
        } else {
            Terminates::No
        };
        if !else_terminated.is_yes() {
            self.state.builder.jump(join_block);
        }

        // join
        if then_terminated.is_yes() && else_terminated.is_yes() {
            return Ok(Terminates::Yes);
        }
        self.state.builder.switch_to_block(join_block);
        Ok(Terminates::No)
    }

    /// Lower a loop statement (while, do-while, or infinite loop).
    fn lower_loop_statement(
        &mut self,
        kind: dir::LoopKind,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        body_id: dir::LocalNodeId<dir::Block>,
        loop_symbol_id: dir::LocalSymbolId,
        label_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Terminates> {
        let header_block = self.state.builder.block();
        let body_block = self.state.builder.block();
        let exit_block = self.state.builder.block();

        // register loop context for break/continue
        let global_loop_symbol_id = loop_symbol_id.into_global(self.context.module_id);
        let loop_context = LoopContext {
            continue_block: header_block,
            break_block: exit_block,
        };
        self.state
            .control
            .loops_by_symbol
            .insert(global_loop_symbol_id, loop_context);
        self.state.control.loop_stack.push(loop_context);
        self.state.control.break_stack.push(BreakContext {
            break_block: exit_block,
        });
        if let Some(label_symbol) = label_symbol {
            self.state
                .control
                .loops_by_symbol
                .insert(label_symbol, loop_context);
        }

        match kind {
            dir::LoopKind::NoTest => {
                // infinite loop: jump to body, loop back unconditionally
                self.state.builder.jump(body_block);

                self.state.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.state.builder.jump(body_block);
                }
            }
            dir::LoopKind::PreTest => {
                // while loop: check condition first, then body
                self.state.builder.jump(header_block);

                // header: evaluate condition and branch
                self.state.builder.switch_to_block(header_block);
                let condition_id = condition
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            body_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "while loop missing condition".to_string(),
                    })
                    .map_err(CompilerError::from)?;

                // branch on the condition (with union tag checks when possible)
                let did_check = self.lower_union_tag_check(condition_id, body_block, exit_block)?;
                if !did_check {
                    // lower the condition value
                    let (condition_value, condition_type) =
                        self.lower_value_expression(condition_id)?;
                    self.check_type_is_bool(condition_id, condition_type, "while loop condition")?;

                    // branch
                    self.state
                        .builder
                        .branch(condition_value, body_block, exit_block);
                }

                // body: execute and loop back
                self.state.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.state.builder.jump(header_block);
                }
            }
            dir::LoopKind::PostTest => {
                // do-while loop: body first, then check condition
                self.state.builder.jump(body_block);

                // body: execute first
                self.state.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.state.builder.jump(header_block);
                }

                // header: evaluate condition and branch back or exit
                self.state.builder.switch_to_block(header_block);
                let condition_id = condition
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            body_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "do-while loop missing condition".to_string(),
                    })
                    .map_err(CompilerError::from)?;

                // branch on the condition (with union tag checks when possible)
                let did_check = self.lower_union_tag_check(condition_id, body_block, exit_block)?;
                if !did_check {
                    // lower the condition value
                    let (condition_value, condition_type) =
                        self.lower_value_expression(condition_id)?;
                    self.check_type_is_bool(
                        condition_id,
                        condition_type,
                        "do-while loop condition",
                    )?;

                    // branch
                    self.state
                        .builder
                        .branch(condition_value, body_block, exit_block);
                }
            }
        }

        // cleanup loop context
        self.state.control.loop_stack.pop();
        self.state.control.break_stack.pop();
        self.state
            .control
            .loops_by_symbol
            .remove(&global_loop_symbol_id);
        if let Some(label_symbol) = label_symbol {
            self.state.control.loops_by_symbol.remove(&label_symbol);
        }

        // continue after the loop
        self.state.builder.switch_to_block(exit_block);
        Ok(Terminates::No)
    }

    /// Lower a for statement (C-style for loop).
    fn lower_for_statement(
        &mut self,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body_id: dir::LocalNodeId<dir::Block>,
        loop_symbol_id: dir::LocalSymbolId,
        label_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Terminates> {
        // lower initialization in current block
        if let Some(init_id) = initialization {
            self.lower_statement_expression(init_id)?;
        }

        let header_block = self.state.builder.block();
        let body_block = self.state.builder.block();
        let increment_block = self.state.builder.block();
        let exit_block = self.state.builder.block();

        // register loop context: continue goes to increment block, break goes to exit
        let global_loop_symbol_id = loop_symbol_id.into_global(self.context.module_id);
        let loop_context = LoopContext {
            continue_block: increment_block,
            break_block: exit_block,
        };
        self.state
            .control
            .loops_by_symbol
            .insert(global_loop_symbol_id, loop_context);
        self.state.control.loop_stack.push(loop_context);
        self.state.control.break_stack.push(BreakContext {
            break_block: exit_block,
        });
        if let Some(label_symbol) = label_symbol {
            self.state
                .control
                .loops_by_symbol
                .insert(label_symbol, loop_context);
        }

        // jump to header to start loop
        self.state.builder.jump(header_block);

        // header: evaluate condition (if present) and branch
        self.state.builder.switch_to_block(header_block);
        if let Some(condition_id) = condition {
            // branch on the condition (with union tag checks when possible)
            let did_check = self.lower_union_tag_check(condition_id, body_block, exit_block)?;
            if !did_check {
                // lower the condition value
                let (condition_value, condition_type) =
                    self.lower_value_expression(condition_id)?;
                self.check_type_is_bool(condition_id, condition_type, "for loop condition")?;

                // branch
                self.state
                    .builder
                    .branch(condition_value, body_block, exit_block);
            }
        } else {
            // no condition means infinite loop (like for(;;))
            self.state.builder.jump(body_block);
        }

        // body: execute statements then jump to increment
        self.state.builder.switch_to_block(body_block);
        let body_terminated = self.lower_block_body(body_id)?;
        if !body_terminated.is_yes() {
            self.state.builder.jump(increment_block);
        }

        // increment block: evaluate increment and loop back to header
        self.state.builder.switch_to_block(increment_block);
        if let Some(increment_id) = increment {
            self.lower_value_expression(increment_id)?;
        }
        self.state.builder.jump(header_block);

        // cleanup loop context
        self.state.control.loop_stack.pop();
        self.state.control.break_stack.pop();
        self.state
            .control
            .loops_by_symbol
            .remove(&global_loop_symbol_id);
        if let Some(label_symbol) = label_symbol {
            self.state.control.loops_by_symbol.remove(&label_symbol);
        }

        // continue after the loop
        self.state.builder.switch_to_block(exit_block);
        Ok(Terminates::No)
    }

    /// Lower a break statement.
    fn lower_break_statement(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Terminates> {
        // labeled break
        let break_block = if let Some(target_symbol) = target_symbol {
            if let Some(label_context) = self.state.control.labels_by_symbol.get(&target_symbol) {
                label_context.break_block
            } else if let Some(loop_context) =
                self.state.control.loops_by_symbol.get(&target_symbol)
            {
                loop_context.break_block
            } else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "break outside of loop or label".to_string(),
                }
                .into());
            }
        }
        // unlabeled break
        else {
            let context = self
                .state
                .control
                .break_stack
                .last()
                .copied()
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "break outside of loop or switch".to_string(),
                })
                .map_err(CompilerError::from)?;
            context.break_block
        };
        self.state.builder.jump(break_block);
        Ok(Terminates::Yes)
    }

    /// Lower a continue statement.
    fn lower_continue_statement(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Terminates> {
        let loop_context = self.resolve_loop_context(expression_id, target_symbol)?;
        self.state.builder.jump(loop_context.continue_block);
        Ok(Terminates::Yes)
    }

    /// Resolve the loop context for break/continue statements.
    fn resolve_loop_context(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<LoopContext> {
        // labeled: look up by symbol, unlabeled: use innermost from stack
        let context = match target_symbol {
            Some(symbol) => self.state.control.loops_by_symbol.get(&symbol).copied(),
            None => self.state.control.loop_stack.last().copied(),
        };

        context
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "continue outside of loop".to_string(),
            })
            .map_err(CompilerError::from)
    }

    /// Lower a block body (list of expressions).
    fn lower_block_body(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Terminates> {
        let block = self.context.dir_tree.get(block_id);
        for expr_id in block.iter_expressions() {
            let terminated = self.lower_statement_expression(expr_id)?;
            if terminated.is_yes() {
                return Ok(Terminates::Yes);
            }
        }
        Ok(Terminates::No)
    }

    /// Lower a switch statement.
    ///
    /// For switches on scalar values, generates a chain of comparisons.
    fn lower_match_statement(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        form: dir::MatchForm,
        value_id: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
        _symbol: dir::LocalSymbolId,
    ) -> CompilerResult<Terminates> {
        // match expressions must be elaborated before lowering
        if form == dir::MatchForm::Match {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "match expressions must be elaborated before lowering".to_string(),
            }
            .into());
        }

        // lower the match value
        let (match_value, match_type) = self.lower_value_expression(value_id)?;

        // create blocks
        let exit_block = self.state.builder.block();
        let case_blocks: Vec<_> = cases.iter().map(|_| self.state.builder.block()).collect();
        self.state.control.break_stack.push(BreakContext {
            break_block: exit_block,
        });

        // analyze cases: find default and collect pattern case indices
        let mut default_index: Option<usize> = None;
        let mut pattern_cases: Vec<usize> = Vec::new();

        for (i, case_id) in cases.iter().enumerate() {
            let case = self.context.dir_tree.get(*case_id);
            match Self::selector_of(case) {
                dir::MatchSelector::Default => {
                    if default_index.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "multiple default cases".to_string(),
                        }
                        .into());
                    }
                    default_index = Some(i);
                }
                dir::MatchSelector::Pattern { .. } => {
                    pattern_cases.push(i);
                }
            }
        }

        // fallthrough target when no pattern matches
        let no_match_block = default_index.map_or(exit_block, |idx| case_blocks[idx]);

        // generate comparison chain for pattern cases
        let mut current_block = self.state.builder.current_block();
        for (pos, &case_index) in pattern_cases.iter().enumerate() {
            let case = self.context.dir_tree.get(cases[case_index]);
            let dir::MatchSelector::Pattern { pattern, guard } = Self::selector_of(case) else {
                unreachable!()
            };

            // guards not supported
            if guard.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "match guards not supported".to_string(),
                }
                .into());
            }

            // determine where to go if this pattern doesn't match
            let is_last_pattern = pos + 1 == pattern_cases.len();
            let next_check = if is_last_pattern {
                no_match_block
            } else {
                self.state.builder.block()
            };

            match self.context.dir_tree.get(*pattern) {
                dir::Pattern::Wildcard => {
                    // wildcard matches everything
                    self.state.builder.switch_to_block(current_block);
                    self.state.builder.jump(case_blocks[case_index]);
                    break;
                }
                dir::Pattern::Expression { value } => {
                    // lower the pattern value
                    let (pattern_value, _) = self.lower_value_expression(*value)?;

                    // emit comparison and branch
                    self.state.builder.switch_to_block(current_block);
                    let eq_op = self.equality_op_for_type(match_type);
                    let cmp = self
                        .state
                        .builder
                        .binary_op(eq_op, match_value, pattern_value);
                    self.state
                        .builder
                        .branch(cmp, case_blocks[case_index], next_check);

                    current_block = next_check;
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported switch pattern".to_string(),
                    }
                    .into());
                }
            }
        }

        // lower each case body
        let mut all_terminate = true;
        for (i, case_id) in cases.iter().enumerate() {
            let case = self.context.dir_tree.get(*case_id);
            self.state.builder.switch_to_block(case_blocks[i]);

            let terminated = match Self::body_of(case) {
                Err(expr_body) => self.lower_statement_expression(expr_body)?,
                Ok(block_body) => self.lower_block_body(block_body)?,
            };

            if !terminated.is_yes() {
                let fallthrough_target =
                    if form == dir::MatchForm::Switch && i + 1 < case_blocks.len() {
                        case_blocks[i + 1]
                    } else {
                        exit_block
                    };
                self.state.builder.jump(fallthrough_target);
                all_terminate = false;
            }
        }

        // switch to exit block for continuation
        if !all_terminate || default_index.is_none() {
            self.state.builder.switch_to_block(exit_block);
        }

        self.state.control.break_stack.pop();

        // terminates only if all cases terminate and there's a default
        let terminates = all_terminate && default_index.is_some();
        Ok(if terminates {
            Terminates::Yes
        } else {
            Terminates::No
        })
    }

    /// Extract the selector from a match case.
    fn selector_of(case: &dir::MatchCase) -> &dir::MatchSelector {
        match case {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        }
    }

    /// Extract the body from a match case.
    ///
    /// Returns `Err(expression)` for expression bodies, `Ok(block)` for block bodies.
    fn body_of(
        case: &dir::MatchCase,
    ) -> Result<dir::LocalNodeId<dir::Block>, dir::LocalNodeId<dir::Expression>> {
        match case {
            dir::MatchCase::Expression { body, .. } => Err(*body),
            dir::MatchCase::Block { body, .. } => Ok(*body),
        }
    }

    /// Return the equality operator for a MIR type.
    fn equality_op_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> mir::BinaryOperator {
        if matches!(self.state.builder.tree().get(ty), mir::Type::Float { .. }) {
            mir::BinaryOperator::FloatEqual
        } else {
            mir::BinaryOperator::Equal
        }
    }

    /// Bind a pattern to a value in the current block.
    pub(crate) fn bind_pattern_value(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
        mutability: Option<dir::Mutability>,
    ) -> CompilerResult<()> {
        // read the pattern
        let pattern = self.context.dir_tree.get(pattern_id);
        match pattern {
            dir::Pattern::Wildcard => Ok(()),
            dir::Pattern::Binding {
                symbol, pattern, ..
            } => {
                if pattern.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            pattern_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported binding pattern".to_string(),
                    }
                    .into());
                }
                self.define_local_binding(
                    pattern_id.into_any(),
                    *symbol,
                    mutability,
                    value,
                    value_type,
                )
            }
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    pattern_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "unsupported let pattern".to_string(),
            }
            .into()),
        }
    }

    /// Bind a pattern to uninitialized storage in the current block.
    fn bind_uninitialized_pattern(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        type_expression: Option<dir::LocalNodeId<dir::TypeExpression>>,
        mutability: Option<dir::Mutability>,
    ) -> CompilerResult<()> {
        // read the pattern
        let pattern = self.context.dir_tree.get(pattern_id);
        match pattern {
            dir::Pattern::Wildcard => Ok(()),
            dir::Pattern::Binding {
                symbol, pattern, ..
            } => {
                if pattern.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            pattern_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported binding pattern".to_string(),
                    }
                    .into());
                }

                // resolve the local type and allocate storage without an initializer
                let value_type = self.uninitialized_binding_type(
                    expression_id,
                    pattern_id,
                    *symbol,
                    type_expression,
                )?;
                self.define_uninitialized_local_binding(
                    pattern_id.into_any(),
                    *symbol,
                    mutability,
                    value_type,
                )
            }
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    pattern_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "unsupported let pattern".to_string(),
            }
            .into()),
        }
    }

    /// Resolve the MIR type for an uninitialized local binding.
    fn uninitialized_binding_type(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        symbol_id: dir::LocalSymbolId,
        _type_expression: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the analyzed value type for the symbol
        let symbol = symbol_id.into_global(self.context.module_id);
        let type_id = self.value_type_id_for_symbol_or_error(pattern_id.into_any(), symbol)?;
        let type_id = self.unwrap_value_type_id(type_id);

        // use prelowered aggregate/reference types when available
        if let Some(mir_type) = self.context.type_lowerer.cached_type(type_id) {
            return Ok(mir_type);
        }

        // fall back to scalar and string builtins
        let dir_type = self.context.types.get_type(type_id);
        if let Some(scalar_type) = self.context.type_lowerer.scalar_type_for_dir_type(dir_type) {
            return match scalar_type {
                ScalarType::Bool => Ok(self.context.type_lowerer.ty_bool),
                ScalarType::SignedInt { width: 32 } => Ok(self.context.type_lowerer.ty_i32),
                ScalarType::SignedInt { width: 64 } => Ok(self.context.type_lowerer.ty_i64),
                ScalarType::UnsignedInt { width: 32 } => Ok(self.context.type_lowerer.ty_u32),
                ScalarType::UnsignedInt { width }
                    if width == self.context.type_lowerer.pointer_width_bits() =>
                {
                    Ok(self.context.type_lowerer.ty_usize)
                }
                ScalarType::Float { width: 32 } => Ok(self.context.type_lowerer.ty_f32),
                ScalarType::Float { width: 64 } => Ok(self.context.type_lowerer.ty_f64),
                _ => Err(self.missing_type_error(expression_id).into()),
            };
        }
        if matches!(
            dir_type,
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::String)
            })
        ) && let Some(string_type) = self.context.type_lowerer.string_type()
        {
            return Ok(string_type);
        }

        Err(self.missing_type_error(expression_id).into())
    }

    /// Define a local binding without an initializer value.
    fn define_uninitialized_local_binding(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        mutability: Option<dir::Mutability>,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // convert the symbol id for lookup
        let global_symbol_id = symbol_id.into_global(self.context.module_id);

        // reject duplicate bindings
        if self
            .state
            .bindings
            .locals_by_symbol
            .contains_key(&global_symbol_id)
        {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    node_id.into_anchored(self.context.module_id, Some(self.context.profile)),
                ),
                message: "duplicate local binding".to_string(),
            }
            .into());
        }

        // use boxed capture cell for by-reference locals
        if self.symbol_needs_reference_cell(global_symbol_id) {
            // allocate a reference cell and defer pointee initialization
            let mir_mutability = mutability
                .map(lower_mutability)
                .unwrap_or(mir::Mutability::Immutable);
            let reference_type = self.state.builder.type_reference(
                mir::ReferenceKind::Managed,
                value_type,
                mir_mutability,
                mir::AddressSpace::Local,
                false,
            );
            let reference_value = self.state.builder.new_(value_type, reference_type);

            let variable = self.state.builder.variable(reference_type);
            self.state
                .builder
                .define_variable(variable, reference_value);
            self.state.bindings.locals_by_symbol.insert(
                global_symbol_id,
                LocalBinding::indirect_binding(variable, reference_type, value_type),
            );
            return Ok(());
        }

        // use local storage so later assignments can initialize the value
        let local_mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let local = self.state.builder.local(value_type, local_mutability);
        self.state
            .bindings
            .locals_by_symbol
            .insert(global_symbol_id, LocalBinding::local(local, value_type));
        Ok(())
    }

    /// Define a local binding for a symbol.
    pub(crate) fn define_local_binding(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        mutability: Option<dir::Mutability>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // convert the symbol id for lookup
        let global_symbol_id = symbol_id.into_global(self.context.module_id);

        // reject duplicate bindings
        // (this is different from re-declared _names_ since those have different symbol ids)
        if self
            .state
            .bindings
            .locals_by_symbol
            .contains_key(&global_symbol_id)
        {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    node_id.into_anchored(self.context.module_id, Some(self.context.profile)),
                ),
                message: "duplicate local binding".to_string(),
            }
            .into());
        }

        // use boxed capture cell for by-reference locals
        if self.symbol_needs_reference_cell(global_symbol_id) {
            // wrap value in reference type
            let mir_mutability = mutability
                .map(lower_mutability)
                .unwrap_or(mir::Mutability::Immutable);
            let reference_type = self.state.builder.type_reference(
                mir::ReferenceKind::Managed,
                value_type,
                mir_mutability,
                mir::AddressSpace::Local,
                false,
            );
            let reference_value = self.state.builder.new_(value_type, reference_type);
            self.state.builder.store(reference_value, value);

            // allocate indirect binding
            let variable = self.state.builder.variable(reference_type);
            self.state
                .builder
                .define_variable(variable, reference_value);
            self.state.bindings.locals_by_symbol.insert(
                global_symbol_id,
                LocalBinding::indirect_binding(variable, reference_type, value_type),
            );
            return Ok(());
        }

        // use stack slot for address taken locals
        if self.symbol_needs_addressable_local(global_symbol_id) {
            let mutability = mutability
                .map(lower_mutability)
                .unwrap_or(mir::Mutability::Immutable);
            let local = self.state.builder.local(value_type, mutability);
            self.state.builder.local_set(local, value);
            self.state
                .bindings
                .locals_by_symbol
                .insert(global_symbol_id, LocalBinding::local(local, value_type));
            return Ok(());
        }

        // allocate the mir variable
        let variable = self.state.builder.variable(value_type);

        // define the initial value
        self.state.builder.define_variable(variable, value);

        // record the binding for later references
        self.state.bindings.locals_by_symbol.insert(
            global_symbol_id,
            LocalBinding::variable(variable, value_type),
        );

        Ok(())
    }

    /// Check that a type is boolean, returning an error otherwise.
    pub(crate) fn check_type_is_bool(
        &self,
        node_id: dir::LocalNodeId<dir::Expression>,
        ty: mir::LocalNodeId<mir::Type>,
        context: &str,
    ) -> CompilerResult<()> {
        if ty != self.context.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    node_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: format!("{context} requires boolean"),
            }
            .into());
        }
        Ok(())
    }
}
