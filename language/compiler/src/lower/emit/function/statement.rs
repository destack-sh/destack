use destack_dir::{
    Declarator, Expression, GlobalSymbolId, IfCondition, LocalNodeId, LocalNodeIdAny, MatchCase,
    MatchKind, MatchSelector, Pattern,
};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::{BreakContext, LocalBinding, LoopContext, Terminates};
use super::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a statement expression.
    pub(crate) fn lower_statement_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<Terminates> {
        match self.dir_tree.get(expression_id) {
            Expression::Statement { statement } => self.lower_statement_expression(*statement),
            Expression::Labelled { body, symbol, .. } => {
                self.lower_labelled_statement(expression_id, *symbol, *body)
            }
            Expression::Block { block } => {
                let block = self.dir_tree.get(*block);
                for expr_id in &block.expressions {
                    let terminated = self.lower_statement_expression(*expr_id)?;
                    if terminated.is_yes() {
                        return Ok(Terminates::Yes);
                    }
                }
                Ok(Terminates::No)
            }
            Expression::Let { declarators, .. } => {
                self.lower_let_expression(expression_id, declarators)
            }
            Expression::Return { value } => {
                // handle constructor returns separately
                if self.constructor_state.is_some() {
                    if value.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "constructor cannot return a value".to_string(),
                        });
                    }

                    let node = expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile));
                    self.return_constructor_value(node)?;
                    return Ok(Terminates::Yes);
                }

                // lower standard returns
                let return_value = if let Some(value) = value {
                    let (value, _) = self.lower_value_expression(*value)?;
                    Some(value)
                } else {
                    None
                };
                self.builder.return_(return_value);
                Ok(Terminates::Yes)
            }
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => match condition {
                IfCondition::Expression { condition } => {
                    self.lower_if_statement(*condition, *then_expression, *else_expression)
                }
                IfCondition::Let { .. } => {
                    // if let should be elaborated before lowering
                    Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported if-let condition".to_string(),
                    })
                }
            },

            Expression::Loop {
                kind,
                condition,
                body,
                symbol,
                ..
            } => self.lower_loop_statement(*kind, *condition, *body, *symbol, None),

            Expression::For {
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

            Expression::Break { target_symbol, .. } => {
                self.lower_break_statement(expression_id, *target_symbol)
            }

            Expression::Continue { target_symbol, .. } => {
                self.lower_continue_statement(expression_id, *target_symbol)
            }

            Expression::Match {
                kind,
                value,
                cases,
                symbol,
                ..
            } => self.lower_match_statement(expression_id, *kind, *value, cases, *symbol),

            _ => {
                let _ = self.lower_value_expression(expression_id)?;
                Ok(Terminates::No)
            }
        }
    }

    /// Lower a let binding statement into locals.
    fn lower_let_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        declarators: &[LocalNodeId<Declarator>],
    ) -> LowerResult<Terminates> {
        // lower each declarator in order
        for declarator_id in declarators {
            let declarator = self.dir_tree.get(*declarator_id);

            // require an initializer for native lowering
            let value_id = declarator
                .value
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing let initializer".to_string(),
                })?;

            // lower the initializer value first
            let (value, value_type) = self.lower_value_expression(value_id)?;

            // lower the binding pattern
            let pattern_id = declarator.pattern;
            self.bind_pattern_value(pattern_id, value, value_type)?;
        }

        Ok(Terminates::No)
    }

    /// Lower a labelled statement with an explicit break target.
    fn lower_labelled_statement(
        &mut self,
        _expression_id: LocalNodeId<Expression>,
        label_symbol: dir::LocalSymbolId,
        body_id: LocalNodeId<Expression>,
    ) -> LowerResult<Terminates> {
        let label_symbol = label_symbol.into_global(self.module_id);
        let body = self.dir_tree.get(body_id);

        match body {
            Expression::Loop {
                kind,
                condition,
                body,
                symbol,
                ..
            } => self.lower_loop_statement(*kind, *condition, *body, *symbol, Some(label_symbol)),
            Expression::For {
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
                let break_block = self.builder.create_block();
                let label_context = BreakContext { break_block };
                self.labels_by_symbol.insert(label_symbol, label_context);

                let terminated = self.lower_statement_expression(body_id)?;
                if !terminated.is_yes() {
                    self.builder.jump(break_block);
                }

                self.labels_by_symbol.remove(&label_symbol);
                self.builder.switch_to_block(break_block);
                Ok(Terminates::No)
            }
        }
    }

    /// Lower an if statement into blocks and branches.
    fn lower_if_statement(
        &mut self,
        condition_id: LocalNodeId<Expression>,
        then_id: LocalNodeId<Expression>,
        else_id: Option<LocalNodeId<Expression>>,
    ) -> LowerResult<Terminates> {
        let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
        self.check_type_is_bool(condition_id, condition_type, "if condition")?;

        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let join_block = self.builder.create_block();

        // branch
        self.builder.branch(condition_value, then_block, else_block);

        // then branch
        self.builder.switch_to_block(then_block);
        let then_terminated = self.lower_statement_expression(then_id)?;
        if !then_terminated.is_yes() {
            self.builder.jump(join_block);
        }

        // else branch
        self.builder.switch_to_block(else_block);
        let else_terminated = if let Some(else_id) = else_id {
            self.lower_statement_expression(else_id)?
        } else {
            Terminates::No
        };
        if !else_terminated.is_yes() {
            self.builder.jump(join_block);
        }

        // join
        if then_terminated.is_yes() && else_terminated.is_yes() {
            return Ok(Terminates::Yes);
        }
        self.builder.switch_to_block(join_block);
        Ok(Terminates::No)
    }

    /// Lower a loop statement (while, do-while, or infinite loop).
    fn lower_loop_statement(
        &mut self,
        kind: dir::LoopKind,
        condition: Option<LocalNodeId<Expression>>,
        body_id: LocalNodeId<dir::Block>,
        loop_symbol_id: dir::LocalSymbolId,
        label_symbol: Option<GlobalSymbolId>,
    ) -> LowerResult<Terminates> {
        let header_block = self.builder.create_block();
        let body_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        // register loop context for break/continue
        let global_loop_symbol_id = loop_symbol_id.into_global(self.module_id);
        let loop_context = LoopContext {
            continue_block: header_block,
            break_block: exit_block,
        };
        self.loops_by_symbol
            .insert(global_loop_symbol_id, loop_context);
        self.loop_stack.push(loop_context);
        self.break_stack.push(BreakContext {
            break_block: exit_block,
        });
        if let Some(label_symbol) = label_symbol {
            self.loops_by_symbol.insert(label_symbol, loop_context);
        }

        match kind {
            dir::LoopKind::NoTest => {
                // infinite loop: jump to body, loop back unconditionally
                self.builder.jump(body_block);

                self.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.builder.jump(body_block);
                }
            }
            dir::LoopKind::PreTest => {
                // while loop: check condition first, then body
                self.builder.jump(header_block);

                // header: evaluate condition and branch
                self.builder.switch_to_block(header_block);
                let condition_id = condition.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: body_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "while loop missing condition".to_string(),
                })?;
                let (condition_value, condition_type) =
                    self.lower_value_expression(condition_id)?;
                self.check_type_is_bool(condition_id, condition_type, "while loop condition")?;
                self.builder.branch(condition_value, body_block, exit_block);

                // body: execute and loop back
                self.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.builder.jump(header_block);
                }
            }
            dir::LoopKind::PostTest => {
                // do-while loop: body first, then check condition
                self.builder.jump(body_block);

                // body: execute first
                self.builder.switch_to_block(body_block);
                let body_terminated = self.lower_block_body(body_id)?;
                if !body_terminated.is_yes() {
                    self.builder.jump(header_block);
                }

                // header: evaluate condition and branch back or exit
                self.builder.switch_to_block(header_block);
                let condition_id = condition.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: body_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "do-while loop missing condition".to_string(),
                })?;
                let (condition_value, condition_type) =
                    self.lower_value_expression(condition_id)?;
                self.check_type_is_bool(condition_id, condition_type, "do-while loop condition")?;
                self.builder.branch(condition_value, body_block, exit_block);
            }
        }

        // cleanup loop context
        self.loop_stack.pop();
        self.break_stack.pop();
        self.loops_by_symbol.remove(&global_loop_symbol_id);
        if let Some(label_symbol) = label_symbol {
            self.loops_by_symbol.remove(&label_symbol);
        }

        // continue after the loop
        self.builder.switch_to_block(exit_block);
        Ok(Terminates::No)
    }

    /// Lower a for statement (C-style for loop).
    fn lower_for_statement(
        &mut self,
        initialization: Option<LocalNodeId<Expression>>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body_id: LocalNodeId<dir::Block>,
        loop_symbol_id: dir::LocalSymbolId,
        label_symbol: Option<GlobalSymbolId>,
    ) -> LowerResult<Terminates> {
        // lower initialization in current block
        if let Some(init_id) = initialization {
            self.lower_statement_expression(init_id)?;
        }

        let header_block = self.builder.create_block();
        let body_block = self.builder.create_block();
        let increment_block = self.builder.create_block();
        let exit_block = self.builder.create_block();

        // register loop context: continue goes to increment block, break goes to exit
        let global_loop_symbol_id = loop_symbol_id.into_global(self.module_id);
        let loop_context = LoopContext {
            continue_block: increment_block,
            break_block: exit_block,
        };
        self.loops_by_symbol
            .insert(global_loop_symbol_id, loop_context);
        self.loop_stack.push(loop_context);
        self.break_stack.push(BreakContext {
            break_block: exit_block,
        });
        if let Some(label_symbol) = label_symbol {
            self.loops_by_symbol.insert(label_symbol, loop_context);
        }

        // jump to header to start loop
        self.builder.jump(header_block);

        // header: evaluate condition (if present) and branch
        self.builder.switch_to_block(header_block);
        if let Some(condition_id) = condition {
            let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
            self.check_type_is_bool(condition_id, condition_type, "for loop condition")?;
            self.builder.branch(condition_value, body_block, exit_block);
        } else {
            // no condition means infinite loop (like for(;;))
            self.builder.jump(body_block);
        }

        // body: execute statements then jump to increment
        self.builder.switch_to_block(body_block);
        let body_terminated = self.lower_block_body(body_id)?;
        if !body_terminated.is_yes() {
            self.builder.jump(increment_block);
        }

        // increment block: evaluate increment and loop back to header
        self.builder.switch_to_block(increment_block);
        if let Some(increment_id) = increment {
            self.lower_value_expression(increment_id)?;
        }
        self.builder.jump(header_block);

        // cleanup loop context
        self.loop_stack.pop();
        self.break_stack.pop();
        self.loops_by_symbol.remove(&global_loop_symbol_id);
        if let Some(label_symbol) = label_symbol {
            self.loops_by_symbol.remove(&label_symbol);
        }

        // continue after the loop
        self.builder.switch_to_block(exit_block);
        Ok(Terminates::No)
    }

    /// Lower a break statement.
    fn lower_break_statement(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: Option<GlobalSymbolId>,
    ) -> LowerResult<Terminates> {
        // labeled break
        let break_block = if let Some(target_symbol) = target_symbol {
            if let Some(label_context) = self.labels_by_symbol.get(&target_symbol) {
                label_context.break_block
            } else if let Some(loop_context) = self.loops_by_symbol.get(&target_symbol) {
                loop_context.break_block
            } else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "break outside of loop or label".to_string(),
                });
            }
        }
        // unlabeled break
        else {
            let context = self.break_stack.last().copied().ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "break outside of loop or switch".to_string(),
                }
            })?;
            context.break_block
        };
        self.builder.jump(break_block);
        Ok(Terminates::Yes)
    }

    /// Lower a continue statement.
    fn lower_continue_statement(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: Option<GlobalSymbolId>,
    ) -> LowerResult<Terminates> {
        let loop_context = self.resolve_loop_context(expression_id, target_symbol)?;
        self.builder.jump(loop_context.continue_block);
        Ok(Terminates::Yes)
    }

    /// Resolve the loop context for break/continue statements.
    fn resolve_loop_context(
        &self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: Option<GlobalSymbolId>,
    ) -> LowerResult<LoopContext> {
        // labeled: look up by symbol, unlabeled: use innermost from stack
        let context = match target_symbol {
            Some(symbol) => self.loops_by_symbol.get(&symbol).copied(),
            None => self.loop_stack.last().copied(),
        };

        context.ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile)),
            message: "continue outside of loop".to_string(),
        })
    }

    /// Lower a block body (list of expressions).
    fn lower_block_body(&mut self, block_id: LocalNodeId<dir::Block>) -> LowerResult<Terminates> {
        let block = self.dir_tree.get(block_id);
        for expr_id in &block.expressions {
            let terminated = self.lower_statement_expression(*expr_id)?;
            if terminated.is_yes() {
                return Ok(Terminates::Yes);
            }
        }
        Ok(Terminates::No)
    }

    /// Lower a match/switch statement.
    ///
    /// For simple switches on scalar values, generates a chain of comparisons.
    fn lower_match_statement(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        kind: MatchKind,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        _symbol: dir::LocalSymbolId,
    ) -> LowerResult<Terminates> {
        // lower the match value
        let (match_value, match_type) = self.lower_value_expression(value_id)?;

        // create blocks
        let exit_block = self.builder.create_block();
        let case_blocks: Vec<_> = cases.iter().map(|_| self.builder.create_block()).collect();
        if kind == MatchKind::Switch {
            self.break_stack.push(BreakContext {
                break_block: exit_block,
            });
        }

        // analyze cases: find default and collect pattern case indices
        let mut default_index: Option<usize> = None;
        let mut pattern_cases: Vec<usize> = Vec::new();

        for (i, case_id) in cases.iter().enumerate() {
            let case = self.dir_tree.get(*case_id);
            match Self::selector_of(case) {
                MatchSelector::Default => {
                    if default_index.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "multiple default cases".to_string(),
                        });
                    }
                    default_index = Some(i);
                }
                MatchSelector::Pattern { .. } => {
                    pattern_cases.push(i);
                }
            }
        }

        // fallthrough target when no pattern matches
        let no_match_block = default_index.map_or(exit_block, |idx| case_blocks[idx]);

        // generate comparison chain for pattern cases
        let mut current_block = self.builder.current_block();
        for (pos, &case_index) in pattern_cases.iter().enumerate() {
            let case = self.dir_tree.get(cases[case_index]);
            let MatchSelector::Pattern { pattern, guard } = Self::selector_of(case) else {
                unreachable!()
            };

            // guards not supported
            if guard.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "match guards not supported".to_string(),
                });
            }

            // determine where to go if this pattern doesn't match
            let is_last_pattern = pos + 1 == pattern_cases.len();
            let next_check = if is_last_pattern {
                no_match_block
            } else {
                self.builder.create_block()
            };

            let pattern = self.dir_tree.get(*pattern);
            match pattern {
                Pattern::Wildcard => {
                    // wildcard matches everything
                    self.builder.switch_to_block(current_block);
                    self.builder.jump(case_blocks[case_index]);
                    break;
                }
                Pattern::Expression { value } => {
                    let (pattern_value, _) = self.lower_value_expression(*value)?;

                    // emit comparison and branch
                    self.builder.switch_to_block(current_block);
                    let eq_op = self.equality_op_for_type(match_type);
                    let cmp = self.builder.binary_op(eq_op, match_value, pattern_value);
                    self.builder
                        .branch(cmp, case_blocks[case_index], next_check);

                    current_block = next_check;
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported match pattern".to_string(),
                    });
                }
            }
        }

        // lower each case body
        let mut all_terminate = true;
        for (i, case_id) in cases.iter().enumerate() {
            let case = self.dir_tree.get(*case_id);
            self.builder.switch_to_block(case_blocks[i]);

            let terminated = match Self::body_of(case) {
                Err(expr_body) => self.lower_statement_expression(expr_body)?,
                Ok(block_body) => self.lower_block_body(block_body)?,
            };

            if !terminated.is_yes() {
                let fallthrough_target = if kind == MatchKind::Switch && i + 1 < case_blocks.len() {
                    case_blocks[i + 1]
                } else {
                    exit_block
                };
                self.builder.jump(fallthrough_target);
                all_terminate = false;
            }
        }

        // switch to exit block for continuation
        if !all_terminate || default_index.is_none() {
            self.builder.switch_to_block(exit_block);
        }

        if kind == MatchKind::Switch {
            self.break_stack.pop();
        }

        // terminates only if all cases terminate and there's a default
        let terminates = all_terminate && default_index.is_some();
        Ok(if terminates {
            Terminates::Yes
        } else {
            Terminates::No
        })
    }

    /// Extract the selector from a match case.
    fn selector_of(case: &MatchCase) -> &MatchSelector {
        match case {
            MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => selector,
        }
    }

    /// Extract the body from a match case.
    ///
    /// Returns `Err(expression)` for expression bodies, `Ok(block)` for block bodies.
    fn body_of(case: &MatchCase) -> Result<LocalNodeId<dir::Block>, LocalNodeId<Expression>> {
        match case {
            MatchCase::Expression { body, .. } => Err(*body),
            MatchCase::Block { body, .. } => Ok(*body),
        }
    }

    /// Return the equality operator for a MIR type.
    fn equality_op_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> mir::BinaryOperator {
        if matches!(self.builder.tree().get(ty), mir::Type::Float { .. }) {
            mir::BinaryOperator::FloatEqual
        } else {
            mir::BinaryOperator::Equal
        }
    }

    /// Bind a pattern to a value in the current block.
    pub(crate) fn bind_pattern_value(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        // read the pattern
        let pattern = self.dir_tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Binding {
                symbol, pattern, ..
            } => {
                if pattern.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: pattern_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported binding pattern".to_string(),
                    });
                }
                self.define_local_binding(pattern_id.into_any(), *symbol, value, value_type)
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: pattern_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "unsupported let pattern".to_string(),
            }),
        }
    }

    /// Define a local binding for a symbol.
    pub(crate) fn define_local_binding(
        &mut self,
        node_id: LocalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        // convert the symbol id for lookup
        let global_symbol_id = symbol_id.into_global(self.module_id);

        // reject duplicate bindings
        if self.locals_by_symbol.contains_key(&global_symbol_id) {
            return Err(LowerError::UnsupportedConstruct {
                node: node_id.into_anchored(self.module_id, Some(self.profile)),
                message: "duplicate local binding".to_string(),
            });
        }

        // allocate the mir variable
        let variable = self.builder.create_variable(value_type);

        // define the initial value
        self.builder.define_variable(variable, value);

        // record the binding for later references
        self.locals_by_symbol.insert(
            global_symbol_id,
            LocalBinding {
                variable,
                ty: value_type,
            },
        );

        Ok(())
    }

    /// Check that a type is boolean, returning an error otherwise.
    pub(crate) fn check_type_is_bool(
        &self,
        node_id: LocalNodeId<Expression>,
        ty: mir::LocalNodeId<mir::Type>,
        context: &str,
    ) -> LowerResult<()> {
        if ty != self.type_lowerer.ty_bool {
            return Err(LowerError::UnsupportedConstruct {
                node: node_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!("{context} requires boolean"),
            });
        }
        Ok(())
    }
}
