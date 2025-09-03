use crate::{NodeId, Parameter, ParseResult, Parser};
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
    pub fn eat_parameter(&mut self) -> ParseResult<NodeId<Parameter>> {
        let start = self.mark();

        // name: type
        let name = self.eat_identifier()?;
        self.eat_colon()?;
        let r#type = self.eat_type()?;

        // default value
        let parameter = if self.peek_next_token(TokenType::Assign).is_ok() {
            // has default value
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression()?;
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
        let parameter_id = self.tree.allocate(parameter, self.span_from(start));
        Ok(parameter_id)
    }

    /// Eat a parameter list. May be comma or newline separated.
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// x: int32, y: int32
    /// ```
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
}

#[cfg(test)]
mod tests {
    use crate::{Expression, IntType, Parser, PrimitiveType, ScalarLiteral, Type};
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
}
