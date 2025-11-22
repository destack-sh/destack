use dyst_ast::{
    Argument, BindingAnchor, BindingKind, BindingModifier, BindingOperator, Expression, Keyword,
    LocalNodeId, Mutability, Name, NodeType, Parameter, Pattern, ScalarLiteral, StringId,
    TokenType,
};

use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a binding modifiers prefix (visibility and mutability).
    pub fn eat_binding_modifiers_prefix_maybe(&mut self) -> ParseResult<Option<BindingModifier>> {
        let mut modifiers: Option<BindingModifier> = None;
        // visibility
        if let Ok(Some(visibility)) = self.peek_visibility() {
            self.bump(); // eat visibility
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().visibility = Some(visibility);
        }
        // scope
        if self.peek_keyword(Keyword::Static).is_ok() {
            self.bump(); // eat static
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().anchor = Some(BindingAnchor::Static);
        }
        // mutability
        if self.peek_keyword(Keyword::Readonly).is_ok() {
            self.bump(); // eat readonly
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().mutability = Some(Mutability::Immutable);
        }
        // operator
        if self.peek_keyword(Keyword::Const).is_ok() {
            self.bump(); // eat const
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().operator = Some(BindingOperator::AsConst);
        }
        Ok(modifiers)
    }

    /// Eat a binding modifiers postfix (maybe).
    #[inline]
    pub fn eat_binding_modifiers_postfix_maybe(
        &mut self,
        modifiers: Option<BindingModifier>,
    ) -> ParseResult<Option<BindingModifier>> {
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat maybe
            if let Some(modifiers) = modifiers {
                Ok(Some(BindingModifier {
                    kind: Some(BindingKind::Maybe),
                    ..modifiers
                }))
            } else {
                Ok(Some(BindingModifier {
                    kind: Some(BindingKind::Maybe),
                    ..BindingModifier::default()
                }))
            }
        } else {
            Ok(modifiers)
        }
    }

    /// Eat a binding modifiers postfix.
    #[inline]
    pub fn eat_binding_modifiers_postfix(
        &mut self,
        modifiers: Option<BindingModifier>,
    ) -> ParseResult<Option<BindingModifier>> {
        self.eat_token(TokenType::Maybe)?;
        if let Some(modifiers) = modifiers {
            Ok(Some(BindingModifier {
                kind: Some(BindingKind::Maybe),
                ..modifiers
            }))
        } else {
            Ok(Some(BindingModifier {
                kind: Some(BindingKind::Maybe),
                ..BindingModifier::default()
            }))
        }
    }

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
    pub fn eat_parameter(&mut self) -> ParseResult<LocalNodeId<Parameter>> {
        let start = self.mark();

        let mut modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // variadic
        let is_variadic = if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat range or range wide
            true
        } else {
            false
        };

        // pattern/name
        let (pattern, name): (Option<LocalNodeId<Pattern>>, Option<StringId>) = {
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
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat maybe
            if modifiers.is_none() {
                modifiers = Some(BindingModifier::default());
            }
            modifiers.as_mut().unwrap().kind = Some(BindingKind::Maybe);
        }

        // : type (or keyword for #Compatibility)
        let ty = {
            if self.peek_colon().is_ok()
                || (self.options.in_static
                    && (self.peek_keyword(Keyword::Extends).is_ok()
                        || self.peek_keyword(Keyword::Implements).is_ok()))
            {
                self.bump(); // eat colon or keyword
                self.eat_newlines_maybe()?;
                let ty = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })
                    .for_node_type(NodeType::Parameter)?;
                Some(ty)
            } else {
                None
            }
        };

        // = value
        let parameter = {
            // has default value
            if !is_variadic && self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let value = self
                    .with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })
                    .for_node_type(NodeType::Parameter)?;
                // named with default
                if let Some(name) = name {
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default: Some(value),
                    }
                }
                // pattern with default
                else {
                    Parameter::Pattern {
                        modifiers,
                        pattern: pattern.expect("peeked"),
                        ty,
                        default: Some(value),
                    }
                }
            }
            // variadic parameter (cannot have a default value)
            else if is_variadic {
                Parameter::Variadic {
                    modifiers,
                    name: name.expect("peeked"),
                    ty,
                }
            }
            // no default value
            else {
                // named without default
                if let Some(name) = name {
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default: None,
                    }
                }
                // pattern without default
                else {
                    Parameter::Pattern {
                        modifiers,
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
    pub fn eat_parameters_body(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let mut parameters: Vec<LocalNodeId<Parameter>> = Vec::new();
        self.eat_newlines_maybe()?;
        while self.peek_token(TokenType::Identifier).is_ok()
            // spread
            || self.peek_token(TokenType::Spread).is_ok()
            // pattern
            || self.peek_token(TokenType::OpenParenthesis).is_ok()
            || self.peek_token(TokenType::OpenBracket).is_ok()
            || self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Wildcard).is_ok()
        {
            let parameter = self.eat_parameter().for_node_type(NodeType::Parameter)?;
            parameters.push(parameter);
            self.eat_newlines_maybe()?;
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }

    /// Eat static parameters (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_parameters_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_parameters()?));
        }
        Ok(None)
    }

    /// Eat static parameters (including the `<` and `>` tokens).
    pub fn eat_static_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // empty static parameters
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            self.bump(); // eat greater than
            return Ok(vec![]);
        }

        // regular static parameters
        let parameters = self.with_options(self.options.nested().in_static(), |parser| {
            parser.eat_parameters_body()
        })?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(parameters)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_parameters_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Parameter>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_parameters()?));
        }
        Ok(None)
    }

    /// Eat dynamic parameters (including the `(` and `)` tokens).
    pub fn eat_dynamic_parameters(&mut self) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
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

    /// Eat an argument name (like `name:` prefix) if it exists.
    pub(crate) fn eat_argument_name_maybe(&mut self) -> ParseResult<Option<StringId>> {
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.bump(); // eat colon
            Ok(Some(name))
        } else {
            Ok(None)
        }
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
    pub fn eat_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();
        let modifiers = self.eat_binding_modifiers_prefix_maybe()?;
        // named argument
        if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
            let name = self.eat_name().for_node_type(NodeType::Argument)?;
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            // value
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Named {
                    modifiers,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // named maybe argument
        else if self.peek_name().is_ok()
            && self.peek_next_token(TokenType::Maybe).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            // name
            let name = self.eat_name()?;
            let modifiers = self.eat_binding_modifiers_postfix(modifiers)?;
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;
            // value
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Named {
                    modifiers,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // spread argument
        else if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat range
            let name = self.eat_argument_name_maybe()?;
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self.tree.insert(
                Argument::Positional { modifiers, value },
                self.get_span_from(start),
            );
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
    pub fn eat_tree_literal_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();
        let modifiers: Option<BindingModifier> = None;
        // spread argument
        if self.peek_token(TokenType::Spread).is_ok() {
            self.bump(); // eat range
            let name = self.eat_argument_name_maybe()?;
            let value = self.with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression()
            })?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // nested spread argument (like {...b} in tree literals for #Compatibility)
        else if self.peek_token(TokenType::OpenBrace).is_ok()
            && self.peek_next_token(TokenType::Spread).is_ok()
            && self.peek_next_next_token(TokenType::Identifier).is_ok()
        {
            self.bump(); // eat open brace
            self.bump(); // eat range
            let name = self.eat_argument_name_maybe()?;
            let value = self.with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression()
            })?;
            self.eat_token(TokenType::CloseBrace)?;
            let argument_id = self.tree.insert(
                Argument::Spread {
                    modifiers,
                    name,
                    value,
                },
                self.get_span_from(start),
            );
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
                self.eat_newlines_maybe()?;
                self.with_options(self.options.in_statement_position(), |parser| {
                    parser.eat_expression()
                })?
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
                    modifiers,
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
    }

    /// Eat static arguments (including the `<` and `>` tokens) if they exist.
    pub fn eat_static_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_token(TokenType::LessThan).is_ok() {
            return Ok(Some(self.eat_static_arguments()?));
        }
        Ok(None)
    }

    /// Eat static arguments (including the `<` and `>` tokens).
    pub fn eat_static_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // empty static arguments
        if self.peek_token(TokenType::GreaterThan).is_ok() {
            self.bump(); // eat greater than
            return Ok(vec![]);
        }

        // regular static arguments
        let static_arguments = self.with_options(self.options.nested().in_static(), |parser| {
            parser.eat_arguments_body(TokenType::GreaterThan)
        })?;

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(static_arguments)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens) if they exist.
    pub fn eat_dynamic_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            return Ok(Some(self.eat_dynamic_arguments()?));
        }
        Ok(None)
    }

    /// Eat dynamic arguments (including the `(` and `)` tokens).
    pub fn eat_dynamic_arguments(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
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
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments: Vec<LocalNodeId<Argument>> = Vec::new();
        self.eat_newlines_maybe()?;
        while self.peek().is_ok() {
            if self.peek_token(terminator).is_ok() {
                break;
            }
            let argument_id = self.eat_argument()?;
            arguments.push(argument_id);
            self.eat_newlines_maybe()?;
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
    use dyst_ast::{
        Argument, BindingKind, BindingOperator, Expression, IntType, Mutability, Name, Parameter,
        Pattern, PatternField, ScalarLiteral, TypeLiteral, Visibility,
    };

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_parameter_type_only() {
        // T
        let mut test = TestParser::new("T");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
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
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
        });
    }

    #[test]
    fn test_parse_parameter_with_default() {
        // validate: boolean = false
        let mut test = TestParser::new("validate: boolean = false");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty, default } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::Pattern { modifiers: _, pattern, ty: Some(ty), default: Some(default) } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { modifiers: _, name, ty } => {
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
        assert_node!(parser.tree, parameter_id, Parameter::Variadic { modifiers: _, name, ty } => {
            assert_string!(parser, *name, "args");
            assert!(ty.is_some());
        });
    }

    #[test]
    fn test_parse_parameter_multiline() {
        // x: int32
        let mut test = TestParser::new("x:\n\tint32");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: _, name, ty: Some(ty), default: None } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })));
        });
    }

    #[test]
    fn test_parse_parameter_with_modifiers() {
        // private readonly const x: 1
        let mut test = TestParser::new("private readonly const x: 1");
        let mut parser = test.prepare();
        let parameter_id = parser.eat_parameter().unwrap();
        assert_node!(parser.tree, parameter_id, Parameter::Named { modifiers: Some(modifiers), .. } => {
            assert_eq!(modifiers.visibility, Some(Visibility::Private));
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_eq!(modifiers.operator, Some(BindingOperator::AsConst));
        });
    }

    #[test]
    fn test_parse_argument_named() {
        // x: 1
        let mut test = TestParser::new("x: 1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            // x
            assert_string!(parser, *name, "x");
            // 1
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_argument_named_multiline() {
        // x: 1
        let mut test = TestParser::new("x:\n\t1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_argument_named_string() {
        // "Content-Type": "application/json"
        let mut test = TestParser::new(r#""Content-Type": "application/json""#);
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();

        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: _, name: Name::String(name), value } => {
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

        assert_node!(parser.tree, argument_id, Argument::Positional { modifiers: _, value } => {
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
        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, name: None, value } => {
            // ...args
            assert_node!(parser.tree, *value, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "args");
            });
        });
    }

    #[test]
    fn test_parse_argument_spread_with_name() {
        // ...args
        let mut test = TestParser::new("...args: x");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Spread { modifiers: _, name: Some(name), value } => {
            // ...args
            assert_string!(parser, *name, "args");
            // x
            assert_expression_path!(parser, parser.tree.get(*value), "x");
        });
    }

    #[test]
    fn test_parse_argument_with_modifiers() {
        // private readonly const x: 1
        let mut test = TestParser::new("private readonly const x: 1");
        let mut parser = test.prepare();
        let argument_id = parser.eat_argument().unwrap();
        assert_node!(parser.tree, argument_id, Argument::Named { modifiers: Some(modifiers), .. } => {
            assert_eq!(modifiers.visibility, Some(Visibility::Private));
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_eq!(modifiers.operator, Some(BindingOperator::AsConst));
        });
    }
}
