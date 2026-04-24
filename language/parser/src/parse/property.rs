#![allow(clippy::type_complexity)]

use destack_ast::{
    Ambientness, AssignOperator, Asynchrony, BlockContext, ConstructorTypeDeclaration, Expression,
    FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature, FunctionTypeDeclaration,
    Key, Keyword, LocalNodeId, Member, Name, NodeType, Parameter, Property, StringId, TokenType,
    TypeExpression, TypeMember, Visibility,
};
use destack_source::{NodeSpanType, Span};

use super::PendingDecorators;
use crate::parse::argument::BindingModifiers;
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

/// Parsed head for one property or member.
#[derive(Debug)]
struct ParsedPropertyMemberHead {
    /// The parsed modifiers.
    modifiers: Option<BindingModifiers>,
    /// The parsed key.
    key: Option<Key>,
    /// The key span.
    key_span: Option<Span>,
    /// The parsed method mode.
    mode: Option<FunctionMode>,
    /// Whether the head is async.
    is_async: bool,
    /// Whether the head is a generator.
    is_generator: bool,
    /// Whether the head is method-shaped.
    is_method: bool,
    /// The associated comptime constant name when present.
    associated_comptime_name: Option<StringId>,
}

/// Parsed method signature tail.
#[derive(Debug)]
struct ParsedMethodTail {
    /// The parsed signature.
    signature: FunctionSignature,
    /// The generic parameter container span.
    generic_parameter_span: Option<Span>,
    /// The parameter container span.
    parameter_span: Span,
    /// The optional body.
    body: Option<LocalNodeId<Expression>>,
    /// The return type span.
    return_type_span: Option<Span>,
}

/// Build one call signature declaration from a parsed function signature.
fn function_type_declaration_from_signature(
    signature: FunctionSignature,
) -> FunctionTypeDeclaration {
    debug_assert!(signature.mode.is_none() || signature.mode == Some(FunctionMode::Call));

    FunctionTypeDeclaration {
        generic_parameters: signature.generic_parameters,
        where_clauses: signature.where_clauses,
        this_parameter: signature.this_parameter,
        parameters: signature.parameters,
        return_type: signature.return_type,
    }
}

/// Build one construct signature declaration from a parsed function signature.
fn constructor_type_declaration_from_signature(
    signature: FunctionSignature,
) -> ConstructorTypeDeclaration {
    debug_assert!(matches!(
        signature.mode,
        Some(FunctionMode::Constructor | FunctionMode::New)
    ));

    ConstructorTypeDeclaration {
        is_abstract: signature.is_abstract,
        generic_parameters: signature.generic_parameters,
        where_clauses: signature.where_clauses,
        parameters: signature.parameters,
        return_type: signature.return_type,
    }
}

impl Parser {
    /// Eat one `async` keyword when it plausibly starts a method head.
    #[inline]
    fn eat_method_async_maybe(&mut self) -> bool {
        if self.is_keyword(Keyword::Async)
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
        }
    }

    /// Eat late modifiers after an `async` head.
    #[inline]
    fn eat_method_late_modifiers_maybe(
        &mut self,
        mut modifiers: Option<BindingModifiers>,
        is_async: bool,
    ) -> Option<BindingModifiers> {
        if !is_async {
            return modifiers;
        }

        let mut modifier_set = modifiers.unwrap_or_default();
        let mut has_modifiers = !modifier_set.is_empty();

        loop {
            if self.is_keyword(Keyword::Abstract) {
                self.bump(); // eat abstract
                modifier_set.is_abstract = true;
                has_modifiers = true;
                continue;
            }

            if self.is_keyword(Keyword::Override) {
                self.bump(); // eat override
                modifier_set.is_override = true;
                has_modifiers = true;
                continue;
            }

            break;
        }

        if has_modifiers && !modifier_set.is_empty() {
            modifiers = Some(modifier_set);
        }

        modifiers
    }

    /// Eat one method mode.
    #[inline]
    fn eat_method_mode_maybe(&mut self) -> Option<FunctionMode> {
        if self.is_keyword(Keyword::Get)
            && self.next_token_starts_member_name()
            && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
        {
            self.bump(); // eat get keyword
            Some(FunctionMode::Getter)
        } else if self.is_keyword(Keyword::Set)
            && self.next_token_starts_member_name()
            && !self.is_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
        {
            self.bump(); // eat set keyword
            Some(FunctionMode::Setter)
        } else if self.is_keyword(Keyword::Constructor)
            && (self.peek_next_is(TokenType::LessThan)
                || self.peek_next_is(TokenType::OpenParenthesis))
        {
            self.bump(); // eat constructor keyword
            Some(FunctionMode::Constructor)
        } else if self.is_keyword(Keyword::New)
            && (self.peek_next_is(TokenType::LessThan)
                || self.peek_next_is(TokenType::OpenParenthesis))
        {
            self.bump(); // eat new keyword
            Some(FunctionMode::New)
        } else {
            None
        }
    }

    /// Return true when the current head must parse as a method.
    #[inline]
    fn head_starts_method(
        &mut self,
        mode: Option<FunctionMode>,
        is_async: bool,
        is_generator: bool,
    ) -> bool {
        is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter))
    }

    /// Eat one definite member modifier when present.
    #[inline]
    fn eat_definite_modifier_maybe(
        &mut self,
        modifiers: Option<BindingModifiers>,
    ) -> Option<BindingModifiers> {
        if !self.peek_is(TokenType::Not) {
            return modifiers;
        }

        self.bump(); // eat !
        let mut modifiers = modifiers.unwrap_or_default();
        modifiers.is_definite = true;

        Some(modifiers)
    }

    /// Validate one parsed member or property head after key and postfix modifiers.
    /// FUGU #Architecture: double check which (if any) Parser::validate_* methods are really local?
    fn validate_method_head_modifiers(
        &mut self,
        key: Option<&Key>,
        modifiers: Option<&BindingModifiers>,
        is_async: bool,
    ) -> ParseResult<()> {
        // private keys do not take explicit visibility modifiers
        // typed member forms allow `private accessor #name`
        let allow_private_accessor_visibility = matches!(key, Some(Key::Private(_)))
            && modifiers.is_some_and(|modifiers| {
                modifiers.visibility == Some(Visibility::Private) && modifiers.is_accessor
            });
        if matches!(key, Some(Key::Private(_)))
            && modifiers.is_some_and(|modifiers| modifiers.visibility.is_some())
            && !allow_private_accessor_visibility
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // reject optional + definite assignment combo
        if modifiers.is_some_and(|modifiers| modifiers.is_optional && modifiers.is_definite) {
            let error_span = self
                .prev()
                .map(|token| token.span)
                .unwrap_or(self.peek()?.span);
            return Err(ParseError::unexpected(error_span));
        }

        // reject async? method(...) token glue
        if !is_async
            && modifiers.is_some_and(|modifiers| modifiers.is_optional)
            && matches!(
                key,
                Some(Key::Name(Name::Identifier(name))) if self.strings.get(*name) == "async"
            )
            && !self.peek_is(TokenType::Newline)
            && self.peek_is(TokenType::Identifier)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        Ok(())
    }

    /// Return the associated comptime constant name when the parsed head forms one.
    fn associated_comptime_name_maybe(
        &mut self,
        key: Option<&Key>,
        modifiers: Option<&BindingModifiers>,
    ) -> ParseResult<Option<StringId>> {
        if !modifiers.is_some_and(|modifiers| modifiers.is_comptime && modifiers.is_const_asserted)
        {
            return Ok(None);
        }

        match key {
            Some(Key::Name(Name::Identifier(name))) => Ok(Some(*name)),
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Return true when `readonly` starts a type property modifier.
    #[inline]
    fn type_member_readonly_modifier_maybe(&mut self) -> bool {
        if !self.is_keyword(Keyword::Readonly) {
            return false;
        }

        let next_index = self.next_non_newline_index_from(self.pos_index() + 1);
        matches!(
            self.token_type_at(next_index),
            TokenType::Identifier | TokenType::Literal | TokenType::Hash | TokenType::OpenBracket
        )
    }

    /// Return true when `abstract` starts a type property modifier.
    #[inline]
    fn type_member_abstract_modifier_maybe(&mut self) -> bool {
        if !self.is_keyword(Keyword::Abstract) {
            return false;
        }

        let next_index = self.next_non_newline_index_from(self.pos_index() + 1);
        matches!(
            self.token_type_at(next_index),
            TokenType::Identifier
                | TokenType::Literal
                | TokenType::Hash
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::LessThan
        ) || self.keyword_for_index(next_index) == Some(Keyword::New)
    }

    /// Return true when the current `[` starts an index signature.
    #[inline]
    fn type_member_starts_index_signature(&mut self) -> bool {
        if !self.peek_is(TokenType::OpenBracket) {
            return false;
        }

        let name_index = self.next_non_newline_index_from(self.pos_index() + 1);
        if self.token_type_at(name_index) != TokenType::Identifier {
            return false;
        }

        let colon_index = self.next_non_newline_index_from(name_index + 1);
        self.token_type_at(colon_index) == TokenType::Colon
    }

    /// Return true when the current type property head must parse as a method.
    #[inline]
    fn type_member_head_starts_method(&mut self, mode: Option<FunctionMode>) -> bool {
        self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(
                mode,
                Some(
                    FunctionMode::Getter
                        | FunctionMode::Setter
                        | FunctionMode::Constructor
                        | FunctionMode::New
                )
            )
    }

    /// Eat one spread or embed value expression.
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

    /// Return the ambientness implied by one modifier set.
    #[inline]
    fn ambientness_for_modifiers(&self, modifiers: Option<&BindingModifiers>) -> Ambientness {
        if modifiers.is_some_and(|modifiers| modifiers.is_ambient) {
            Ambientness::Ambient
        } else {
            Ambientness::Concrete
        }
    }

    /// Eat method parameters in property or member contexts.
    #[inline]
    fn eat_method_parameters(
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

    /// Eat one method or field type expression.
    #[inline]
    fn eat_method_return_type(
        &mut self,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let mut ambient_context = self.options.nested().with_type(true);
        if !self.peek_is(TokenType::OpenBrace) {
            ambient_context = ambient_context.with_before_block(true);
        }
        let expression_context = self
            .options
            .nested()
            .not_in_left_precedence()
            .not_in_sequence_expression();
        self.eat_type_expression_node_or_recover_missing(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            owner,
        )
    }

    /// Eat one method body expression.
    #[inline]
    fn eat_method_body_expression(
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

    /// Eat one shared property or member head.
    fn eat_property_member_head(
        &mut self,
        mut modifiers: Option<BindingModifiers>,
        allow_newlines_after_modifiers: bool,
    ) -> ParseResult<ParsedPropertyMemberHead> {
        // allow newline between modifiers and the key
        if allow_newlines_after_modifiers && modifiers.is_some() {
            self.eat_newlines_maybe()?;
        }

        // async and late abstraction modifiers
        let is_async = self.eat_method_async_maybe();
        modifiers = self.eat_method_late_modifiers_maybe(modifiers, is_async);

        // mode and accessor newline
        let mode = self.eat_method_mode_maybe();
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) {
            self.eat_newlines_maybe()?;
        }

        // generator and key
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;
        let (key, key_span) = if let Some((key, span)) = self.eat_property_key_with_span()? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

        // postfix modifiers
        let modifiers = self.eat_definite_modifier_maybe(modifiers);
        let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;
        self.validate_method_head_modifiers(key.as_ref(), modifiers.as_ref(), is_async)?;

        // classify the head
        let associated_comptime_name =
            self.associated_comptime_name_maybe(key.as_ref(), modifiers.as_ref())?;
        let is_method = self.head_starts_method(mode, is_async, is_generator);

        Ok(ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            mode,
            is_async,
            is_generator,
            is_method,
            associated_comptime_name,
        })
    }

    /// Eat the shared tail of one method head.
    ///
    /// Examples:
    /// ```
    /// (): int32
    /// <T>(value: T): T
    /// get value(): int32
    /// set value(next: int32): void
    /// where T: Copy { value }
    /// ```
    fn eat_method_tail(
        &mut self,
        owner: NodeType,
        mode: Option<FunctionMode>,
        asynchrony: Asynchrony,
        cardinality: FunctionCardinality,
        is_abstract: bool,
        is_override: bool,
        allows_body: bool,
    ) -> ParseResult<ParsedMethodTail> {
        // generic parameters and parameters
        let generic_parameter_start = self.mark_span();
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)?
            .unwrap_or_default();
        let generic_parameter_span =
            (!generic_parameters.is_empty()).then(|| self.get_span_from(&generic_parameter_start));

        self.eat_newlines_maybe()?;
        let parameter_start = self.mark_span();
        let parameters =
            self.eat_method_parameters(cardinality == FunctionCardinality::Generator)?;
        let parameter_span = self.get_span_from(&parameter_start);

        // return type
        let has_return_type_marker = self.peek_colon_is()
            || self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Colon);
        let (return_type, return_type_span) = if has_return_type_marker {
            self.eat_newlines_maybe()?;
            let type_start = self.mark_span();
            self.eat_token(TokenType::Colon)?;
            self.eat_newlines_maybe()?;

            let return_type = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                self.recover_missing_type_expression_here(owner)
            } else {
                self.eat_method_return_type(owner)?
            };
            (Some(return_type), Some(self.get_span_from(&type_start)))
        } else {
            (None, None)
        };

        // where clauses
        let where_clauses = if self.language.is_destack() {
            self.eat_where_maybe()?.unwrap_or_default()
        } else {
            Vec::new()
        };

        // body
        let body = if allows_body
            && self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
        {
            self.eat_newlines_maybe()?;
            Some(self.eat_method_body_expression(cardinality == FunctionCardinality::Generator)?)
        } else {
            None
        };

        // reject invalid trailing tokens
        if allows_body {
            if body.is_none() && !self.is_any_stop() && !self.peek_is(TokenType::CloseBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        } else {
            if self.peek_is(TokenType::OpenBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if !self.is_any_stop() && !self.peek_is(TokenType::CloseBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        }

        // split out the explicit this parameter
        let (this_parameter, parameters) = self.split_this_parameter_maybe(parameters);

        // build the signature
        let signature = FunctionSignature {
            is_abstract,
            is_override,
            asynchrony,
            cardinality,
            mode,
            kind: FunctionKind::Function,
            generic_parameters,
            where_clauses,
            this_parameter,
            parameters,
            return_type,
        };

        Ok(ParsedMethodTail {
            signature,
            generic_parameter_span,
            parameter_span,
            body,
            return_type_span,
        })
    }

    /// Eat one member type expression.
    #[inline]
    fn eat_member_type_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let ambient_context = self.options.with_type(true);
        self.eat_type_expression_node_or_recover_missing(
            self.options.with_ambient_context(ambient_context),
            NodeType::Member,
        )
    }

    /// Eat a key with private hash parsing enabled.
    #[inline]
    fn eat_property_key_with_span(&mut self) -> ParseResult<Option<(Key, Span)>> {
        let ambient_context = self.options.with_allow_private_hash_key(true);
        self.with_options(
            self.options.with_ambient_context(ambient_context),
            |parser| parser.eat_key_maybe_with_span(),
        )
    }

    /// Try to eat one associated type member.
    ///
    /// Examples:
    /// ```
    /// type Item
    /// type Item = string
    /// type Item: Display
    /// type Item<T> where T: Copy = Vec<T>
    /// ```
    fn try_eat_associated_type_member(
        &mut self,
        start: &ParserMark,
        modifiers: Option<BindingModifiers>,
    ) -> ParseResult<Option<LocalNodeId<Member>>> {
        if !(self.is_keyword(Keyword::Type) && self.peek_next_is(TokenType::Identifier)) {
            return Ok(None);
        }

        // keyword and name
        self.bump(); // eat type keyword
        let (name, name_span) = self.eat_identifier_with_span()?;

        // generic parameters and where clauses
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)?
            .unwrap_or_default();
        let where_clauses = self.eat_where_maybe()?.unwrap_or_default();

        // declared type
        let constraint = if self.peek_colon_is() {
            self.bump(); // eat colon
            self.eat_newlines_maybe()?;

            let constraint = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.is_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::Member)
            } else {
                self.eat_member_type_expression()?
            };

            Some(constraint)
        } else {
            None
        };

        // value
        let value = if self.peek_is(TokenType::Assign) {
            self.bump(); // eat assign
            self.eat_newlines_maybe()?;

            let value = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                self.recover_missing_type_expression_here(NodeType::Member)
            } else {
                self.eat_member_type_expression()?
            };

            Some(value)
        } else {
            None
        };

        // member
        let member = Member::AssociatedType {
            name,
            generic_parameters,
            where_clauses,
            constraint,
            value,
            visibility: modifiers.and_then(|modifiers| modifiers.visibility),
            ambient: self.ambientness_for_modifiers(modifiers.as_ref()),
            is_abstract: modifiers.is_some_and(|modifiers| modifiers.is_abstract),
            is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
            is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
        };
        let member_id = self.insert_node(member, self.get_span_from(start));
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(member_id))
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
            let property = Property::Spread { value };
            return Ok(self.insert_node(property, self.get_span_from(&start)));
        }

        // modifiers and head
        let modifiers = self.eat_binding_modifiers_prefix_maybe(true, true, false, false, false)?;
        let ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            mode,
            is_async,
            is_generator,
            is_method,
            associated_comptime_name,
        } = self.eat_property_member_head(modifiers, false)?;

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

        // modifiers without a key or call signature are invalid
        if key.is_none() && modifiers.is_some() && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // getters and setters require method form
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method form
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // abstraction
            // methods without key or mode are implicit calls
            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;
            let ParsedMethodTail {
                signature,
                body,
                generic_parameter_span: _,
                parameter_span: _,
                return_type_span,
            } = self.eat_method_tail(
                NodeType::Property,
                mode,
                if is_async {
                    Asynchrony::Async
                } else {
                    Asynchrony::Sync
                },
                if is_generator {
                    FunctionCardinality::Generator
                } else {
                    FunctionCardinality::Scalar
                },
                modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                modifiers.is_some_and(|modifiers| modifiers.is_override),
                true,
            )?;

            // method property
            let property = Property::Method {
                key,
                signature,
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
            let (value, type_span): (Option<LocalNodeId<Expression>>, Option<Span>) =
                if self.peek_colon_is() {
                    let type_start = self.mark_span();
                    self.bump(); // eat colon
                    self.eat_newlines_maybe()?;

                    // field type
                    let is_type_context = self.options.is_in_variant() || self.options.is_in_type();
                    let value = if self.peek_is(TokenType::Assign)
                        || self.peek_is(TokenType::Comma)
                        || self.peek_is(TokenType::CloseBrace)
                        || self.is_any_stop()
                    {
                        self.recover_missing_expression_here(NodeType::Property)
                    } else {
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
                        )?
                    };
                    let type_span = self.get_span_from(&type_start);
                    (Some(value), Some(type_span))
                } else {
                    (None, None)
                };

            // default
            let (default, assign_operator_span) = if self.peek_is(TokenType::Assign) {
                let assign_start = self.mark_span();
                self.bump(); // eat assign
                self.eat_newlines_maybe()?;

                // keep associated comptime defaults in expression mode
                let default = if self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    let ambient_context = if associated_comptime_name.is_some() {
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
                    )?
                };
                (Some(default), Some(self.get_span_from(&assign_start)))
            } else {
                (None, None)
            };

            // preserve both the declared type and the default
            let value = match (value, default) {
                (Some(value), Some(default)) => {
                    let value_span = self.tree.get_span(value);
                    let default_span = self.tree.get_span(default);
                    let assign_span =
                        Span::new(value_span.file, value_span.start, default_span.end);
                    let left = self.expression_to_assign_pattern(value)?;
                    let assign_id = self.insert_node(
                        Expression::Assign {
                            left,
                            operator: AssignOperator::Assign,
                            right: default,
                        },
                        assign_span,
                    );

                    if let Some(assign_operator_span) = assign_operator_span {
                        self.tree.set_main_span(assign_id, assign_operator_span);
                    }

                    Some(assign_id)
                }
                (Some(value), None) => Some(value),
                (None, Some(default)) => Some(default),
                (None, None) => None,
            };

            // shorthand field value
            let is_shorthand = value.is_none()
                && default.is_none()
                && modifiers.is_none()
                && matches!(key, Some(Key::Name(Name::Identifier(_))));

            let value = if is_shorthand {
                match key {
                    Some(Key::Name(Name::Identifier(name))) => {
                        let value = self.insert_node(
                            Expression::Identifier { name },
                            self.get_span_from(&start),
                        );
                        Some(value)
                    }
                    _ => value,
                }
            } else {
                value
            };

            // unkeyed empty heads are not properties
            if modifiers.is_none() && key.is_none() && value.is_none() && default.is_none() {
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::Identifier,
                ));
            }

            // keyed fields without `:` or `=` are invalid unless they were shorthand
            if value.is_none() {
                return Err(ParseError::expected(self.peek()?.span, TokenType::Colon));
            }

            let property = Property::Field {
                key: key.expect("field property requires key"),
                value: value.expect("field property requires value"),
                is_shorthand,
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

    /// Try to eat a type member and recover into one error slot when possible.
    pub fn try_eat_type_member(
        &mut self,
        _recover: TokenType,
    ) -> ParseResult<LocalNodeId<TypeMember>> {
        match self.eat_type_member() {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::TypeMember);
                let span = err.leaf_span();
                let start = ParserMark::from_span(span);
                self.try_recover_in_body(&start, Some(err.clone()))?;

                Ok(self.insert_node(TypeMember::Error, self.get_span_from(&start)))
            }
        }
    }

    /// Eat a type member.
    pub fn eat_type_member(&mut self) -> ParseResult<LocalNodeId<TypeMember>> {
        let start = self.mark_span();

        // readonly
        let is_readonly = if self.type_member_readonly_modifier_maybe() {
            self.bump(); // eat readonly
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };

        // abstract
        let is_abstract = if self.type_member_abstract_modifier_maybe() {
            self.bump(); // eat abstract
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };

        // mode
        let mode = self.eat_method_mode_maybe();

        // allow newlines after get/set
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) {
            self.eat_newlines_maybe()?;
        }

        // index signature
        if self.type_member_starts_index_signature() {
            if is_abstract {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let key_start = self.mark_span();
            self.eat_token(TokenType::OpenBracket)?;
            self.eat_newlines_maybe()?;
            let (name, name_span) = self.eat_binding_identifier_with_span()?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Colon)?;
            self.eat_newlines_maybe()?;

            let key_type = self.eat_type_expression_node_or_recover_missing(
                self.options
                    .not_in_position()
                    .not_in_left_precedence()
                    .not_in_sequence_expression()
                    .in_type(),
                NodeType::TypeMember,
            )?;

            self.eat_newlines_maybe()?;
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseBracket,
                NodeType::TypeMember,
                |_, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type)
                        || matches!(token_type, TokenType::Colon | TokenType::Maybe)
                },
            )?;

            // optional index signatures
            let optional_start = self.mark_span();
            let is_optional = self.eat_token_maybe(TokenType::Maybe)?;
            let optional_span = is_optional.then(|| self.get_span_from(&optional_start));

            self.eat_newlines_maybe()?;
            let type_start = self.mark_span();
            self.eat_token(TokenType::Colon)?;
            self.eat_newlines_maybe()?;

            let value_type = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.eat_method_return_type(NodeType::TypeMember)?
            };

            let member = TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

            self.tree.set_main_span(member_id, name_span);
            self.tree
                .set_head_span(member_id, self.get_span_from(&key_start));
            self.tree.set_side_span(
                member_id,
                NodeSpanType::Type,
                self.get_span_from(&type_start),
            );

            if let Some(span) = optional_span {
                self.tree
                    .set_side_span(member_id, NodeSpanType::Trailing, span);
            }

            return Ok(member_id);
        }

        // key
        let (key, key_span) = if matches!(mode, Some(FunctionMode::Constructor | FunctionMode::New))
        {
            (None, None)
        } else if let Some((key, span)) = self.eat_property_key_with_span()? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

        // optional
        let optional_start = self.mark_span();
        let is_optional = self.eat_token_maybe(TokenType::Maybe)?;
        let optional_span = is_optional.then(|| self.get_span_from(&optional_start));

        // method
        let is_method = self.type_member_head_starts_method(mode);
        if is_method {
            if is_readonly {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) && key.is_none() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            let ParsedMethodTail {
                signature,
                generic_parameter_span,
                parameter_span,
                body: _,
                return_type_span,
            } = self.eat_method_tail(
                NodeType::TypeMember,
                mode,
                Asynchrony::Sync,
                FunctionCardinality::Scalar,
                is_abstract,
                false,
                false,
            )?;

            let member = match (key, mode) {
                (Some(key), _) => TypeMember::Method {
                    is_optional,
                    key,
                    signature,
                    body: None,
                },
                (None, Some(FunctionMode::New | FunctionMode::Constructor)) => {
                    TypeMember::ConstructSignature {
                        signature: constructor_type_declaration_from_signature(signature),
                    }
                }
                (None, None | Some(FunctionMode::Call)) => TypeMember::CallSignature {
                    signature: function_type_declaration_from_signature(signature),
                },
                (None, Some(FunctionMode::Getter | FunctionMode::Setter)) => {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            if let Some(span) = return_type_span {
                self.tree.set_side_span(member_id, NodeSpanType::Type, span);
            }

            if let Some(span) = generic_parameter_span {
                self.tree
                    .set_side_span(member_id, NodeSpanType::GenericParameters, span);
            }

            self.tree
                .set_side_span(member_id, NodeSpanType::Parameters, parameter_span);

            if let Some(span) = optional_span {
                self.tree
                    .set_side_span(member_id, NodeSpanType::Trailing, span);
            }

            return Ok(member_id);
        }

        // field
        let Some(key) = key else {
            return Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ));
        };

        if is_abstract {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        let type_start = self.mark_span();
        let declared_type = if self.eat_token_maybe(TokenType::Colon)? {
            self.eat_newlines_maybe()?;

            Some(
                if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                    self.recover_missing_type_expression_here(NodeType::TypeMember)
                } else {
                    self.eat_method_return_type(NodeType::TypeMember)?
                },
            )
        } else {
            None
        };
        let member = TypeMember::Field {
            is_optional,
            is_readonly,
            key,
            declared_type,
        };
        let member_id = self.insert_node(member, self.get_span_from(&start));

        if let Some(span) = key_span {
            self.tree.set_main_span(member_id, span);
        }

        if declared_type.is_some() {
            self.tree.set_side_span(
                member_id,
                NodeSpanType::Type,
                self.get_span_from(&type_start),
            );
        }

        if let Some(span) = optional_span {
            self.tree
                .set_side_span(member_id, NodeSpanType::Trailing, span);
        }

        Ok(member_id)
    }

    /// Eat type members inside one object type body.
    pub fn eat_type_members(&mut self) -> ParseResult<Vec<LocalNodeId<TypeMember>>> {
        let mut members: Vec<LocalNodeId<TypeMember>> = Vec::new();
        let mut pending_member_decorators = PendingDecorators::new();
        while self.has_more_tokens() {
            self.eat_newlines_maybe()?;
            let token_type = self.peek_token_type();

            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                if !pending_member_decorators.is_empty() {
                    let error = ParseError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_member_decorators.clear();
                }
                break;
            } else if token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_member_decorators.extend(decorators);
                continue;
            } else if token_type == TokenType::Comma {
                self.eat_item_stop_with_newlines()?;
                continue;
            } else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop_with_newlines()?;
                continue;
            } else {
                match self.try_eat_type_member(TokenType::Newline) {
                    Ok(member_id) => {
                        if !pending_member_decorators.is_empty() {
                            self.attach_decorators(
                                member_id.id,
                                std::mem::take(&mut pending_member_decorators),
                            );
                        }
                        members.push(member_id);
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(members)
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
            let embed_start = self.mark_span();
            self.bump(); // eat spread
            let value = self.eat_member_type_expression()?;
            let member = Member::Embed {
                value,
                visibility: None,
                ambient: Ambientness::Concrete,
                is_static: false,
            };

            return Ok(self.insert_node(member, self.get_span_from(&embed_start)));
        }

        // modifiers prefix
        let modifiers = self.eat_binding_modifiers_prefix_maybe(true, true, true, true, true)?;

        // duplicate static modifier across newlines
        if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.is_static)
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
        if modifiers.is_some_and(|modifiers| modifiers.is_static)
            && self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
        {
            self.eat_newlines_maybe()?;
            let body_start = self.mark_span();
            let body_block = self.eat_block(BlockContext::Statement)?;
            let body = self.insert_node(
                Expression::Block(body_block),
                self.get_span_from(&body_start),
            );

            return Ok(self.insert_node(Member::StaticBlock { body }, self.get_span_from(&start)));
        }

        // type member: `type Name<U> = ...` or `type Name: Bound`
        if let Some(member_id) = self.try_eat_associated_type_member(&start, modifiers)? {
            return Ok(member_id);
        }

        // comptime block: `comptime { ... }` (timing modifier already consumed)
        if modifiers.is_some_and(|modifiers| modifiers.is_comptime)
            && self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
        {
            self.eat_newlines_maybe()?;
            let body_start = self.mark_span();
            let body_block = self.eat_block(BlockContext::Statement)?;
            let body = self.insert_node(
                Expression::Block(body_block),
                self.get_span_from(&body_start),
            );

            return Ok(self.insert_node(Member::ComptimeBlock { body }, self.get_span_from(&start)));
        }

        // head
        let ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            mode,
            is_async,
            is_generator,
            is_method,
            associated_comptime_name,
        } = self.eat_property_member_head(modifiers, true)?;

        // getters and setters require method form
        if matches!(mode, Some(FunctionMode::Getter | FunctionMode::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method form
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // abstraction
            // methods without key or mode are implicit calls
            let mode = if key.is_none() && mode.is_none() {
                Some(FunctionMode::Call)
            } else {
                mode
            };

            // modifiers postfix (again after parameters)
            let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;
            let ParsedMethodTail {
                signature,
                generic_parameter_span,
                parameter_span,
                body,
                return_type_span,
            } = self.eat_method_tail(
                NodeType::Member,
                mode,
                if is_async {
                    Asynchrony::Async
                } else {
                    Asynchrony::Sync
                },
                if is_generator {
                    FunctionCardinality::Generator
                } else {
                    FunctionCardinality::Scalar
                },
                modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                modifiers.is_some_and(|modifiers| modifiers.is_override),
                true,
            )?;

            // method member
            let member = Member::Method {
                key,
                signature,
                body,
                visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                ambient: self.ambientness_for_modifiers(modifiers.as_ref()),
                is_abstract: modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
                is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                is_accessor: modifiers.is_some_and(|modifiers| modifiers.is_accessor),
                is_comptime: modifiers.is_some_and(|modifiers| modifiers.is_comptime),
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

            if let Some(span) = generic_parameter_span {
                self.tree
                    .set_side_span(member_id, NodeSpanType::GenericParameters, span);
            }

            self.tree
                .set_side_span(member_id, NodeSpanType::Parameters, parameter_span);

            Ok(member_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, comptime_type, type_span) = if self.peek_colon_is() {
                let type_start = self.mark_span();
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;

                // member field annotations are always type positions
                let is_missing_type = self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop();

                let value = if associated_comptime_name.is_some() {
                    None
                } else if is_missing_type {
                    Some(self.recover_missing_type_expression_here(NodeType::Member))
                } else {
                    Some(self.eat_member_type_expression()?)
                };

                let comptime_type = if associated_comptime_name.is_none() {
                    None
                } else if is_missing_type {
                    Some(self.recover_missing_type_expression_here(NodeType::Member))
                } else {
                    Some(self.eat_member_type_expression()?)
                };
                let type_span = self.get_span_from(&type_start);
                (value, comptime_type, Some(type_span))
            } else {
                (None, None, None)
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
                Member::AssociatedConst {
                    name,
                    declared_type: comptime_type,
                    value: default,
                    visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                    ambient: self.ambientness_for_modifiers(modifiers.as_ref()),
                    is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                }
            } else {
                Member::Field {
                    key: key.expect("field member requires key"),
                    declared_type: value,
                    default,
                    is_optional: modifiers.is_some_and(|modifiers| modifiers.is_optional),
                    is_readonly: modifiers.is_some_and(|modifiers| modifiers.is_readonly),
                    mutability: None,
                    visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                    ambient: self.ambientness_for_modifiers(modifiers.as_ref()),
                    is_abstract: modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                    is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
                    is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                    is_definite: modifiers.is_some_and(|modifiers| modifiers.is_definite),
                    is_accessor: modifiers.is_some_and(|modifiers| modifiers.is_accessor),
                    is_comptime: modifiers.is_some_and(|modifiers| modifiers.is_comptime),
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
        Ambientness, Argument, Asynchrony, BinaryOperator, Block, ClassDeclaration, CommentKind,
        Declaration, Expression, FunctionDeclaration, FunctionKind, FunctionMode, GenericArgument,
        GenericParameter, IntType, InterfaceDeclaration, Key, Member, Name, Parameter, Property,
        ScalarLiteral, TypeExpression, TypeLiteral, TypeMember, TypePredicateSubject, Visibility,
    };
    use destack_source::LanguageType;

    use crate::tests::TestParser;
    use crate::{
        assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
        block_expression_ids,
    };

    #[test]
    fn test_parse_member_with_private_hash_name() {
        let mut test = TestParser::new_with_options(r#"#name: string"#, LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { key: Key::Private(name), declared_type: Some(ty), default: None, .. } => {
            assert_string!(parser, *name, "name");
            assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    }

    #[test]
    fn test_parse_member_definite_assignment() {
        let mut test = TestParser::new_with_options("prop!: Foo", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "prop");
            assert_expression_path!(parser, parser.tree.get(*value), "Foo");
        });
    }

    #[test]
    fn test_parse_member_accessor_definite_assignment() {
        let mut test = TestParser::new_with_options("accessor a!: any", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(value), is_accessor, .. } => {
            assert!(*is_accessor);
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Any);
            });
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
        assert_node!(parser.tree, member, Member::Field { key: Key::Private(name), declared_type: Some(value), visibility, ambient, is_accessor, .. } => {
            assert_string!(parser, *name, "value");
            assert_eq!(*visibility, Some(Visibility::Private));
            assert_eq!(*ambient, Ambientness::Ambient);
            assert!(*is_accessor);
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    }

    #[test]
    fn test_parse_member_rejects_optional_definite_assignment_combo() {
        let mut test = TestParser::new_with_options("prop!?: Foo", LanguageType::TypeScript);
        let mut parser = test.prepare();

        assert!(parser.eat_member().is_err());
    }

    #[test]
    fn test_parse_member_override_field() {
        let mut test =
            TestParser::new_with_options("override foo: int32", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(value), is_override, .. } => {
            assert!(*is_override);
            assert_string!(parser, *name, "foo");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(
                    *value,
                    TypeLiteral::Int(IntType::Arbitrary {
                        is_signed: true,
                        width: Some(32),
                    })
                );
            });
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
        assert_node!(parser.tree, member_id, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: None, default: Some(default), .. } => {
            assert_string!(parser, *name, "port2");
            assert_node!(parser.tree, *default, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                    assert_string!(parser, *name, "postMessage");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { .. } => {
                                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::Call { arguments, .. } => {
                                            assert_eq!(arguments.len(), 2);
                                            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
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
    }

    #[test]
    fn test_parse_member_abstract_override_method() {
        let mut test =
            TestParser::new_with_options("abstract override foo(): void", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, is_abstract, is_override, .. } => {
            assert_string!(parser, *name, "foo");
            assert!(signature.is_abstract);
            assert!(*is_abstract);
            assert!(*is_override);
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
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, visibility, is_override, .. } => {
            assert_eq!(*visibility, Some(Visibility::Public));
            assert_string!(parser, *name, "foo");
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert!(*is_override);
        });
    }

    #[test]
    fn test_parse_member_method_parameter_type_then_default_value() {
        let mut test = TestParser::new_with_options(
            "usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        // parse one method where a typed parameter is followed by a defaulted parameter
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(_), .. } => {
            assert_string!(parser, *name, "usersLimitReached");
            assert_eq!(signature.parameters.len(), 2);

            // userCount: number
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), default, .. } => {
                assert_string!(parser, *name, "userCount");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });

            // userLimit = get(this.store).userLimit
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default: Some(default), .. } => {
                assert_string!(parser, *name, "userLimit");
                assert!(declared_type.is_none());
                assert_node!(parser.tree, *default, Expression::Member { name, .. } => {
                    assert_string!(parser, *name, "userLimit");
                });
            });
        });

        // this signature parses without recovery diagnostics
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_member_method_generic_with_newline_before_parameters() {
        let mut test = TestParser::new_with_options(
            "private method<T>\n(value: T): T { return value }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        // parse one method with a generic parameter and a newline before dynamic parameters
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(body), visibility, .. } => {
            assert_eq!(*visibility, Some(Visibility::Private));
            assert_string!(parser, *name, "method");

            // parse the generic, dynamic parameter, and return type as one coherent signature
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "T");
            });
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                assert_string!(parser, *name, "value");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "T");

            // keep a method body attached after the multiline signature
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_method_with_newline_before_return_type() {
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
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });

            // keep a method body attached after the multiline return type annotation
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_parse_member_method_object_union_return_type() {
        let mut test = TestParser::new_with_options(
            "overlaps(): { overlaps: false } | { overlaps: true; reason: string }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();

        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: None, .. } => {
            assert_string!(parser, *name, "overlaps");

            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 2);
                });
            });
        });
    }

    #[test]
    fn test_parse_member_method_body_boundary_comment_on_return_type() {
        let mut test = TestParser::new_with_options(
            "method(): number // method-body\n{ return 1 }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        parser.attach_comments();
        assert_node!(parser.tree, member, Member::Method { signature, body: Some(body), .. } => {
            let return_type = signature.return_type.expect("expected return type");
            let return_type_annotations = parser.tree.get_decorators(return_type.id);
            assert!(return_type_annotations.is_empty());

            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let body_statement_annotations = parser.tree.get_decorators(expressions[0].id);
                    assert!(body_statement_annotations.is_empty());
                });
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "method-body");
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
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                assert_string!(parser, *name, "name");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Promise");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Boolean);
                        });
                });
            });
            assert_node!(parser.tree, body.expect("expected method body"), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_member_method_named_public() {
        let mut test = TestParser::new_with_options("public() {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), visibility, .. } => {
            assert!(visibility.is_none());
            assert_string!(parser, *name, "public");
        });
    }

    #[test]
    fn test_parse_member_static_method_named_protected() {
        let mut test =
            TestParser::new_with_options("static protected() {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Method { key: Some(Key::Name(Name::Identifier(name))), is_static, .. } => {
            assert!(*is_static);
            assert_string!(parser, *name, "protected");
        });
    }

    #[test]
    fn test_parse_member_field_named_static() {
        let mut test = TestParser::new_with_options("static", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let member = parser.eat_member().unwrap();
        assert_node!(parser.tree, member, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: None, default: None, .. } => {
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
        assert_node!(parser.tree, member, Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: None, default: Some(default), .. } => {
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
        assert_node!(parser.tree, members[1], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "y");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(
                    *value,
                    TypeLiteral::Int(IntType::Arbitrary {
                        width: Some(32),
                        is_signed: true,
                    })
                );
            });
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
            assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
                let mut getter: Option<Key> = None;
                let mut setter: Option<Key> = None;
                for member_id in members {
                    if let TypeMember::Method { signature, key, .. } = parser.tree.get(*member_id) {
                        match signature.mode {
                            Some(FunctionMode::Getter) => getter = Some(*key),
                            Some(FunctionMode::Setter) => setter = Some(*key),
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
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
            assert_string!(parser, *name, "x");
            assert!(!*is_shorthand);
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(
                        *value,
                        TypeLiteral::Int(IntType::Arbitrary {
                            width: Some(32),
                            is_signed: true,
                        })
                    );
                });
            });
        });
    }

    #[test]
    fn test_parse_property_with_default_value() {
        let mut test = TestParser::new("x = 42");
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value: default, is_shorthand } => {
            assert_string!(parser, *name, "x");
            assert!(!*is_shorthand);
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
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
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
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "reproFunc");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(_), .. }) => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.parameters.len(), 1);
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
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::Assign { .. });
        });
    }

    #[test]
    fn test_parse_property_definite_assignment() {
        let mut test = TestParser::new_with_options("prop!: LongType[]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
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
        assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "y");
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(
                        *value,
                        TypeLiteral::Int(IntType::Arbitrary {
                            width: Some(32),
                            is_signed: true,
                        })
                    );
                });
            });
        });
    }

    #[test]
    fn test_parse_property_rejects_optional_definite_assignment_combo() {
        let mut test = TestParser::new_with_options("prop!?: LongType[]", LanguageType::TypeScript);
        let mut parser = test.prepare();

        assert!(parser.eat_property().is_err());
    }

    #[test]
    fn test_parse_property_method_call() {
        let mut test = TestParser::new("<T = any>(x: T): T");
        let mut parser = test.prepare();
        let property_id = parser.eat_property().unwrap();
        // <T = any>(x: T): T
        assert_node!(parser.tree, property_id, Property::Method { signature, .. } => {
            assert_eq!(signature.mode, Some(FunctionMode::Call));
            let generic_parameters = &signature.generic_parameters;
            // <T = any>
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                assert_string!(parser, *name, "T");
                assert!(constraint.is_none());
                assert_node!(parser.tree, default.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Any);
                });
            });
            // x: T
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "T");
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
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Object { members: properties } => {
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
            assert!(signature.generic_parameters.is_empty());
            // x: int32
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true }));
                });
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
        assert_node!(parser.tree, member_id, Member::Method { key: Some(Key::Expression(key)), signature, .. } => {
            assert_expression_path!(parser, parser.tree.get(*key), "EventEmitter.captureRejectionSymbol");
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_eq!(signature.parameters.len(), 1);
            assert!(signature.return_type.is_some());
        });
    }

    #[test]
    fn test_parse_member_type_with_value() {
        let mut test = TestParser::new("type Item = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: None, value: Some(value), visibility, ambient, .. } => {
            assert_string!(parser, *name, "Item");
            assert!(generic_parameters.is_empty());
            assert!(where_clauses.is_empty());
            assert!(visibility.is_none());
            assert_eq!(*ambient, Ambientness::Concrete);
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    }

    #[test]
    fn test_parse_member_type_with_bound() {
        let mut test = TestParser::new("type Item: Hashable");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: Some(ty), value: None, .. } => {
            assert_string!(parser, *name, "Item");
            assert!(generic_parameters.is_empty());
            assert!(where_clauses.is_empty());
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
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, constraint: Some(ty), value: None, .. } => {
            assert_string!(parser, *name, "Item");
            assert_node!(parser.tree, *ty, TypeExpression::Union { .. });
        });
    }

    #[test]
    fn test_parse_member_type_with_bound_and_value() {
        let mut test = TestParser::new("type Item: Hashable = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, constraint: Some(ty), value: Some(value), .. } => {
            assert_string!(parser, *name, "Item");
            assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    }

    #[test]
    fn test_parse_member_type_with_visibility() {
        let mut test = TestParser::new("public type Item = string");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, value: Some(_), visibility, .. } => {
            assert_eq!(*visibility, Some(Visibility::Public));
            assert_string!(parser, *name, "Item");
        });
    }

    #[test]
    fn test_parse_member_type_with_generic_parameters() {
        let mut test = TestParser::new("type View<U> = [Item, U]");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: None, value: Some(_), .. } => {
            assert_string!(parser, *name, "View");
            assert!(where_clauses.is_empty());
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "U");
            });
        });
    }

    #[test]
    fn test_parse_member_associated_comptime_const() {
        let mut test = TestParser::new("comptime const Rows: number = 128");
        let mut parser = test.prepare();
        let member_id = parser.eat_member().unwrap();
        assert_node!(parser.tree, member_id, Member::AssociatedConst { name, declared_type: Some(ty), value: Some(value), is_static, .. } => {
            assert_string!(parser, *name, "Rows");
            assert!(!*is_static);
            assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
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
        assert_node!(parser.tree, member_id, Member::AssociatedConst { value: Some(value), .. } => {
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
        assert_node!(parser.tree, member_id, Member::ComptimeBlock { body } => {
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
        assert_node!(parser.tree, member_id, Member::ComptimeBlock { body } => {
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
            assert_eq!(signature.parameters.len(), 1);
            assert!(signature.return_type.is_some());
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Union { .. });
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
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Predicate { asserts, subject, target } => {
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
                assert_eq!(members.len(), 2);

                let first_annotations = parser.tree.get_decorators(members[0].id);
                assert!(first_annotations.is_empty());

                let second_annotations = parser.tree.get_decorators(members[1].id);
                assert!(second_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 2);
        assert_comment!(parser, 0, CommentKind::Line, "first-tail");
        assert_comment!(parser, 1, CommentKind::Line, "second-tail");
    }
}
