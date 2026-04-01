#![allow(clippy::type_complexity)]

use destack_ast::{
    AbstractionModifier, Asynchrony, BindingAnchor, BindingKind, BindingModifier, BindingOperator,
    BlockContext, Expression, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Generics, Key, Keyword, LocalNodeId, Member, Name, NodeType, Parameter,
    Property, Timing, TokenType, Visibility,
};
use destack_source::NodeSpanType;

use super::annotation::PendingDecorators;
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
    /// Eat a spread or embed value expression.
    #[inline]
    fn eat_property_value_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self.options;
        let expression_context = self
            .options
            .not_in_position()
            .not_in_left_precedence()
            .not_in_sequence_expression();
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat method parameters in property or member contexts.
    #[inline]
    fn eat_property_dynamic_parameters(
        &mut self,
        is_generator: bool,
    ) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let ambient_context = self
            .options
            .with_generator(is_generator)
            .with_forbid_yield(is_generator);
        self.with_options(
            self.options.with_ambient_context(ambient_context),
            |parser| parser.eat_dynamic_parameters(),
        )
    }

    /// Eat a property or member return type.
    #[inline]
    fn eat_property_return_type(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let mut ambient_context = self.options.nested().with_type(true);
        if !self.peek_is(TokenType::OpenBrace) {
            ambient_context = ambient_context.with_before_block(true);
        }
        let expression_context = self.options.nested();
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat a property or member method body expression.
    #[inline]
    fn eat_property_method_body(
        &mut self,
        is_generator: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self
            .options
            .with_generator(is_generator)
            .with_decorator(false);
        let expression_context = self
            .options
            .not_in_position()
            .with_statement_position(true)
            .with_sequence_expression(true);
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat a property field type expression.
    #[inline]
    fn eat_property_field_type(
        &mut self,
        is_type_context: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self.options.nested().with_type(is_type_context);
        let expression_context = self
            .options
            .not_in_position()
            .not_in_left_precedence()
            .not_in_sequence_expression();
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat a property field default expression.
    #[inline]
    fn eat_property_default_expression(
        &mut self,
        preserve_nested_context: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = if preserve_nested_context {
            self.options.nested()
        } else {
            self.options
        };
        let expression_context = self
            .options
            .not_in_position()
            .not_in_left_precedence()
            .not_in_sequence_expression();
        self.eat_expression(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat a member type expression.
    #[inline]
    fn eat_member_type_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self.options.with_type(true);
        self.eat_expression(self.options.with_ambient_context(ambient_context))
    }

    /// Eat a key with private hash parsing enabled.
    #[inline]
    fn eat_property_key_with_span(&mut self) -> ParseResult<Option<(Key, destack_source::Span)>> {
        let ambient_context = self.options.with_allow_private_hash_key(true);
        self.with_options(
            self.options.with_ambient_context(ambient_context),
            |parser| parser.eat_key_maybe_with_span(),
        )
    }

    /// Try to eat a property and recover into one error slot when possible.
    pub fn try_eat_property(&mut self, _recover: TokenType) -> ParseResult<LocalNodeId<Property>> {
        match self.eat_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Property);
                let span = err.leaf_span();
                let start = ParserMark::from_span(span);
                self.try_recover_in_body(&start, Some(err.clone()))?;

                Ok(self.insert_node(Property::Error, self.get_span_from(&start)))
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
        let start = self.mark_span();

        // spread property
        if self.peek_is(TokenType::Spread) {
            let start = self.mark_span();
            self.bump(); // eat spread
            let value = self.eat_property_value_expression()?;
            let property = Property::Spread {
                modifiers: None,
                value,
            };
            return Ok(self.insert_node(property, self.get_span_from(&start)));
        }

        // modifiers prefix
        let mut modifiers =
            self.eat_binding_modifiers_prefix_maybe(true, true, false, false, false)?;

        // async
        let is_async = if self.is_keyword(Keyword::Async)
            && (self.peek_next_is(TokenType::Identifier)
                || self.peek_next_is(TokenType::Literal)
                || self.peek_next_is(TokenType::Hash)
                || self.peek_next_is(TokenType::OpenBracket)
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
                if self.is_keyword(Keyword::Abstract) {
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
                if self.is_keyword(Keyword::Override) {
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
            if self.is_keyword(Keyword::Get)
                && self.next_token_starts_member_name()
                && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
            {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.is_keyword(Keyword::Set)
                && self.next_token_starts_member_name()
                && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.is_keyword(Keyword::Constructor)
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.is_keyword(Keyword::New)
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat new keyword
                Some(FunctionMode::New)
            } else {
                None
            }
        };

        // (allow newlines after get/set)
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) {
            self.eat_newlines_maybe()?;
        }

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

        // private keys usually cannot have explicit visibility modifiers
        // ts compatibility: allow `private accessor #name` forms
        let allow_private_accessor_visibility = matches!(key, Some(Key::Private(_)))
            && modifiers.as_ref().is_some_and(|modifiers| {
                modifiers.visibility == Some(Visibility::Private) && modifiers.accessor.is_some()
            });
        if matches!(key, Some(Key::Private(_)))
            && modifiers
                .as_ref()
                .is_some_and(|modifiers| modifiers.visibility.is_some())
            && !allow_private_accessor_visibility
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // reject optional + definite assignment combo
        if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe))
            && self.peek_is(TokenType::Not)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // reject async? method(...) token glue
        if !is_async
            && modifiers
                .as_ref()
                .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe))
            && matches!(
                key,
                Some(Key::Name(Name::Identifier(name))) if self.strings.get(name) == "async"
            )
            && !self.peek_is(TokenType::Newline)
            && self.peek_is(TokenType::Identifier)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // object fields cannot start with an unkeyed call signature
        if !self.options.is_in_type()
            && key.is_none()
            && mode.is_none()
            && !is_async
            && !is_generator
            && self.peek_is(TokenType::OpenParenthesis)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // associated comptime constants are field-like members with explicit names
        let associated_comptime_name = if modifiers.as_ref().is_some_and(|modifiers| {
            modifiers.timing == Some(Timing::Comptime)
                && modifiers.operator == Some(BindingOperator::AsConst)
        }) {
            match key.as_ref() {
                Some(Key::Name(Name::Identifier(name))) => Some(*name),
                _ => return Err(ParseError::unexpected(self.peek()?.span)),
            }
        } else {
            None
        };
        // method
        let is_method = is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter));

        // modifiers without a key or call signature are invalid
        if key.is_none() && modifiers.is_some() && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // getters and setters require method syntax
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method syntax
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
            let static_parameters = self.eat_static_parameters_maybe(false)?;

            // dynamic parameters
            self.eat_newlines_maybe()?;
            let dynamic_parameters = self.eat_property_dynamic_parameters(is_generator)?;

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

            // return type
            let has_return_type_marker = self.peek_colon_is()
                || self.peek_is(TokenType::Newline)
                    && self.is_token_after_newlines(self.pos(), TokenType::Colon);
            let (return_type, return_type_span) = if has_return_type_marker {
                let type_start = self.mark_span();
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::Colon)?;
                self.eat_newlines_maybe()?;
                let return_type = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    self.eat_property_return_type()?
                };
                (Some(return_type), Some(self.get_span_from(&type_start)))
            } else {
                (None, None)
            };

            // where clauses
            let where_clauses = if self.language.is_destack() {
                self.eat_where_maybe()?
            } else {
                None
            };
            let generics = Generics::new(static_parameters, where_clauses).into_option();

            // body
            let body = if self
                .is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
            {
                self.eat_newlines_maybe()?;
                Some(self.eat_property_method_body(is_generator)?)
            } else {
                None
            };

            // methods without bodies must end at a member boundary
            if body.is_none() && !self.is_any_stop() && !self.peek_is(TokenType::CloseBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
            let property_id = self.insert_node(property, self.get_span_from(&start));

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
            ) = if self.peek_colon_is() {
                let type_start = self.mark_span();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;

                // parse type annotations in type or variant contexts
                let is_type_context = self.options.is_in_variant() || self.options.is_in_type();
                let value = if self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    self.eat_property_field_type(is_type_context)?
                };
                (Some(value), Some(self.get_span_from(&type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                // keep associated comptime defaults in expression mode
                let default = if self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    self.eat_property_default_expression(associated_comptime_name.is_some())?
                };
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
            let property_id = self.insert_node(property, self.get_span_from(&start));

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
        let mut pending_property_decorators = PendingDecorators::new();
        while self.has_more_tokens() {
            // normalize cursor to the next non newline token
            self.eat_newlines_maybe()?;
            let token_type = self.peek_token_type();

            // stop on closing brace
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                if !pending_property_decorators.is_empty() {
                    let error = ParseError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_property_decorators.clear();
                }
                break;
            }
            // consume decorator prefixes in type literal properties
            else if self.options.is_in_type() && token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_property_decorators.extend(decorators);
                continue;
            }
            // consume comma separators between properties
            else if token_type == TokenType::Comma {
                self.eat_item_stop_with_newlines()?;
                continue;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop_with_newlines()?;
                continue;
            }
            // keep eating properties
            else {
                match self.try_eat_property(TokenType::Newline) {
                    Ok(property_id) => {
                        if !pending_property_decorators.is_empty() {
                            self.attach_decorators(
                                property_id.id,
                                std::mem::take(&mut pending_property_decorators),
                            );
                        }
                        properties.push(property_id);
                    }
                    Err(_) => continue, // keep eating other properties
                }
            }
        }
        Ok(properties)
    }

    /// Try to eat a member and recover into one error slot when possible.
    pub fn try_eat_member(&mut self, _recover: TokenType) -> ParseResult<LocalNodeId<Member>> {
        match self.eat_member() {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Member);
                let span = err.leaf_span();
                let start = ParserMark::from_span(span);
                self.try_recover_in_body(&start, Some(err.clone()))?;

                Ok(self.insert_node(Member::Error, self.get_span_from(&start)))
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
        let start = self.mark_span();

        // embed (type embedding via ...Type)
        if self.peek_is(TokenType::Spread) {
            let start = self.mark_span();
            self.bump(); // eat spread
            let value = self.eat_expression(
                self.options
                    .not_in_position()
                    .not_in_left_precedence()
                    .not_in_sequence_expression(),
            )?;
            let member = Member::Embed {
                modifiers: None,
                value,
            };
            return Ok(self.insert_node(member, self.get_span_from(&start)));
        }

        // modifiers prefix
        let mut modifiers =
            self.eat_binding_modifiers_prefix_maybe(true, true, true, true, true)?;

        // duplicate static modifier across newlines
        if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static))
            && self.peek_is(TokenType::Newline)
        {
            let static_index = self.next_non_newline_index_from(self.pos_index() + 1);
            let is_duplicate_static = self.keyword_for_index(static_index) == Some(Keyword::Static);
            if is_duplicate_static {
                let after_static = self.next_non_newline_index_from(static_index + 1);
                let keyword_after_static = self.keyword_for_index(after_static);
                let has_member_name_after = self.token_ref_at(after_static).is_some_and(|token| {
                    token.token.ty == TokenType::Identifier && keyword_after_static.is_none()
                });
                if has_member_name_after {
                    let span = if let Some(token) = self.token_ref_at(static_index) {
                        token.span
                    } else {
                        self.peek()?.span
                    };
                    let error = ParseError::unexpected(span);
                    self.error(&error);
                    self.eat_newlines_maybe()?;
                    self.bump(); // eat static
                }
            }
        }

        // static block: `static { ... }` or `static\n{ ... }`
        // must check before key parsing since static is already a modifier
        if modifiers
            .as_ref()
            .is_some_and(|m| m.anchor == Some(BindingAnchor::Static))
            && self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
        {
            self.eat_newlines_maybe()?;
            let body_start = self.mark_span();
            let body_block = self.eat_block(BlockContext::Statement)?;
            let body = self.insert_node(
                Expression::Block(body_block),
                self.get_span_from(&body_start),
            );
            // preserve modifiers for validation (static blocks shouldn't have other modifiers)
            let member = Member::StaticBlock { modifiers, body };
            return Ok(self.insert_node(member, self.get_span_from(&start)));
        }

        // type member: `type Name<U> = ...` or `type Name: Bound`
        if self.is_keyword(Keyword::Type) && self.peek_next_is(TokenType::Identifier) {
            self.bump(); // eat type keyword

            // parse type member name
            let (name, name_span) = self.eat_identifier_with_span()?;

            // parse optional static parameters and where clauses
            let static_parameters = self.eat_static_parameters_maybe(false)?;
            let where_clauses = self.eat_where_maybe()?;

            // optional type bound: `: Bound`
            let ty = if self.peek_colon_is() {
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                Some(
                    if self.peek_is(TokenType::Assign)
                        || self.peek_is(TokenType::CloseBrace)
                        || self.is_any_stop()
                    {
                        self.recover_missing_expression_here(NodeType::Member)
                    } else {
                        self.eat_member_type_expression()?
                    },
                )
            } else {
                None
            };
            // optional value: `= Type`
            let value = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                Some(
                    if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                        self.recover_missing_expression_here(NodeType::Member)
                    } else {
                        self.eat_member_type_expression()?
                    },
                )
            } else {
                None
            };
            let member = Member::Type {
                modifiers,
                name,
                static_parameters,
                where_clauses,
                ty,
                value,
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));
            self.tree.set_main_span(member_id, name_span);

            return Ok(member_id);
        }

        // comptime block: `comptime { ... }` (timing modifier already consumed)
        if modifiers
            .as_ref()
            .is_some_and(|m| m.timing == Some(Timing::Comptime))
            && self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
        {
            self.eat_newlines_maybe()?;
            let body_start = self.mark_span();
            let body_block = self.eat_block(BlockContext::Statement)?;
            let body = self.insert_node(
                Expression::Block(body_block),
                self.get_span_from(&body_start),
            );
            let member = Member::ComptimeBlock { modifiers, body };
            return Ok(self.insert_node(member, self.get_span_from(&start)));
        }

        // allow newline between modifiers and the member key
        if modifiers.is_some() {
            self.eat_newlines_maybe()?;
        }

        // async
        let is_async = if self.is_keyword(Keyword::Async)
            && (self.peek_next_is(TokenType::Identifier)
                || self.peek_next_is(TokenType::Literal)
                || self.peek_next_is(TokenType::Hash)
                || self.peek_next_is(TokenType::OpenBracket)
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
                if self.is_keyword(Keyword::Abstract) {
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
                if self.is_keyword(Keyword::Override) {
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
            if self.is_keyword(Keyword::Get)
                && self.next_token_starts_member_name()
                && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
            {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.is_keyword(Keyword::Set)
                && self.next_token_starts_member_name()
                && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.is_keyword(Keyword::Constructor)
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.is_keyword(Keyword::New)
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::OpenParenthesis))
            {
                self.bump(); // eat new keyword
                Some(FunctionMode::New)
            } else {
                None
            }
        };
        // (allow newlines after get/set)
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) {
            self.eat_newlines_maybe()?;
        }

        // generator
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;

        // key
        let key_result = self.eat_property_key_with_span()?;
        let (key, key_span) = if let Some((key, span)) = key_result {
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

        // private keys usually cannot have explicit visibility modifiers
        // ts compatibility: allow `private accessor #name` forms
        let allow_private_accessor_visibility = matches!(key, Some(Key::Private(_)))
            && modifiers.as_ref().is_some_and(|modifiers| {
                modifiers.visibility == Some(Visibility::Private) && modifiers.accessor.is_some()
            });
        if matches!(key, Some(Key::Private(_)))
            && modifiers
                .as_ref()
                .is_some_and(|modifiers| modifiers.visibility.is_some())
            && !allow_private_accessor_visibility
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // reject optional + definite assignment combo
        if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe))
            && self.peek_is(TokenType::Not)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // reject async? method(...) token glue
        if !is_async
            && modifiers
                .as_ref()
                .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe))
            && matches!(
                key,
                Some(Key::Name(Name::Identifier(name))) if self.strings.get(name) == "async"
            )
            && !self.peek_is(TokenType::Newline)
            && self.peek_is(TokenType::Identifier)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // associated comptime constants are field-like members with explicit names
        let associated_comptime_name = if modifiers.as_ref().is_some_and(|modifiers| {
            modifiers.timing == Some(Timing::Comptime)
                && modifiers.operator == Some(BindingOperator::AsConst)
        }) {
            match key.as_ref() {
                Some(Key::Name(Name::Identifier(name))) => Some(*name),
                _ => return Err(ParseError::unexpected(self.peek()?.span)),
            }
        } else {
            None
        };

        // method
        let is_method = is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter));

        // getters and setters require method syntax
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method syntax
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
            let static_parameters = self.eat_static_parameters_maybe(false)?;

            // allow line breaks before the parameter list
            self.eat_newlines_maybe()?;

            // dynamic parameters
            let dynamic_parameters = self.eat_property_dynamic_parameters(is_generator)?;

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

            // return type
            let has_return_type_marker = self.peek_colon_is()
                || self.peek_is(TokenType::Newline)
                    && self.is_token_after_newlines(self.pos(), TokenType::Colon);
            let (return_type, return_type_span) = if has_return_type_marker {
                let type_start = self.mark_span();
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::Colon)?;
                self.eat_newlines_maybe()?;
                let return_type = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                    self.recover_missing_expression_here(NodeType::Member)
                } else {
                    self.eat_property_return_type()?
                };
                (Some(return_type), Some(self.get_span_from(&type_start)))
            } else {
                (None, None)
            };

            // where clauses are a destack only feature
            let where_clauses = if self.language.is_destack() {
                self.eat_where_maybe()?
            } else {
                None
            };
            let generics = Generics::new(static_parameters, where_clauses).into_option();

            // body
            let body = if self
                .is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
            {
                self.eat_newlines_maybe()?;
                Some(self.eat_property_method_body(is_generator)?)
            } else {
                None
            };

            // methods without bodies must end at a member boundary
            if body.is_none() && !self.is_any_stop() && !self.peek_is(TokenType::CloseBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
            let member_id = self.insert_node(member, self.get_span_from(&start));

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
            let (value, type_span) = if self.peek_colon_is() {
                let type_start = self.mark_span();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                // member field annotations are always type positions
                let value = if self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Member)
                } else {
                    self.eat_expression(
                        self.options
                            .nested()
                            .not_in_position()
                            .not_in_left_precedence()
                            .not_in_sequence_expression()
                            .in_type(),
                    )?
                };
                (Some(value), Some(self.get_span_from(&type_start)))
            } else {
                (None, None)
            };

            // default
            let default = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;
                let default = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                    self.recover_missing_expression_here(NodeType::Member)
                } else {
                    self.eat_expression(
                        self.options
                            .not_in_position()
                            .not_in_left_precedence()
                            .not_in_sequence_expression(),
                    )?
                };
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
            // fields without initializers must end at a statement boundary
            if value.is_none()
                && default.is_none()
                && !self.is_statement_stop()
                && !self.peek_is(TokenType::CloseBrace)
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            let member = if let Some(name) = associated_comptime_name {
                Member::ComptimeConst {
                    modifiers,
                    name,
                    ty: value,
                    value: default,
                }
            } else {
                Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

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
    pub fn eat_members(
        &mut self,
        allow_comma_separators: bool,
    ) -> ParseResult<Vec<LocalNodeId<Member>>> {
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut pending_member_decorators = PendingDecorators::new();
        while self.has_more_tokens() {
            // normalize cursor to the next non newline token
            self.eat_newlines_maybe()?;
            let token_type = self.peek_token_type();

            // stop on closing brace
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                if !pending_member_decorators.is_empty() {
                    let error = ParseError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_member_decorators.clear();
                }
                break;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                // declarations that disallow comma separators
                if !allow_comma_separators && token_type == TokenType::Comma {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                self.eat_any_stop_with_newlines()?;
                continue;
            }
            // consume decorator prefixes
            else if token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_member_decorators.extend(decorators);
                continue;
            }
            // keep eating members
            else {
                match self.try_eat_member(TokenType::Newline) {
                    Ok(member_id) => {
                        if !pending_member_decorators.is_empty() {
                            self.attach_decorators(
                                member_id.id,
                                std::mem::take(&mut pending_member_decorators),
                            );
                        }
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
        AbstractionModifier, AccessorKind, Argument, Asynchrony, BinaryOperator, BindingAnchor,
        BindingKind, BindingOperator, Block, CommentStyle, Declaration, DeclarationKind,
        Expression, FunctionAbstraction, FunctionKind, FunctionMode, IntType, Key, Member, Name,
        Parameter, Property, ScalarLiteral, Timing, TypeLiteral, TypePredicateSubject, Visibility,
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
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
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
    fn test_parse_member_accessor_definite_assignment() {
        let mut test = TestParser::new_with_options("accessor a!: any", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_eq!(modifiers.accessor, Some(AccessorKind::Accessor));
            assert_eq!(modifiers.kind, Some(BindingKind::Must));
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Any));
        });
    }

    #[test]
    fn test_parse_member_declare_accessor_private_hash() {
        let mut test = TestParser::new_with_options(
            "private declare accessor #value: string",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: Some(modifiers), key: Some(Key::Private(name)), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "value");
            assert_eq!(modifiers.visibility, Some(Visibility::Private));
            assert_eq!(modifiers.declaration, Some(DeclarationKind::Declaration));
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
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
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { is_signed: true, width: Some(32) })));
        });
    }

    #[test]
    fn test_parse_member_default_object_arrow_with_this_member_call_argument() {
        let mut test = TestParser::new_with_options(
            r"
port2 = {
  postMessage: () => {
    setTimeout(this.port1.onmessage, 0);
  }
}
",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        parser.options.set_in_variant(true);
        let member_id = parser.eat_member().unwrap();

        // port2 = { postMessage: () => { setTimeout(this.port1.onmessage, 0) } }
        assert_node!(parser.tree, member_id, Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: None, default: Some(default), .. } => {
            assert_string!(parser, *name, "port2");
            assert_node!(parser.tree, *default, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                    assert_string!(parser, *name, "postMessage");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::Statement(expression) => {
                                        assert_node!(parser.tree, *expression, Expression::Call { dynamic_arguments, .. } => {
                                            assert_eq!(dynamic_arguments.len(), 2);
                                            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                                                assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                                                    assert_string!(parser, *name, "onmessage");
                                                    assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                                        assert_string!(parser, *name, "port1");
                                                        assert_node!(parser.tree, *left, Expression::This);
                                                    });
                                                });
                                            });
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            });
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
    fn test_parse_member_method_parameter_type_then_default_value_typescript() {
        let mut test = TestParser::new_with_options(
            "usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        // parse one method where a typed parameter is followed by a defaulted parameter
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(_), .. } => {
            assert_string!(parser, *name, "usersLimitReached");
            assert_eq!(signature.dynamic_parameters.len(), 2);

            // first parameter: userCount: number
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), default, .. } => {
                assert_string!(parser, *name, "userCount");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
            });

            // second parameter: userLimit = get(this.store).userLimit
            assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { name, ty, default: Some(default), .. } => {
                assert_string!(parser, *name, "userLimit");
                assert!(ty.is_none());
                assert_node!(parser.tree, *default, Expression::Member { name, .. } => {
                    assert_string!(parser, *name, "userLimit");
                });
            });
        });

        // parsing this signature should not emit recovery diagnostics
        assert!(parser.errors.is_empty());
    }

    #[test]
    fn test_parse_member_method_generic_with_newline_before_parameters_typescript() {
        let mut test = TestParser::new_with_options(
            "private method<T>\n(value: T): T { return value }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        // parse one method with a generic parameter and a newline before dynamic parameters
        assert_node!(parser.tree, member, Member::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(body) } => {
            assert_eq!(modifiers.visibility, Some(Visibility::Private));
            assert_string!(parser, *name, "method");

            // parse the generic, dynamic parameter, and return type as one coherent signature
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "T");
            });
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "value");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "T");

            // keep a method body attached after the multiline signature
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_method_with_newline_before_return_type_typescript() {
        let mut test = TestParser::new_with_options(
            "method(value: string)\n: string { return value }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        // parse one method with a newline before return type marker
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(body), .. } => {
            assert_string!(parser, *name, "method");

            // keep the dynamic parameter and return type attached to the same method signature
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
            });
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypeLiteral(TypeLiteral::String));

            // keep a method body attached after the multiline return type annotation
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_method_object_union_return_type_typescript() {
        let mut test = TestParser::new_with_options(
            "overlaps(): { overlaps: false } | { overlaps: true; reason: string }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: None, .. } => {
            assert_string!(parser, *name, "overlaps");

            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(parser.tree, *left, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                });
                assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 2);
                });
            });
        });
    }

    #[test]
    fn test_parse_member_method_body_boundary_comment_on_return_type_typescript() {
        let mut test = TestParser::new_with_options(
            "method(): number // method-body\n{ return 1 }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        parser.attach_trivia();
        assert_node!(parser.tree, member, Member::Method { signature, body: Some(body), .. } => {
            let return_type = signature.return_type.expect("expected return type");
            let return_type_annotations = parser.tree.get_annotations(return_type.id);
            assert!(return_type_annotations.is_empty());

            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let body_statement_annotations = parser.tree.get_annotations(expressions[0].id);
                    assert!(body_statement_annotations.is_empty());
                });
            });
        });
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "method-body");
    }

    #[test]
    fn test_parse_member_async_string_literal_name() {
        let mut test = TestParser::new_with_options(
            r#"async 'delete'(name: string): Promise<boolean> { return true }"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::String(name))), signature, body, .. } => {
            assert_string!(parser, *name, "delete");
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "name");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
            });
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::QualifiedReference { path, static_arguments: Some(static_arguments) } => {
                assert_path!(parser, *path, "Promise");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Boolean));
                });
            });
            assert_node!(parser.tree, body.expect("expected method body"), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_member_method_named_public_in_javascript() {
        let mut test = TestParser::new_with_options("public() {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { modifiers, key: Some(Key::Name(Name::Identifier(name))), .. } => {
            assert!(modifiers.is_none());
            assert_string!(parser, *name, "public");
        });
    }

    #[test]
    fn test_parse_member_static_method_named_protected_in_javascript() {
        let mut test =
            TestParser::new_with_options("static protected() {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), .. } => {
            assert_eq!(modifiers.anchor, Some(BindingAnchor::Static));
            assert_string!(parser, *name, "protected");
        });
    }

    #[test]
    fn test_parse_member_field_named_static_in_javascript() {
        let mut test = TestParser::new_with_options("static", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
            assert_string!(parser, *name, "static");
        });
    }

    #[test]
    fn test_parse_member_missing_default_expression() {
        // x =
        let mut test = TestParser::new("x =");
        let mut parser = test.prepare();
        let member = parser.eat_member().unwrap();

        assert_eq!(parser.errors.len(), 1);

        // x =
        assert_node!(parser.tree, member, Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: None, default: Some(default), .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *default, Expression::Missing);
        });
    }

    #[test]
    fn test_parse_members_recover_error_slot() {
        // +\ny: int32
        let mut test = TestParser::new("+\ny: int32");
        let mut parser = test.prepare();
        let members = parser.eat_members(false).unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_eq!(members.len(), 2);

        // error, y: int32
        assert_node!(parser.tree, members[0], Member::Error);
        assert_node!(parser.tree, members[1], Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "y");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    }

    #[test]
    fn test_reject_member_method_signature_without_separator() {
        let mut test = TestParser::new_with_options("method() method2()", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let result = parser.eat_member();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_interface_get_set_with_newlines() {
        let mut test = TestParser::new_with_options(
            r#"interface Foo {
  get
  foo(): string;
  set
  bar(v);
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();
        // parse interface members with get and set
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Interface { members, .. } => {
                let mut getter: Option<Key> = None;
                let mut setter: Option<Key> = None;
                for member_id in members {
                    if let Member::Method { signature, key, .. } = parser.tree.get(*member_id) {
                        match signature.mode {
                            Some(FunctionMode::Getter) => getter = *key,
                            Some(FunctionMode::Setter) => setter = *key,
                            _ => {}
                        }
                    }
                }
                let getter = getter.expect("expected getter member");
                let setter = setter.expect("expected setter member");
                assert!(matches!(getter, Key::Name(Name::Identifier(_))));
                assert!(matches!(setter, Key::Name(Name::Identifier(_))));
            });
        });
    }

    #[test]
    fn test_parse_member_get_set_newline_only() {
        let mut test = TestParser::new_with_options(
            r#"get
foo(): string;"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let member = parser.with_options(parser.options.in_variant(), |parser| parser.eat_member());
        assert!(
            member.is_ok(),
            "unexpected member parse error: {:#?}",
            member.err()
        );
    }

    #[test]
    fn test_parse_property_with_value() {
        let mut test = TestParser::new("x: int32");
        let mut parser = test.prepare();
        parser.options.set_in_variant(true);
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
    fn test_parse_property_missing_value_expression() {
        // x:
        let mut test = TestParser::new("x:");
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();

        assert_eq!(parser.errors.len(), 1);

        // x:
        assert_node!(parser.tree, property, Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::Missing);
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
        parser.options.set_in_variant(true);
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
    fn test_parse_properties_recover_error_slot() {
        // +\ny: int32
        let mut test = TestParser::new("+\ny: int32");
        let mut parser = test.prepare();
        parser.options.set_in_variant(true);
        let properties = parser.eat_properties().unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_eq!(properties.len(), 2);

        // error, y: int32
        assert_node!(parser.tree, properties[0], Property::Error);
        assert_node!(parser.tree, properties[1], Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "y");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
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
    fn test_parse_property_method_object_return_type() {
        let mut test = TestParser::new("method(): { value: string; count: number }");
        let mut parser = test.prepare();
        let property_id = parser.eat_property().unwrap();

        assert_node!(parser.tree, property_id, Property::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
            assert_string!(parser, *name, "method");
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);
            });
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
    fn test_parse_constructor_parameter_property_readonly_public_modifier_order_reports_error() {
        let mut test = TestParser::new_with_options(
            r"class D extends B {
  constructor(readonly public foo: string) {}
}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.parse();

        assert_eq!(parser.errors.len(), 1);
    }

    #[test]
    fn test_parse_member_computed_optional_method() {
        let mut test = TestParser::new_with_options(
            "[EventEmitter.captureRejectionSymbol]?<K>(error: Error): void",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.options.set_in_variant(true);

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
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, static_parameters: None, where_clauses: None, ty: None, value: Some(value) } => {
            assert_string!(parser, *name, "Item");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    #[test]
    fn test_parse_member_type_with_bound() {
        let mut test = TestParser::new("type Item: Hashable");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, static_parameters: None, where_clauses: None, ty: Some(ty), value: None } => {
            assert_string!(parser, *name, "Item");
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
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, static_parameters: None, where_clauses: None, ty: Some(ty), value: None } => {
            assert_string!(parser, *name, "Item");
            assert_node!(parser.tree, *ty, Expression::Binary { .. });
        });
    }

    #[test]
    fn test_parse_member_type_with_bound_and_value() {
        let mut test = TestParser::new("type Item: Hashable = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: None, name, static_parameters: None, where_clauses: None, ty: Some(ty), value: Some(value) } => {
            assert_string!(parser, *name, "Item");
            assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    #[test]
    fn test_parse_member_type_with_visibility() {
        let mut test = TestParser::new("public type Item = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { modifiers: Some(modifiers), name, static_parameters: None, where_clauses: None, ty: None, value: Some(_) } => {
            assert_eq!(modifiers.visibility.unwrap(), Visibility::Public);
            assert_string!(parser, *name, "Item");
        });
    }

    #[test]
    fn test_parse_member_type_with_static_parameters() {
        let mut test = TestParser::new("type View<U> = [Item, U]");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Type { name, static_parameters: Some(static_parameters), where_clauses: None, ty: None, value: Some(_), .. } => {
            assert_string!(parser, *name, "View");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "U");
            });
        });
    }

    #[test]
    fn test_parse_member_associated_comptime_const() {
        let mut test = TestParser::new("comptime const Rows: number = 128");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::ComptimeConst { modifiers: Some(modifiers), name, ty: Some(ty), value: Some(value) } => {
            assert_eq!(modifiers.timing, Some(Timing::Comptime));
            assert_eq!(modifiers.operator, Some(BindingOperator::AsConst));
            assert_string!(parser, *name, "Rows");
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(value) => {
                assert_eq!(*value, ScalarLiteral::Integer(128));
            });
        });
    }

    #[test]
    fn test_parse_member_associated_comptime_const_binary_default() {
        let mut test = TestParser::new("comptime const LaneWidth: number = WidthHint * 2");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::ComptimeConst { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::Multiply);
            });
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
        parser.options.set_in_variant(true);

        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Method { key: Some(Key::Name(name)), signature, .. } => {
            assert_string!(parser, name.string(), "Type");
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert!(signature.return_type.is_some());
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Binary { .. });
        });
    }

    #[test]
    fn test_parse_member_method_with_type_predicate_return_type() {
        let mut test = TestParser::new_with_options(
            "public isDynamicModule(module: Type<any> | DynamicModule): module is DynamicModule",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.options.set_in_variant(true);

        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::Method { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypePredicate { asserts, subject, target } => {
                assert!(!asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("module")));
                assert_expression_path!(parser, parser.tree.get(target.unwrap()), "DynamicModule");
            });
        });
    }

    #[test]
    fn test_parse_class_member_trailing_comments_stay_on_member_owner() {
        let mut test = TestParser::new_with_options(
            r#"class Box {
  first = 1 // first-tail
  second = 2 // second-tail
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 2);

                let first_annotations = parser.tree.get_annotations(members[0].id);
                assert!(first_annotations.is_empty());

                let second_annotations = parser.tree.get_annotations(members[1].id);
                assert!(second_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comment_trivia().len(), 2);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "first-tail");
        crate::assert_comment_trivia!(parser, 1, CommentStyle::Slash, "second-tail");
    }
}
