#![allow(clippy::type_complexity)]

use destack_ast::{
    AbstractionModifier, Asynchrony, BindingKind, BindingModifier, Expression, FunctionAbstraction,
    FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature, Generics, Keyword,
    LocalNodeId, Member, NodeType, Property, TokenType,
};
use destack_source::NodeSpanType;

use crate::parse::timing::tags;
use crate::{ParseError, ParseResult, Parser, ParserMark};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 8] = [
    Keyword::Static,
    Keyword::Abstract,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
    Keyword::Comptime,
];

impl Parser {
    /// Try to eat a property (return Property::Error if error and recovery is possible).
    pub fn try_eat_property(&mut self, recover: TokenType) -> ParseResult<LocalNodeId<Property>> {
        match self.eat_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize, None, false);
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
        let _timing = self.timing_scope(tags::PARSE_PROPERTY);
        let start = self.mark();

        // spread property
        if self.peek_is(TokenType::Spread) {
            let start = self.mark();
            self.bump(); // eat spread
            let value = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            let property = Property::Spread {
                modifiers: None,
                value,
            };
            return Ok(self.tree.insert(property, self.get_span_from(start)));
        }

        // modifiers prefix
        let mut modifiers = self.eat_binding_modifiers_prefix_maybe(true, true, false)?;

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok()
            && (self.peek_next_is(TokenType::Identifier)
                || self.peek_next_is(TokenType::Multiply)
                || self.peek_next_is(TokenType::OpenParenthesis)
                || self.peek_next_is(TokenType::LessThan))
        {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // late abstraction modifiers after async
        if is_async {
            let mut abstraction = modifiers.and_then(|modifiers| modifiers.abstraction);
            let mut has_abstraction = abstraction.is_some();
            loop {
                if self.peek_keyword(Keyword::Abstract).is_ok() {
                    self.bump(); // eat abstract
                    abstraction = Some(match abstraction {
                        None => AbstractionModifier::Abstract,
                        Some(AbstractionModifier::Override) => {
                            AbstractionModifier::AbstractOverride
                        }
                        Some(AbstractionModifier::Abstract) => AbstractionModifier::Abstract,
                        Some(AbstractionModifier::AbstractOverride) => {
                            AbstractionModifier::AbstractOverride
                        }
                    });
                    has_abstraction = true;
                    continue;
                }
                if self.peek_keyword(Keyword::Override).is_ok() {
                    self.bump(); // eat override
                    abstraction = Some(match abstraction {
                        None => AbstractionModifier::Override,
                        Some(AbstractionModifier::Abstract) => {
                            AbstractionModifier::AbstractOverride
                        }
                        Some(AbstractionModifier::Override) => AbstractionModifier::Override,
                        Some(AbstractionModifier::AbstractOverride) => {
                            AbstractionModifier::AbstractOverride
                        }
                    });
                    has_abstraction = true;
                    continue;
                }
                break;
            }

            if has_abstraction {
                let base = modifiers.unwrap_or_default();
                modifiers = Some(BindingModifier {
                    abstraction,
                    ..base
                });
            }
        }

        // mode
        let mode = {
            // getter
            if self.peek_keyword(Keyword::Get).is_ok() && self.peek_next_is(TokenType::Identifier) {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.peek_keyword(Keyword::Set).is_ok()
                && self.peek_next_is(TokenType::Identifier)
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.peek_keyword(Keyword::Constructor).is_ok()
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.peek_keyword(Keyword::New).is_ok()
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
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

        // definite assignment assertion
        let modifiers = if self.peek_is(TokenType::Not) {
            self.bump(); // eat !
            let base = modifiers.unwrap_or_default();
            Some(BindingModifier {
                kind: Some(BindingKind::Must),
                ..base
            })
        } else {
            modifiers
        };

        // modifiers postfix
        let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

        // method
        if is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
        {
            // abstraction
            let abstraction = modifiers
                .and_then(|modifiers| modifiers.abstraction)
                .map(|abstraction| match abstraction {
                    AbstractionModifier::Abstract => FunctionAbstraction::Abstract,
                    AbstractionModifier::Override => FunctionAbstraction::ConcreteOverride,
                    AbstractionModifier::AbstractOverride => FunctionAbstraction::AbstractOverride,
                })
                .unwrap_or(FunctionAbstraction::Concrete);
            let modifiers = modifiers
                .map(|modifiers| BindingModifier {
                    abstraction: None,
                    ..modifiers
                })
                .and_then(|modifiers| {
                    if modifiers == BindingModifier::default() {
                        None
                    } else {
                        Some(modifiers)
                    }
                });

            // methods without key or mode are implicit calls
            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // dynamic parameters
            let dynamic_parameters = self.with_options(
                self.options
                    .with_generator(is_generator)
                    .with_forbid_yield(is_generator),
                |parser| parser.eat_dynamic_parameters(),
            )?;

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

            // return type
            let (return_type, return_type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
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
                let options = self
                    .options
                    .not_in_position()
                    .in_statement_position()
                    .with_generator(is_generator);
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
                    abstraction,
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
            let (value, type_span): (
                Option<LocalNodeId<Expression>>,
                Option<destack_source::Span>,
            ) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;

                // parse type annotations in type or variant contexts
                let mut type_options = self
                    .options
                    .not_in_position()
                    .not_in_left_precedence()
                    .not_in_sequence_expression();
                if self.options.in_variant || self.options.in_type {
                    type_options = type_options.in_type();
                }
                let value = self.with_options(type_options, |parser| parser.eat_expression())?;
                (Some(value), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let default = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
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
        while self.has_more_tokens() {
            // stop on closing brace
            if self.peek_is(TokenType::CloseBrace) || self.peek_is(TokenType::End) {
                break;
            }
            // consume any stop
            else if self.is_any_stop() {
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
    pub fn try_eat_member(&mut self, recover: TokenType) -> ParseResult<LocalNodeId<Member>> {
        match self.eat_member() {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize, None, false);
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
        if self.peek_is(TokenType::Spread) {
            let start = self.mark();
            self.bump(); // eat spread
            let value = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            let member = Member::Embed {
                modifiers: None,
                value,
            };
            return Ok(self.tree.insert(member, self.get_span_from(start)));
        }

        // modifiers prefix
        let mut modifiers = self.eat_binding_modifiers_prefix_maybe(true, true, true)?;

        // static block: `static { ... }` or `static\n{ ... }`
        // must check before key parsing since static is already a modifier
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

        // type member: `type Name = ...` or `type Name: Bound`
        if self.peek_keyword(Keyword::Type).is_ok() && self.peek_next_is(TokenType::Identifier) {
            self.bump(); // eat type keyword
            // name is just an identifier (path)
            let path_start = self.mark();
            let (path, name_span) = self.eat_path_with_last_span()?;
            let path_span = self.get_span_from(path_start);
            let name = self.tree.insert(
                Expression::Path {
                    path,
                    static_arguments: None,
                },
                path_span,
            );
            self.tree.set_main_span(name, name_span);
            // optional type bound: `: Bound`
            let ty = if self.peek_colon().is_ok() {
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                Some(self.with_options(self.options.in_type(), |parser| parser.eat_expression())?)
            } else {
                None
            };
            // optional value: `= Type`
            let value = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                Some(self.with_options(self.options.in_type(), |parser| parser.eat_expression())?)
            } else {
                None
            };
            let member = Member::Type {
                modifiers,
                name,
                ty,
                value,
            };
            return Ok(self.tree.insert(member, self.get_span_from(start)));
        }

        // comptime block: `comptime { ... }` (timing modifier already consumed)
        if modifiers
            .as_ref()
            .is_some_and(|m| m.timing == Some(destack_ast::Timing::Comptime))
            && self
                .peek_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
                .is_ok()
        {
            self.eat_newlines_maybe()?;
            let body = self.with_options(
                self.options.not_in_position().in_statement_position(),
                |parser| parser.eat_expression(),
            )?;
            let member = Member::ComptimeBlock { modifiers, body };
            return Ok(self.tree.insert(member, self.get_span_from(start)));
        }

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok()
            && (self.peek_next_is(TokenType::Identifier)
                || self.peek_next_is(TokenType::Multiply)
                || self.peek_next_is(TokenType::OpenParenthesis)
                || self.peek_next_is(TokenType::LessThan))
        {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // late abstraction modifiers after async
        if is_async {
            let mut abstraction = modifiers.and_then(|modifiers| modifiers.abstraction);
            let mut has_abstraction = abstraction.is_some();
            loop {
                if self.peek_keyword(Keyword::Abstract).is_ok() {
                    self.bump(); // eat abstract
                    abstraction = Some(match abstraction {
                        None => AbstractionModifier::Abstract,
                        Some(AbstractionModifier::Override) => {
                            AbstractionModifier::AbstractOverride
                        }
                        Some(AbstractionModifier::Abstract) => AbstractionModifier::Abstract,
                        Some(AbstractionModifier::AbstractOverride) => {
                            AbstractionModifier::AbstractOverride
                        }
                    });
                    has_abstraction = true;
                    continue;
                }
                if self.peek_keyword(Keyword::Override).is_ok() {
                    self.bump(); // eat override
                    abstraction = Some(match abstraction {
                        None => AbstractionModifier::Override,
                        Some(AbstractionModifier::Abstract) => {
                            AbstractionModifier::AbstractOverride
                        }
                        Some(AbstractionModifier::Override) => AbstractionModifier::Override,
                        Some(AbstractionModifier::AbstractOverride) => {
                            AbstractionModifier::AbstractOverride
                        }
                    });
                    has_abstraction = true;
                    continue;
                }
                break;
            }

            if has_abstraction {
                let base = modifiers.unwrap_or_default();
                modifiers = Some(BindingModifier {
                    abstraction,
                    ..base
                });
            }
        }

        // mode
        let mode = {
            // getter
            if self.peek_keyword(Keyword::Get).is_ok() && self.peek_next_is(TokenType::Identifier) {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.peek_keyword(Keyword::Set).is_ok()
                && self.peek_next_is(TokenType::Identifier)
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.peek_keyword(Keyword::Constructor).is_ok()
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.peek_keyword(Keyword::New).is_ok()
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
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
        let (key, key_span) = if let Some((key, span)) = self
            .with_options(self.options.allow_private_hash_key(), |parser| {
                parser.eat_key_maybe_with_span()
            })? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

        // definite assignment assertion
        let modifiers = if self.peek_is(TokenType::Not) {
            self.bump(); // eat !
            let base = modifiers.unwrap_or_default();
            Some(BindingModifier {
                kind: Some(BindingKind::Must),
                ..base
            })
        } else {
            modifiers
        };

        // modifiers postfix
        let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

        // method
        if is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
        {
            // abstraction
            let abstraction = modifiers
                .and_then(|modifiers| modifiers.abstraction)
                .map(|abstraction| match abstraction {
                    AbstractionModifier::Abstract => FunctionAbstraction::Abstract,
                    AbstractionModifier::Override => FunctionAbstraction::ConcreteOverride,
                    AbstractionModifier::AbstractOverride => FunctionAbstraction::AbstractOverride,
                })
                .unwrap_or(FunctionAbstraction::Concrete);
            let modifiers = modifiers
                .map(|modifiers| BindingModifier {
                    abstraction: None,
                    ..modifiers
                })
                .and_then(|modifiers| {
                    if modifiers == BindingModifier::default() {
                        None
                    } else {
                        Some(modifiers)
                    }
                });

            // methods without key or mode are implicit calls
            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // dynamic parameters
            let dynamic_parameters = self.with_options(
                self.options
                    .with_generator(is_generator)
                    .with_forbid_yield(is_generator),
                |parser| parser.eat_dynamic_parameters(),
            )?;

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

            // return type
            let (return_type, return_type_span) = if self.peek_colon().is_ok() {
                let type_start = self.mark();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
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
                let options = self
                    .options
                    .not_in_position()
                    .in_statement_position()
                    .with_generator(is_generator);
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
                    abstraction,
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
                            .not_in_sequence_expression()
                            .in_type(),
                        |parser| parser.eat_expression(),
                    )?
                } else {
                    self.with_options(
                        self.options
                            .not_in_position()
                            .not_in_left_precedence()
                            .not_in_sequence_expression(),
                        |parser| parser.eat_expression(),
                    )?
                };
                (Some(value), Some(self.get_span_from(type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let default = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
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
        while self.has_more_tokens() {
            // stop on closing brace
            if self.peek_is(TokenType::CloseBrace) || self.peek_is(TokenType::End) {
                break;
            }
            // consume any stop
            else if self.is_any_stop() {
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
        AbstractionModifier, Asynchrony, BindingKind, Declaration, Expression, FunctionAbstraction,
        FunctionKind, FunctionMode, IntType, Key, Member, Name, Parameter, Property, ScalarLiteral,
        TypeLiteral, Visibility,
    };
    use destack_source::LanguageType;

    use crate::tests::TestParser;
    use crate::{assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_member_with_private_hash_name() {
        let mut test = TestParser::new_with_options(r#"#name: string"#, LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: None, key: Some(Key::Private(name)), value: Some(ty), default: None, .. } => {
            assert_string!(parser, *name, "name");
            assert_expression_path!(parser, parser.tree.get(*ty), "string");
        });
    }

    #[test]
    fn test_parse_member_definite_assignment() {
        let mut test = TestParser::new_with_options("prop!: Foo", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_eq!(modifiers.kind, Some(BindingKind::Must));
            assert_string!(parser, *name, "prop");
            assert_expression_path!(parser, parser.tree.get(*value), "Foo");
        });
    }

    #[test]
    fn test_parse_member_override_field() {
        let mut test =
            TestParser::new_with_options("override foo: int32", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_eq!(modifiers.abstraction, Some(AbstractionModifier::Override));
            assert_string!(parser, *name, "foo");
            assert_expression_path!(parser, parser.tree.get(*value), "int32");
        });
    }

    #[test]
    fn test_parse_member_abstract_override_method() {
        let mut test =
            TestParser::new_with_options("abstract override foo(): void", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { modifiers, key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
            assert!(modifiers.is_none());
            assert_string!(parser, *name, "foo");
            assert_eq!(signature.abstraction, FunctionAbstraction::AbstractOverride);
        });
    }

    #[test]
    fn test_parse_member_async_override_method() {
        let mut test = TestParser::new_with_options(
            "public async override foo(): void",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { modifiers, key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
            let modifiers = modifiers.expect("expected modifiers");
            assert_eq!(modifiers.visibility, Some(Visibility::Public));
            assert_string!(parser, *name, "foo");
            assert_eq!(signature.abstraction, FunctionAbstraction::ConcreteOverride);
            assert_eq!(signature.asynchrony, Asynchrony::Async);
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
    fn test_parse_property_with_typed_arrow_value() {
        let mut test = TestParser::new_with_options(
            "reproFunc: (_: any): any => { }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "reproFunc");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(_), .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                });
            });
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
    fn test_parse_property_definite_assignment() {
        let mut test = TestParser::new_with_options("prop!: LongType[]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_eq!(modifiers.kind, Some(BindingKind::Must));
            assert_string!(parser, *name, "prop");
            assert_node!(parser.tree, *value, Expression::Index { left, index, .. } => {
                assert!(index.is_none());
                assert_expression_path!(parser, parser.tree.get(*left), "LongType");
            });
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

    #[test]
    fn test_parse_member_computed_optional_method() {
        let mut test = TestParser::new_with_options(
            "[EventEmitter.captureRejectionSymbol]?<K>(error: Error): void",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.options.in_variant = true;

        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Method { modifiers, key: Some(Key::Expression(key)), signature, .. } => {
            let modifiers = modifiers.as_ref().expect("expected modifiers");
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_expression_path!(parser, parser.tree.get(*key), "EventEmitter.captureRejectionSymbol");
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert!(signature.return_type.is_some());
        });
    }

    #[test]
    fn test_parse_member_type_with_value() {
        let mut test = TestParser::new("type Item = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, ty: None, value: Some(value) } => {
            assert_expression_path!(parser, parser.tree.get(*name), "Item");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    #[test]
    fn test_parse_member_type_with_bound() {
        let mut test = TestParser::new("type Item: Hashable");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, ty: Some(ty), value: None } => {
            assert_expression_path!(parser, parser.tree.get(*name), "Item");
            assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
        });
    }

    #[test]
    fn test_parse_member_type_with_multiline_bound() {
        let mut test = TestParser::new(
            r#"type Item:
    | Foo
    | Bar"#,
        );
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, ty: Some(ty), value: None } => {
            assert_expression_path!(parser, parser.tree.get(*name), "Item");
            assert_node!(parser.tree, *ty, Expression::Binary { .. });
        });
    }

    #[test]
    fn test_parse_member_type_with_bound_and_value() {
        let mut test = TestParser::new("type Item: Hashable = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, ty: Some(ty), value: Some(value) } => {
            assert_expression_path!(parser, parser.tree.get(*name), "Item");
            assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    #[test]
    fn test_parse_member_type_with_visibility() {
        let mut test = TestParser::new("public type Item = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: Some(modifiers), name, ty: None, value: Some(_) } => {
            assert_eq!(modifiers.visibility.unwrap(), Visibility::Public);
            assert_expression_path!(parser, parser.tree.get(*name), "Item");
        });
    }

    #[test]
    fn test_parse_member_comptime_block() {
        let mut test = TestParser::new("comptime { assert(true) }");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::ComptimeBlock { modifiers: _, body } => {
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_comptime_block_newline() {
        let mut test = TestParser::new(
            r#"comptime
{ assert(true) }"#,
        );
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::ComptimeBlock { modifiers: _, body } => {
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_method_with_multiline_return_type() {
        let mut test = TestParser::new_with_options(
            r#"Type(object: unknown):
    | 'Undefined'
    | 'Boolean'
    | 'String'"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.options.in_variant = true;

        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Method { key: Some(Key::Name(name)), signature, .. } => {
            assert_string!(parser, name.string(), "Type");
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert!(signature.return_type.is_some());
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Binary { .. });
        });
    }
}
