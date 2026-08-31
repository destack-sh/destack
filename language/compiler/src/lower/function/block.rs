use destack_dir as dir;

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
            let value = self.lower_expression(body)?;
            self.builder.return_(Some(value));

            return Ok(());
        };

        // lower the statements until one terminates the block
        for index in 0..self.block_statement_count(block) {
            let statement = self.block_statement(block, index);
            if self.lower_statement(statement)? {
                return Ok(());
            }
        }

        // return the block's tail value, or void when it has none
        match self.source().tree().get(block).tail_expression {
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
            // return void from a block without a tail
            None => self.builder.return_(None),
        }

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
        // lower the statements until one terminates the block
        for index in 0..self.block_statement_count(block) {
            let statement = self.block_statement(block, index);
            if self.lower_statement(statement)? {
                return Ok(true);
            }
        }

        match self.source().tree().get(block).tail_expression {
            // keep lowering valueless tails as statements
            Some(tail) if self.tail_is_valueless(tail)? => self.lower_statement(tail),
            // discard the tail value in statement blocks
            Some(tail) => {
                self.lower_expression(tail)?;

                Ok(false)
            }
            // fall through a block without a tail
            None => Ok(false),
        }
    }

    /// Return whether one block tail yields no value.
    fn tail_is_valueless(&self, tail: dir::LocalNodeId<dir::Expression>) -> CompilerResult<bool> {
        let ty = self.node_type(tail)?;

        Ok(matches!(ty, dir::Type::Never | dir::Type::Void))
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
