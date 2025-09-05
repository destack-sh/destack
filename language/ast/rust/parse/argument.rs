use crate::{Argument, NodeId, Parameter, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat a parameter
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// Validate: bool = false
    /// baz: @someMacro(T)
    /// ```
    #[inline]
    pub fn eat_parameter(&mut self) -> ParseResult<NodeId<Parameter>> {
        let start = self.mark();

        // name: type
        let name = self.eat_identifier()?;
        self.eat_colon()?;
        let r#type = self.eat_type()?;

        // default value
        let parameter = if self.peek_token(TokenType::Assign).is_ok() {
            // has default value
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression(None)?;
            Parameter {
                name,
                r#type,
                default: Some(value),
            }
        } else {
            // no default value
            Parameter {
                name,
                r#type,
                default: None,
            }
        };
        let parameter_id = self.tree.allocate(parameter, self.get_span_from(start));
        Ok(parameter_id)
    }

    /// Eat a parameter list. May be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// x: int32, y: int32
    /// ```
    #[inline]
    pub fn eat_parameters_body(&mut self) -> ParseResult<Vec<NodeId<Parameter>>> {
        let mut parameters: Vec<NodeId<Parameter>> = Vec::new();
        while self.peek_identifier().is_ok() {
            let parameter = self.eat_parameter()?;
            parameters.push(parameter);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat an argument (e.g., `x: 1` or `y`).
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// y
    /// 2
    /// ```
    #[inline]
    pub fn eat_argument(&mut self) -> ParseResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let value = self.eat_expression(None)?;
            let argument_id = self
                .tree
                .allocate(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression(None)?;
            let argument_id = self
                .tree
                .allocate(Argument::Positional { value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat an argument list. May be comma or newline separated.
    /// Empty arguments are an error.
    ///
    /// Examples:
    /// ```
    /// x: 1, y: 2
    ///
    /// 3 // positional
    ///
    /// y: 2 // multiline
    /// z
    /// ```
    #[inline]
    pub fn eat_arguments_body(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        let mut arguments: Vec<NodeId<Argument>> = Vec::new();
        loop {
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
    use crate::{Argument, Expression, IntType, Parser, PrimitiveType, ScalarLiteral, Type};
    use destack_language_token::{SourceFile, tokenize_semantic};

    #[test]
    fn test_parameters() {
        let input = r###"
x: int32
validate: boolean = false
        "###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // x: int32
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);
        // x
        assert_eq!(parameter.name, parser.strings.intern("x"));
        // int32
        let type_node = parser.tree.get(parameter.r#type);
        match type_node {
            &Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                assert_eq!(width, 32);
                assert!(is_signed);
            }
            _ => panic!("expected int32 type"),
        }
        assert!(parameter.default.is_none());
        parser.eat_newline().unwrap();

        // validate: bool = false
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);
        // validate
        assert_eq!(parameter.name, parser.strings.intern("validate"));
        // bool
        let type_node = parser.tree.get(parameter.r#type);
        match type_node {
            &Type::Primitive(PrimitiveType::Boolean) => {}
            _ => panic!("expected boolean type"),
        }
        // false
        assert!(parameter.default.is_some());
        let default_value = parameter.default.unwrap();
        let expression = parser.tree.get(default_value);
        match expression {
            &Expression::ScalarLiteral(scalar_literal_id) => {
                let scalar_literal = parser.tree.get(scalar_literal_id);
                match scalar_literal {
                    &ScalarLiteral::Boolean(value) => assert!(!value),
                    _ => panic!("expected boolean literal"),
                }
            }
            _ => panic!("expected scalar literal"),
        }
        parser.eat_newline().unwrap();
    }

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
