use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::LoopFrame;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one if statement, returning whether every arm terminated.
    pub(in crate::lower) fn lower_if(
        &mut self,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        // branch on the condition; without an else the join doubles as the else edge
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
        let Some(else_expression) = else_expression else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing an else arm on one ternary".to_string(),
            });
        };

        // branch on the condition with a join local for the arm values
        let condition = self.lower_condition(condition)?;
        let join_value = self.value_slot(expression)?;
        let then_block = self.builder.block();
        let else_block = self.builder.block();
        let join = self.builder.block();
        self.builder.branch(condition, then_block, else_block);

        // each arm writes the join local before joining
        self.builder.switch_to_block(then_block);
        let value = self.lower_expression(then_expression)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);
        self.builder.switch_to_block(else_block);
        let value = self.lower_expression(else_expression)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);

        // read the winning arm's value at the join
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one while or do-while loop.
    pub(in crate::lower) fn lower_while(
        &mut self,
        label: Option<StringId>,
        form: dir::WhileForm,
        condition: dir::LocalNodeId<dir::Expression>,
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
        let condition = self.lower_expression(condition)?;
        self.builder.branch(condition, body_block, exit);

        // lower the body back-edged through the condition header
        self.builder.switch_to_block(body_block);
        let terminated = self.lower_loop_body(label, header, exit, body)?;
        if !terminated {
            self.builder.jump(header);
        }
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one three-part for loop.
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

        // lower the body; continue re-enters through the increment
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
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one unconditional loop.
    pub(in crate::lower) fn lower_loop(
        &mut self,
        label: Option<StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // loop the body onto itself: only break reaches the exit
        let body_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(body_block);
        self.builder.switch_to_block(body_block);

        let terminated = self.lower_loop_body(label, body_block, exit, body)?;
        if !terminated {
            self.builder.jump(body_block);
        }
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Lower one break statement.
    pub(in crate::lower) fn lower_break(
        &mut self,
        label: Option<StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<bool> {
        if value.is_some() {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a valued break".to_string(),
            }
            .into());
        }
        let exit = self.loop_frame(label)?.exit;

        self.builder.jump(exit);
        Ok(true)
    }

    /// Lower one continue statement.
    pub(in crate::lower) fn lower_continue(
        &mut self,
        label: Option<StringId>,
    ) -> CompilerResult<bool> {
        let target = self.loop_frame(label)?.continue_target;

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
        self.loops.push(LoopFrame {
            label,
            continue_target,
            exit,
        });
        let terminated = self.lower_block(body)?;
        self.loops.pop();

        Ok(terminated)
    }

    /// Lower one condition to a boolean value.
    fn lower_condition(&mut self, condition: &dir::Condition) -> CompilerResult<mir::Value> {
        let Some(condition) = condition.as_expression() else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a binding condition".to_string(),
            }
            .into());
        };

        self.lower_expression(condition)
    }

    /// Return the loop frame one break or continue targets.
    fn loop_frame(&self, label: Option<StringId>) -> CompilerResult<&LoopFrame> {
        let frame = match label {
            None => self.loops.last(),
            Some(label) => self
                .loops
                .iter()
                .rev()
                .find(|frame| frame.label == Some(label)),
        };

        frame.ok_or_else(|| CompilerError::Internal {
            message: "checked DIR is missing an enclosing loop for one break or continue"
                .to_string(),
        })
    }
}
