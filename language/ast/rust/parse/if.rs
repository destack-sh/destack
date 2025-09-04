use crate::{If, Keyword, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Parse an if / else statement.
    ///
    /// Examples:
    /// ```
    /// // if
    /// if x > 0 {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if x > 0 {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if x > 0 {
    ///     print("positive")
    /// } else if x == 0 {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    pub fn eat_if(&mut self) -> ParseResult<NodeId<If>> {
        let start = self.mark();
        self.eat_keyword(Keyword::If)?;
        let condition_id = self.eat_expression()?;
        let then_block_id = self.eat_block()?;
        let if_node = if self.peek_keyword(Keyword::Else).is_ok() {
            self.eat_keyword(Keyword::Else)?;
            if self.peek_keyword(Keyword::If).is_ok() {
                // if ... else if ...
                let else_if_id = self.eat_if()?;
                If::IfElseIf {
                    condition: condition_id,
                    then_block: then_block_id,
                    else_if: else_if_id,
                }
            } else {
                // if ... else ...
                let else_block_id = self.eat_block()?;
                If::IfElse {
                    condition: condition_id,
                    then_block: then_block_id,
                    else_block: else_block_id,
                }
            }
        } else {
            // if ...
            If::If {
                condition: condition_id,
                then_block: then_block_id,
            }
        };

        let if_id = self.tree.allocate(if_node, self.get_span_from(start));
        Ok(if_id)
    }
}

#[cfg(test)]
mod tests {
    // todo!: test if / else / else if
}
