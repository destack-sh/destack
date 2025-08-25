use destack_language_lexer::TokenType;

use crate::{
    ArrayLiteral, Expression, ParseResult, Parser, ScalarLiteral, StructLiteral, TupleLiteral,
};

impl<'a> Parser<'a> {
    /// Eat a literal.
    pub fn eat_literal(&mut self) -> ParseResult<Expression> {
        // array
        if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            let array_literal = self.eat_array_literal()?;
            Ok(Expression::ArrayLiteral(array_literal))
        // tuple
        } else if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            let tuple_literal = self.eat_tuple_literal()?;
            Ok(Expression::TupleLiteral(tuple_literal))
        // struct
        } else if self.peek_next_token(TokenType::OpenBrace).is_ok() {
            let struct_literal = self.eat_struct_literal()?;
            Ok(Expression::StructLiteral(struct_literal))
        // scalar
        } else {
            let scalar_literal = self.eat_scalar_literal()?;
            Ok(Expression::ScalarLiteral(scalar_literal))
        }
    }

    /// Eat a scalar literal.
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        todo!()
    }

    /// Eat an array literal.
    /// The individual elements are full expressions, not just literals.
    pub fn eat_array_literal(&mut self) -> ParseResult<ArrayLiteral> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseBracket)?;
        Ok(ArrayLiteral::Fixed { elements })
    }

    /// Eat a tuple literal.
    pub fn eat_tuple_literal(&mut self) -> ParseResult<TupleLiteral> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(TupleLiteral { elements })
    }

    /// Eat a struct literal.
    pub fn eat_struct_literal(&mut self) -> ParseResult<StructLiteral> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::{SourceFile, tokenize_semantic};

    use crate::{Expression, FloatType, IntType, ParseResult, Parser, ScalarLiteral};

    fn parse_literal(input: &str) -> ParseResult<Expression> {
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_literal()
    }

    #[test]
    fn test_literals() {
        // 1
        assert_eq!(
            parse_literal("1"),
            Ok(Expression::ScalarLiteral(ScalarLiteral::Integer(
                1,
                IntType::Int32
            )))
        );

        // 17.0
        assert_eq!(
            parse_literal("17.0"),
            Ok(Expression::ScalarLiteral(ScalarLiteral::Float(
                17.0,
                FloatType::Float32
            )))
        );

        // 0x32
        assert_eq!(
            parse_literal("0x32"),
            Ok(Expression::ScalarLiteral(ScalarLiteral::Integer(
                0x32,
                IntType::Int32
            )))
        );
    }
}
