use crate::parse::expression::ExpressionParserOptions;
use crate::{Argument, NodeId, Parameter, ParseResult, Parser};
use dyst_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat a parameter
    ///
    /// Examples:
    /// ```
    /// T
    /// x: int32
    /// Validate: bool = false
    /// baz: @someMacro(T)
    /// ```
    #[inline]
    pub fn eat_parameter(&mut self) -> ParseResult<NodeId<Parameter>> {
        let start = self.mark();

        // name
        let name = self.eat_identifier()?;

        // : type
        let r#type = if self.peek_colon().is_ok() {
            self.eat_colon()?;
            let r#type = self.eat_type()?;
            Some(r#type)
        } else {
            None
        };

        // = value
        let parameter = if self.peek_token(TokenType::Assign).is_ok() {
            // has default value
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression(ExpressionParserOptions::default())?;
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
                self.eat_item_stop_with_newlines()?;
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
    /// z: foo() > 7
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
            let value = self.eat_expression(ExpressionParserOptions::default())?;
            let argument_id = self
                .tree
                .allocate(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression(ExpressionParserOptions::default())?;
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
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, IntType, PrimitiveType, Type, assert_bool, assert_int, assert_node,
    };

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let test = TestParser::new("T");
        let mut parser = test.parser();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);
        assert_eq!(parameter.name, parser.strings.intern("T"));
        assert!(parameter.r#type.is_none());
        assert!(parameter.default.is_none());
    }

    #[test]
    fn test_parse_parameter_with_type() {
        // x: int32
        let test = TestParser::new("x: int32");
        let mut parser = test.parser();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);

        // x
        assert_eq!(parameter.name, parser.strings.intern("x"));

        // int32
        assert_node!(parser.tree, parameter.r#type.unwrap(),
            Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                assert_eq!(*width, 32);
                assert!(*is_signed);
            }
        );
        assert!(parameter.default.is_none());
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let test = TestParser::new("validate: boolean = false");
        let mut parser = test.parser();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);

        // validate
        assert_eq!(parameter.name, parser.strings.intern("validate"));

        // boolean
        assert_node!(
            parser.tree,
            parameter.r#type.unwrap(),
            Type::Primitive(PrimitiveType::Boolean)
        );

        // false
        assert!(parameter.default.is_some());
        assert_node!(parser.tree, parameter.default.unwrap(), Expression::ScalarLiteral(literal_id) => {
            assert_bool!(parser.tree, *literal_id, false);
        });
    }

    #[test]
    fn test_parse_argument_named() {
        // x: 1
        let test = TestParser::new("x: 1");
        let mut parser = test.parser();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { name, value } => {
            // x
            assert_eq!(*name, parser.strings.intern("x"));
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 1);
            });
        });
    }

    #[test]
    fn test_parse_argument_positional() {
        // 3
        let test = TestParser::new("3");
        let mut parser = test.parser();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Positional { value } => {
            // 3
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 3);
            });
        });
    }
}
