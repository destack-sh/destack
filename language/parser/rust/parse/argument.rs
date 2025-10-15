use dyst_ast::{Expression, ScalarLiteral};

use crate::parse::prelude::*;
use crate::{Argument, NodeId, NodeType, Parameter, Parser, ParserResult, TokenType};

impl<'a> Parser<'a> {
    /// Eat a parameter
    ///
    /// Examples:
    /// ```
    /// x
    /// T
    /// x: int32
    /// Validate: bool = false
    /// baz: @someMacro(T)
    /// ...T
    /// ...args: int32[]
    /// ```
    #[inline]
    pub fn eat_parameter(&mut self) -> ParserResult<NodeId<Parameter>> {
        let start = self.mark();

        let is_variadic = if self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            self.bump(); // eat range or range wide
            true
        } else {
            false
        };

        // name
        let name = self.eat_identifier()?;

        // : type
        let ty = if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let ty = self
                .with_options(self.options.in_type(), |parser| parser.eat_expression())
                .for_node_type(NodeType::Parameter)?;
            Some(ty)
        } else {
            None
        };

        // = value
        let parameter = if !is_variadic && self.peek_token(TokenType::Assign).is_ok() {
            // has default value
            self.bump(); // eat assign
            let value = self.eat_expression().for_node_type(NodeType::Parameter)?;
            Parameter::Scalar {
                name,
                ty,
                default: Some(value),
            }
        }
        // variadic parameter (cannot have a default value)
        else if is_variadic {
            Parameter::Variadic { name, ty }
        }
        // named parameter (without a default value)
        else {
            Parameter::Scalar {
                name,
                ty,
                default: None,
            }
        };

        let parameter_id = self.tree.insert(parameter, self.get_span_from(start));
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
    pub fn eat_parameters_body(&mut self) -> ParserResult<Vec<NodeId<Parameter>>> {
        let mut parameters: Vec<NodeId<Parameter>> = Vec::new();
        while self.peek_identifier().is_ok()
            || self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;
            parameters.push(parameter);
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat static parameters (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_parameters_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Parameter>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_parameters()?));
        }
        Ok(None)
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    pub fn eat_static_parameters(&mut self) -> ParserResult<Vec<NodeId<Parameter>>> {
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

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
    pub fn eat_dynamic_parameters_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Parameter>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_parameters()?));
        }
        Ok(None)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens).
    pub fn eat_dynamic_parameters(&mut self) -> ParserResult<Vec<NodeId<Parameter>>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty dynamic parameters
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic parameters
        let parameters = self.with_options(self.options.nested(), |parser| {
            parser.eat_parameters_body()
        })?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(parameters)
    }

    /// Eat an argument (e.g., `x: 1` or `y`).
    /// Does not support named shorthand arguments.
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// y
    /// 2
    /// ...args
    /// ```
    #[inline]
    pub fn eat_argument(&mut self) -> ParserResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_token(TokenType::Identifier).is_ok()
            && (self.peek_next_token(TokenType::Colon).is_ok())
        {
            let name = self.eat_identifier().for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self
                .tree
                .insert(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // spread argument
        else if self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            self.bump(); // eat range
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self
                .tree
                .insert(Argument::Spread { value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .insert(Argument::Positional { value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat a tree literal argument (e.g., `x=1` or `long-name=2` or `flag-is-set`).
    /// Does not support named shorthand arguments.
    ///
    /// Examples:
    /// ```
    /// x: 1
    /// y
    /// 2
    /// ...args
    /// ```
    #[inline]
    pub fn eat_tree_literal_argument(&mut self) -> ParserResult<NodeId<Argument>> {
        let start = self.mark();
        // spread argument
        if self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            self.bump(); // eat range
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self
                .tree
                .insert(Argument::Spread { value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // named argument
        else {
            let name = self.eat_tree_literal_identifier()?;

            // named argument with value
            let value = if self.peek_token(TokenType::Colon).is_ok()
                || self.peek_token(TokenType::Assign).is_ok()
            {
                self.bump(); // eat colon or assign
                self.eat_expression().for_node_type(NodeType::Argument)?
            }
            // implicit boolean true
            else {
                self.tree.insert(
                    Expression::ScalarLiteral(ScalarLiteral::Boolean(true)),
                    self.get_span_from(start),
                )
            };

            let argument_id = self
                .tree
                .insert(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat static arguments (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_arguments_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Argument>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_arguments()?));
        }
        Ok(None)
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    pub fn eat_static_arguments(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
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
    pub fn eat_dynamic_arguments_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Argument>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_arguments()?));
        }
        Ok(None)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens).
    pub fn eat_dynamic_arguments(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty dynamic arguments
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let dynamic_arguments = self.with_options(self.options.nested(), |parser| {
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
    pub fn eat_arguments_body(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
        let mut arguments: Vec<NodeId<Argument>> = Vec::new();
        loop {
            let argument_id = self.eat_argument()?;
            arguments.push(argument_id);
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
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
        Argument, Expression, IntType, Parameter, ScalarLiteral, TypeLiteral, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Scalar { name, ty, default } => {
            assert_string!(parser, *name, "T");
            assert!(ty.is_none());
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_type() {
        // x: int32
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Scalar { name, ty, default } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType {
                width: Some(32),
                is_signed: true
            })));
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Scalar { name, ty, default } => {
            assert_string!(parser, *name, "validate");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            assert!(default.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_variadic() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_variadic_with_type() {
        // ...args: int32[]
        let mut test = TestParser::new("...args: int32[]");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_some());
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
            assert_string!(parser, *name, "x");
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
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
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    }

    #[test]
    fn test_parse_argument_spread() {
        // ...args
        let mut test = TestParser::new("...args");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Spread { value } => {
            // ...args
            assert_node!(parser.tree, *value, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "args");
            });
        });
    }
}
