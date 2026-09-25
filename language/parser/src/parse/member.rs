use tspp_dir::{
    Asynchrony, BlockContext, Expression, FunctionForm, FunctionPhase, FunctionRole,
    FunctionSignature, Keyword, LocalNodeId, Member, Name, NodeType, StringId, TokenType,
    TypeExpression,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

use crate::parse::{
    BindingModifiers, BindingPosition, DeclarationNesting, ExpressionPosition, ExpressionStop,
    FunctionModifiers, TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};

/// The shared head of one property or member.
#[derive(Debug)]
pub(crate) struct MemberHead {
    /// The consumed modifiers.
    pub(crate) modifiers: BindingModifiers,
    /// The member name.
    pub(crate) name: Option<Name>,
    /// The name range.
    pub(crate) name_range: Option<ByteRange>,
    /// The method role.
    pub(crate) role: Option<FunctionRole>,
    /// The method role keyword range.
    pub(crate) role_range: Option<ByteRange>,
    /// The function modifiers.
    pub(crate) function_modifiers: FunctionModifiers,
    /// Whether the head is method-shaped.
    pub(crate) is_method: bool,
    /// The associated const name when present.
    pub(crate) associated_const_name: Option<StringId>,
}

/// One completed method signature and body.
#[derive(Debug)]
pub(crate) struct Method {
    /// The method signature.
    pub(crate) signature: FunctionSignature,
    /// The generic parameter container range.
    pub(crate) generic_parameter_range: Option<ByteRange>,
    /// The parameter container range.
    pub(crate) parameter_range: ByteRange,
    /// The optional body.
    pub(crate) body: Option<LocalNodeId<Expression>>,
    /// The return type range.
    pub(crate) return_type_range: Option<ByteRange>,
}

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse one `async` keyword when it plausibly starts a method head.
    #[inline]
    fn parse_method_asynchrony(&mut self) -> Asynchrony {
        if !self.peek_is_keyword(Keyword::Async) {
            return Asynchrony::Sync;
        }

        let peek_next_token = self.peek_next_token();
        let can_start_async_method = !peek_next_token.is_on_new_line()
            && matches!(
                peek_next_token.ty(),
                TokenType::Identifier
                    | TokenType::Literal
                    | TokenType::Hash
                    | TokenType::OpenBracket
                    | TokenType::Multiply
                    | TokenType::OpenParenthesis
                    | TokenType::LessThan
            );
        if can_start_async_method {
            self.bump();
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        }
    }

    /// Parse late modifiers after an `async` head.
    #[inline]
    fn parse_method_late_modifiers(
        &mut self,
        mut modifiers: BindingModifiers,
        asynchrony: Asynchrony,
    ) -> BindingModifiers {
        if asynchrony == Asynchrony::Sync {
            return modifiers;
        }

        loop {
            // parse each supported late modifier
            match self.peek_keyword() {
                Some(Keyword::Abstract) => modifiers.is_abstract = true,
                Some(Keyword::Override) => modifiers.is_override = true,
                _ => break,
            }

            self.bump();
        }

        modifiers
    }

    /// Parse one method role.
    #[inline]
    pub(crate) fn parse_method_role(
        &mut self,
        constructor_role: Option<FunctionRole>,
    ) -> Option<(FunctionRole, ByteRange)> {
        let keyword = self.peek_keyword()?;

        // accessors need one member-name lookahead
        if matches!(keyword, Keyword::Get | Keyword::Set) {
            let peek_next_token = self.peek_next_token();
            if !Self::is_member_name_start(peek_next_token)
                || peek_next_token.is(TokenType::OpenParenthesis)
            {
                return None;
            }

            let range = self.eat().token.range();
            let role = match keyword {
                Keyword::Get => FunctionRole::Getter,
                Keyword::Set => FunctionRole::Setter,
                _ => return None,
            };

            return Some((role, range));
        }

        // constructors and new methods only need delimiter lookahead
        let role = match (constructor_role, keyword) {
            (Some(FunctionRole::Constructor), Keyword::Constructor) => FunctionRole::Constructor,
            (Some(FunctionRole::New), Keyword::New) => FunctionRole::New,
            _ => return None,
        };

        if matches!(
            self.peek_next_token_type(),
            TokenType::LessThan | TokenType::OpenParenthesis
        ) {
            let range = self.eat().token.range();

            Some((role, range))
        } else {
            None
        }
    }

    /// Validate one member or property head after its name and postfix modifiers.
    fn validate_method_head_modifiers(
        &mut self,
        name: Option<&Name>,
        modifiers: &BindingModifiers,
        asynchrony: Asynchrony,
    ) -> ParserResult<()> {
        // reject impossible modifier combinations
        let is_invalid = modifiers.is_abstract && modifiers.is_virtual
            || modifiers.is_static && modifiers.is_virtual;
        if is_invalid {
            let error_range = match self.peek_previous_token() {
                Some(token) => token.span.range(),
                None => self.peek_token().range(),
            };

            return Err(ParserError::unexpected(error_range));
        }

        // reject an optional marker joined to an async method head
        if asynchrony == Asynchrony::Sync
            && modifiers.is_optional
            && matches!(
                name,
                Some(Name::Identifier(name)) if self.strings.get(*name) == "async"
            )
            && !self.peek_is_on_new_line()
            && self.peek_is(TokenType::Identifier)
        {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        Ok(())
    }

    /// Parse one method or field type expression.
    #[inline]
    pub(crate) fn parse_method_return_type(
        &mut self,
        owner: NodeType,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.parse_type_or_recover_missing(TypePosition::Type, TypeStop::default(), owner)
    }

    /// Parse one method body expression.
    #[inline]
    fn parse_method_body(
        &mut self,
        modifiers: FunctionModifiers,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let body_keywords = self.keywords.body(modifiers);

        if self.peek_is(TokenType::OpenBrace) {
            let block = self.with_keywords(body_keywords, |parser| {
                parser.parse_block(BlockContext::Expression)
            })?;

            return Ok(self.insert_node(Expression::Block(block), self.tree.get_range(block)));
        }

        self.with_keywords(body_keywords, |parser| {
            parser.parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        })
    }

    /// Parse one shared property or member head.
    pub(crate) fn parse_member_head(
        &mut self,
        mut modifiers: BindingModifiers,
        constructor_role: Option<FunctionRole>,
    ) -> ParserResult<MemberHead> {
        // async and late abstraction modifiers
        let asynchrony = self.parse_method_asynchrony();
        modifiers = self.parse_method_late_modifiers(modifiers, asynchrony);

        // role and accessor marker
        let parsed_role = self.parse_method_role(constructor_role);
        let (role, role_range) = match parsed_role {
            Some((role, range)) => (Some(role), Some(range)),
            None => (None, None),
        };

        // generator and name
        let is_generator = self.eat_token_if(TokenType::Multiply);
        let function_modifiers = FunctionModifiers {
            asynchrony,
            is_generator,
        };
        let (name, name_range) =
            if let Some((name, range)) = self.eat_property_name_with_range_if_present()? {
                (Some(name), Some(range))
            } else {
                (None, None)
            };

        // postfix modifiers
        let modifiers = self.parse_postfix_binding_modifier(modifiers);
        self.validate_method_head_modifiers(name.as_ref(), &modifiers, asynchrony)?;

        // classify the head
        let associated_const_name = if modifiers.is_const_asserted {
            match name.as_ref() {
                Some(Name::Identifier(name)) => Some(*name),
                _ => return Err(ParserError::unexpected(self.peek_token_span())),
            }
        } else {
            None
        };
        let is_method = asynchrony == Asynchrony::Async
            || is_generator
            || self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::OpenParenthesis)
            || matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter));

        Ok(MemberHead {
            modifiers,
            name,
            name_range,
            role,
            role_range,
            function_modifiers,
            is_method,
            associated_const_name,
        })
    }

    /// Parse the shared tail of one method head.
    ///
    /// Examples:
    /// ```tspp
    /// (): int32
    /// <T>(value: T): T
    /// get value(): int32
    /// set value(next: int32): void
    /// where T: Copy { value }
    /// ```
    pub(crate) fn parse_method(
        &mut self,
        owner: NodeType,
        role: Option<FunctionRole>,
        function_modifiers: FunctionModifiers,
        is_abstract: bool,
        is_override: bool,
        is_body_allowed: bool,
    ) -> ParserResult<Method> {
        let parameter_keywords = self.keywords.parameters(function_modifiers);
        let (
            generic_parameters,
            generic_parameter_range,
            parameters,
            parameter_range,
            return_type,
            return_type_range,
            where_clauses,
        ) = self.with_keywords(parameter_keywords, |parser| {
            // parse generic parameters
            let generic_parameter_start = parser.mark_parse_start();
            let generic_parameters = parser
                .parse_generic_parameters_if_present(false)?
                .unwrap_or_default();
            let generic_parameter_range = (!generic_parameters.is_empty())
                .then(|| parser.range_since(&generic_parameter_start));

            // parse parameters
            let parameter_start = parser.mark_parse_start();
            let parameters = parser.parse_dynamic_parameters(ExpressionPosition::Value)?;
            let parameter_range = parser.range_since(&parameter_start);

            // parse the return type
            let has_return_type_marker = parser.peek_is(TokenType::Colon);
            let (return_type, return_type_range) = if has_return_type_marker {
                let type_start = parser.mark_parse_start();
                parser.eat_token(TokenType::Colon)?;

                let return_type = if parser.peek_is(TokenType::CloseBrace) || parser.peek_any_stop()
                {
                    parser.recover_missing_type_expression_here(owner)
                } else {
                    parser.parse_method_return_type(owner)?
                };
                (Some(return_type), Some(parser.range_since(&type_start)))
            } else {
                (None, None)
            };

            // parse trailing constraints
            let where_clauses = parser.parse_where_clauses()?;

            Ok((
                generic_parameters,
                generic_parameter_range,
                parameters,
                parameter_range,
                return_type,
                return_type_range,
                where_clauses,
            ))
        })?;

        // body
        let body = if is_body_allowed && self.peek_is(TokenType::OpenBrace) {
            Some(self.parse_method_body(function_modifiers)?)
        } else {
            None
        };

        // reject a body where the containing declaration forbids one
        if !is_body_allowed && self.peek_is(TokenType::OpenBrace) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // require a separator after every bodyless method
        let has_terminator = self.peek_is_on_new_line()
            || self.peek_any_stop()
            || self.peek_is(TokenType::CloseBrace);
        if body.is_none() && !has_terminator {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // split out the receiver parameter
        let (this_form, this_parameter, parameters) = self.split_this_parameter(parameters);

        // assemble the signature
        let signature = FunctionSignature {
            asynchrony: function_modifiers.asynchrony,
            role,
            form: FunctionForm::Function,
            phase: FunctionPhase::Normal,
            generic_parameters,
            where_clauses,
            this_form,
            this_parameter,
            parameters,
            return_type,
            is_abstract,
            is_override,
            is_generator: function_modifiers.is_generator,
        };

        Ok(Method {
            signature,
            generic_parameter_range,
            parameter_range,
            body,
            return_type_range,
        })
    }

    /// Parse one member type expression.
    #[inline]
    pub(crate) fn parse_member_type(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.parse_type_or_recover_missing(
            TypePosition::Type,
            TypeStop::default(),
            NodeType::Member,
        )
    }

    /// Parse one associated type member when present.
    ///
    /// Examples:
    /// ```tspp
    /// type Item
    /// type Item = string
    /// type Item: Display
    /// type Item<T> where T: Copy = Vec<T>
    /// ```
    fn parse_associated_type_member_if_present(
        &mut self,
        start: &ParseStart,
        modifiers: BindingModifiers,
    ) -> ParserResult<Option<LocalNodeId<Member>>> {
        if !self.peek_is_keyword(Keyword::Type)
            || self.peek_next_token_type() != TokenType::Identifier
        {
            return Ok(None);
        }

        // reject impossible associated modifiers
        if modifiers.is_static {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // keyword and name
        self.bump();
        let (name, name_range) = self.eat_identifier_with_range()?;

        // generic parameters and where clauses
        let generic_parameters = self
            .parse_generic_parameters_if_present(false)?
            .unwrap_or_default();
        let where_clauses = self.parse_where_clauses()?;

        // declared type
        let constraint = if self.peek_is(TokenType::Colon) {
            self.bump();

            let constraint = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.peek_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::Member)
            } else {
                self.parse_member_type()?
            };

            Some(constraint)
        } else {
            None
        };

        // value
        let value = if self.peek_is(TokenType::Assign) {
            self.bump();

            let value = if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
                self.recover_missing_type_expression_here(NodeType::Member)
            } else {
                self.parse_member_type()?
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
            visibility: modifiers.visibility,
            is_ambient: modifiers.is_ambient,
            is_abstract: modifiers.is_abstract,
            is_override: modifiers.is_override,
        };
        let member_id = self.insert_node(member, self.range_since(start));
        self.tree.set_main_range(member_id, name_range);

        Ok(Some(member_id))
    }

    /// Parse one member, recovering malformed input as an error node.
    pub(crate) fn parse_member_or_recover(&mut self) -> LocalNodeId<Member> {
        match self.parse_member() {
            Ok(member_id) => member_id,
            Err(error) => {
                let error = error.in_node(NodeType::Member);
                let recovered_range = self.recover_body(error.range(), error);

                self.insert_node(Member::Error, recovered_range)
            }
        }
    }

    /// Parse a member (class/struct/interface/extension body element).
    ///
    /// Examples:
    /// ```tspp
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
    pub(crate) fn parse_member(&mut self) -> ParserResult<LocalNodeId<Member>> {
        let start = self.mark_parse_start();

        // modifiers prefix
        let modifiers = self.parse_binding_modifiers(BindingPosition::Member);

        // duplicate static modifier across newlines
        if modifiers.is_static
            && self.peek_is_on_new_line()
            && self.peek_is_keyword(Keyword::Static)
        {
            let static_range = self.peek_token().range();
            let has_member_name_after = self.peek_next_token_type() == TokenType::Identifier
                && self.peek_next_keyword().is_none();
            if has_member_name_after {
                let error = ParserError::unexpected(static_range);
                self.report_error(error);
                self.bump();
            }
        }

        // static block: `static { ... }` or `static\n{ ... }`
        // must check before name parsing since static is already a modifier
        if modifiers.is_static && self.peek_is(TokenType::OpenBrace) {
            let body_start = self.mark_parse_start();
            let body_block = self.parse_block(BlockContext::Statement)?;
            let body =
                self.insert_node(Expression::Block(body_block), self.range_since(&body_start));

            return Ok(self.insert_node(Member::StaticBlock { body }, self.range_since(&start)));
        }

        // type member: `type Name<U> = ...` or `type Name: Bound`
        if let Some(member_id) = self.parse_associated_type_member_if_present(&start, modifiers)? {
            return Ok(member_id);
        }

        // const block: `const { ... }` (block head already consumed)
        if modifiers.is_const_block && self.peek_is(TokenType::OpenBrace) {
            let body_start = self.mark_parse_start();
            let body_block = self.parse_block(BlockContext::Statement)?;
            let body =
                self.insert_node(Expression::Block(body_block), self.range_since(&body_start));

            return Ok(self.insert_node(Member::ConstBlock { body }, self.range_since(&start)));
        }

        // head
        let constructor_role = if modifiers.is_static {
            None
        } else {
            Some(FunctionRole::Constructor)
        };
        let MemberHead {
            modifiers,
            name,
            name_range,
            role,
            role_range,
            function_modifiers,
            is_method,
            associated_const_name,
        } = self.parse_member_head(modifiers, constructor_role)?;

        // reject impossible associated modifiers
        if associated_const_name.is_some() && modifiers.is_static {
            return Err(ParserError::unexpected(self.range_since(&start)));
        }

        // getters and setters require method form
        if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && !is_method {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        if is_method {
            // associated consts cannot use method form
            if associated_const_name.is_some() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            // abstraction
            // methods without a name or role are implicit calls
            let role = if name.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
            };

            // modifiers postfix (again after parameters)
            let modifiers = self.parse_postfix_binding_modifier(modifiers);
            let Method {
                signature,
                generic_parameter_range,
                parameter_range,
                body,
                return_type_range,
            } = self.parse_method(
                NodeType::Member,
                role,
                function_modifiers,
                modifiers.is_abstract,
                modifiers.is_override,
                true,
            )?;

            // method member
            let member = Member::Method {
                name,
                signature,
                abstraction: modifiers.method_abstraction(),
                body,
                visibility: modifiers.visibility,
                is_optional: modifiers.is_optional,
                is_ambient: modifiers.is_ambient,
                is_override: modifiers.is_override,
                is_static: modifiers.is_static,
                is_accessor: modifiers.is_accessor,
            };
            let member_id = self.insert_node(member, self.range_since(&start));

            // set the main source range to the declared name or role
            if let Some(range) = name_range.or(role_range) {
                self.tree.set_main_range(member_id, range);
            }

            // set the type source range for return type annotation
            if let Some(range) = return_type_range {
                self.tree.set_side_range(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    range,
                );
            }

            if let Some(range) = generic_parameter_range {
                self.tree.set_side_range(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                    range,
                );
            }

            self.tree.set_side_range(
                member_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
                parameter_range,
            );

            Ok(member_id)
        }
        // field
        else {
            // value (type annotation)
            let (value, const_type, type_range) = if self.peek_is(TokenType::Colon) {
                let type_start = self.mark_parse_start();
                self.bump();

                // member field annotations are always type positions
                let is_missing_type = self.peek_is(TokenType::Assign)
                    || self.peek_is(TokenType::CloseBrace)
                    || self.peek_any_stop();

                let declared_type = if is_missing_type {
                    self.recover_missing_type_expression_here(NodeType::Member)
                } else {
                    self.parse_member_type()?
                };
                let (value, const_type) = if associated_const_name.is_some() {
                    (None, Some(declared_type))
                } else {
                    (Some(declared_type), None)
                };
                let type_range = self.range_since(&type_start);
                (value, const_type, Some(type_range))
            } else {
                (None, None, None)
            };

            // default
            let default = if self.peek_is(TokenType::Assign) {
                self.bump();
                let default = self.parse_expression_or_recover_missing(
                    ExpressionPosition::Value,
                    ExpressionStop::default(),
                    NodeType::Member,
                )?;
                Some(default)
            } else {
                None
            };

            // member
            if modifiers.is_empty() && name.is_none() && value.is_none() && default.is_none() {
                // not a member
                return Err(ParserError::expected(
                    self.peek_token().range(),
                    TokenType::Identifier,
                ));
            }
            // fields without initializers must end at a statement boundary
            if value.is_none() && default.is_none() && !self.peek_semicolon_insertion() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }
            if associated_const_name.is_none() && modifiers.is_virtual {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }
            let member = if let Some(name) = associated_const_name {
                Member::AssociatedConst {
                    name,
                    declared_type: const_type,
                    value: default,
                    visibility: modifiers.visibility,
                    is_ambient: modifiers.is_ambient,
                    is_abstract: modifiers.is_abstract,
                    is_override: modifiers.is_override,
                }
            } else {
                let Some(name) = name else {
                    return Err(ParserError::unexpected(self.range_since(&start)));
                };

                Member::Field {
                    name,
                    declared_type: value,
                    default,
                    mutability: None,
                    is_optional: modifiers.is_optional,
                    is_readonly: modifiers.is_readonly,
                    visibility: modifiers.visibility,
                    is_ambient: modifiers.is_ambient,
                    is_abstract: modifiers.is_abstract,
                    is_override: modifiers.is_override,
                    is_static: modifiers.is_static,
                    is_accessor: modifiers.is_accessor,
                }
            };
            let member_id = self.insert_node(member, self.range_since(&start));

            // set the main source range to the name
            if let Some(range) = name_range {
                self.tree.set_main_range(member_id, range);
            }

            // set the type source range for field type annotation
            if let Some(range) = type_range {
                self.tree.set_side_range(
                    member_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    range,
                );
            }

            Ok(member_id)
        }
    }

    /// Parse members (class/struct/interface/extension body).
    pub(crate) fn parse_members(&mut self) -> ParserResult<Vec<LocalNodeId<Member>>> {
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut is_previous_member_damaged = false;

        while self.has_more_tokens() {
            // read the current token once per iteration
            let token_type = self.peek_token_type();

            // stop on closing brace
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                break;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                if token_type == TokenType::Comma {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }
                if token_type == TokenType::Semicolon
                    && is_previous_member_damaged
                    && self.peek_semicolon_declaration_boundary(DeclarationNesting::Member)
                {
                    break;
                }

                self.eat_any_stop()?;
                is_previous_member_damaged = false;

                continue;
            }
            // release a declaration after a damaged member
            else if is_previous_member_damaged
                && self.peek_declaration_boundary(DeclarationNesting::Member)
            {
                break;
            }
            // parse documentation, decorators and one member
            else {
                let documentation = self.parse_documentation();
                let decorators = self.parse_decorators();

                // reject decorator prefixes without an owner
                if !decorators.is_empty()
                    && matches!(
                        self.peek_token_type(),
                        TokenType::CloseBrace | TokenType::End
                    )
                {
                    let error = ParserError::unexpected(self.peek_token_span());
                    self.report_error(error);

                    break;
                }

                let error_count = self.errors.len();
                let member_id = self.parse_member_or_recover();
                is_previous_member_damaged = self.errors.len() > error_count
                    || matches!(self.tree.get(member_id), Member::Error);

                if !matches!(self.tree.get(member_id), Member::Error) {
                    self.attach_documentation(member_id, documentation);
                }
                self.attach_decorators(member_id.id, decorators);
                members.push(member_id);
            }
        }
        Ok(members)
    }
}
