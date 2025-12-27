#![allow(clippy::type_complexity)]

use destack_ast::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Generics, Keyword, LocalNodeId, Member, NodeType, Property, TokenType,
};
use destack_source::NodeSpanType;

use crate::{ParseError, ParseResult, Parser, ParserMark};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 7] = [
    Keyword::Static,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
    Keyword::Comptime,
];

impl Parser {
    /// Try to eat a property (return Property::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_property(&mut self, recover: TokenType) -> ParseResult<LocalNodeId<Property>> {
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
    pub fn eat_property(&mut self) -> ParseResult<LocalNodeId<Property>> {
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
        let abstraction = if self.peek_keyword(Keyword::Abstract).is_ok()
            && self.peek_next_token(TokenType::LessThan).is_err()
            && self.peek_next_token(TokenType::OpenParenthesis).is_err()
        {
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
        let (key, key_span) = if let Some((key, span)) = self.eat_key_maybe_with_span()? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

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
            let (return_type, return_type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                let return_type = self.with_options(
                    self.options.nested().in_type().in_before_block(),
                    |parser| parser.eat_expression(),
                )?;
                (Some(return_type), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // where clauses
            let where_clauses = self.eat_where_maybe()?;

            let generics = Generics::new(static_parameters, where_clauses).into_option();

            // body
            let body = if self
                .peek_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
                .is_ok()
            {
                self.eat_newlines_maybe()?;
                let options = if is_generator {
                    self.options
                        .not_in_position()
                        .in_statement_position()
                        .in_generator()
                } else {
                    self.options.not_in_position().in_statement_position()
                };
                Some(self.with_options(options, |parser| parser.eat_expression())?)
            } else {
                None
            };

            // split out explicit this parameter
            let (this_parameter, dynamic_parameters) =
                self.split_this_parameter_maybe(dynamic_parameters);

            // method property
            let property = Property::Method {
                modifiers,
                key,
                signature: FunctionSignature {
                    abstraction: abstraction.unwrap_or(FunctionAbstraction::Concrete),
                    asynchrony: if is_async {
                        Asynchrony::Async
                    } else {
                        Asynchrony::Sync
                    },
                    cardinality: if is_generator {
                        FunctionCardinality::Generator
                    } else {
                        FunctionCardinality::Scalar
                    },
                    mode,
                    kind: FunctionKind::Function,
                    generics,
                    this_parameter,
                    dynamic_parameters,
                    return_type,
                },
                body,
            };
            let property_id = self.tree.insert(property, self.get_span_from(start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(property_id, span);
            }

            // set type span for return type annotation
            if let Some(span) = return_type_span {
                self.tree
                    .set_side_span(property_id, NodeSpanType::Type, span);
            }

            Ok(property_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                // keep in type / in variant (for `type x = { .. }` expressions)
                let value = if self.options.in_variant || self.options.in_type {
                    self.with_options(
                        self.options
                            .not_in_position()
                            .not_in_left_precedence()
                            .in_type(),
                        |parser| parser.eat_expression(),
                    )?
                } else {
                    self.with_options(
                        self.options.not_in_position().not_in_left_precedence(),
                        |parser| parser.eat_expression(),
                    )?
                };
                (Some(value), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
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
            let property_id = self.tree.insert(property, self.get_span_from(start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(property_id, span);
            }

            // set type span for field type annotation
            if let Some(span) = type_span {
                self.tree
                    .set_side_span(property_id, NodeSpanType::Type, span);
            }

            Ok(property_id)
        }
    }

    /// Eat a variant body (without the header or `{` and `}`).
    pub fn eat_properties(&mut self) -> ParseResult<Vec<LocalNodeId<Property>>> {
        // eat everything
        let mut properties: Vec<LocalNodeId<Property>> = Vec::new();
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

    /// Try to eat a member (return Member::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_member(&mut self, recover: TokenType) -> ParseResult<LocalNodeId<Member>> {
        match self.eat_member() {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err.clone()))?;
                Err(err)
            }
        }
    }

    /// Eat a member (class/struct/interface/extension body element).
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
    ///
    /// // static block (ES2022)
    /// static { console.log("init") }
    /// ```
    pub fn eat_member(&mut self) -> ParseResult<LocalNodeId<Member>> {
        let start = self.mark();

        // embed (type embedding via ...Type)
        if self.peek_token(TokenType::Spread).is_ok() {
            let start = self.mark();
            self.bump(); // eat spread
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let member = Member::Embed {
                modifiers: None,
                value,
            };
            return Ok(self.tree.insert(member, self.get_span_from(start)));
        }

        // modifiers prefix
        let modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // static block: `static { ... }` or `static\n{ ... }`
        // (must check *before* abstraction parsing since `static` is also a modifier)
        if modifiers
            .as_ref()
            .is_some_and(|m| m.anchor == Some(destack_ast::BindingAnchor::Static))
            && self
                .peek_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
                .is_ok()
        {
            self.eat_newlines_maybe()?;
            let body = self.with_options(
                self.options.not_in_position().in_statement_position(),
                |parser| parser.eat_expression(),
            )?;
            // preserve modifiers for validation (static blocks shouldn't have other modifiers)
            let member = Member::StaticBlock { modifiers, body };
            return Ok(self.tree.insert(member, self.get_span_from(start)));
        }

        // abstraction
        let abstraction = if self.peek_keyword(Keyword::Abstract).is_ok()
            && self.peek_next_token(TokenType::LessThan).is_err()
            && self.peek_next_token(TokenType::OpenParenthesis).is_err()
        {
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
        let (key, key_span) = if let Some((key, span)) = self.eat_key_maybe_with_span()? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

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
            let (return_type, return_type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                let return_type = self.with_options(
                    self.options.nested().in_type().in_before_block(),
                    |parser| parser.eat_expression(),
                )?;
                (Some(return_type), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // where clauses
            let where_clauses = self.eat_where_maybe()?;

            let generics = Generics::new(static_parameters, where_clauses).into_option();

            // body
            let body = if self
                .peek_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
                .is_ok()
            {
                self.eat_newlines_maybe()?;
                let options = if is_generator {
                    self.options
                        .not_in_position()
                        .in_statement_position()
                        .in_generator()
                } else {
                    self.options.not_in_position().in_statement_position()
                };
                Some(self.with_options(options, |parser| parser.eat_expression())?)
            } else {
                None
            };

            // split out explicit this parameter
            let (this_parameter, dynamic_parameters) =
                self.split_this_parameter_maybe(dynamic_parameters);

            // method member
            let member = Member::Method {
                modifiers,
                key,
                signature: FunctionSignature {
                    abstraction: abstraction.unwrap_or(FunctionAbstraction::Concrete),
                    asynchrony: if is_async {
                        Asynchrony::Async
                    } else {
                        Asynchrony::Sync
                    },
                    cardinality: if is_generator {
                        FunctionCardinality::Generator
                    } else {
                        FunctionCardinality::Scalar
                    },
                    mode,
                    kind: FunctionKind::Function,
                    generics,
                    this_parameter,
                    dynamic_parameters,
                    return_type,
                },
                body,
            };
            let member_id = self.tree.insert(member, self.get_span_from(start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            // set type span for return type annotation
            if let Some(span) = return_type_span {
                self.tree.set_side_span(member_id, NodeSpanType::Type, span);
            }

            Ok(member_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                // keep in type / in variant (for `type x = { .. }` expressions)
                let value = if self.options.in_variant || self.options.in_type {
                    self.with_options(
                        self.options
                            .not_in_position()
                            .not_in_left_precedence()
                            .in_type(),
                        |parser| parser.eat_expression(),
                    )?
                } else {
                    self.with_options(
                        self.options.not_in_position().not_in_left_precedence(),
                        |parser| parser.eat_expression(),
                    )?
                };
                (Some(value), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let default = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(default)
            } else {
                None
            };

            // member
            if modifiers.is_none() && key.is_none() && value.is_none() && default.is_none() {
                // not a member
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::Identifier,
                ));
            }
            let member = Member::Field {
                modifiers,
                key,
                value,
                default,
            };
            let member_id = self.tree.insert(member, self.get_span_from(start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            // set type span for field type annotation
            if let Some(span) = type_span {
                self.tree.set_side_span(member_id, NodeSpanType::Type, span);
            }

            Ok(member_id)
        }
    }

    /// Eat members (class/struct/interface/extension body).
    pub fn eat_members(&mut self) -> ParseResult<Vec<LocalNodeId<Member>>> {
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
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
            // keep eating members
            else {
                match self.try_eat_member(TokenType::Newline) {
                    Ok(member_id) => {
                        members.push(member_id);
                    }
                    Err(_) => continue, // keep eating other members
                }
            }
        }
        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Expression, FunctionMode, IntType, Key, Name, Parameter, Property, ScalarLiteral,
        TypeLiteral, Visibility,
    };
    use destack_source::LanguageType;

    use crate::tests::TestParser;
    use crate::{assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_property_with_es_visibility_modifier() {
        let mut test = TestParser::new_with_options(r#"#name: string"#, LanguageType::TypeScript);
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
        assert_node!(parser.tree, property_id, Property::Method { signature, .. } => {
            assert_eq!(signature.mode, Some(FunctionMode::Call));
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            // <T = any>
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, default.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
            });
            // x: T
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "T");
            });
            // T
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
        });
    }

    #[test]
    fn test_parse_property_method_constructor() {
        let mut test = TestParser::new("constructor(x: int32);");
        let mut parser = test.prepare();

        let property_id = parser.eat_property().unwrap();
        assert_node!(parser.tree, property_id, Property::Method { signature, .. } => {
            // constructor
            assert_eq!(signature.mode, Some(FunctionMode::Constructor));
            assert!(signature.generics.is_none());
            // x: int32
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }
}
