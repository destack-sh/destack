use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    Expression, LocalNodeId, Name, NodeType, OperatorPrecedence, Pattern, PatternField, RangeEnd,
    ScalarLiteral, TokenLiteral, TokenType, TypeExpression, TypeLiteral,
};
use destack_source::Span;

impl Parser {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// 1
    /// 2 | 3
    /// (x, 0, ...)
    /// { a: 2 }
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ...] }
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<LocalNodeId<Pattern>> {
        let start = self.span_start();

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let mut pattern_id = {
            // startless range pattern
            if self.language.is_destack()
                && matches!(
                    self.peek_token_type(),
                    TokenType::Range | TokenType::RangeInclusive
                )
            {
                self.eat_startless_range_pattern(&start)?
            }
            // wildcard
            else if self.language.is_destack() && self.peek_identifier_str_is("_") {
                self.bump(); // eat wildcard
                self.tree
                    .insert(Pattern::Wildcard, self.get_span_from(&start))
            }
            // reference of
            else if self.peek_is(TokenType::ElementwiseAnd) {
                self.bump(); // eat &
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::BorrowOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // value of
            else if self.peek_is(TokenType::ElementwiseXor) {
                self.bump(); // eat ^
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::MoveOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // tuple (without type, no struct tuples)
            else if self.peek_is(TokenType::OpenParenthesis) {
                self.bump(); // eat open parenthesis
                let field_flags = self.flags.nested();
                let fields_result = self.with_flags(field_flags, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                })?;
                let fields = fields_result;
                let pattern = Pattern::Tuple { fields };
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseParenthesis,
                    NodeType::Pattern,
                )?;
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // struct (without type)
            else if self.peek_is(TokenType::OpenBrace) {
                self.bump(); // eat open brace
                let field_flags = self.flags.nested();
                let fields_result = self.with_flags(field_flags, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                })?;
                let fields = fields_result;
                let pattern = Pattern::Object { fields };
                self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Pattern)?;
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // array or slice
            else if self.peek_is(TokenType::OpenBracket) {
                self.bump(); // eat open bracket
                let field_flags = self.flags.nested();
                let fields_result = self.with_flags(field_flags, |parser| {
                    parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBracket)
                })?;
                let fields = fields_result;
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseBracket,
                    NodeType::Pattern,
                )?;
                self.tree
                    .insert(Pattern::Sequence { fields }, self.get_span_from(&start))
            }
            // literal expression
            else if self.is_scalar_literal_start() {
                let scalar_literal_id =
                    self.eat_scalar_literal().for_node_type(NodeType::Pattern)?;
                let expression_id = self.insert_node(
                    Expression::ScalarLiteral(scalar_literal_id),
                    self.get_span_from(&start),
                );
                if let Some(end_kind) = self.peek_range_pattern_end_kind() {
                    self.eat_range_pattern_with_start(&start, expression_id, end_kind)?
                } else {
                    self.insert_node(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(&start),
                    )
                }
            }
            // null and undefined literals
            else if self.peek_identifier_str_is("null")
                || self.peek_identifier_str_is("undefined")
            {
                let is_null = self.peek_identifier_str_is("null");
                self.bump(); // eat literal identifier

                let pattern = if is_null {
                    let expression_id = self.tree.insert(
                        Expression::ScalarLiteral(ScalarLiteral::Null),
                        self.get_span_from(&start),
                    );
                    Pattern::Expression {
                        value: expression_id,
                    }
                } else {
                    let expression_id = self.tree.insert(
                        TypeExpression::Literal {
                            value: TypeLiteral::Undefined,
                        },
                        self.get_span_from(&start),
                    );
                    Pattern::TypeExpression {
                        value: expression_id,
                    }
                };
                self.insert_node(pattern, self.get_span_from(&start))
            }
            // binding with expression or pattern
            else if !self.flags.is_in_before_type()
                && self.peek_identifier_is()
                && self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Colon)
                })
            {
                let (name, name_span) = self.eat_binding_identifier_with_span()?;
                self.bump(); // eat colon
                let inner_pattern_flags = self.flags.in_static().in_before_block();
                let inner_pattern_result =
                    self.with_flags(inner_pattern_flags, |parser| parser.eat_pattern())?;
                let inner_pattern_id = inner_pattern_result;
                let pattern_id = self.insert_node(
                    Pattern::Binding {
                        name,
                        pattern: Some(inner_pattern_id),
                    },
                    self.get_span_from(&start),
                );
                self.tree.set_main_span(pattern_id, name_span);
                pattern_id
            }
            // path or identifier
            else {
                let (path, segment_spans, last_span) = self
                    .eat_path_with_endpoint_spans()
                    .for_node_type(NodeType::Pattern)?;
                let range_end_kind = self.peek_range_pattern_end_kind();
                // tuple with path
                if self.peek_is(TokenType::OpenParenthesis) {
                    self.bump(); // eat open parenthesis
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Pattern)?;
                    let expression_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    let pattern = Pattern::TaggedTuple {
                        ty: expression_id,
                        fields,
                    };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseParenthesis,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // struct with path
                else if !self.flags.is_in_before_block() && self.peek_is(TokenType::OpenBrace) {
                    self.bump(); // eat open brace
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                        .for_node_type(NodeType::Pattern)?;
                    let ty_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    let pattern = Pattern::TaggedObject { ty: ty_id, fields };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseBrace,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // range with path or identifier start
                else if let Some(end_kind) = range_end_kind {
                    let expression_id = if path.segments.len() > 1 {
                        let expression_id = self.insert_node(
                            Expression::QualifiedReference {
                                path,
                                generic_arguments: vec![],
                            },
                            self.get_span_from(&start),
                        );
                        self.set_path_expression_spans(expression_id, &segment_spans);
                        expression_id
                    } else {
                        let expression_id = self.insert_node(
                            Expression::Identifier {
                                name: path.segments[0],
                            },
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, last_span);
                        expression_id
                    };

                    self.eat_range_pattern_with_start(&start, expression_id, end_kind)?
                }
                // path
                else if path.segments.len() > 1 {
                    let expression_id = self.insert_node(
                        Expression::QualifiedReference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.get_span_from(&start),
                    );
                    self.set_path_expression_spans(expression_id, &segment_spans);
                    self.insert_node(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(&start),
                    )
                }
                // nullish literals in patterns
                else if path.segments[0] == self.state.type_literal_identifiers.null_
                    || path.segments[0] == self.state.type_literal_identifiers.undefined
                {
                    let is_null = path.segments[0] == self.state.type_literal_identifiers.null_;
                    let pattern = if is_null {
                        let expression_id = self.insert_node(
                            Expression::ScalarLiteral(ScalarLiteral::Null),
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, last_span);
                        Pattern::Expression {
                            value: expression_id,
                        }
                    } else {
                        let expression_id = self.insert_node(
                            TypeExpression::Literal {
                                value: TypeLiteral::Undefined,
                            },
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, last_span);
                        Pattern::TypeExpression {
                            value: expression_id,
                        }
                    };
                    self.insert_node(pattern, self.get_span_from(&start))
                }
                // identifier
                else {
                    let pattern_id = self.insert_node(
                        Pattern::Binding {
                            name: path.segments[0],
                            pattern: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(pattern_id, last_span);
                    pattern_id
                }
            }
        };

        // ------------------------------------------------------------
        // Postfix / infix patterns
        // ------------------------------------------------------------

        // must
        if self.peek_is(TokenType::Not) {
            self.bump(); // eat !
            let pattern = Pattern::Must(pattern_id);
            pattern_id = self.insert_node(pattern, self.get_span_from(&start));
        }
        // union
        if self.peek_is(TokenType::ElementwiseOr) && !self.flags.is_in_union_pattern() {
            // eat all union "fields" (just unnamed patterns)
            let mut patterns: Vec<LocalNodeId<Pattern>> = vec![pattern_id];
            while self.peek_is(TokenType::ElementwiseOr) {
                self.bump(); // eat '|'
                let union_flags = self.flags.in_union_pattern();
                let field_pattern_result =
                    self.with_flags(union_flags, |parser| parser.eat_pattern())?;
                let field_pattern_id = field_pattern_result;
                patterns.push(field_pattern_id);
            }
            let pattern = Pattern::Union { patterns };
            let pattern_id = self.insert_node(pattern, self.get_span_from(&start));
            Ok(pattern_id)
        }
        // no infix
        else {
            Ok(pattern_id)
        }
    }

    /// Return the range end kind when the next token continues a pattern range.
    fn peek_range_pattern_end_kind(&mut self) -> Option<RangeEnd> {
        if !self.language.is_destack() || self.current_token_is_on_new_line() {
            return None;
        }

        match self.peek_token_type() {
            TokenType::Range => Some(RangeEnd::Open),
            TokenType::RangeInclusive => Some(RangeEnd::Inclusive),
            _ => None,
        }
    }

    /// Return whether the current range pattern end is omitted.
    fn range_pattern_end_is_omitted(&mut self) -> bool {
        if self.current_token_is_on_new_line() {
            return true;
        }

        matches!(
            self.peek_token_type(),
            TokenType::ArrowWide | TokenType::ElementwiseOr
        ) || Self::is_expression_slot_boundary_token(self.peek_token_type())
    }

    /// Eat one range pattern endpoint after a range operator.
    fn eat_range_pattern_end_maybe(
        &mut self,
        end_kind: RangeEnd,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let is_omitted = self.range_pattern_end_is_omitted();

        // open-ended ranges may omit the right endpoint
        if is_omitted && end_kind == RangeEnd::Open {
            return Ok(None);
        }

        // inclusive ranges require a syntactic right endpoint
        if is_omitted {
            let missing_id = self.recover_missing_expression_here(NodeType::Pattern);

            return Ok(Some(missing_id));
        }

        let right_flags = self
            .flags
            .not_in_position()
            .in_left_precedence(OperatorPrecedence::Range as u16);
        let end_id = self
            .with_flags(self.flags.with_expression_context(right_flags), |parser| {
                parser.eat_expression_in_scope()
            })?;

        Ok(Some(end_id))
    }

    /// Eat one startless range pattern.
    fn eat_startless_range_pattern(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Pattern>> {
        let end_kind = match self.peek_token_type() {
            TokenType::Range => RangeEnd::Open,
            TokenType::RangeInclusive => RangeEnd::Inclusive,
            _ => unreachable!("checked range token"),
        };
        self.bump(); // eat range operator

        let mut end = self.eat_range_pattern_end_maybe(end_kind)?;
        if end.is_none() {
            let missing_id = self.recover_missing_expression_here(NodeType::Pattern);
            end = Some(missing_id);
        }

        Ok(self.insert_node(
            Pattern::Range {
                start: None,
                end,
                end_kind,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat one range pattern after a parsed start expression.
    fn eat_range_pattern_with_start(
        &mut self,
        start: &ParserSpanStart,
        start_id: LocalNodeId<Expression>,
        end_kind: RangeEnd,
    ) -> ParseResult<LocalNodeId<Pattern>> {
        self.bump(); // eat range operator
        let end = self.eat_range_pattern_end_maybe(end_kind)?;

        Ok(self.insert_node(
            Pattern::Range {
                start: Some(start_id),
                end,
                end_kind,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat a pattern field list (like `x, y, z` or `1 | 2`).
    fn eat_pattern_field_list(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<PatternField>>> {
        let mut fields: Vec<LocalNodeId<PatternField>> = Vec::new();
        let is_object_pattern = terminator == TokenType::CloseBrace;
        let enforce_terminal_spread =
            self.language.is_javascript() || self.language.is_typescript();
        let mut has_spread_field = false;

        while self.has_more_tokens() {
            // stop at the pattern terminator
            if self.peek_token_type() == terminator {
                break;
            }

            // spread fields must be terminal in typed and untyped patterns
            if enforce_terminal_spread && has_spread_field {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // parse one field
            let field_start = self.span_start();
            let (pattern_field, name_span) =
                self.eat_pattern_field(separator, terminator, is_object_pattern, field_start)?;
            let pattern_field_id = self
                .tree
                .insert(pattern_field, self.get_span_from(&field_start));
            if let Some(name_span) = name_span {
                self.tree.set_main_span(pattern_field_id, name_span);
            }
            fields.push(pattern_field_id);

            // typed and untyped object patterns require spread fields to terminate the list
            if enforce_terminal_spread
                && matches!(self.tree.get(pattern_field_id), PatternField::Spread { .. })
            {
                has_spread_field = true;
                let has_separator_after_spread = self.peek_token_type() == separator;
                let has_non_terminal_newline_after_spread =
                    self.current_token_is_on_new_line() && !self.peek_is(terminator);
                if has_separator_after_spread || has_non_terminal_newline_after_spread {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }

            // consume separators and newline separators
            if self.peek_token_type() == separator {
                self.bump(); // eat separator
            } else if self.current_token_is_on_new_line() {
                if self.peek_is(terminator) {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(fields)
    }

    /// Eat one pattern field according to its surrounding delimiter.
    fn eat_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        is_object_pattern: bool,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        // array and tuple elisions are empty fields before a separator
        if !is_object_pattern && self.peek_token_type() == separator {
            return Ok((PatternField::Elision, None));
        }

        // object patterns use property shaped fields
        if is_object_pattern {
            return self.eat_object_pattern_field(separator, terminator, field_start);
        }

        self.eat_list_pattern_field(separator, terminator, field_start)
    }

    /// Eat an object pattern property field.
    fn eat_object_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        // computed property
        if self.peek_is(TokenType::OpenBracket) {
            let pattern_field = self.eat_computed_pattern_field(field_start)?;
            return Ok((pattern_field, None));
        }

        // named property or rest property
        if self.peek_is(TokenType::Spread)
            || self.peek_name_is()
            || self.peek_object_pattern_alias_head()
        {
            return self.eat_named_or_spread_pattern_field(separator, terminator, field_start);
        }

        // non property patterns are retained for TS++ object patterns
        let pattern_field = self.eat_positional_pattern_field()?;

        Ok((pattern_field, None))
    }

    /// Eat a tuple or array pattern field.
    fn eat_list_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        // wildcard fields are positional unless explicitly used as labels
        if self.peek_identifier_str_is("_")
            && !self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            })
        {
            let pattern_field = self.eat_positional_pattern_field()?;
            return Ok((pattern_field, None));
        }

        // spread fields belong to the list element grammar
        if self.peek_is(TokenType::Spread) {
            let pattern_field = self.eat_spread_pattern_field(separator, terminator)?;
            return Ok((pattern_field, None));
        }

        // TS++ list labels are simple names, not tagged pattern heads
        if self.peek_list_pattern_label(separator, terminator) {
            return self.eat_named_pattern_field(terminator, field_start);
        }

        // otherwise the element is a binding pattern
        let pattern_field = self.eat_positional_pattern_field()?;

        Ok((pattern_field, None))
    }

    /// Eat a computed object pattern property.
    fn eat_computed_pattern_field(
        &mut self,
        field_start: ParserSpanStart,
    ) -> ParseResult<PatternField> {
        self.eat_token(TokenType::OpenBracket)?;
        let key = self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?;
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseBracket,
            NodeType::PatternField,
            |_, token_type| {
                Self::is_close_delimiter_boundary_token(token_type)
                    || token_type == TokenType::Colon
            },
        )?;
        self.eat_token(TokenType::Colon)?;
        let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
        let pattern =
            self.eat_pattern_assignment_maybe(pattern, self.get_span_from(&field_start))?;

        Ok(PatternField::Computed { key, pattern })
    }

    /// Eat either a named object property or an object rest property.
    fn eat_named_or_spread_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        if self.peek_is(TokenType::Spread) {
            let pattern_field = self.eat_spread_pattern_field(separator, terminator)?;
            return Ok((pattern_field, None));
        }

        self.eat_named_pattern_field(terminator, field_start)
    }

    /// Eat a named pattern field.
    fn eat_named_pattern_field(
        &mut self,
        terminator: TokenType,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        let has_named_colon_field = self.peek_name_is()
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            })
            || terminator == TokenType::CloseBrace && self.peek_object_pattern_alias_head();
        if has_named_colon_field {
            return self.eat_named_colon_pattern_field(terminator, field_start);
        }

        let (name, span) = self.eat_pattern_field_name_with_span(terminator)?;
        let shorthand_pattern = self.eat_pattern_field_shorthand_assignment_maybe(
            name,
            span,
            self.get_span_from(&field_start),
        )?;
        let pattern_field = PatternField::Named {
            name,
            is_shorthand: true,
            pattern: shorthand_pattern,
        };

        Ok((pattern_field, Some(span)))
    }

    /// Eat a named field with an explicit nested pattern.
    fn eat_named_colon_pattern_field(
        &mut self,
        terminator: TokenType,
        field_start: ParserSpanStart,
    ) -> ParseResult<(PatternField, Option<Span>)> {
        let (name, _name_span) = self.eat_pattern_field_name_with_span(terminator)?;
        self.bump(); // eat colon

        let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
        let pattern =
            self.eat_pattern_assignment_maybe(pattern, self.get_span_from(&field_start))?;
        let pattern_field = PatternField::Named {
            name,
            is_shorthand: false,
            pattern: Some(pattern),
        };

        Ok((pattern_field, None))
    }

    /// Eat a spread pattern field with an optional target.
    fn eat_spread_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
    ) -> ParseResult<PatternField> {
        self.bump(); // eat spread

        // omitted targets are allowed before separators and terminators
        let has_omitted_target = self.peek_token_type() == separator || self.peek_is(terminator);
        let has_line_omitted_target = self.current_token_is_on_new_line() && {
            let next_token_type = self.next_token_type();
            next_token_type == separator || next_token_type == terminator
        };
        let pattern = if has_omitted_target || has_line_omitted_target {
            None
        } else {
            let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
            Some(pattern)
        };

        Ok(PatternField::Spread { pattern })
    }

    /// Return whether a list element starts a TS++ shorthand label.
    fn peek_list_pattern_label(&mut self, separator: TokenType, terminator: TokenType) -> bool {
        self.peek_name_is()
            && self.lookahead(|parser| {
                parser.bump();
                matches!(
                    parser.peek_token_type(),
                    TokenType::Colon | TokenType::Assign | TokenType::Maybe
                ) || parser.peek_is(separator)
                    || parser.peek_is(terminator)
            })
    }

    // eat a positional pattern field with an optional default
    fn eat_positional_pattern_field(&mut self) -> ParseResult<PatternField> {
        let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
        let pattern = self.eat_pattern_assignment_maybe(pattern, self.tree.get_span(pattern))?;

        Ok(PatternField::Positional { pattern })
    }

    // wrap one pattern in an assignment pattern if `=` follows
    fn eat_pattern_assignment_maybe(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        span: Span,
    ) -> ParseResult<LocalNodeId<Pattern>> {
        let has_immediate_default = self.peek_is(TokenType::Assign);
        let has_newline_default =
            self.current_token_is_on_new_line() && self.peek_is(TokenType::Assign);
        if !has_immediate_default && !has_newline_default {
            return Ok(pattern_id);
        }

        self.bump(); // eat assign
        let value =
            self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?;
        let pattern = Pattern::Assign {
            pattern: pattern_id,
            value,
        };

        Ok(self.insert_node(pattern, span))
    }

    // build one shorthand assignment pattern if `=` follows
    fn eat_pattern_field_shorthand_assignment_maybe(
        &mut self,
        name: Name,
        name_span: Span,
        field_span: Span,
    ) -> ParseResult<Option<LocalNodeId<Pattern>>> {
        let identifier = match name {
            Name::Identifier(name) | Name::String(name) | Name::Number(name) => name,
        };

        let binding_pattern = self.insert_node(
            Pattern::Binding {
                name: identifier,
                pattern: None,
            },
            name_span,
        );
        let pattern = self.eat_pattern_assignment_maybe(binding_pattern, field_span)?;

        if pattern == binding_pattern {
            Ok(None)
        } else {
            Ok(Some(pattern))
        }
    }

    // check whether object pattern field head is a literal alias key before `:`
    fn peek_object_pattern_alias_head(&mut self) -> bool {
        let has_numeric_alias_head = self.peek_numeric_literal_is()
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            });
        let has_boolean_alias_head = self.peek_boolean_pattern_name_head()
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            });
        has_numeric_alias_head || has_boolean_alias_head
    }

    // check whether next token is a boolean literal key in object patterns
    fn peek_boolean_pattern_name_head(&mut self) -> bool {
        self.peek().is_ok_and(|token| {
            token.token.ty == TokenType::Literal
                && matches!(token.token.literal, Some(TokenLiteral::Boolean { .. }))
        })
    }

    // eat a name for object pattern fields
    fn eat_pattern_field_name_with_span(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<(Name, Span)> {
        let is_numeric_object_key =
            terminator == TokenType::CloseBrace && self.peek_numeric_literal_is();
        if is_numeric_object_key {
            return self.eat_numeric_pattern_name_with_span();
        }
        let is_boolean_object_key =
            terminator == TokenType::CloseBrace && self.peek_boolean_pattern_name_head();
        if is_boolean_object_key {
            return self.eat_boolean_pattern_name_with_span();
        }

        self.eat_name_with_span()
    }

    // eat a numeric pattern field name as Name::Number
    fn eat_numeric_pattern_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        let token = *self.peek_numeric_literal()?;
        let key_string = self.file.span_str(token.span).to_string();

        let numeric_literal = self.eat_scalar_literal()?;
        if !matches!(
            numeric_literal,
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_)
        ) {
            return Err(ParseError::unexpected(token.span));
        }

        let key_name = self.strings.intern(&key_string);
        Ok((Name::Number(key_name), token.span))
    }

    // eat a boolean pattern field name as Name::Identifier
    fn eat_boolean_pattern_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        let token = *self.peek()?;
        if token.token.ty != TokenType::Literal
            || !matches!(token.token.literal, Some(TokenLiteral::Boolean { .. }))
        {
            return Err(ParseError::unexpected(token.span));
        }

        self.bump();
        let key_name = self.get_span_str(token.span).to_owned();
        let key_name = self.strings.intern(&key_name);
        Ok((Name::Identifier(key_name), token.span))
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Expression, LocalNodeId, Mutability, Name, Pattern, PatternField, RangeEnd, ScalarLiteral,
        TokenType, Tree, TypeExpression,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    fn assert_integer_expression(tree: &Tree, id: LocalNodeId<Expression>, value: i64) {
        assert_node!(tree, id, Expression::ScalarLiteral(ScalarLiteral::Integer(actual)) => {
            assert_eq!(*actual, value);
        });
    }

    #[test]
    fn test_parse_pattern_wildcard() {
        // _
        let mut test = TestParser::new("_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_underscore_identifier() {
        // _ in TypeScript patterns is a normal binding name
        let mut test = TestParser::new_with_language("_", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "_");
        });
    }

    #[test]
    fn test_parse_tuple_pattern_with_missing_close_parenthesis() {
        let mut test = TestParser::new("(first, second");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
        });
    }

    #[test]
    fn test_parse_computed_pattern_field_with_missing_close_bracket() {
        let mut test = TestParser::new("{ [key: value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern, .. } => {
                assert_expression_path!(parser, parser.tree.get(*key), "key");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_reference() {
        // &_
        let mut test = TestParser::new("&_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id,
            Pattern::BorrowOf { mutability: Some(mutability), right } => {
                assert_eq!(*mutability, Mutability::Mutable);
                // _
                assert_node!(parser.tree, *right, Pattern::Wildcard)
            }
        );

        // &1
        let mut test = TestParser::new("&1");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id, Pattern::BorrowOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            // 1
            assert_node!(parser.tree, *right, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        })
    }

    #[test]
    fn test_parse_pattern_value() {
        // ^x
        let mut test = TestParser::new("^x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::MoveOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "x");
            });
        });
    }

    #[test]
    fn test_parse_pattern_identifier() {
        let mut test = TestParser::new("x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "x");
        });
    }

    #[test]
    fn test_parse_pattern_path() {
        let mut test = TestParser::new("MyEnum.A");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "MyEnum.A");
        });
    }

    #[test]
    fn test_parse_pattern_range_half_open() {
        let mut test = TestParser::new("0..10");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 0);
            assert_integer_expression(&parser.tree, *end, 10);
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_inclusive() {
        let mut test = TestParser::new("0..=10");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
            assert_integer_expression(&parser.tree, *start, 0);
            assert_integer_expression(&parser.tree, *end, 10);
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_open_ended() {
        let mut test = TestParser::new("0..");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 0);
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_startless() {
        let mut test = TestParser::new("..10");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *end, 10);
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_startless_inclusive() {
        let mut test = TestParser::new("..=10");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
            assert_integer_expression(&parser.tree, *end, 10);
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_identifier_bounds() {
        let mut test = TestParser::new("MIN..MAX");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_node!(parser.tree, *start, Expression::Identifier { name } => {
                assert_string!(parser, *name, "MIN");
            });
            assert_node!(parser.tree, *end, Expression::Identifier { name } => {
                assert_string!(parser, *name, "MAX");
            });
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_path_bounds() {
        let mut test = TestParser::new("Limits.Min..=Limits.Max");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
            assert_expression_path!(parser, parser.tree.get(*start), "Limits.Min");
            assert_expression_path!(parser, parser.tree.get(*end), "Limits.Max");
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_union() {
        let mut test = TestParser::new("0..10 | 20..30");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            assert_node!(parser.tree, patterns[0], Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert_integer_expression(&parser.tree, *start, 0);
                assert_integer_expression(&parser.tree, *end, 10);
            });
            assert_node!(parser.tree, patterns[1], Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert_integer_expression(&parser.tree, *start, 20);
                assert_integer_expression(&parser.tree, *end, 30);
            });
        });
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_pattern_range_stops_before_match_arrow() {
        let mut test = TestParser::new("0.. => value");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 0);
        });
        assert!(parser.errors.is_empty());
        assert!(parser.peek_is(TokenType::ArrowWide));
    }

    #[test]
    fn test_parse_pattern_range_recovers_inclusive_end_before_union() {
        let mut test = TestParser::new("0..= | 1");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            assert_node!(parser.tree, patterns[0], Pattern::Range { start: Some(_), end: Some(_), end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Inclusive);
            });
            assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
                assert_integer_expression(&parser.tree, *value, 1);
            });
        });
    }

    #[test]
    fn test_parse_pattern_range_recovers_bare_range() {
        let mut test = TestParser::new("..");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(_), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
        });
    }

    #[test]
    fn test_parse_pattern_range_recovers_missing_inclusive_end() {
        let mut test = TestParser::new("0..=");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(_), end: Some(_), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
        });
    }

    #[test]
    fn test_parse_pattern_tuple() {
        let mut test = TestParser::new("(x: 1, 2, y, z, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, y, z, ...)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // y
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });

            // z
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "z");
            });

            // ...
            assert_node!(parser.tree, fields[4], PatternField::Spread { pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_with_path() {
        let mut test = TestParser::new("Result.Success(_, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // Result.Success(_, ..)
        assert_node!(parser.tree, pattern_id, Pattern::TaggedTuple { ty, fields } => {
            // Result.Success
            let _ = ty; // ty is required for TaggedTuple
            assert_eq!(fields.len(), 2);

            // _
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_field_const_alias() {
        let mut test =
            TestParser::new_with_language("{ const: value, title }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // { const: value, title }
        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);

            // const: value
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern) } => {
                assert_name!(parser, *name, "const");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });

            // title
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "title");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_spread_non_terminal() {
        let mut test = TestParser::new("(x, ...rest, z)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "z");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_newline_separated() {
        let mut test = TestParser::new(
            "
(
    x: 1
    2,
    ...
)",
        );
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[2], PatternField::Spread { pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_union() {
        // 1 | 2 | 3
        let mut test = TestParser::new("1 | 2 | 3");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 3);

            // 1
            assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // 2
            assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });

            // 3
            assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_anonymous() {
        let mut test = TestParser::new("{ x: 1, y, z, w: 4, ... }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });

            // z
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "z");
            });

            // w: 4
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "w");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[4], PatternField::Spread { pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_named_default_after_comment_newline() {
        let mut test =
            TestParser::new_with_language("{d //comment\n= b}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern), .. } => {
                assert_name!(parser, *name, "d");
                assert_node!(parser.tree, *pattern, Pattern::Assign { pattern, value } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                        assert_string!(parser, *name, "d");
                    });
                    assert_expression_path!(parser, parser.tree.get(*value), "b");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_name_aliases() {
        let mut test = TestParser::new("{ 0: fieldNameOrOptions, 1: from, length: argc }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "0");
                });
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "fieldNameOrOptions");
                });
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "1");
                });
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "from");
                });
            });
            assert_node!(parser.tree, fields[2], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_name!(parser, *name, "length");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "argc");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_boolean_name_aliases() {
        let mut test = TestParser::new_with_language(
            "{ false: decorators, true: metadata }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_name!(parser, *name, "false");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "decorators");
                });
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_name!(parser, *name, "true");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "metadata");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_literal_field() {
        let mut test = TestParser::new_with_language("{ 5 }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_computed_field() {
        let mut test = TestParser::new("{ [key]: value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern } => {
                assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "key");
                });
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_with_path() {
        let mut test = TestParser::new("Vector2 { x: 0, y }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::TaggedObject { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Vector2");
            });
            assert_eq!(fields.len(), 2);

            // x: 0
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_with_nested_tagged_object_field() {
        let mut test = TestParser::new("Shape.Line { start: Point { x, y }, end }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert!(parser.errors.is_empty());

        assert_node!(parser.tree, pattern_id, Pattern::TaggedObject { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Shape.Line");
            });
            assert_eq!(fields.len(), 2);

            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
                assert_name!(parser, *name, "start");
                assert_node!(parser.tree, *pattern, Pattern::TaggedObject { ty, fields } => {
                    assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "Point");
                    });
                    assert_eq!(fields.len(), 2);

                    assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                        assert_name!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                        assert_name!(parser, *name, "y");
                    });
                });
            });

            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "end");
            });
        });
    }

    #[test]
    fn test_parse_pattern_slice() {
        let mut test = TestParser::new("[1, ...]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);
            // 1
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
            // ...
            assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: None } => {
            });
        });
    }

    /// Parse a spread field with a sequence pattern.
    #[test]
    fn test_parse_pattern_spread_array_pattern() {
        let mut test = TestParser::new("[...[x, y]]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Spread { pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                    assert_eq!(fields.len(), 2);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "y");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand() {
        let mut test = TestParser::new_with_language("{ readonly }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand_with_newline() {
        let mut test = TestParser::new_with_language("{ readonly\n}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }
    #[test]
    fn test_parse_pattern_object_readonly_shorthand_in_value_block_mode() {
        let mut test = TestParser::new("{ readonly }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_readonly_identifier() {
        let mut test =
            TestParser::new_with_language("[readonly, setReadonly]", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "readonly");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "setReadonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_readonly_identifier_in_value_block_mode() {
        let mut test = TestParser::new("[readonly, setReadonly]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "readonly");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "setReadonly");
            });
        });
    }

    #[test]
    fn test_reject_pattern_object_readonly_modifier_with_name_in_value_block_mode() {
        let mut test = TestParser::new("{ readonly value }");
        let mut parser = test.prepare();

        let result = parser.eat_pattern();

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pattern_object_spread_newline_before_terminator() {
        let mut test = TestParser::new_with_language(
            "{\n  onSuccess,\n  ...rest\n}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "onSuccess");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_must() {
        let mut test = TestParser::new("1!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_union_with_must_arms() {
        let mut test = TestParser::new("1! | 2!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            // 1!
            assert_node!(parser.tree, patterns[0], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
            // 2!
            assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_union_with_trailing_must_arm() {
        let mut test = TestParser::new("1 | 2!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 2);
            // 1
            assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // 2!
            assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_elision() {
        // [,a] - elision before 'a'
        let mut test = TestParser::new("[,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);

            // elision (empty slot)
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_multiple_elisions() {
        // [,,a] - two elisions before 'a'
        let mut test = TestParser::new("[,,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 3);

            // first elision
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // second elision
            assert_node!(parser.tree, fields[1], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[2], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_trailing_elision() {
        // [a,] - element followed by trailing comma (not elision)
        let mut test = TestParser::new("[a,]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
            // trailing comma doesn't create elision, just 'a'
            assert_eq!(fields.len(), 1);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_defaults_do_not_consume_following_fields() {
        // {a,b=1,c:d,e:f=2,[g]:[h]}
        let mut test =
            TestParser::new_with_language("{a,b=1,c:d,e:f=2,[g]:[h]}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // b=1
            assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
            });

            // c:d
            assert_node!(parser.tree, fields[2], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { .. });
            });

            // e:f=2
            assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
            });

            // [g]:[h]
            assert_node!(parser.tree, fields[4], PatternField::Computed { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Sequence { .. });
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_alias_and_computed_defaults() {
        // {c, d:e=1, [f]:g=2, h=i}
        let mut test =
            TestParser::new_with_language("{c, d:e=1, [f]:g=2, h=i}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 4);

            // d:e=1
            assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
            });

            // [f]:g=2
            assert_node!(parser.tree, fields[2], PatternField::Computed { pattern, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
            });

            // h=i
            assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_computed_field_with_newline_after_colon() {
        // { [key]:\nvalue }
        let mut test = TestParser::new_with_language("{ [key]:\nvalue }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);

            assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern } => {
                assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "key");
                });

                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_alias_with_newline_after_colon() {
        // { source:\ntarget }
        let mut test =
            TestParser::new_with_language("{ source:\ntarget }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);

            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern) } => {
                assert_name!(parser, *name, "source");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "target");
                });
            });
        });
    }
}
