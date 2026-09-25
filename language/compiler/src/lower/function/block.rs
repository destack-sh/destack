use tspp_dir as dir;

use crate::CompilerResult;
use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one function body expression.
    pub(in crate::lower) fn lower_body(
        &mut self,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // return the value of expression bodies directly
        let dir::Expression::Block(block) = *self.source().tree().get(body) else {
            return self.lower_anchored(body, |function| {
                let value = function.lower_value(body)?;

                function.return_value(Some(value))
            });
        };

        // lower the statements until one terminates the block
        let depth = self.open_disposals();
        for index in 0..self.block_statement_count(block) {
            let statement = self.block_statement(block, index);
            if self.lower_statement(statement)? {
                self.close_disposals(depth, true)?;

                return Ok(());
            }
        }

        // dispose the body's resources, then return the tail value or void
        let tail_expression = self.source().tree().get(block).tail_expression;
        match tail_expression {
            // run valueless tails for their control flow alone
            Some(tail) if self.is_valueless(tail)? => {
                if !self.lower_statement(tail)? {
                    self.dispose_down_to(0)?;
                    self.return_value(None)?;
                }
            }
            // return value tails as the function result
            Some(tail) => self.lower_anchored(tail, |function| {
                let value = function.lower_value(tail)?;
                function.dispose_down_to(0)?;

                function.return_value(Some(value))
            })?,
            // return void from a block without a tail
            None => self.lower_anchored(body, |function| {
                function.dispose_down_to(0)?;

                function.return_value(None)
            })?,
        }
        self.close_disposals(depth, true)?;

        Ok(())
    }

    /// Lower one statement-position block or arm, returning whether it terminated.
    pub(in crate::lower) fn lower_arm(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let dir::Expression::Block(block) = *self.source().tree().get(expression) else {
            return self.lower_statement(expression);
        };

        self.lower_block(block)
    }

    /// Lower one statement-position block, returning whether it terminated.
    pub(in crate::lower) fn lower_block(
        &mut self,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // dispose the block's resources on fallthrough
        let depth = self.open_disposals();
        let terminated = self.lower_block_statements(block)?;
        self.close_disposals(depth, terminated)?;

        Ok(terminated)
    }

    /// Lower one block's statements and tail, returning whether it terminated.
    fn lower_block_statements(
        &mut self,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<bool> {
        // lower the statements until one terminates the block
        for index in 0..self.block_statement_count(block) {
            let statement = self.block_statement(block, index);
            if self.lower_statement(statement)? {
                return Ok(true);
            }
        }

        // lower the block's tail expression
        let tail_expression = self.source().tree().get(block).tail_expression;
        match tail_expression {
            // keep lowering valueless tails as statements
            Some(tail) if self.is_valueless(tail)? => self.lower_statement(tail),
            // discard the tail value in statement blocks
            Some(tail) => {
                self.lower_value(tail)?;

                Ok(false)
            }
            // fall through a block without a tail
            None => Ok(false),
        }
    }

    /// Return the leading statement count of one block.
    fn block_statement_count(&self, block: dir::LocalNodeId<dir::Block>) -> usize {
        self.source().tree().get(block).leading_expressions.len()
    }

    /// Return one leading statement of one block by position.
    fn block_statement(
        &self,
        block: dir::LocalNodeId<dir::Block>,
        index: usize,
    ) -> dir::LocalNodeId<dir::Expression> {
        self.source().tree().get(block).leading_expressions[index]
    }
}
