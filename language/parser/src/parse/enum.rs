use super::Decorators;
use crate::parse::DeclarationHeader;
use crate::parse::context::{ExpressionContext, FunctionContext};
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    Declaration, EnumDeclaration, EnumField, EnumKind, Keyword, LocalNodeId, Member, Name,
    NodeType, TemplateLiteral, TokenLiteral, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};
use std::mem;

/// The fields and members of one enum body.
struct EnumBody {
    /// The enum value fields.
    fields: Vec<LocalNodeId<EnumField>>,
    /// The associated enum members.
    members: Vec<LocalNodeId<Member>>,
}

impl Parser {
    /// Parse one enum declaration.
    ///
    /// Examples:
    /// ```ds
    /// enum Result<T, E> { Ok(T); Error(E) }
    /// ```
    pub(crate) fn parse_enum(
        &mut self,
        start: &ParseStart,
        kind: EnumKind,
        header: DeclarationHeader,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // enum
        let enum_range = self.eat_keyword(Keyword::Enum)?.span.range();

        // require declaration heads on one line
        if self.peek_is_on_new_line() && self.peek_is(TokenType::Identifier) {
            let error = ParserError::unexpected(enum_range);
            self.report_error(error);
            return Err(error);
        }

        // enum Name
        let (name, name_range) =
            if let Some((name, range)) = self.eat_name_with_range_if_present()? {
                (Some(name), Some(range))
            } else {
                (None, None)
            };

        // <parameters>
        let generic_parameter_container_start = self.mark_parse_start();
        let generic_parameters = self
            .parse_generic_parameters_if_present(false, function)
            .in_node(NodeType::Declaration)?;
        let generic_parameter_container_range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&generic_parameter_container_start));

        // extends Base
        let extends_types = self
            .parse_extends_types_if_present(function)
            .in_node(NodeType::Declaration)?;

        // implements Trait
        let implements_types = self
            .parse_implements_types_if_present(function)
            .in_node(NodeType::Declaration)?;

        // where constraints
        let where_clauses = self
            .parse_where_clauses(function)
            .in_node(NodeType::Declaration)?;

        // { fields and members }
        self.eat_token(TokenType::OpenBrace)
            .in_node(NodeType::Declaration)?;
        let body = self
            .parse_enum_body(function)
            .in_node(NodeType::Declaration)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let enum_id = self.insert_node(
            Declaration::Enum(EnumDeclaration {
                name,
                export: header.export,
                place: header.place,
                is_ambient: header.is_ambient,
                kind,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                implements_types: implements_types.or(extends_types).unwrap_or_default(),
                fields: body.fields,
                members: body.members,
            }),
            self.range_since(start),
        );

        // set the name identifier as the main source range
        if let Some(range) = name_range {
            self.tree.set_main_range(enum_id, range);
        }
        if let Some(range) = generic_parameter_container_range {
            self.tree.set_side_range(
                enum_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        Ok(enum_id)
    }

    /// Parse one enum body without its delimiters.
    fn parse_enum_body(&mut self, function: FunctionContext) -> ParserResult<EnumBody> {
        // collect fields and associated members
        let mut fields: Vec<LocalNodeId<EnumField>> = Vec::new();
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut pending_decorators = Decorators::new();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                if !pending_decorators.is_empty() {
                    let error = ParserError::unexpected(self.peek_token_span());
                    self.report_error(error);
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
                let decorators = self.parse_decorators(function);
                pending_decorators.extend(decorators);
            }
            // enum field
            else if self.peek_enum_field() {
                let field = self
                    .parse_enum_field(function)
                    .in_node(NodeType::EnumField)?;
                if !pending_decorators.is_empty() {
                    self.attach_decorators(field.id, mem::take(&mut pending_decorators));
                }
                fields.push(field);
            }
            // (static) members
            else {
                let member_id = self.parse_member_or_recover(function);
                if !pending_decorators.is_empty() {
                    self.attach_decorators(member_id.id, mem::take(&mut pending_decorators));
                }
                members.push(member_id);
            }
        }

        Ok(EnumBody { fields, members })
    }

    /// Return true when the next tokens can start an enum field.
    #[inline]
    fn peek_enum_field(&self) -> bool {
        let is_computed_name = self.peek_is(TokenType::OpenBracket);
        let is_bare_name = (self.peek_name_start() || self.peek_numeric_literal_start())
            && (self.peek_next_token().is_on_new_line()
                || matches!(
                    self.peek_token_type_at(1),
                    TokenType::Assign
                        | TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseBrace
                ));

        is_computed_name || is_bare_name
    }

    /// Parse one enum field.
    fn parse_enum_field(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<EnumField>> {
        let start = self.mark_parse_start();
        let (name, name_range) = self
            .eat_enum_field_name_with_range(function)
            .in_node(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_is(TokenType::Assign) {
            self.eat_token(TokenType::Assign)?;
            let value = self.parse_expression_or_recover_missing(
                ExpressionContext {
                    function,
                    ..ExpressionContext::default()
                },
                NodeType::EnumField,
            )?;
            Some(value)
        } else {
            None
        };

        let field_id = self.insert_node(EnumField { name, value }, self.range_since(&start));

        // set the main source range to the name identifier
        self.tree.set_main_range(field_id, name_range);
        Ok(field_id)
    }

    /// Eat an enum field name, including computed string/number names.
    fn eat_enum_field_name_with_range(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<(Name, ByteRange)> {
        if self.peek_is(TokenType::OpenBracket) {
            let start = self.mark_parse_start();
            self.bump();

            let name = if self.peek_is(TokenType::Literal)
                && matches!(
                    self.peek_token_span().token.literal(),
                    Some(TokenLiteral::String { .. })
                ) {
                let token = self.peek_token_span();
                let content = self.string_literal_str(token).to_owned();
                let string_id = self.strings.intern(&content);
                self.bump();
                Name::String(string_id)
            } else if self.peek_numeric_literal_start() {
                let (index, _) = self.eat_index_key_with_range()?;
                Name::Index(index)
            } else if self.peek_is(TokenType::TemplateString) {
                let template = self.parse_template_literal(function)?;
                match template {
                    TemplateLiteral::String { string } => Name::String(string),
                    TemplateLiteral::InterpolatedString { .. } => {
                        return Err(ParserError::unexpected(self.peek_token_span()));
                    }
                }
            } else {
                return Err(ParserError::unexpected(self.peek_token_span()));
            };

            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
            Ok((name, self.range_since(&start)))
        } else if self.peek_numeric_literal_start() {
            let (index, range) = self.eat_index_key_with_range()?;
            Ok((Name::Index(index), range))
        } else {
            self.eat_name_with_range()
        }
    }
}
