use dyst_ast::{Expression, Keyword, PostfixPosition, Name, Pattern, ScalarLiteral, StringId};

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
    /// _
    /// { x }
    /// { x }: MyType = Foo
    /// ...T
    /// ...args: int32[]
    /// ```
    #[inline]
    pub fn eat_parameter(&mut self) -> ParserResult<NodeId<Parameter>> {
        let start = self.mark();

        // variadic
        let is_variadic = if self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            self.bump(); // eat range or range wide
            true
        } else {
            false
        };

        // pattern/name
        let (pattern, name): (Option<NodeId<Pattern>>, Option<StringId>) = {
            // pattern
            if !is_variadic
                && self
                    .peek_token_in(&[
                        TokenType::OpenParenthesis,
                        TokenType::OpenBracket,
                        TokenType::OpenBrace,
                        TokenType::Wildcard,
                    ])
                    .is_ok()
            {
                let pattern = self
                    .with_options(self.options.in_before_type(), |parser| parser.eat_pattern())?;
                (Some(pattern), None)
            }
            // name
            else {
                let name = self.eat_identifier()?;
                (None, Some(name))
            }
        };

        // ? maybe
        let is_maybe = if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat maybe
            true
        } else {
            false
        };

        // : type (or keyword for #Leniency)
        let ty = {
            if self.peek_colon().is_ok()
                || (self.options.in_static
                    && (self.peek_keyword(Keyword::Extends).is_ok()
                        || self.peek_keyword(Keyword::Implements).is_ok()))
            {
                self.bump(); // eat colon or keyword
                let ty = self
                    .with_options(self.options.in_type(), |parser| parser.eat_expression())
                    .for_node_type(NodeType::Parameter)?;
                Some(ty)
            } else {
                None
            }
        };
        // wrap type in maybe if needed
        let ty = ty.map(|ty| {
            if is_maybe {
                self.tree.insert(
                    Expression::Maybe {
                        left: ty,
                        position: PostfixPosition::Direct,
                    },
                    self.tree.spans.get(ty),
                )
            } else {
                ty
            }
        });

        // = value
        let parameter = {
            // has default value
            if !is_variadic && self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                let value = self.eat_expression().for_node_type(NodeType::Parameter)?;
                // named with default
                if let Some(name) = name {
                    Parameter::Named {
                        name,
                        ty,
                        default: Some(value),
                    }
                }
                // pattern with default
                else {
                    Parameter::Pattern {
                        pattern: pattern.expect("peeked"),
                        ty,
                        default: Some(value),
                    }
                }
            }
            // variadic parameter (cannot have a default value)
            else if is_variadic {
                Parameter::Variadic {
                    name: name.expect("peeked"),
                    ty,
                }
            }
            // no default value
            else {
                // named without default
                if let Some(name) = name {
                    Parameter::Named {
                        name,
                        ty,
                        default: None,
                    }
                }
                // pattern without default
                else {
                    Parameter::Pattern {
                        pattern: pattern.expect("peeked"),
                        ty,
                        default: None,
                    }
                }
            }
        };

        // parameter
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
        while
        // named
        self.peek_token(TokenType::Identifier).is_ok()
            // range
            || self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
            // pattern
            || self.peek_token(TokenType::OpenParenthesis).is_ok()
            || self.peek_token(TokenType::OpenBracket).is_ok()
            || self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Wildcard).is_ok()
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
        let parameters = self.with_options(self.options.nested_in_static(), |parser| {
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
        let parameters =
            self.with_options(self.options.nested(), |parser| parser.eat_parameters_body())?;
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
    /// "Content-Type": "application/json"
    /// [x: string]: any
    /// [string]: woof
    /// [var] = "hello"
    /// ```
    #[inline]
    pub fn eat_argument(&mut self) -> ParserResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
            let name = self.eat_name().for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self
                .tree
                .insert(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // named maybe argument
        else if self.peek_name().is_ok()
            && self.peek_next_token(TokenType::Maybe).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            // name
            let name = self.eat_name()?;
            self.bump(); // eat maybe
            self.bump(); // eat colon
            // value
            let value =
                self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
            let value = self.tree.insert(
                Expression::Maybe {
                    left: value,
                    position: PostfixPosition::Direct,
                },
                self.tree.spans.get(value),
            );
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
        // dynamic argument (has a colon after the closing bracket)
        else if self.peek_token(TokenType::OpenBracket).is_ok()
            && self
                .find_matching_pair(TokenType::OpenBracket, TokenType::CloseBracket)
                .map(|pos| {
                    self.tokens
                        .get(pos as usize + 1)
                        .map(|token| token.token.ty == TokenType::Colon)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        {
            self.bump(); // eat open bracket
            // name
            let name = if self.peek_token(TokenType::Identifier).is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
            {
                let name = self.eat_identifier()?;
                self.bump(); // eat colon
                Some(name)
            } else {
                None
            };
            // key
            let key = self.eat_expression().for_node_type(NodeType::Argument)?;
            self.eat_token(TokenType::CloseBracket)?;
            // value
            self.eat_token(TokenType::Colon)?;
            let value = self.eat_expression().for_node_type(NodeType::Argument)?;
            let argument_id = self.tree.insert(
                Argument::Dynamic { name, key, value },
                self.get_span_from(start),
            );
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

            let argument_id = self.tree.insert(
                Argument::Named {
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(start),
            );
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
        self.eat_newlines_maybe()?;

        // empty static arguments
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            self.bump(); // eat greater than
            return Ok(vec![]);
        }

        // regular static arguments
        let static_arguments = self.with_options(self.options.nested_in_static(), |parser| {
            parser.eat_arguments_body(TokenType::GreaterThan)
        })?;

        self.eat_newlines_maybe()?;
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
        self.eat_newlines_maybe()?;

        // empty dynamic arguments
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump(); // eat close parenthesis
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let dynamic_arguments = self.with_options(self.options.nested(), |parser| {
            parser.eat_arguments_body(TokenType::CloseParenthesis)
        })?;

        self.eat_newlines_maybe()?;
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
    pub fn eat_arguments_body(
        &mut self,
        terminator: TokenType,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        let mut arguments: Vec<NodeId<Argument>> = Vec::new();
        loop {
            if self.peek_token(terminator).is_ok() {
                break;
            }
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
    use dyst_ast::{Name, Pattern, PatternField};

    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, IntType, Parameter, ScalarLiteral, TypeLiteral, assert_name,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, ty, default } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, ty, default } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType {
                width: Some(32),
                is_signed: true
            })));
            assert!(default.is_none());
        });
    }

    #[test]
    fn test_parse_parameter_with_maybe_type() {
        // x?: int32
        let mut test = TestParser::new("x?: int32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *ty, Expression::Maybe { left, position: _ } => {
                assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::Int(IntType {
                    width: Some(32),
                    is_signed: true
                })));
            });
        });
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { name, ty, default } => {
            assert_string!(parser, *name, "validate");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            assert!(default.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_with_pattern_and_defaults() {
        // { x }: T = false
        let mut test = TestParser::new("{ x = 4 }: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, ty: Some(ty), default: Some(default) } => {
            // { x = 4 }
            assert_node!(parser.tree, *pattern, Pattern::Struct { ty: None, fields } => {
                assert_node!(parser.tree, fields[0], PatternField::Named { mutability: None, name, pattern: None, default: Some(default) } => {
                    // x
                    assert_name!(parser, *name, "x");
                    // 4
                    assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });
            // boolean
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            // = false
            assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
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

        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
            // x
            assert_string!(parser, *name, "x");
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_argument_named_string() {
        // "Content-Type": "application/json"
        let mut test = TestParser::new(r#""Content-Type": "application/json""#);
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { name: Name::String(name), value } => {
            // "Content-Type"
            assert_string!(parser, *name, "Content-Type");
            // "application/json"
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "application/json");
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

    #[test]
    fn test_parse_argument_dynamic() {
        // [x: string]: any
        let mut test = TestParser::new("[x: string]: any");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Dynamic { name, key, value } => {
            // x
            assert_string!(parser, name.unwrap(), "x");
            // string
            assert_node!(parser.tree, *key, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "string");
            });
            // any
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Any));
        });
    }
}
