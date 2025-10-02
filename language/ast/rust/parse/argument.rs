use crate::parse::prelude::*;
use crate::{Argument, NodeId, NodeType, Parameter, ParseResult, Parser};
use dyst_token::TokenType;

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
            self.bump(); // eat colon
            let r#type = self
                .with_options(self.options.in_type(), |parser| parser.eat_expression())
                .for_node_type(NodeType::Parameter)?;
            Some(r#type)
        } else {
            None
        };

        // = value
        let parameter = if self.peek_token(TokenType::Assign).is_ok() {
            // has default value
            self.bump(); // eat assign
            let value = self.eat_expression().for_node_type(NodeType::Parameter)?;
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
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;
            parameters.push(parameter);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat static parameters (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_parameters_maybe(&mut self) -> ParseResult<Option<Vec<NodeId<Parameter>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_parameters()?));
        }
        Ok(None)
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    pub fn eat_static_parameters(&mut self) -> ParseResult<Vec<NodeId<Parameter>>> {
        self.eat_token(TokenType::LessThan)?;

        // empty static parameters
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            self.bump(); // eat greater than
            return Ok(vec![]);
        }

        // regular static parameters
        let parameters = self.with_options(self.options.in_static(), |parser| {
            parser.eat_parameters_body()
        })?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(parameters)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_parameters_maybe(&mut self) -> ParseResult<Option<Vec<NodeId<Parameter>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_parameters()?));
        }
        Ok(None)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens).
    pub fn eat_dynamic_parameters(&mut self) -> ParseResult<Vec<NodeId<Parameter>>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        // empty dynamic parameters
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic parameters
        let parameters = self.with_options(self.options.in_nested(), |parser| {
            parser.eat_parameters_body()
        })?;
        self.eat_token(TokenType::CloseParenthesis)?;
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
            let name = self.eat_identifier().for_node_type(NodeType::Argument)?;
            self.eat_colon()?;
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self
                .tree
                .allocate(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument::Positional { value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat static arguments (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_arguments_maybe(&mut self) -> ParseResult<Option<Vec<NodeId<Argument>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_arguments()?));
        }
        Ok(None)
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    pub fn eat_static_arguments(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::LessThan)?;

        // empty static arguments
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            self.bump(); // eat greater than
            return Ok(vec![]);
        }

        // regular static arguments
        let static_arguments = self.with_options(self.options.in_static(), |parser| {
            parser.eat_arguments_body()
        })?;

        self.eat_token(TokenType::GreaterThan)?;
        Ok(static_arguments)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_arguments_maybe(&mut self) -> ParseResult<Option<Vec<NodeId<Argument>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_arguments()?));
        }
        Ok(None)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens).
    pub fn eat_dynamic_arguments(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty dynamic arguments
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let dynamic_arguments = self.with_options(self.options.in_nested(), |parser| {
            parser.eat_arguments_body()
        })?;

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(dynamic_arguments)
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
        Argument, Expression, TypeLiteral, assert_bool, assert_int, assert_node, assert_string,
    };

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);
        assert_string!(parser.session, parameter.name, "T");
        assert!(parameter.r#type.is_none());
        assert!(parameter.default.is_none());
    }

    #[test]
    fn test_parse_parameter_with_type() {
        // x: int32
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);

        // x
        assert_string!(parser.session, parameter.name, "x");

        // int32
        assert_node!(parser.tree, parameter.r#type.unwrap(),
            Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                    assert_eq!(int_ty.width, Some(32));
                    assert!(int_ty.is_signed);
                });
            }
        );
        assert!(parameter.default.is_none());
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        let parameter = parser.tree.get(parameter_id);

        // validate
        assert_string!(parser.session, parameter.name, "validate");

        // boolean
        assert_node!(
            parser.tree,
            parameter.r#type.unwrap(),
            Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Boolean);
            }
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
        let mut test = TestParser::new("x: 1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { name, value } => {
            // x
            assert_string!(parser.session, *name, "x");
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 1);
            });
        });
    }

    #[test]
    fn test_parse_argument_positional() {
        // 3
        let mut test = TestParser::new("3");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Positional { value } => {
            // 3
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 3);
            });
        });
    }
}
