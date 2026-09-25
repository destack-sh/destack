use crate::lex::{InvalidEscape, cook};
use crate::parse::error::ParserResultExt;
use crate::parse::{DeclarationHeader, DeclarationNesting, ExpressionPosition, ExpressionStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use tspp_dir::{
    Declaration, EnumDeclaration, EnumField, Keyword, LocalNodeId, Member, Name, NodeType,
    TemplateLiteral, TokenLiteral, TokenType,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

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
    /// ```tspp
    /// enum Result<T, E> { Ok(T); Error(E) }
    /// ```
    pub(crate) fn parse_enum(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // enum
        let enum_range = self.eat_keyword(Keyword::Enum)?.range();

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
            .parse_generic_parameters_if_present(false)
            .in_node(NodeType::Declaration)?;
        let generic_parameter_container_range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&generic_parameter_container_start));

        // reject an extends clause
        self.skip_rejected_extends()?;

        // implements Trait
        let implements_types = self
            .parse_implements_types_if_present()
            .in_node(NodeType::Declaration)?;

        // where constraints
        let where_clauses = self.parse_where_clauses().in_node(NodeType::Declaration)?;

        // { fields and members }
        self.eat_token(TokenType::OpenBrace)
            .in_node(NodeType::Declaration)?;
        let body = self.parse_enum_body().in_node(NodeType::Declaration)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let enum_id = self.insert_node(
            Declaration::Enum(EnumDeclaration {
                name,
                export: header.export,
                is_shared: header.is_shared,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                implements_types: implements_types.unwrap_or_default(),
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
    fn parse_enum_body(&mut self) -> ParserResult<EnumBody> {
        // collect fields and associated members
        let mut fields: Vec<LocalNodeId<EnumField>> = Vec::new();
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut is_previous_item_damaged = false;

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                break;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                if token_type == TokenType::Semicolon
                    && is_previous_item_damaged
                    && self.peek_semicolon_declaration_boundary(DeclarationNesting::Member)
                {
                    break;
                }

                self.eat_any_stop()?;
                is_previous_item_damaged = false;
            }
            // release a declaration after a damaged item
            else if is_previous_item_damaged
                && self.peek_declaration_boundary(DeclarationNesting::Member)
            {
                break;
            }
            // parse documentation, decorators and one field or member
            else {
                let documentation = self.parse_documentation();
                let decorators = self.parse_decorators();

                // reject decorator prefixes without an owner
                if !decorators.is_empty() && self.peek_is(TokenType::CloseBrace) {
                    let error = ParserError::unexpected(self.peek_token_span());
                    self.report_error(error);

                    break;
                }

                let error_count = self.errors.len();

                // enum field
                if self.peek_enum_field() {
                    let field = self.parse_enum_field().in_node(NodeType::EnumField)?;
                    self.attach_documentation(field, documentation);
                    self.attach_decorators(field.id, decorators);
                    fields.push(field);
                    is_previous_item_damaged = self.errors.len() > error_count;
                }
                // associated member
                else {
                    let member_id = self.parse_member_or_recover();
                    is_previous_item_damaged = self.errors.len() > error_count
                        || matches!(self.tree.get(member_id), Member::Error);

                    if !matches!(self.tree.get(member_id), Member::Error) {
                        self.attach_documentation(member_id, documentation);
                    }
                    self.attach_decorators(member_id.id, decorators);
                    members.push(member_id);
                }
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
    fn parse_enum_field(&mut self) -> ParserResult<LocalNodeId<EnumField>> {
        let start = self.mark_parse_start();
        let (name, name_range) = self
            .eat_enum_field_name_with_range()
            .in_node(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_is(TokenType::Assign) {
            self.eat_token(TokenType::Assign)?;
            let value = self.parse_expression_or_recover_missing(
                ExpressionPosition::Value,
                ExpressionStop::default(),
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
    fn eat_enum_field_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        if self.peek_is(TokenType::OpenBracket) {
            let start = self.mark_parse_start();
            self.bump();

            let name = if self.peek_is(TokenType::Literal)
                && matches!(
                    self.peek_token().literal(),
                    Some(TokenLiteral::String { .. })
                ) {
                let token = self.peek_token();
                let content = cook(self.string_literal_str(token))
                    .map_err(|InvalidEscape| ParserError::expected(token, TokenType::Literal))?;
                let string_id = self.strings.intern(&content);
                self.bump();
                Name::String(string_id)
            } else if self.peek_numeric_literal_start() {
                let (index, _) = self.eat_index_name_with_range()?;
                Name::Index(index)
            } else if self.peek_is(TokenType::TemplateString) {
                let template = self.parse_template_literal()?;
                match template {
                    TemplateLiteral::String { chunk } => {
                        let cooked = chunk
                            .cooked
                            .ok_or_else(|| ParserError::unexpected(self.peek_token_span()))?;

                        Name::String(cooked)
                    }
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
            let (index, range) = self.eat_index_name_with_range()?;
            Ok((Name::Index(index), range))
        } else {
            self.eat_name_with_range()
        }
    }
}
