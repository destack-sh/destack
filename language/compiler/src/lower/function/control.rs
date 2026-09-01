use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::{ChainFrame, ControlFrame};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one if statement, returning whether every arm terminated.
    pub(in crate::lower) fn lower_if(
        &mut self,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        // branch on the condition, with the join serving as the else edge of a bare if
        let condition = self.lower_condition(condition)?;
        let then_block = self.builder.block();
        let join = self.builder.block();
        let else_block = match else_expression {
            Some(_) => self.builder.block(),
            None => join,
        };
        self.builder.branch(condition, then_block, else_block);

        // lower the then arm, joining its fallthrough edge
        self.builder.switch_to_block(then_block);
        let then_terminated = self.lower_arm(then_expression)?;
        if !then_terminated {
            self.builder.jump(join);
        }

        // lower the else arm the same way
        let mut else_terminated = false;
        if let Some(else_expression) = else_expression {
            self.builder.switch_to_block(else_block);
            else_terminated = self.lower_arm(else_expression)?;
            if !else_terminated {
                self.builder.jump(join);
            }
        }

        // leave the join unreachable when both arms terminate
        if then_terminated && else_terminated {
            return Ok(true);
        }

        // continue lowering at the join
        self.builder.switch_to_block(join);

        Ok(false)
    }

    /// Lower one ternary expression, joining the arm values.
    pub(in crate::lower) fn lower_ternary(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<mir::Value> {
        // require the else arm
        let Some(else_expression) = else_expression else {
            return Err(CompilerError::Internal {
                message: "a missing else arm on one ternary".to_string(),
            });
        };

        // branch on the condition with a join local for the arm values
        let condition = self.lower_condition(condition)?;
        let join_value = self.value_slot(expression)?;
        let then_block = self.builder.block();
        let else_block = self.builder.block();
        let join = self.builder.block();
        self.builder.branch(condition, then_block, else_block);

        // write the join local in the then arm
        self.builder.switch_to_block(then_block);
        let value = self.lower_expression(then_expression)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);

        // write the join local in the else arm
        self.builder.switch_to_block(else_block);
        let value = self.lower_expression(else_expression)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);

        // read the arm value at the join
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one while or do-while loop.
    pub(in crate::lower) fn lower_while(
        &mut self,
        label: Option<StringId>,
        form: dir::WhileForm,
        condition: &dir::Condition,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // enter through the condition for while, through the body for do-while
        let header = self.builder.block();
        let body_block = self.builder.block();
        let exit = self.builder.block();
        match form {
            dir::WhileForm::While => self.builder.jump(header),
            dir::WhileForm::DoWhile => self.builder.jump(body_block),
        }

        // re-evaluate the condition in the header on every iteration
        self.builder.switch_to_block(header);
        let condition = self.lower_condition(condition)?;
        self.builder.branch(condition, body_block, exit);

        // lower the body, looping back through the condition header
        self.builder.switch_to_block(body_block);
        let terminated = self.lower_loop_body(label, header, exit, body)?;
        if !terminated {
            self.builder.jump(header);
        }

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one for loop over its initialization, condition and increment.
    pub(in crate::lower) fn lower_for(
        &mut self,
        label: Option<StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // run the initialization once before entering the loop
        if let Some(initialization) = initialization {
            self.lower_statement(initialization)?;
        }

        // enter the loop through the condition header
        let header = self.builder.block();
        let body_block = self.builder.block();
        let continue_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(header);

        // re-evaluate the condition in the header on every iteration
        self.builder.switch_to_block(header);
        match condition {
            Some(condition) => {
                let condition = self.lower_expression(condition)?;
                self.builder.branch(condition, body_block, exit);
            }
            None => self.builder.jump(body_block),
        }

        // lower the body, routing continue through the increment
        self.builder.switch_to_block(body_block);
        let terminated = self.lower_loop_body(label, continue_block, exit, body)?;
        if !terminated {
            self.builder.jump(continue_block);
        }

        // run the increment and loop back to the condition
        self.builder.switch_to_block(continue_block);
        if let Some(increment) = increment {
            self.lower_statement(increment)?;
        }
        self.builder.jump(header);

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one unconditional loop.
    pub(in crate::lower) fn lower_loop(
        &mut self,
        label: Option<StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // loop the body onto itself
        let body_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(body_block);
        self.builder.switch_to_block(body_block);

        // loop back to the body when it falls through
        let terminated = self.lower_loop_body(label, body_block, exit, body)?;
        if !terminated {
            self.builder.jump(body_block);
        }

        // continue lowering after the loop
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one break statement.
    pub(in crate::lower) fn lower_break(
        &mut self,
        label: Option<StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        // reject a break carrying a value
        if value.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a valued break".to_string(),
            }
            .into());
        }

        // jump to the enclosing statement's exit
        let target = self.break_target(label)?;
        self.builder.jump(target);

        Ok(true)
    }

    /// Lower one continue statement.
    pub(in crate::lower) fn lower_continue(
        &mut self,
        label: Option<StringId>,
    ) -> CompilerResult<bool> {
        let target = self.continue_target(label)?;
        self.builder.jump(target);

        Ok(true)
    }

    /// Lower one loop body under its control frame.
    fn lower_loop_body(
        &mut self,
        label: Option<StringId>,
        continue_target: mir::LocalNodeId<mir::Block>,
        exit: mir::LocalNodeId<mir::Block>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        self.enter_control(label, exit, Some(continue_target));
        let terminated = self.lower_block(body)?;
        self.leave_control();

        Ok(terminated)
    }

    /// Lower one condition to a boolean value.
    pub(in crate::lower) fn lower_condition(
        &mut self,
        condition: &dir::Condition,
    ) -> CompilerResult<mir::Value> {
        // reject a binding condition
        let Some(condition) = condition.as_expression() else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a binding condition".to_string(),
            }
            .into());
        };

        self.lower_expression(condition)
    }

    /// Enter one statement's break and continue targets.
    pub(in crate::lower) fn enter_control(
        &mut self,
        label: Option<StringId>,
        break_target: mir::LocalNodeId<mir::Block>,
        continue_target: Option<mir::LocalNodeId<mir::Block>>,
    ) {
        self.controls.push(ControlFrame {
            label,
            break_target,
            continue_target,
        });
    }

    /// Leave the innermost break and continue target.
    pub(in crate::lower) fn leave_control(&mut self) {
        self.controls.pop();
    }

    /// Return the block one break targets.
    fn break_target(
        &self,
        label: Option<StringId>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Block>> {
        // take the labeled frame, or the innermost one for a bare break
        let frame = match label {
            None => self.controls.last(),
            Some(label) => self
                .controls
                .iter()
                .rev()
                .find(|frame| frame.label == Some(label)),
        };

        // require an enclosing breakable statement
        let Some(frame) = frame else {
            return Err(CompilerError::Internal {
                message: "a missing enclosing statement for one break".to_string(),
            });
        };

        Ok(frame.break_target)
    }

    /// Return the block one continue targets.
    fn continue_target(
        &self,
        label: Option<StringId>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Block>> {
        // take the innermost loop frame carrying the label
        let frame = self.controls.iter().rev().find(|frame| {
            frame.continue_target.is_some()
                && match label {
                    Some(label) => frame.label == Some(label),
                    None => true,
                }
        });

        // require an enclosing loop
        let Some(target) = frame.and_then(|frame| frame.continue_target) else {
            return Err(CompilerError::Internal {
                message: "a missing enclosing loop for one continue".to_string(),
            });
        };

        Ok(target)
    }

    /// Lower one optional chain, joining its value with the short-circuit undefined.
    pub(in crate::lower) fn lower_chain(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // stage the joined result slot and the exit block
        let ty = self.node_type_id(expression)?;
        let representation = self.lower_type(ty)?;
        let slot = self
            .builder
            .local(representation, mir::Mutability::Immutable);
        let exit = self.builder.block();

        // lower the chained accesses under the frame their guards exit through
        self.chains.push(ChainFrame {
            representation,
            slot,
            exit,
        });
        let value = self.lower_expression(inner)?;
        self.chains.pop();

        // join the completed chain value
        let value = self.adapt_to_representation(value, representation)?;
        self.builder.local_set(slot, value);
        self.builder.jump(exit);
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Branch one optional receiver, short-circuiting the enclosing chain when it is absent.
    pub(in crate::lower) fn lower_chain_guard(
        &mut self,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // pass receivers through outside a chain
        let Some(frame) = self.chains.last() else {
            return Ok(value);
        };
        let (representation, slot, exit) = (frame.representation, frame.slot, frame.exit);

        // branch the receiver on its undefined case
        let received = self.value_representation(value)?;
        let present = self.builder.block();
        let absent = self.builder.block();
        match self.builder.tree().get(received).clone() {
            // split a variant receiver on its undefined case
            mir::Type::Variant { .. } => {
                let Some(mir::NullishCase::Case(case)) =
                    self.builder.tree().undefined_case(received)
                else {
                    return Ok(value);
                };
                self.builder
                    .variant_switch(value, Some(present), vec![(case, absent)]);
            }
            // compare a nullable reference receiver against undefined
            other if other.is_reference_representation() => {
                let undefined = self.builder.constant(mir::Constant::Undefined, received);
                let is_absent = self
                    .builder
                    .binary(mir::BinaryOperator::Equal, value, undefined);
                self.builder.branch(is_absent, absent, present);
            }
            // fall back to passing the receiver through
            _ => return Ok(value),
        }

        // short-circuit the chain with undefined when the receiver is absent
        self.builder.switch_to_block(absent);
        let undefined = self.absent_representation_value(representation)?;
        self.builder.local_set(slot, undefined);
        self.builder.jump(exit);

        // continue the chain with the whole receiver
        self.builder.switch_to_block(present);

        Ok(value)
    }
}
