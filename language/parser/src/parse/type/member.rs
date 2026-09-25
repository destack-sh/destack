use crate::parse::{ExpressionPosition, ExpressionStop};
use tspp_dir::{
    Asynchrony, FunctionRole, Keyword, LocalNodeId, NodeType, TokenType, TypeKind, TypeMember,
};
use tspp_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

use crate::parse::member::Method;
use crate::parse::{
    BindingModifiers, DeclarationNesting, FunctionModifiers, TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};

/// The kind of type member container being parsed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeMemberContainerKind {
    /// A structural object type literal.
    TypeLiteral,
    /// A structural interface declaration.
    StructuralInterface,
    /// A nominal interface declaration.
    NominalInterface,
}

impl From<TypeKind> for TypeMemberContainerKind {
    /// Convert one interface kind into its member container kind.
    #[inline]
    fn from(kind: TypeKind) -> Self {
        match kind {
            TypeKind::Nominal => Self::NominalInterface,
            TypeKind::Structural => Self::StructuralInterface,
        }
    }
}

impl TypeMemberContainerKind {
    /// Return whether method bodies are accepted.
    #[inline]
    const fn allows_body(self) -> bool {
        matches!(self, Self::NominalInterface)
    }

    /// Return whether associated type and constant members are accepted.
    #[inline]
    pub(crate) const fn allows_associated_members(self) -> bool {
        matches!(self, Self::StructuralInterface | Self::NominalInterface)
    }
}

#[allow(clippy::type_complexity)]
impl Parser {
    /// Parse one object type literal (including the surrounding braces).
    ///
    /// Examples:
    /// ```tspp
    /// { name: string; read(): string }
    /// ```
    pub(crate) fn parse_type_object_literal(
        &mut self,
    ) -> ParserResult<Vec<LocalNodeId<TypeMember>>> {
        self.eat_token(TokenType::OpenBrace)?;
        let members = self.parse_type_members(TypeMemberContainerKind::TypeLiteral)?;

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;
        Ok(members)
    }

    /// Return true when the current `[` starts an index signature.
    #[inline]
    fn peek_index_signature(&self) -> bool {
        if !self.peek_is(TokenType::OpenBracket) {
            return false;
        }

        self.peek_token_type_at(1) == TokenType::Identifier
            && self.peek_token_type_at(2) == TokenType::Colon
    }

    /// Return whether the current type member modifiers introduce an associated member.
    fn peek_associated_type_member(&self) -> bool {
        if !matches!(
            self.peek_keyword(),
            Some(Keyword::Abstract | Keyword::Override)
        ) {
            return false;
        }

        let mut probe = self.cursor.probe(&self.file);
        while matches!(
            probe.peek_keyword(),
            Some(Keyword::Abstract | Keyword::Override)
        ) {
            probe.bump();
        }

        // abstract|override type
        if probe.peek_keyword() == Some(Keyword::Type) {
            return true;
        }

        // abstract|override const
        probe.peek_keyword() == Some(Keyword::Const)
    }

    /// Parse abstraction modifiers before an associated member.
    fn parse_type_member_associated_modifiers(&mut self) -> BindingModifiers {
        if !self.peek_associated_type_member() {
            return BindingModifiers::default();
        }

        let mut modifiers = BindingModifiers::default();

        loop {
            // parse each supported associated-member modifier
            match self.peek_keyword() {
                Some(Keyword::Abstract) => modifiers.is_abstract = true,
                Some(Keyword::Override) => modifiers.is_override = true,
                _ => break,
            }

            self.bump();
        }

        modifiers
    }

    /// Parse one associated type member when present.
    fn parse_type_associated_type_if_present(
        &mut self,
        start: &ParseStart,
        modifiers: &BindingModifiers,
    ) -> ParserResult<Option<LocalNodeId<TypeMember>>> {
        if !self.peek_is_keyword(Keyword::Type)
            || self.peek_next_token_type() != TokenType::Identifier
        {
            return Ok(None);
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
                self.recover_missing_type_expression_here(NodeType::TypeMember)
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
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.parse_member_type()?
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
            is_abstract: modifiers.is_abstract,
            is_override: modifiers.is_override,
        };
        let member_id = self.insert_node(member, self.range_since(start));
        self.tree.set_main_range(member_id, name_range);

        Ok(Some(member_id))
    }

    /// Parse one associated constant member when present.
    fn parse_type_associated_const_if_present(
        &mut self,
        start: &ParseStart,
        modifiers: &BindingModifiers,
    ) -> ParserResult<Option<LocalNodeId<TypeMember>>> {
        if !self.peek_is_keyword(Keyword::Const)
            || self.peek_next_token_type() != TokenType::Identifier
        {
            return Ok(None);
        }

        // keyword and name
        self.bump();
        let (name, name_range) = self.eat_identifier_with_range()?;

        // declared type
        let declared_type = if self.peek_is(TokenType::Colon) {
            self.bump();

            let declared_type = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.peek_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.parse_member_type()?
            };

            Some(declared_type)
        } else {
            None
        };

        // value
        let value = if self.peek_is(TokenType::Assign) {
            self.bump();

            let value = if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
                self.recover_missing_expression_here(NodeType::TypeMember)
            } else {
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?
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
            is_abstract: modifiers.is_abstract,
            is_override: modifiers.is_override,
        };
        let member_id = self.insert_node(member, self.range_since(start));
        self.tree.set_main_range(member_id, name_range);

        Ok(Some(member_id))
    }

    /// Parse one type member, recovering malformed input as an error node.
    pub(crate) fn parse_type_member_or_recover(
        &mut self,
        container_kind: TypeMemberContainerKind,
    ) -> LocalNodeId<TypeMember> {
        match self.parse_type_member(container_kind) {
            Ok(member_id) => member_id,
            Err(error) => {
                let error = error.in_node(NodeType::TypeMember);
                let recovered_range = self.recover_body(error.range(), error);

                self.insert_node(TypeMember::Error, recovered_range)
            }
        }
    }

    /// Parse a type member.
    pub(crate) fn parse_type_member(
        &mut self,
        container_kind: TypeMemberContainerKind,
    ) -> ParserResult<LocalNodeId<TypeMember>> {
        let start = self.mark_parse_start();

        // plain fields
        if let Some(member_id) = self.parse_plain_type_field_member_if_present(&start)? {
            return Ok(member_id);
        }

        // eat associated member modifiers
        let associated_modifiers = self.parse_type_member_associated_modifiers();

        // parse associated members
        if container_kind.allows_associated_members() {
            if let Some(member_id) =
                self.parse_type_associated_type_if_present(&start, &associated_modifiers)?
            {
                return Ok(member_id);
            }
            if let Some(member_id) =
                self.parse_type_associated_const_if_present(&start, &associated_modifiers)?
            {
                return Ok(member_id);
            }
        }

        if !associated_modifiers.is_empty() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // read a visibility keyword only ahead of a same-line member name,
        //  keeping a bare keyword name as the member itself
        let visibility = if self.peek_next_same_line_member_name() {
            self.parse_visibility_if_present()
        } else {
            None
        };

        // static
        let is_static = if self.peek_is_keyword(Keyword::Static) && self.peek_next_member_name() {
            self.bump();
            true
        } else {
            false
        };

        // readonly
        let is_readonly =
            if self.peek_is_keyword(Keyword::Readonly) && self.peek_next_same_line_member_name() {
                self.bump();
                true
            } else {
                false
            };

        // abstract
        let is_abstract = if self.peek_is_keyword(Keyword::Abstract) {
            let peek_next_token = self.peek_next_token();
            let abstract_is_modifier = !peek_next_token.is_on_new_line()
                && matches!(
                    peek_next_token.ty(),
                    TokenType::Identifier
                        | TokenType::Literal
                        | TokenType::Hash
                        | TokenType::OpenBracket
                        | TokenType::OpenParenthesis
                        | TokenType::LessThan
                );

            if abstract_is_modifier {
                self.bump();
                true
            } else {
                false
            }
        } else {
            false
        };

        // role
        let parsed_role = self.parse_method_role(Some(FunctionRole::New));
        let (role, role_range) = match parsed_role {
            Some((role, range)) => (Some(role), Some(range)),
            None => (None, None),
        };

        // index signature
        if self.peek_index_signature() {
            if is_abstract {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let key_start = self.mark_parse_start();
            self.eat_token(TokenType::OpenBracket)?;
            let (name, name_range) = self.eat_binding_identifier_with_range()?;
            self.eat_token(TokenType::Colon)?;

            let key_type = self.parse_type_or_recover_missing(
                TypePosition::Type,
                TypeStop::default(),
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
            let optional_start = self.mark_parse_start();
            let is_optional = self.eat_token_if(TokenType::Maybe);
            let optional_range = is_optional.then(|| self.range_since(&optional_start));

            let type_start = self.mark_parse_start();
            self.eat_token(TokenType::Colon)?;

            let value_type = if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.parse_method_return_type(NodeType::TypeMember)?
            };

            let member = TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
            };
            let member_id = self.insert_node(member, self.range_since(&start));

            self.tree.set_main_range(member_id, name_range);
            self.tree
                .set_head_range(member_id, self.range_since(&key_start));
            self.tree.set_side_range(
                member_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                self.range_since(&type_start),
            );

            if let Some(range) = optional_range {
                self.tree.set_side_range(
                    member_id,
                    NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                    range,
                );
            }

            return Ok(member_id);
        }

        // name
        let (name, name_range) =
            if matches!(role, Some(FunctionRole::Constructor | FunctionRole::New)) {
                (None, None)
            } else if let Some((name, range)) = self.eat_property_name_with_range_if_present()? {
                (Some(name), Some(range))
            } else {
                (None, None)
            };

        // optional
        let optional_start = self.mark_parse_start();
        let is_optional = self.eat_token_if(TokenType::Maybe);
        let optional_range = is_optional.then(|| self.range_since(&optional_start));

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
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && name.is_none() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let role = if name.is_none() && role.is_none() {
                Some(FunctionRole::Call)
            } else {
                role
            };

            let Method {
                signature,
                generic_parameter_range,
                parameter_range,
                body,
                return_type_range,
            } = self.parse_method(
                NodeType::TypeMember,
                role,
                FunctionModifiers {
                    asynchrony: Asynchrony::Sync,
                    is_generator: false,
                },
                is_abstract,
                false,
                container_kind.allows_body(),
            )?;

            let member = match (name, role) {
                (Some(name), _) => TypeMember::Method {
                    is_static,
                    is_optional,
                    visibility,
                    name,
                    signature,
                    body,
                },
                (None, Some(FunctionRole::New | FunctionRole::Constructor)) => {
                    TypeMember::ConstructSignature {
                        signature: signature.into_constructor_type(),
                    }
                }
                (None, None | Some(FunctionRole::Call)) => TypeMember::CallSignature {
                    signature: signature.into_function_type(),
                },
                (None, Some(FunctionRole::Getter | FunctionRole::Setter)) => {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }
            };
            let member_id = self.insert_node(member, self.range_since(&start));

            if let Some(range) = name_range.or(role_range) {
                self.tree.set_main_range(member_id, range);
            }

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

            if let Some(range) = optional_range {
                self.tree.set_side_range(
                    member_id,
                    NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                    range,
                );
            }

            return Ok(member_id);
        }

        // field
        let Some(name) = name else {
            return Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::Identifier,
            ));
        };

        if is_abstract {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        let type_start = self.mark_parse_start();
        let declared_type = if self.eat_token_if(TokenType::Colon) {
            Some(
                if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
                    self.recover_missing_type_expression_here(NodeType::TypeMember)
                } else {
                    self.parse_method_return_type(NodeType::TypeMember)?
                },
            )
        } else {
            None
        };
        let member = TypeMember::Field {
            is_static,
            is_optional,
            is_readonly,
            visibility,
            name,
            declared_type,
        };
        let member_id = self.insert_node(member, self.range_since(&start));

        if let Some(range) = name_range {
            self.tree.set_main_range(member_id, range);
        }

        if declared_type.is_some() {
            self.tree.set_side_range(
                member_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                self.range_since(&type_start),
            );
        }

        if let Some(range) = optional_range {
            self.tree.set_side_range(
                member_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                range,
            );
        }

        Ok(member_id)
    }

    /// Parse a plain type field member when the head is unambiguous.
    ///
    /// Examples:
    /// ```tspp
    /// name: string
    /// name?: string
    /// "kind": "ready"
    /// ```
    fn parse_plain_type_field_member_if_present(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<Option<LocalNodeId<TypeMember>>> {
        if !self.peek_plain_type_field_member() {
            return Ok(None);
        }

        let (name, name_range) = self.eat_property_name_with_range()?;

        let optional_start = self.mark_parse_start();
        let is_optional = self.eat_token_if(TokenType::Maybe);
        let optional_range = is_optional.then(|| self.range_since(&optional_start));

        let type_start = self.mark_parse_start();
        self.eat_token(TokenType::Colon)?;
        let declared_type = if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
            self.recover_missing_type_expression_here(NodeType::TypeMember)
        } else {
            self.parse_method_return_type(NodeType::TypeMember)?
        };

        let member_id = self.insert_node(
            TypeMember::Field {
                is_static: false,
                is_optional,
                is_readonly: false,
                visibility: None,
                name,
                declared_type: Some(declared_type),
            },
            self.range_since(start),
        );
        self.tree.set_main_range(member_id, name_range);
        self.tree.set_side_range(
            member_id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.range_since(&type_start),
        );

        if let Some(range) = optional_range {
            self.tree.set_side_range(
                member_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
                range,
            );
        }

        Ok(Some(member_id))
    }

    /// Return whether the current member is a plain field.
    fn peek_plain_type_field_member(&self) -> bool {
        if !matches!(
            self.peek_token_type(),
            TokenType::Identifier | TokenType::Literal
        ) {
            return false;
        }

        let peek_next_token_type = self.peek_token_type_at(1);
        if peek_next_token_type == TokenType::Colon {
            return true;
        }

        peek_next_token_type == TokenType::Maybe && self.peek_token_type_at(2) == TokenType::Colon
    }

    /// Parse type members inside one object type body.
    pub(crate) fn parse_type_members(
        &mut self,
        container_kind: TypeMemberContainerKind,
    ) -> ParserResult<Vec<LocalNodeId<TypeMember>>> {
        let mut members: Vec<LocalNodeId<TypeMember>> = Vec::new();
        let mut is_previous_member_damaged = false;
        let declaration_nesting = if container_kind.allows_associated_members() {
            DeclarationNesting::Member
        } else {
            DeclarationNesting::None
        };

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // close the member list
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                break;
            }
            // skip item separators
            else if token_type == TokenType::Comma {
                self.eat_token(TokenType::Comma)?;
                is_previous_member_damaged = false;

                continue;
            }
            // let a damaged member release the next declaration
            else if token_type == TokenType::Semicolon {
                if is_previous_member_damaged
                    && self.peek_semicolon_declaration_boundary(declaration_nesting)
                {
                    break;
                }

                self.eat_any_stop()?;
                is_previous_member_damaged = false;

                continue;
            }
            // skip statement separators
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop()?;
                is_previous_member_damaged = false;

                continue;
            }
            // let a damaged nested body release the next declaration
            else if is_previous_member_damaged
                && self.peek_declaration_boundary(declaration_nesting)
            {
                break;
            }

            // parse documentation, decorators and one member
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
            let member_id = self.parse_type_member_or_recover(container_kind);
            is_previous_member_damaged = self.errors.len() > error_count
                || matches!(self.tree.get(member_id), TypeMember::Error);

            if !matches!(self.tree.get(member_id), TypeMember::Error) {
                self.attach_documentation(member_id, documentation);
            }
            self.attach_decorators(member_id.id, decorators);

            members.push(member_id);
        }

        Ok(members)
    }
}
