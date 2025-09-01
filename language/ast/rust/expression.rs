//! Parse expressions. Mostly defers to other parsers.

use crate::{Expression, ParseResult, Parser, ScalarLiteral};

impl<'a> Parser<'a> {
    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<Expression> {
        // todo!: parse all expressions
        let literal: ScalarLiteral = self.eat_scalar_literal()?;
        Ok(Expression::ScalarLiteral(literal))
    }
}
