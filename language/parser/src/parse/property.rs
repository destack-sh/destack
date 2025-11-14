#![allow(clippy::type_complexity)]

use dyst_ast::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionMode, Keyword, NodeId, NodeType,
    Property, TokenType,
};

use crate::{ParseError, ParseResult, Parser, ParserMark};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 6] = [
    Keyword::Static,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
];

impl<'a> Parser<'a> {
    /// Eat a tuple property.
    ///
    /// Examples:
    /// ```
    /// int32
    /// x: int32
    /// ```
    pub fn eat_tuple_type_property(&mut self) -> ParseResult<NodeId<Property>> {
        assert!(self.options.in_variant);
        let start = self.mark();

        // modifiers prefix
        let modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // key: type
        if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
            // key
            let key = self.eat_key()?;
            // colon
            self.eat_token(TokenType::Colon)?;
            // type
            let ty = self.with_options(self.options.not_in_position().in_type(), |parser| {
                parser.eat_expression()
            })?;
            // modifiers postfix
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;
            // property
            let property = Property::Field {
                modifiers,
                key: Some(key),
                value: Some(ty),
                default: None,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        }
        // type
        else {
            // type
            let ty = self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            // modifiers postfix
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;
            // property
            let property = Property::Field {
                modifiers,
                key: None,
                value: Some(ty),
                default: None,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        }
    }

    /// Eat a tuple property list (excluding the parenthesis).
    ///
    /// Examples:
    /// ```
    /// (x: int32, y: string)
    /// (x: int32, y: string, z: bool)
    /// ```
    pub fn eat_tuple_property_list(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<Vec<NodeId<Property>>> {
        let mut properties: Vec<NodeId<Property>> = Vec::new();
        while self.peek().is_ok() {
            // stop on terminator
            if self.peek_token(terminator).is_ok() || self.peek_token(TokenType::End).is_ok() {
                break;
            }
            // consume item separator
            else if self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                continue;
            }
            // property
            else {
                let property = self.eat_tuple_type_property()?;
                properties.push(property);
            }
        }
        Ok(properties)
    }

    /// Try to eat a property (return Property::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_property(&mut self, recover: TokenType) -> ParseResult<NodeId<Property>> {
        match self.eat_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err.clone()))?;
                Err(err)
            }
        }
    }

    /// Eat a property.
    ///
    /// Examples:
    /// ```
    /// // field
    /// x: int32
    /// x
    /// ...Bar
    /// a: T
    /// a?: T
    /// private b: int32 = 4
    /// public static c: int32 = 4
    ///
    /// // method
    /// foo()
    /// <T>(): T
    /// get x(): int32
    /// set x(value: int32): void
    /// private static foo(): void
    /// ```
    pub fn eat_property(&mut self) -> ParseResult<NodeId<Property>> {
        let start = self.mark();

        // spread property
        if self.peek_token(TokenType::Spread).is_ok() {
            let start = self.mark();
            self.bump(); // eat spread
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let property = Property::Spread {
                modifiers: None,
                value,
            };
            return Ok(self.tree.insert(property, self.get_span_from(start)));
        }

        // modifiers prefix
        let modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // abstraction
        let abstraction = if self.peek_keyword(Keyword::Abstract).is_ok() {
            self.bump(); // eat abstract keyword
            if self.peek_keyword(Keyword::Override).is_ok() {
                self.bump(); // eat override keyword
                Some(FunctionAbstraction::AbstractOverride)
            } else {
                Some(FunctionAbstraction::Abstract)
            }
        } else if self.peek_keyword(Keyword::Override).is_ok() {
            self.bump(); // eat override keyword
            Some(FunctionAbstraction::ConcreteOverride)
        } else {
            None
        };

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok() {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // mode
        let mode = {
            // getter
            if self.peek_keyword(Keyword::Get).is_ok()
                && self.peek_next_token(TokenType::Identifier).is_ok()
            {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.peek_keyword(Keyword::Set).is_ok()
                && self.peek_next_token(TokenType::Identifier).is_ok()
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.peek_keyword(Keyword::Constructor).is_ok()
                && (self.peek_next_token(TokenType::LessThan).is_ok()
                    || self.peek_next_token(TokenType::OpenParenthesis).is_ok())
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.peek_keyword(Keyword::New).is_ok()
                && (self.peek_next_token(TokenType::LessThan).is_ok()
                    || self.peek_next_token(TokenType::OpenParenthesis).is_ok())
            {
                self.bump(); // eat new keyword
                Some(FunctionMode::New)
            } else {
                None
            }
        };

        // generator
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;

        // key
        let key = self.eat_key_maybe()?;

        // modifiers postfix
        let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

        // method
        if abstraction.is_some()
            || is_async
            || is_generator
            || self.peek_token(TokenType::LessThan).is_ok()
            || self.peek_token(TokenType::OpenParenthesis).is_ok()
        {
            // methods without key or mode are implicit calls
            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // dynamic parameters
            let dynamic_parameters = self.eat_dynamic_parameters()?;

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

            // return type
            let return_type = if self.peek_colon().is_ok() {
                self.bump(); // eat colon
                let return_type = self.with_options(
                    self.options.nested().in_type().in_before_block(),
                    |parser| parser.eat_expression(),
                )?;
                Some(return_type)
            } else {
                None
            };

            // with clauses
            let with_clauses = self.eat_with_header_maybe()?;

            // where clauses
            let where_clauses = self.eat_where_maybe()?;

            // body
            let body = if self.peek_token(TokenType::OpenBrace).is_ok() {
                let body = self.with_options(
                    self.options.not_in_position().in_statement_position(),
                    |parser| parser.eat_expression(),
                )?;
                Some(body)
            } else {
                None
            };

            // method property
            let property = Property::Method {
                modifiers,
                key,
                asynchrony: if is_async {
                    Asynchrony::Async
                } else {
                    Asynchrony::Sync
                },
                abstraction: abstraction.unwrap_or(FunctionAbstraction::Concrete),
                cardinality: if is_generator {
                    FunctionCardinality::Generator
                } else {
                    FunctionCardinality::Scalar
                },
                mode,
                static_parameters,
                dynamic_parameters,
                return_type,
                with_clauses,
                where_clauses,
                body,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        }
        // field
        else {
            // value
            let value = if self.peek_colon().is_ok() {
                self.bump(); // eat colon
                let value = if self.options.in_variant {
                    self.with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })?
                } else {
                    self.with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })?
                };
                Some(value)
            } else {
                None
            };

            // default
            let default = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                let default = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(default)
            } else {
                None
            };

            // property
            if modifiers.is_none() && key.is_none() && value.is_none() && default.is_none() {
                // not a property
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::Identifier,
                ));
            }
            let property = Property::Field {
                modifiers,
                key,
                value,
                default,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        }
    }

    /// Eat a variant body (without the header or `{` and `}`).
    pub fn eat_properties(&mut self) -> ParseResult<Vec<NodeId<Property>>> {
        // eat everything
        let mut properties: Vec<NodeId<Property>> = Vec::new();
        while self.peek().is_ok() {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok()
                || self.peek_token(TokenType::End).is_ok()
            {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
                continue;
            }
            // keep eating properties
            else {
                match self.try_eat_property(TokenType::Newline) {
                    Ok(property_id) => {
                        properties.push(property_id);
                    }
                    Err(_) => continue, // keep eating other properties
                }
            }
        }
        Ok(properties)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Expression, FunctionMode, IntType, Key, Name, Parameter, Property, ScalarLiteral,
        TypeLiteral, Visibility,
    };
    use dyst_source::{LanguageCompatibility, LanguageOptions};

    use crate::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_property_with_es_visibility_modifier() {
        let mut test = TestParser::new_with_options(
            r#"#name: string"#,
            LanguageOptions::default().with_compatibility(LanguageCompatibility::TypeScript),
        );
        let mut parser = test.prepare();
        parser.options.in_type = true;

        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
            // #name means private for #Compatibility
            assert_eq!(modifiers.visibility.unwrap(), Visibility::Private);
            // name
            assert_string!(parser, *name, "name");
            // string
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    #[test]
    fn test_parse_property_with_value() {
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        parser.options.in_variant = true;
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    }

    #[test]
    fn test_parse_property_with_default_value() {
        let mut test = TestParser::new("x = 42");
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: None, default: Some(default), .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Integer(42)));
        });
    }

    #[test]
    fn test_parse_property_with_value_and_default_value() {
        let mut test = TestParser::new("x: int32 = 42");
        let mut parser = test.prepare();
        parser.options.in_variant = true;
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: Some(default), .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Integer(42)));
        });
    }

    #[test]
    fn test_parse_property_method_call() {
        let mut test = TestParser::new("<T = any>(x: T): T");
        let mut parser = test.prepare();
        let property_id = parser.eat_property().unwrap();
        // <T = any>(x: T): T
        assert_node!(parser.tree, property_id, Property::Method {
            mode,
            static_parameters,
            dynamic_parameters,
            return_type,
            ..
        } => {
            assert_eq!(*mode, Some(FunctionMode::Call));
            // <T = any>
            assert_eq!(static_parameters.as_ref().unwrap().len(), 1);
            assert_node!(parser.tree, static_parameters.as_ref().unwrap()[0], Parameter::Named { name, ty: None, default, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, default.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
            });
            // x: T
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_expr_path!(parser, parser.tree.get(ty.unwrap()), "T");
            });
            // T
            assert_expr_path!(parser, parser.tree.get(return_type.unwrap()), "T");
        });
    }

    #[test]
    fn test_parse_property_method_constructor() {
        let mut test = TestParser::new("constructor(x: int32);");
        let mut parser = test.prepare();

        let property_id = parser.eat_property().unwrap();
        assert_node!(parser.tree, property_id, Property::Method { mode, static_parameters, dynamic_parameters, .. } => {
            // constructor
            assert_eq!(*mode, Some(FunctionMode::Constructor));
            assert!(static_parameters.is_none());
            // x: int32
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }
}
