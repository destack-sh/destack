use destack_dir::{
    Asynchrony, FunctionRole, Keyword, LocalNodeId, NodeType, TokenType, TypeKind, TypeMember,
};
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

use super::super::Decorators;
use crate::parse::context::{ExpressionContext, FunctionContext, ParameterSpace, TypeContext};
use crate::parse::member::{Method, MethodContext, MethodRoleGrammar};
use crate::parse::{BindingModifiers, RecoveryPoint};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use std::mem;

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
    /// ```ds
    /// { name: string; read(): string }
    /// ```
    pub(crate) fn parse_type_object_literal(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<Vec<LocalNodeId<TypeMember>>> {
        self.eat_token(TokenType::OpenBrace)?;
        let members = self.parse_type_members(TypeMemberContainerKind::TypeLiteral, function)?;

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

        // abstract|override comptime const
        if probe.peek_keyword() != Some(Keyword::Comptime) {
            return false;
        }
        probe.bump();

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
        function: FunctionContext,
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
            .parse_generic_parameters_if_present(false, function)?
            .unwrap_or_default();
        let where_clauses = self.parse_where_clauses(function)?;

        // declared type
        let constraint = if self.peek_is(TokenType::Colon) {
            self.bump();

            let constraint = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::CloseBrace)
                || self.peek_any_stop()
            {
                self.recover_missing_type_expression_here(NodeType::TypeMember)
            } else {
                self.parse_member_type(function)?
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
                self.parse_member_type(function)?
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
        function: FunctionContext,
    ) -> ParserResult<Option<LocalNodeId<TypeMember>>> {
        if !self.peek_is_keyword(Keyword::Comptime)
            || self.peek_next_keyword() != Some(Keyword::Const)
        {
            return Ok(None);
        }

        // keyword and name
        self.bump();
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
                self.parse_member_type(function)?
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
                self.parse_expression(ExpressionContext {
                    function,
                    ..ExpressionContext::default()
                })?
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
        function: FunctionContext,
    ) -> LocalNodeId<TypeMember> {
        match self.parse_type_member(container_kind, function) {
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
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<TypeMember>> {
        let start = self.mark_parse_start();

        // plain fields
        if let Some(member_id) = self.parse_plain_type_field_member_if_present(&start, function)? {
            return Ok(member_id);
        }

        // eat associated member modifiers
        let associated_modifiers = self.parse_type_member_associated_modifiers();

        // parse associated members
        if container_kind.allows_associated_members() {
            if let Some(member_id) =
                self.parse_type_associated_type_if_present(&start, &associated_modifiers, function)?
            {
                return Ok(member_id);
            }
            if let Some(member_id) = self.parse_type_associated_const_if_present(
                &start,
                &associated_modifiers,
                function,
            )? {
                return Ok(member_id);
            }
        }

        if !associated_modifiers.is_empty() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

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
        let role = self.parse_method_role(MethodRoleGrammar::New);

        // index signature
        if self.peek_index_signature() {
            if is_abstract {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let key_start = self.mark_parse_start();
            self.eat_token(TokenType::OpenBracket)?;
            let (name, name_range) = self.eat_binding_identifier_with_range(function)?;
            self.eat_token(TokenType::Colon)?;

            let key_type = self.parse_type_or_recover_missing(
                TypeContext {
                    function,
                    ..TypeContext::default()
                },
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
                self.parse_method_return_type(NodeType::TypeMember, function)?
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

        // key
        let (key, key_range) =
            if matches!(role, Some(FunctionRole::Constructor | FunctionRole::New)) {
                (None, None)
            } else if let Some((key, range)) = self.eat_key_with_range_if_present(function)? {
                (Some(key), Some(range))
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

            if matches!(role, Some(FunctionRole::Getter | FunctionRole::Setter)) && key.is_none() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let role = if key.is_none() && role.is_none() {
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
                MethodContext {
                    enclosing_function: function,
                    space: ParameterSpace::Type,
                    asynchrony: Asynchrony::Sync,
                    is_generator: false,
                },
                is_abstract,
                false,
                container_kind.allows_body(),
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

            if let Some(range) = key_range {
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
        let Some(key) = key else {
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
                    self.parse_method_return_type(NodeType::TypeMember, function)?
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
        let member_id = self.insert_node(member, self.range_since(&start));

        if let Some(range) = key_range {
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
    /// ```ds
    /// name: string
    /// name?: string
    /// "kind": "ready"
    /// ```
    fn parse_plain_type_field_member_if_present(
        &mut self,
        start: &ParseStart,
        function: FunctionContext,
    ) -> ParserResult<Option<LocalNodeId<TypeMember>>> {
        if !self.peek_plain_type_field_member() {
            return Ok(None);
        }

        let (key, key_range) = self.eat_key_with_range(function)?;

        let optional_start = self.mark_parse_start();
        let is_optional = self.eat_token_if(TokenType::Maybe);
        let optional_range = is_optional.then(|| self.range_since(&optional_start));

        let type_start = self.mark_parse_start();
        self.eat_token(TokenType::Colon)?;
        let declared_type = if self.peek_is(TokenType::CloseBrace) || self.peek_any_stop() {
            self.recover_missing_type_expression_here(NodeType::TypeMember)
        } else {
            self.parse_method_return_type(NodeType::TypeMember, function)?
        };

        let member_id = self.insert_node(
            TypeMember::Field {
                is_static: false,
                is_optional,
                is_readonly: false,
                key,
                declared_type: Some(declared_type),
            },
            self.range_since(start),
        );
        self.tree.set_main_range(member_id, key_range);
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
        function: FunctionContext,
    ) -> ParserResult<Vec<LocalNodeId<TypeMember>>> {
        let mut members: Vec<LocalNodeId<TypeMember>> = Vec::new();
        let mut pending_member_decorators = Decorators::new();
        let mut previous_member_had_error = false;
        let recovery_point = RecoveryPoint::TypeMemberDeclaration(container_kind);

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // close the member list
            if matches!(token_type, TokenType::CloseBrace | TokenType::End) {
                if !pending_member_decorators.is_empty() {
                    let error = ParserError::unexpected(self.peek_token_span());
                    self.report_error(error);
                    pending_member_decorators.clear();
                }

                break;
            }
            // collect decorators for the next member
            else if token_type == TokenType::At {
                let decorators = self.parse_decorators(function);
                pending_member_decorators.extend(decorators);
                previous_member_had_error = false;

                continue;
            }
            // skip item separators
            else if token_type == TokenType::Comma {
                self.eat_token(TokenType::Comma)?;
                previous_member_had_error = false;

                continue;
            }
            // let a damaged member release the next declaration
            else if token_type == TokenType::Semicolon {
                if previous_member_had_error && self.peek_semicolon_recovery_point(recovery_point) {
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
            // let a damaged nested body release the next declaration
            else if previous_member_had_error && self.peek_recovery_point(recovery_point) {
                break;
            }

            let error_count = self.errors.len();
            let member_id = self.parse_type_member_or_recover(container_kind, function);
            previous_member_had_error = self.errors.len() > error_count
                || matches!(self.tree.get(member_id), TypeMember::Error);

            if !pending_member_decorators.is_empty() {
                self.attach_decorators(member_id.id, mem::take(&mut pending_member_decorators));
            }

            members.push(member_id);
        }

        Ok(members)
    }
}
