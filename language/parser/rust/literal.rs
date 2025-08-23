use crate::{Expression, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a literal.
    pub fn eat_literal(&mut self) -> ParseResult<'a, Expression> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::tokenize_semantic;

    use crate::{Expression, FloatType, IntType, ParseResult, Parser, ScalarLiteral};

    fn parse_literal<'a>(input: &str) -> ParseResult<'a, Expression> {
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(&tokens);
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
