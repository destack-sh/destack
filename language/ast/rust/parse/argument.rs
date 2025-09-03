use crate::{Argument, NodeId, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an argument (e.g., `x: 1` or `y`).
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// y
    /// 2
    /// ```
    pub fn eat_argument(&mut self) -> ParseResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_next_token(TokenType::Identifier).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument::Named { name, value }, self.span_from(start));
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument::Positional { value }, self.span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat an argument list. May be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// x: 1, y: 2
    ///
    /// y: 2 // multiline
    /// z
    /// ```
    pub fn eat_arguments_body(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        let mut arguments: Vec<NodeId<Argument>> = Vec::new();
        while self.peek_identifier().is_ok() {
            let argument_id = self.eat_argument()?;
            arguments.push(argument_id);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop()?;
            } else {
                break;
            }
        }
        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Argument, Expression, Parser, ScalarLiteral};
    use destack_language_token::{SourceFile, tokenize_semantic};

    #[test]
    fn test_arguments() {
        let input = r###"
x: 1
3
        "###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // x: 1
        let argument_id = parser.eat_argument().unwrap();
        let argument = parser.tree.get(argument_id);
        match argument {
            &Argument::Named { name, value } => {
                // x
                assert_eq!(name, parser.strings.intern("x"));
                // 1
                let expression = parser.tree.get(value);
                let literal_id = match expression {
                    &Expression::ScalarLiteral(id) => id,
                    _ => panic!("expected scalar literal"),
                };
                let scalar_literal = parser.tree.get(literal_id);
                match scalar_literal {
                    ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                    _ => panic!("expected integer"),
                }
            }
            _ => panic!("expected named argument"),
        }
        parser.eat_newline().unwrap();

        // 3
        let argument_id = parser.eat_argument().unwrap();
        let argument = parser.tree.get(argument_id);
        match argument {
            &Argument::Positional { value } => {
                // 3
                let expression = parser.tree.get(value);
                match expression {
                    Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(*scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 3),
                            _ => panic!("expected integer"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                }
            }
            _ => panic!("expected positional argument"),
        }
        parser.eat_newline().unwrap();
    }
}
