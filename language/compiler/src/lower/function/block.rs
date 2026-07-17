use destack_dir as dir;

use crate::lower::FunctionLowerer;
use crate::{CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one function body expression.
    pub(in crate::lower) fn lower_body(
        &mut self,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // return the value of expression bodies directly
        let dir::Expression::Block(block) = self.lowerer.tree.get(body) else {
            let value = self.lower_expression(body)?;
            self.builder.return_(Some(value));

            return Ok(());
        };
        let block = self.lowerer.tree.get(*block).clone();

        // lower the statements until one terminates the block
        for statement in &block.leading_expressions {
            if self.lower_statement(*statement)? {
                return Ok(());
            }
        }

        match block.tail_expression {
            // run valueless tails for control flow, not for a result
            Some(tail) if self.tail_is_valueless(tail)? => {
                if !self.lower_statement(tail)? {
                    self.builder.return_(None);
                }
            }
            // return value tails as the function result
            Some(tail) => {
                let value = self.lower_expression(tail)?;
                self.builder.return_(Some(value));
            }
            None => self.builder.return_(None),
        }

        Ok(())
    }

    /// Lower one statement-position block or arm, returning whether it terminated.
    pub(in crate::lower) fn lower_arm(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let dir::Expression::Block(block) = self.lowerer.tree.get(expression) else {
            return self.lower_statement(expression);
        };
        let block = self.lowerer.tree.get(*block).clone();

        self.lower_block(&block)
    }

    /// Lower one statement-position block, returning whether it terminated.
    pub(in crate::lower) fn lower_block(&mut self, block: &dir::Block) -> CompilerResult<bool> {
        // lower the statements until one terminates the block
        for statement in &block.leading_expressions {
            if self.lower_statement(*statement)? {
                return Ok(true);
            }
        }

        match block.tail_expression {
            // keep lowering valueless tails as statements
            Some(tail) if self.tail_is_valueless(tail)? => self.lower_statement(tail),
            // discard the tail value in statement blocks
            Some(tail) => {
                self.lower_expression(tail)?;

                Ok(false)
            }
            None => Ok(false),
        }
    }

    /// Lower one statement expression, returning whether it terminated the block.
    pub(in crate::lower) fn lower_statement(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        match self.lowerer.tree.get(statement).clone() {
            // return value
            dir::Expression::Return { value } => {
                let value = value
                    .map(|value| self.lower_expression(value))
                    .transpose()?;
                self.builder.return_(value);

                Ok(true)
            }

            // let x = value
            dir::Expression::Let {
                mutability,
                declarators,
                ..
            } => {
                self.lower_let(mutability, &declarators)?;

                Ok(false)
            }

            // x = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                self.lower_assign(statement, left, operator, right)?;

                Ok(false)
            }

            // x++
            dir::Expression::Unary {
                operator:
                    operator @ (dir::UnaryOperator::PostIncrement
                    | dir::UnaryOperator::PostDecrement
                    | dir::UnaryOperator::PreIncrement
                    | dir::UnaryOperator::PreDecrement),
                right,
            } => {
                self.lower_update(operator, right)?;

                Ok(false)
            }

            // if (cond) { ... } else { ... }
            dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                then_expression,
                else_expression,
            } => self.lower_if(&condition, then_expression, else_expression),

            // while (cond) { ... }
            dir::Expression::While {
                form,
                condition,
                body,
            } => self.lower_while(None, form, condition, body),

            // for (init; cond; step) { ... }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => self.lower_for(None, initialization, condition, increment, body),

            // loop { ... }
            dir::Expression::Loop { body } => self.lower_loop(None, body),

            // outer: while (cond) { ... }
            dir::Expression::Label { label, body } => match self.lowerer.tree.get(body).clone() {
                dir::Expression::While {
                    form,
                    condition,
                    body,
                } => self.lower_while(Some(label), form, condition, body),
                dir::Expression::For {
                    initialization,
                    condition,
                    increment,
                    body,
                } => self.lower_for(Some(label), initialization, condition, increment, body),
                dir::Expression::Loop { body } => self.lower_loop(Some(label), body),
                other => Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a labeled '{}' statement", other.variant_name()),
                }
                .into()),
            },

            // break label
            dir::Expression::Break { label, value } => self.lower_break(label, value),

            // continue label
            dir::Expression::Continue { label } => self.lower_continue(label),

            // call(...)
            dir::Expression::Call { .. } => {
                self.lower_call(statement)?;

                Ok(false)
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("'{}' statements", other.variant_name()),
            }
            .into()),
        }
    }

    /// Return whether one block tail yields no value.
    fn tail_is_valueless(&self, tail: dir::LocalNodeId<dir::Expression>) -> CompilerResult<bool> {
        let ty = self.lowerer.node_type(tail)?;

        Ok(matches!(ty, dir::Type::Never | dir::Type::Void))
    }
}
