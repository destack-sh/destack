use super::PendingDecorators;
use crate::parse::DeclarationHeader;
use crate::parse::flags::ParserFlags;
use crate::parse::prelude::*;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Declaration, EnumDeclaration, EnumField, EnumKind, Keyword, LocalNodeId, Member, Name,
    NodeType, TemplateLiteral, TokenLiteral, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Eat an enum declaration.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum _ {} // explicit anonymous enum (for disambiguation)
    ///
    /// enum Foo {
    ///     A // semicolon optional
    ///     B
    ///     C
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// enum Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    ///
    /// enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    ///     A = 1
    ///     B = T
    ///     @if(IsSomething)
    ///     C = 3
    /// }
    /// ```
    pub(crate) fn eat_enum(
        &mut self,
        start: &ParserSpanStart,
        kind: EnumKind,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // keyword
        let enum_span = self.eat_keyword(Keyword::Enum)?.span;

        // require declaration heads on one line
        if self.current_token_is_on_new_line() && self.peek_is(TokenType::Identifier) {
            let error = ParserError::unexpected(enum_span);
            self.error(&error);
            return Err(error);
        }

        // optional name
        let (name, name_span) = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            (Some(name), Some(span))
        } else {
            (None, None)
        };

        // optional generic parameters: < ... >
        let generic_parameter_container_start = self.span_start();
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)
            .for_node_type(NodeType::Declaration)?;
        let generic_parameter_container_span = generic_parameters
            .as_ref()
            .map(|_| self.get_span_from(&generic_parameter_container_start));

        // optional extends types
        let extends_types = self
            .eat_extends_types_if_present()
            .for_node_type(NodeType::Declaration)?;

        // optional implements types
        let implements_types = self
            .eat_implements_types_if_present()
            .for_node_type(NodeType::Declaration)?;

        // where
        let where_clauses = self
            .eat_where_maybe()
            .for_node_type(NodeType::Declaration)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        let (fields, members) = self.eat_enum_body().for_node_type(NodeType::Declaration)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let enum_id = self.insert_node(
            Declaration::Enum(EnumDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                kind,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                implements_types: implements_types.or(extends_types).unwrap_or_default(),
                fields,
                members,
            }),
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(enum_id, span);
        }
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                enum_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(enum_id)
    }

    /// Return parser flags for enum members.
    #[inline]
    fn enum_member_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.nested().with_variant(true);
        let expression_context = self.flags.nested();

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    #[allow(clippy::type_complexity)]
    fn eat_enum_body(
        &mut self,
    ) -> ParserResult<(Vec<LocalNodeId<EnumField>>, Vec<LocalNodeId<Member>>)> {
        // eat everything
        let mut fields: Vec<LocalNodeId<EnumField>> = Vec::new();
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut pending_decorators = PendingDecorators::new();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                if !pending_decorators.is_empty() {
                    let error = ParserError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_decorators.clear();
                }

                break;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop()?;
            }
            // consume decorator prefixes
            else if token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_decorators.extend(decorators);
            }
            // enum field
            else if self.peek_enum_field_is() {
                let field = self.eat_enum_field().for_node_type(NodeType::EnumField)?;
                if !pending_decorators.is_empty() {
                    self.attach_decorators(field.id, std::mem::take(&mut pending_decorators));
                }
                fields.push(field);
            }
            // (static) members
            else {
                let member_id =
                    self.with_flags(self.enum_member_flags(), |parser| parser.try_eat_member())?;
                if !pending_decorators.is_empty() {
                    self.attach_decorators(member_id.id, std::mem::take(&mut pending_decorators));
                }
                members.push(member_id);
            }
        }

        Ok((fields, members))
    }

    /// Return true when the next tokens can start an enum field.
    #[inline]
    fn peek_enum_field_is(&mut self) -> bool {
        let is_computed_name = self.peek_is(TokenType::OpenBracket);
        let is_bare_name = (self.peek_name_is() || self.peek_numeric_literal_is())
            && (self.next_token().token.is_on_new_line()
                || matches!(
                    self.token_type_at_offset(1),
                    TokenType::Assign
                        | TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseBrace
                ));

        is_computed_name || is_bare_name
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParserResult<LocalNodeId<EnumField>> {
        let start = self.span_start();
        let (name, name_span) = self
            .eat_enum_field_name_with_span()
            .for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_is(TokenType::Assign) {
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression_or_recover_missing(
                self.flags.not_in_position().not_in_sequence_expression(),
                NodeType::EnumField,
            )?;
            Some(value)
        } else {
            None
        };

        let field_id = self
            .tree
            .insert(EnumField { name, value }, self.get_span_from(&start));

        // set main span to the name identifier
        self.tree.set_main_span(field_id, name_span);
        Ok(field_id)
    }

    /// Eat an enum field name, including computed string/number names.
    fn eat_enum_field_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        if self.peek_is(TokenType::OpenBracket) {
            let start = self.span_start();
            self.bump(); // eat open bracket

            let name = if self.peek_is(TokenType::Literal)
                && matches!(
                    self.peek()?.token.literal(),
                    Some(TokenLiteral::String { .. })
                ) {
                let token = *self.peek()?;
                let content = self.get_string_literal_str(token).to_owned();
                let string_id = self.strings.intern(&content);
                self.bump();
                Name::String(string_id)
            } else if self.peek_numeric_literal_is() {
                let (index, _) = self.eat_index_key_with_span()?;
                Name::Index(index)
            } else if self.peek_is(TokenType::TemplateString) {
                let template = self.eat_template_literal()?;
                match template {
                    TemplateLiteral::String { string } => Name::String(string),
                    TemplateLiteral::InterpolatedString { .. } => {
                        return Err(ParserError::unexpected(self.peek()?.span));
                    }
                }
            } else {
                return Err(ParserError::unexpected(self.peek()?.span));
            };

            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
            Ok((name, self.get_span_from(&start)))
        } else if self.peek_numeric_literal_is() {
            let (index, span) = self.eat_index_key_with_span()?;
            Ok((Name::Index(index), span))
        } else {
            self.eat_name_with_span()
        }
    }
}
