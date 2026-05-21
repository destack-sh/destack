#![allow(clippy::type_complexity)]

use destack_dir::{
    AssignOperator, AssignPattern, Asynchrony, BlockContext, ConstructorTypeDeclaration,
    Expression, FunctionForm, FunctionPhase, FunctionRole, FunctionSignature,
    FunctionTypeDeclaration, Key, Keyword, LocalNodeId, Member, MethodAbstraction, Name, NodeType,
    Parameter, Property, StringId, TokenLiteral, TokenType, TypeExpression, TypeKind, TypeMember,
    Visibility,
};
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

use super::PendingDecorators;
use crate::parse::argument::BindingModifiers;
use crate::parse::flags::ParserFlags;
use crate::parse::scope::ExpressionScope;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 9] = [
    Keyword::Static,
    Keyword::Abstract,
    Keyword::Virtual,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
    Keyword::Comptime,
];

/// Whether a type member list accepts method bodies.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeMemberBodyMode {
    /// Accept signatures only.
    SignatureOnly,
    /// Accept default method bodies.
    DefaultBodies,
}

impl TypeMemberBodyMode {
    /// Return the member body mode for one interface kind.
    #[inline]
    pub(crate) const fn for_interface(kind: TypeKind) -> Self {
        match kind {
            TypeKind::Nominal => Self::DefaultBodies,
            TypeKind::Structural => Self::SignatureOnly,
        }
    }

    /// Return whether method bodies are accepted.
    #[inline]
    const fn allows_body(self) -> bool {
        matches!(self, Self::DefaultBodies)
    }
}

/// Parsed head for one property or member.
#[derive(Debug)]
struct ParsedPropertyMemberHead {
    /// The parsed modifiers.
    modifiers: Option<BindingModifiers>,
    /// The parsed key.
    key: Option<Key>,
    /// The key span.
    key_span: Option<Span>,
    /// The parsed method role.
    role: Option<FunctionRole>,
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
    debug_assert!(signature.role.is_none() || signature.role == Some(FunctionRole::Call));

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
        signature.role,
        Some(FunctionRole::Constructor | FunctionRole::New)
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
        if !self.is_keyword(Keyword::Async) {
            return false;
        }

        let next_token = self.next_token();
        let can_start_async_method = !next_token.token.is_on_new_line
            && matches!(
                next_token.token.ty,
                TokenType::Identifier
                    | TokenType::Literal
                    | TokenType::Hash
                    | TokenType::OpenBracket
                    | TokenType::Multiply
                    | TokenType::OpenParenthesis
                    | TokenType::LessThan
            );
        if can_start_async_method {
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

    /// Eat one method role.
    #[inline]
    fn eat_method_role_maybe(
        &mut self,
        allow_constructor_role: bool,
        allow_new_role: bool,
    ) -> Option<FunctionRole> {
        if self.is_keyword(Keyword::Get)
            && self.next_token_starts_member_name()
            && self.next_token_type() != TokenType::OpenParenthesis
        {
            self.bump(); // eat get keyword
            Some(FunctionRole::Getter)
        } else if self.is_keyword(Keyword::Set)
            && self.next_token_starts_member_name()
            && self.next_token_type() != TokenType::OpenParenthesis
        {
            self.bump(); // eat set keyword
            Some(FunctionRole::Setter)
        } else if allow_constructor_role
            && self.is_keyword(Keyword::Constructor)
            && matches!(
                self.next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            )
        {
            self.bump(); // eat constructor keyword
            Some(FunctionRole::Constructor)
        } else if allow_new_role
            && self.is_keyword(Keyword::New)
            && matches!(
                self.next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            )
        {
            self.bump(); // eat new keyword
            Some(FunctionRole::New)
        } else {
            None
        }
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

        // reject impossible optional and definite fields
        if modifiers.is_some_and(|modifiers| modifiers.is_optional && modifiers.is_definite) {
            let error_span = self
                .prev()
                .map(|token| token.span)
                .unwrap_or(self.peek()?.span);
            return Err(ParseError::unexpected(error_span));
        }

        // reject impossible abstraction combinations
        if modifiers.is_some_and(|modifiers| modifiers.is_abstract && modifiers.is_virtual) {
            let error_span = self
                .prev()
                .map(|token| token.span)
                .unwrap_or(self.peek()?.span);
            return Err(ParseError::unexpected(error_span));
        }

        if modifiers.is_some_and(|modifiers| modifiers.is_static && modifiers.is_virtual) {
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
            && !self.current_token_is_on_new_line()
            && self.peek_is(TokenType::Identifier)
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        Ok(())
    }

    /// Return true when the current `[` starts an index signature.
    #[inline]
    fn type_member_starts_index_signature(&mut self) -> bool {
        if !self.peek_is(TokenType::OpenBracket) {
            return false;
        }

        self.token_type_at_offset(1) == TokenType::Identifier
            && self.token_type_at_offset(2) == TokenType::Colon
    }

    /// Eat one spread or embed value expression.
    #[inline]
    fn eat_property_value_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let ambient_context = self.flags;
        let expression_context = self.flags.not_in_position().not_in_sequence_expression();
        self.eat_expression(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Insert the assignment expression used to preserve one property value with a default.
    fn insert_property_default_expression(
        &mut self,
        value: LocalNodeId<Expression>,
        default: LocalNodeId<Expression>,
        assign_operator_span: Option<Span>,
    ) -> LocalNodeId<Expression> {
        let value_span = self.tree.get_span(value);
        let default_span = self.tree.get_span(default);
        let assign_span = Span::new(value_span.file, value_span.start, default_span.end);
        let left = self.insert_node(AssignPattern::Expression { value }, value_span);
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

        assign_id
    }

    /// Return the ambientness implied by one modifier set.
    #[inline]
    fn is_ambient_for_modifiers(&self, modifiers: Option<&BindingModifiers>) -> bool {
        modifiers.is_some_and(|modifiers| modifiers.is_ambient)
    }

    /// Eat method parameters in property or member contexts.
    #[inline]
    fn eat_method_parameters(
        &mut self,
        is_generator: bool,
    ) -> ParseResult<Vec<LocalNodeId<Parameter>>> {
        let ambient_context = self
            .flags
            .with_generator(is_generator)
            .with_forbid_yield(is_generator);
        self.with_flags(self.flags.with_ambient_context(ambient_context), |parser| {
            parser.eat_dynamic_parameters()
        })
    }

    /// Eat one method or field type expression.
    #[inline]
    fn eat_method_return_type(
        &mut self,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let mut ambient_context = self.flags.nested().with_type(true);
        if !self.peek_is(TokenType::OpenBrace) {
            ambient_context = ambient_context.with_before_block(true);
        }
        let expression_context = self.flags.nested().not_in_sequence_expression();
        self.eat_type_expression_or_recover_missing(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context.allow_type_predicate()),
            owner,
        )
    }

    /// Eat one method body expression.
    #[inline]
    fn eat_method_body_expression(
        &mut self,
        is_generator: bool,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.peek_is(TokenType::OpenBrace) {
            let ambient_context = self
                .flags
                .with_generator(is_generator)
                .with_decorator(false);
            let mut flags = self
                .flags
                .with_ambient_context(ambient_context)
                .in_before_block()
                .in_statement_position();
            flags.set_allow_sequence_expression(true);
            let block =
                self.with_flags(flags, |parser| parser.eat_block(BlockContext::Expression))?;

            return Ok(self.insert_node(Expression::Block(block), self.tree.get_span(block)));
        }

        let ambient_context = self
            .flags
            .with_generator(is_generator)
            .with_decorator(false);
        let expression_context = self
            .flags
            .not_in_position()
            .with_statement_position(true)
            .with_sequence_expression(true);
        self.eat_expression(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
        )
    }

    /// Eat one shared property or member head.
    fn eat_property_member_head(
        &mut self,
        mut modifiers: Option<BindingModifiers>,
        allow_constructor_mode: bool,
        allow_new_mode: bool,
    ) -> ParseResult<ParsedPropertyMemberHead> {
        // async and late abstraction modifiers
        let is_async = self.eat_method_async_maybe();
        modifiers = self.eat_method_late_modifiers_maybe(modifiers, is_async);

        // role and accessor marker
        let role = self.eat_method_role_maybe(allow_constructor_mode, allow_new_mode);

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
        let associated_comptime_name = if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.is_comptime && modifiers.is_const_asserted)
        {
            match key.as_ref() {
                Some(Key::Name(Name::Identifier(name))) => Some(*name),
                _ => return Err(ParseError::unexpected(self.peek()?.span)),
            }
        } else {
            None
        };
        let is_method = is_async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter));

        Ok(ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            role,
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
        role: Option<FunctionRole>,
        asynchrony: Asynchrony,
        is_generator: bool,
        is_abstract: bool,
        is_override: bool,
        allows_body: bool,
    ) -> ParseResult<ParsedMethodTail> {
        // generic parameters and parameters
        let generic_parameter_start = self.span_start();
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)?
            .unwrap_or_default();
        let generic_parameter_span =
            (!generic_parameters.is_empty()).then(|| self.get_span_from(&generic_parameter_start));

        let parameter_start = self.span_start();
        let parameters = self.eat_method_parameters(is_generator)?;
        let parameter_span = self.get_span_from(&parameter_start);

        // return type
        let has_return_type_marker = self.peek_colon_is()
            || self.current_token_is_on_new_line() && self.peek_is(TokenType::Colon);
        let (return_type, return_type_span) = if has_return_type_marker {
            let type_start = self.span_start();
            self.eat_token(TokenType::Colon)?;

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
        let body = if allows_body && self.peek_is(TokenType::OpenBrace) {
            Some(self.eat_method_body_expression(is_generator)?)
        } else {
            None
        };

        // reject invalid trailing tokens
        if allows_body {
            if body.is_none()
                && !self.current_token_is_on_new_line()
                && !self.is_any_stop()
                && !self.peek_is(TokenType::CloseBrace)
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        } else {
            if self.peek_is(TokenType::OpenBrace) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if !self.current_token_is_on_new_line()
                && !self.is_any_stop()
                && !self.peek_is(TokenType::CloseBrace)
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        }

        // split out the explicit this parameter
        let (this_parameter, parameters) = self.split_this_parameter_maybe(parameters);

        // build the signature
        let signature = FunctionSignature {
            asynchrony,
            role,
            form: FunctionForm::Function,
            phase: FunctionPhase::Normal,
            generic_parameters,
            where_clauses,
            this_parameter,
            parameters,
            return_type,
            is_abstract,
            is_override,
            is_generator,
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
        let ambient_context = self.flags.with_type(true);
        self.eat_type_expression_or_recover_missing(
            self.flags.with_ambient_context(ambient_context),
            NodeType::Member,
        )
    }

    /// Eat a key with private hash parsing enabled.
    #[inline]
    fn eat_property_key_with_span(&mut self) -> ParseResult<Option<(Key, Span)>> {
        let ambient_context = self.flags.with_allow_private_hash_key(true);
        self.with_flags(self.flags.with_ambient_context(ambient_context), |parser| {
            parser.eat_key_maybe_with_span()
        })
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
        start: &ParserSpanStart,
        modifiers: Option<BindingModifiers>,
    ) -> ParseResult<Option<LocalNodeId<Member>>> {
        if !self.language.is_destack()
            || !self.is_keyword(Keyword::Type)
            || self.next_token_type() != TokenType::Identifier
        {
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
            is_ambient: self.is_ambient_for_modifiers(modifiers.as_ref()),
            is_abstract: modifiers.is_some_and(|modifiers| modifiers.is_abstract),
            is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
            is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
        };
        let member_id = self.insert_node(member, self.get_span_from(start));
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(member_id))
    }

    /// Try to eat one associated type member.
    fn try_eat_type_member_associated_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<TypeMember>>> {
        if !self.language.is_destack()
            || !self.is_keyword(Keyword::Type)
            || self.next_token_type() != TokenType::Identifier
        {
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

            let constraint = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.is_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
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

            let value = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.eat_member_type_expression()?
            };

            Some(value)
        } else {
            None
        };

        // member
        let member = TypeMember::AssociatedType {
            name,
            generic_parameters,
            where_clauses,
            constraint,
            value,
        };
        let member_id = self.insert_node(member, self.get_span_from(start));
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(member_id))
    }

    /// Try to eat one associated constant member.
    fn try_eat_type_member_associated_const(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<TypeMember>>> {
        if !self.language.is_destack()
            || !self.is_keyword(Keyword::Comptime)
            || self.next_keyword() != Some(Keyword::Const)
        {
            return Ok(None);
        }

        // keyword and name
        self.bump(); // eat comptime keyword
        self.bump(); // eat const keyword
        let (name, name_span) = self.eat_identifier_with_span()?;

        // declared type
        let declared_type = if self.peek_colon_is() {
            self.bump(); // eat colon

            let declared_type = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.is_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.eat_member_type_expression()?
            };

            Some(declared_type)
        } else {
            None
        };

        // value
        let value = if self.peek_is(TokenType::Assign) {
            self.bump(); // eat assign

            let value = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                self.recover_missing_expression_here(NodeType::TypeMember)
            } else {
                let expression_flags = self
                    .flags
                    .not_in_type()
                    .not_in_position()
                    .not_in_sequence_expression();
                self.eat_expression(expression_flags)?
            };

            Some(value)
        } else {
            None
        };

        // member
        let member = TypeMember::AssociatedConst {
            name,
            declared_type,
            value,
        };
        let member_id = self.insert_node(member, self.get_span_from(start));
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(member_id))
    }

    /// Try to eat a property and recover one malformed member when possible.
    pub fn try_eat_property(&mut self) -> ParseResult<LocalNodeId<Property>> {
        match self.eat_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Property);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_body_from_span(span, Some(err))?;

                Ok(self.insert_node(Property::Error, recovered_span))
            }
        }
    }

    /// Eat an object literal body.
    ///
    /// Examples:
    /// ```ds
    /// key: value
    /// key
    /// ...other
    /// ```
    pub fn eat_object_properties(&mut self) -> ParseResult<Vec<LocalNodeId<Property>>> {
        let mut properties = Vec::new();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on object close
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                break;
            }

            // stop at declaration recovery boundaries
            if token_type == TokenType::Semicolon {
                if self.current_token_is_declaration_recovery_boundary(token_type) {
                    break;
                }

                self.eat_any_stop()?;
                continue;
            }

            // skip separators
            if token_type == TokenType::Comma {
                self.eat_item_stop()?;
                continue;
            }

            let property = self.try_eat_object_property()?;
            properties.push(property);
        }

        Ok(properties)
    }

    /// Try to eat one object literal property with recovery.
    fn try_eat_object_property(&mut self) -> ParseResult<LocalNodeId<Property>> {
        match self.eat_object_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Property);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_body_from_span(span, Some(err))?;

                Ok(self.insert_node(Property::Error, recovered_span))
            }
        }
    }

    /// Eat one object literal property.
    ///
    /// Examples:
    /// ```ds
    /// key: value
    /// key = fallback
    /// method() {}
    /// ```
    fn eat_object_property(&mut self) -> ParseResult<LocalNodeId<Property>> {
        let start = self.span_start();

        if self.simple_object_property_starts() {
            let property_id = self.eat_simple_object_property(&start)?;

            return Ok(property_id);
        }

        self.eat_property()
    }

    /// Return whether the current object property can use the simple field parser.
    fn simple_object_property_starts(&mut self) -> bool {
        let token_type = self.peek_token_type();
        if token_type == TokenType::Spread {
            return true;
        }

        if !self.token_starts_simple_object_key(token_type) {
            return false;
        }

        let next_token_type = self.next_token_type();
        matches!(
            next_token_type,
            TokenType::Colon | TokenType::Assign | TokenType::Comma | TokenType::CloseBrace
        ) || Self::is_any_stop_token(next_token_type)
    }

    /// Return whether one token starts a simple object literal key.
    fn token_starts_simple_object_key(&self, token_type: TokenType) -> bool {
        match token_type {
            TokenType::Identifier => true,
            TokenType::Literal => matches!(
                self.current_token().token.literal,
                Some(
                    TokenLiteral::String {
                        is_terminated: true,
                        has_invalid_escape: false,
                    } | TokenLiteral::Int { .. }
                        | TokenLiteral::Float { .. }
                        | TokenLiteral::Boolean { .. }
                )
            ),
            _ => false,
        }
    }

    /// Eat the simple object literal property forms.
    ///
    /// Examples:
    /// ```ds
    /// key: value
    /// key
    /// key = fallback
    /// ```
    fn eat_simple_object_property(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Property>> {
        if self.peek_is(TokenType::Spread) {
            self.bump();
            let value = self.eat_property_value_expression()?;
            let property = Property::Spread { value };

            return Ok(self.insert_node(property, self.get_span_from(start)));
        }

        let (key, key_span) = self.eat_key_with_span()?;

        if self.peek_colon_is() {
            return self.eat_simple_object_colon_field(start, key, key_span);
        }

        if self.peek_is(TokenType::Assign) {
            return self.eat_simple_object_default_field(start, key, key_span);
        }

        if self.current_token_ends_shorthand_object_property() {
            return self.eat_simple_object_shorthand_field(start, key, key_span);
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Eat one simple `key: value` object field.
    ///
    /// Examples:
    /// ```ds
    /// key: value
    /// "key": call()
    /// 0: first
    /// ```
    fn eat_simple_object_colon_field(
        &mut self,
        start: &ParserSpanStart,
        key: Key,
        key_span: Span,
    ) -> ParseResult<LocalNodeId<Property>> {
        let type_start = self.span_start();
        self.bump();

        let value = self.eat_simple_object_field_value()?;
        let property_id = self.insert_node(
            Property::Field {
                key,
                value,
                is_shorthand: false,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(property_id, key_span);
        self.tree.set_side_span(
            property_id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.get_span_from(&type_start),
        );

        Ok(property_id)
    }

    /// Eat one simple `key = fallback` object field.
    ///
    /// Examples:
    /// ```ds
    /// key = fallback
    /// value = call()
    /// item = defaultItem
    /// ```
    fn eat_simple_object_default_field(
        &mut self,
        start: &ParserSpanStart,
        key: Key,
        key_span: Span,
    ) -> ParseResult<LocalNodeId<Property>> {
        let Key::Name(Name::Identifier(name)) = key else {
            return Err(ParseError::unexpected(key_span));
        };

        let assign_start = self.span_start();
        self.bump();
        let default = self.eat_simple_object_field_value()?;
        let value = self.insert_node(Expression::Identifier { name }, key_span);
        let value = self.insert_property_default_expression(
            value,
            default,
            Some(self.get_span_from(&assign_start)),
        );
        let property_id = self.insert_node(
            Property::Field {
                key,
                value,
                is_shorthand: true,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(property_id, key_span);

        Ok(property_id)
    }

    /// Eat one simple shorthand object field.
    ///
    /// Examples:
    /// ```ds
    /// key
    /// value
    /// item
    /// ```
    fn eat_simple_object_shorthand_field(
        &mut self,
        start: &ParserSpanStart,
        key: Key,
        key_span: Span,
    ) -> ParseResult<LocalNodeId<Property>> {
        let Key::Name(Name::Identifier(name)) = key else {
            return Err(ParseError::unexpected(key_span));
        };

        let value = self.insert_node(Expression::Identifier { name }, key_span);
        let property_id = self.insert_node(
            Property::Field {
                key,
                value,
                is_shorthand: true,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(property_id, key_span);

        Ok(property_id)
    }

    /// Return whether the current token ends a shorthand object property.
    fn current_token_ends_shorthand_object_property(&mut self) -> bool {
        let token_type = self.peek_token_type();

        token_type == TokenType::Comma
            || token_type == TokenType::CloseBrace
            || Self::is_any_stop_token(token_type)
    }

    /// Eat one simple object field value.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// call()
    /// condition ? yes : no
    /// ```
    fn eat_simple_object_field_value(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let token_type = self.peek_token_type();
        if token_type == TokenType::Assign
            || token_type == TokenType::Comma
            || token_type == TokenType::CloseBrace
            || Self::is_any_stop_token(token_type)
        {
            return Ok(self.recover_missing_expression_here(NodeType::Property));
        }

        let ambient_context = self.flags.nested();
        let expression_context = self.flags.not_in_position().not_in_sequence_expression();
        let flags = self
            .flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context);

        self.eat_simple_object_value_expression(flags)
    }

    /// Eat one object field value without root expression entrypoint overhead.
    ///
    /// Examples:
    /// ```ds
    /// 1
    /// call()
    /// value ? yes : no
    /// ```
    fn eat_simple_object_value_expression(
        &mut self,
        flags: ParserFlags,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        let scope = ExpressionScope::from_flags(flags);

        let expression = if self.flags == flags {
            self.eat_assignment(&start, scope)
        } else {
            let outer_flags = self.swap_flags(flags);
            let expression = self.eat_assignment(&start, scope);
            self.restore_flags(outer_flags);

            expression
        };

        expression
    }

    /// Eat a property.
    ///
    /// Examples:
    /// ```
    /// // field
    /// x: int32
    /// x
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
        let start = self.span_start();

        // spread property
        if self.peek_is(TokenType::Spread) {
            let start = self.span_start();
            self.bump(); // eat spread
            let value = self.eat_property_value_expression()?;
            let property = Property::Spread { value };
            return Ok(self.insert_node(property, self.get_span_from(&start)));
        }

        // modifiers and head
        let modifiers =
            self.eat_binding_modifiers_prefix_maybe(true, true, false, false, false, false)?;
        let ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            role,
            is_async,
            is_generator,
            is_method,
            associated_comptime_name,
        } = self.eat_property_member_head(modifiers, false, false)?;

        // object fields cannot start with an unkeyed call signature
        if !self.flags.is_in_type()
            && key.is_none()
            && role.is_none()
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

        // reject definite assertions on plain properties
        if modifiers.is_some_and(|modifiers| modifiers.is_definite) {
            let error_span = self
                .prev()
                .map(|token| token.span)
                .unwrap_or(self.peek()?.span);
            return Err(ParseError::unexpected(error_span));
        }

        // unkeyed field separators are invalid
        if key.is_none() && (self.peek_colon_is() || self.peek_is(TokenType::Assign)) {
            return Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ));
        }

        // getters and setters require method form
        if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method form
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            if modifiers.is_some_and(|modifiers| modifiers.is_comptime) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // abstraction
            // methods without key or role are implicit calls
            let role = if key.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
            };
            if modifiers.is_some_and(|modifiers| modifiers.is_virtual)
                && matches!(role, Some(FunctionRole::Constructor | FunctionRole::New))
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
                role,
                if is_async {
                    Asynchrony::Async
                } else {
                    Asynchrony::Sync
                },
                is_generator,
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
                self.tree.set_side_span(
                    property_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
            }

            Ok(property_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, type_span): (Option<LocalNodeId<Expression>>, Option<Span>) =
                if self.peek_colon_is() {
                    let type_start = self.span_start();
                    self.bump(); // eat colon

                    // field type
                    let is_type_context = self.flags.is_in_variant() || self.flags.is_in_type();
                    let value = if self.peek_is(TokenType::Assign)
                        || self.peek_is(TokenType::Comma)
                        || self.peek_is(TokenType::CloseBrace)
                        || self.is_any_stop()
                    {
                        self.recover_missing_expression_here(NodeType::Property)
                    } else {
                        if is_type_context {
                            let type_flags = self.flags.nested().in_type();
                            let type_expression = self.eat_type_expression_or_recover_missing(
                                type_flags,
                                NodeType::Property,
                            )?;

                            self.insert_type_expression_value(type_expression)
                        } else {
                            let ambient_context = self.flags.nested();
                            let expression_context =
                                self.flags.not_in_position().not_in_sequence_expression();

                            self.eat_expression(
                                self.flags
                                    .with_ambient_context(ambient_context)
                                    .with_expression_context(expression_context),
                            )?
                        }
                    };
                    let type_span = self.get_span_from(&type_start);
                    (Some(value), Some(type_span))
                } else {
                    (None, None)
                };

            // default
            let (default, assign_operator_span) = if self.peek_is(TokenType::Assign) {
                let assign_start = self.span_start();
                self.bump(); // eat assign

                // keep associated comptime defaults in expression mode
                let default = if self.peek_is(TokenType::Comma)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.is_any_stop()
                {
                    self.recover_missing_expression_here(NodeType::Property)
                } else {
                    let ambient_context = if associated_comptime_name.is_some() {
                        self.flags.nested()
                    } else {
                        self.flags
                    };
                    let expression_context =
                        self.flags.not_in_position().not_in_sequence_expression();
                    self.eat_expression(
                        self.flags
                            .with_ambient_context(ambient_context)
                            .with_expression_context(expression_context),
                    )?
                };
                (Some(default), Some(self.get_span_from(&assign_start)))
            } else {
                (None, None)
            };

            // cover initialized shorthand fields as assignment expressions
            let is_defaulted_shorthand = value.is_none()
                && default.is_some()
                && modifiers.is_none()
                && matches!(key, Some(Key::Name(Name::Identifier(_))));

            // preserve both the declared type and the default
            let value = match (value, default) {
                (Some(value), Some(default)) => Some(self.insert_property_default_expression(
                    value,
                    default,
                    assign_operator_span,
                )),
                (None, Some(default)) if is_defaulted_shorthand => {
                    let Some((Key::Name(Name::Identifier(name)), key_span)) = key.zip(key_span)
                    else {
                        return Err(ParseError::unexpected(self.get_span_from(&start)));
                    };

                    let value = self.insert_node(Expression::Identifier { name }, key_span);

                    Some(self.insert_property_default_expression(
                        value,
                        default,
                        assign_operator_span,
                    ))
                }
                (Some(value), None) => Some(value),
                (None, Some(default)) => Some(default),
                (None, None) => None,
            };

            // shorthand field value
            let is_bare_shorthand = value.is_none()
                && default.is_none()
                && modifiers.is_none()
                && matches!(key, Some(Key::Name(Name::Identifier(_))));
            let is_shorthand = is_bare_shorthand || is_defaulted_shorthand;

            let value = if is_bare_shorthand {
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

            // field construction requires a key
            let Some(key) = key else {
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::Identifier,
                ));
            };

            // keyed fields without `:` or `=` are invalid unless they were shorthand
            if value.is_none() {
                return Err(ParseError::expected(self.peek()?.span, TokenType::Colon));
            }
            let Some(value) = value else {
                return Err(ParseError::expected(self.peek()?.span, TokenType::Colon));
            };

            let property = Property::Field {
                key,
                value,
                is_shorthand,
            };
            let property_id = self.insert_node(property, self.get_span_from(&start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(property_id, span);
            }

            // set type span for field type annotation
            if let Some(span) = type_span {
                self.tree.set_side_span(
                    property_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
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
            // read the current token once per iteration
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
            else if self.flags.is_in_type() && token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_property_decorators.extend(decorators);
                continue;
            }
            // consume comma separators between properties
            else if token_type == TokenType::Comma {
                self.eat_item_stop()?;
                continue;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop()?;
                if !self.flags.is_in_type()
                    && self.current_token_is_on_new_line()
                    && Self::token_can_start_recovered_statement_item(self.peek_token_type())
                {
                    break;
                }

                continue;
            }
            // keep eating properties
            else {
                match self.try_eat_property() {
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

    /// Try to eat a type member and recover one malformed member when possible.
    pub(crate) fn try_eat_type_member(
        &mut self,
        body_mode: TypeMemberBodyMode,
    ) -> ParseResult<LocalNodeId<TypeMember>> {
        match self.eat_type_member(body_mode) {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::TypeMember);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_body_from_span(span, Some(err))?;

                Ok(self.insert_node(TypeMember::Error, recovered_span))
            }
        }
    }

    /// Eat a type member.
    pub(crate) fn eat_type_member(
        &mut self,
        body_mode: TypeMemberBodyMode,
    ) -> ParseResult<LocalNodeId<TypeMember>> {
        let start = self.span_start();

        // plain fields
        if let Some(member_id) = self.eat_plain_type_field_member_if_present(&start)? {
            return Ok(member_id);
        }

        // associated members
        if let Some(member_id) = self.try_eat_type_member_associated_type(&start)? {
            return Ok(member_id);
        }
        if let Some(member_id) = self.try_eat_type_member_associated_const(&start)? {
            return Ok(member_id);
        }

        // static
        let is_static = if self.is_keyword(Keyword::Static) && self.next_token_starts_member_name()
        {
            self.bump(); // eat static
            true
        } else {
            false
        };

        // readonly
        let is_readonly = if self.is_keyword(Keyword::Readonly)
            && self.next_same_line_token_starts_member_name()
        {
            self.bump(); // eat readonly
            true
        } else {
            false
        };

        // abstract
        let next_token = self.next_token();
        let abstract_is_modifier = self.is_keyword(Keyword::Abstract)
            && !next_token.token.is_on_new_line
            && matches!(
                next_token.token.ty,
                TokenType::Identifier
                    | TokenType::Literal
                    | TokenType::Hash
                    | TokenType::OpenBracket
                    | TokenType::OpenParenthesis
                    | TokenType::LessThan
            );
        let is_abstract = if abstract_is_modifier {
            self.bump(); // eat abstract
            true
        } else {
            false
        };

        // role
        let role = self.eat_method_role_maybe(false, true);

        // index signature
        if self.type_member_starts_index_signature() {
            if is_abstract {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let key_start = self.span_start();
            self.eat_token(TokenType::OpenBracket)?;
            let (name, name_span) = self.eat_binding_identifier_with_span()?;
            self.eat_token(TokenType::Colon)?;

            let key_type = self.eat_type_expression_or_recover_missing(
                self.flags
                    .not_in_position()
                    .not_in_sequence_expression()
                    .in_type(),
                NodeType::TypeMember,
            )?;

            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseBracket,
                NodeType::TypeMember,
                |_, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type)
                        || matches!(token_type, TokenType::Colon | TokenType::Maybe)
                },
            )?;

            // optional index signatures
            let optional_start = self.span_start();
            let is_optional = self.eat_token_maybe(TokenType::Maybe)?;
            let optional_span = is_optional.then(|| self.get_span_from(&optional_start));

            let type_start = self.span_start();
            self.eat_token(TokenType::Colon)?;

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
                NodeSpanType::Region(NodeSpanRegion::Type),
                self.get_span_from(&type_start),
            );

            if let Some(span) = optional_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                    span,
                );
            }

            return Ok(member_id);
        }

        // key
        let (key, key_span) = if matches!(role, Some(FunctionRole::Constructor | FunctionRole::New))
        {
            (None, None)
        } else if let Some((key, span)) = self.eat_property_key_with_span()? {
            (Some(key), Some(span))
        } else {
            (None, None)
        };

        // optional
        let optional_start = self.span_start();
        let is_optional = self.eat_token_maybe(TokenType::Maybe)?;
        let optional_span = is_optional.then(|| self.get_span_from(&optional_start));

        // method
        let is_method = self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(
                role,
                Some(
                    FunctionRole::Getter
                        | FunctionRole::Setter
                        | FunctionRole::Constructor
                        | FunctionRole::New
                )
            );
        if is_method {
            if is_readonly {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && key.is_none() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let role = if key.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
            };

            let ParsedMethodTail {
                signature,
                generic_parameter_span,
                parameter_span,
                body,
                return_type_span,
            } = self.eat_method_tail(
                NodeType::TypeMember,
                role,
                Asynchrony::Sync,
                false,
                is_abstract,
                false,
                body_mode.allows_body() && self.language.is_destack(),
            )?;

            let member = match (key, role) {
                (Some(key), _) => TypeMember::Method {
                    is_static,
                    is_optional,
                    key,
                    signature,
                    body,
                },
                (None, Some(FunctionRole::New | FunctionRole::Constructor)) => {
                    TypeMember::ConstructSignature {
                        signature: constructor_type_declaration_from_signature(signature),
                    }
                }
                (None, None | Some(FunctionRole::Call)) => TypeMember::CallSignature {
                    signature: function_type_declaration_from_signature(signature),
                },
                (None, Some(FunctionRole::Getter | FunctionRole::Setter)) => {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            if let Some(span) = return_type_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
            }

            if let Some(span) = generic_parameter_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                    span,
                );
            }

            self.tree.set_side_span(
                member_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                parameter_span,
            );

            if let Some(span) = optional_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                    span,
                );
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

        let type_start = self.span_start();
        let declared_type = if self.eat_token_maybe(TokenType::Colon)? {
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
            is_static,
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
                NodeSpanType::Region(NodeSpanRegion::Type),
                self.get_span_from(&type_start),
            );
        }

        if let Some(span) = optional_span {
            self.tree.set_side_span(
                member_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                span,
            );
        }

        Ok(member_id)
    }

    /// Eat a plain type field member when the head is unambiguous.
    ///
    /// Examples:
    /// ```ds
    /// name: string
    /// name?: string
    /// "kind": "ready"
    /// ```
    fn eat_plain_type_field_member_if_present(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<TypeMember>>> {
        if !self.plain_type_field_member_starts_here() {
            return Ok(None);
        }

        let (key, key_span) = self.eat_key_with_span()?;

        let optional_start = self.span_start();
        let is_optional = self.eat_token_maybe(TokenType::Maybe)?;
        let optional_span = is_optional.then(|| self.get_span_from(&optional_start));

        let type_start = self.span_start();
        self.eat_token(TokenType::Colon)?;
        let declared_type = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
            self.recover_missing_type_expression_here(NodeType::TypeMember)
        } else {
            self.eat_method_return_type(NodeType::TypeMember)?
        };

        let member_id = self.insert_node(
            TypeMember::Field {
                is_static: false,
                is_optional,
                is_readonly: false,
                key,
                declared_type: Some(declared_type),
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, key_span);
        self.tree.set_side_span(
            member_id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.get_span_from(&type_start),
        );

        if let Some(span) = optional_span {
            self.tree.set_side_span(
                member_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                span,
            );
        }

        Ok(Some(member_id))
    }

    /// Return whether the current member is a plain field.
    fn plain_type_field_member_starts_here(&mut self) -> bool {
        if !matches!(
            self.peek_token_type(),
            TokenType::Identifier | TokenType::Literal
        ) {
            return false;
        }

        let next_token_type = self.token_type_at_offset(1);
        if next_token_type == TokenType::Colon {
            return true;
        }

        next_token_type == TokenType::Maybe && self.token_type_at_offset(2) == TokenType::Colon
    }

    /// Eat type members inside one object type body.
    pub(crate) fn eat_type_members(
        &mut self,
        body_mode: TypeMemberBodyMode,
    ) -> ParseResult<Vec<LocalNodeId<TypeMember>>> {
        let mut members: Vec<LocalNodeId<TypeMember>> = Vec::new();
        let mut pending_member_decorators = PendingDecorators::new();
        let mut previous_member_had_error = false;

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // close the member list
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                if !pending_member_decorators.is_empty() {
                    let error = ParseError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_member_decorators.clear();
                }

                break;
            }
            // collect decorators for the next member
            else if token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_member_decorators.extend(decorators);
                previous_member_had_error = false;

                continue;
            }
            // skip item separators
            else if token_type == TokenType::Comma {
                self.eat_item_stop()?;
                previous_member_had_error = false;

                continue;
            }
            // let a damaged member release the next declaration
            else if token_type == TokenType::Semicolon {
                if previous_member_had_error
                    && self.current_token_is_declaration_recovery_boundary(token_type)
                {
                    break;
                }

                self.eat_any_stop()?;
                previous_member_had_error = false;

                continue;
            }
            // skip statement separators
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop()?;
                previous_member_had_error = false;

                continue;
            }

            let error_count = self.errors.len();
            match self.try_eat_type_member(body_mode) {
                Ok(member_id) => {
                    previous_member_had_error = self.errors.len() > error_count
                        || matches!(self.tree.get(member_id), TypeMember::Error);

                    if !pending_member_decorators.is_empty() {
                        self.attach_decorators(
                            member_id.id,
                            std::mem::take(&mut pending_member_decorators),
                        );
                    }

                    members.push(member_id);
                }
                Err(_) => {
                    previous_member_had_error = true;
                }
            }
        }

        Ok(members)
    }

    /// Try to eat a member and recover one malformed member when possible.
    pub fn try_eat_member(&mut self) -> ParseResult<LocalNodeId<Member>> {
        match self.eat_member() {
            Ok(member_id) => Ok(member_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Member);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_body_from_span(span, Some(err))?;

                Ok(self.insert_node(Member::Error, recovered_span))
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
        let start = self.span_start();

        // modifiers prefix
        let modifiers =
            self.eat_binding_modifiers_prefix_maybe(true, true, true, true, true, true)?;

        // duplicate static modifier across newlines
        if modifiers
            .as_ref()
            .is_some_and(|modifiers| modifiers.is_static)
            && self.current_token_is_on_new_line()
            && self.is_keyword(Keyword::Static)
        {
            let static_span = self.peek()?.span;
            let has_member_name_after =
                self.next_token_type() == TokenType::Identifier && self.next_keyword().is_none();
            if has_member_name_after {
                let error = ParseError::unexpected(static_span);
                self.error(&error);
                self.bump(); // eat static
            }
        }

        // static block: `static { ... }` or `static\n{ ... }`
        // must check before key parsing since static is already a modifier
        if modifiers.is_some_and(|modifiers| modifiers.is_static)
            && self.peek_is(TokenType::OpenBrace)
        {
            let body_start = self.span_start();
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
            && self.peek_is(TokenType::OpenBrace)
        {
            let body_start = self.span_start();
            let body_block = self.eat_block(BlockContext::Statement)?;
            let body = self.insert_node(
                Expression::Block(body_block),
                self.get_span_from(&body_start),
            );

            return Ok(self.insert_node(Member::ComptimeBlock { body }, self.get_span_from(&start)));
        }

        // head
        let allow_constructor_mode = !modifiers.is_some_and(|modifiers| modifiers.is_static);
        let ParsedPropertyMemberHead {
            modifiers,
            key,
            key_span,
            role,
            is_async,
            is_generator,
            is_method,
            associated_comptime_name,
        } = self.eat_property_member_head(modifiers, allow_constructor_mode, false)?;

        // getters and setters require method form
        if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && !is_method {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if is_method {
            // associated comptime constants cannot use method form
            if associated_comptime_name.is_some() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // abstraction
            // methods without key or role are implicit calls
            let role = if key.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
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
                role,
                if is_async {
                    Asynchrony::Async
                } else {
                    Asynchrony::Sync
                },
                is_generator,
                modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                modifiers.is_some_and(|modifiers| modifiers.is_override),
                true,
            )?;

            // method member
            let member = Member::Method {
                key,
                signature,
                abstraction: modifiers.map_or(MethodAbstraction::Concrete, |modifiers| {
                    modifiers.method_abstraction()
                }),
                body,
                visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                is_optional: modifiers.is_some_and(|modifiers| modifiers.is_optional),
                is_ambient: self.is_ambient_for_modifiers(modifiers.as_ref()),
                is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
                is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                is_accessor: modifiers.is_some_and(|modifiers| modifiers.is_accessor),
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            // set type span for return type annotation
            if let Some(span) = return_type_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
            }

            if let Some(span) = generic_parameter_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                    span,
                );
            }

            self.tree.set_side_span(
                member_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                parameter_span,
            );

            Ok(member_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, comptime_type, type_span) = if self.peek_colon_is() {
                let type_start = self.span_start();
                self.bump(); // eat colon

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
                let default = if self.peek_is(TokenType::CloseBrace) || self.is_any_stop() {
                    self.recover_missing_expression_here(NodeType::Member)
                } else {
                    self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?
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
            if value.is_none() && default.is_none() && !self.can_insert_semicolon() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            if associated_comptime_name.is_none()
                && modifiers.is_some_and(|modifiers| modifiers.is_comptime || modifiers.is_virtual)
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            let member = if let Some(name) = associated_comptime_name {
                Member::AssociatedConst {
                    name,
                    declared_type: comptime_type,
                    value: default,
                    visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                    is_ambient: self.is_ambient_for_modifiers(modifiers.as_ref()),
                    is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                }
            } else {
                let Some(key) = key else {
                    return Err(ParseError::unexpected(self.get_span_from(&start)));
                };

                Member::Field {
                    key,
                    declared_type: value,
                    default,
                    mutability: None,
                    is_optional: modifiers.is_some_and(|modifiers| modifiers.is_optional),
                    is_definite: modifiers.is_some_and(|modifiers| modifiers.is_definite),
                    is_readonly: modifiers.is_some_and(|modifiers| modifiers.is_readonly),
                    visibility: modifiers.and_then(|modifiers| modifiers.visibility),
                    is_ambient: self.is_ambient_for_modifiers(modifiers.as_ref()),
                    is_abstract: modifiers.is_some_and(|modifiers| modifiers.is_abstract),
                    is_override: modifiers.is_some_and(|modifiers| modifiers.is_override),
                    is_static: modifiers.is_some_and(|modifiers| modifiers.is_static),
                    is_accessor: modifiers.is_some_and(|modifiers| modifiers.is_accessor),
                }
            };
            let member_id = self.insert_node(member, self.get_span_from(&start));

            // set main span to the key identifier
            if let Some(span) = key_span {
                self.tree.set_main_span(member_id, span);
            }

            // set type span for field type annotation
            if let Some(span) = type_span {
                self.tree.set_side_span(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    span,
                );
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
            // read the current token once per iteration
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
                self.eat_any_stop()?;
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
                match self.try_eat_member() {
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
