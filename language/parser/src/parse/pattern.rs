use crate::parse::flags::ParserFlags;
use crate::parse::prelude::*;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

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
    pub fn eat_pattern(&mut self) -> ParserResult<LocalNodeId<Pattern>> {
        self.with_recursive_descent(NodeType::Pattern, |parser| {
            parser.eat_pattern_at_current_depth()
        })
    }

    /// Eat a pattern after recursive descent state has been entered.
    fn eat_pattern_at_current_depth(&mut self) -> ParserResult<LocalNodeId<Pattern>> {
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
            // dereference
            else if self.language.is_destack() && self.peek_is(TokenType::Multiply) {
                self.bump(); // eat *
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::DereferenceOf { right: right_id },
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
                && self.token_type_at_offset(1) == TokenType::Colon
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
                    let pattern = Pattern::Newtype {
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
                else if self.peek_is(TokenType::OpenBrace) {
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
                    let pattern = Pattern::NominalObject { ty: ty_id, fields };
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
                        self.set_path_expression_spans(expression_id, &segment_spans)?;
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
                    self.set_path_expression_spans(expression_id, &segment_spans)?;
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
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
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

        let right_flags = self.flags.not_in_position();
        let end_id = self.eat_range_pattern_end_expression(right_flags)?;

        Ok(Some(end_id))
    }

    /// Eat one range endpoint expression.
    fn eat_range_pattern_end_expression(
        &mut self,
        flags: ParserFlags,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // match arrows terminate symbolic endpoints
        if self.peek_is(TokenType::Identifier) && self.next_token_type() == TokenType::ArrowWide {
            return self.eat_identifier_expression_path(&start);
        }

        self.eat_expression_at_precedence(
            self.flags.with_expression_context(flags),
            OperatorPrecedence::Range as u16,
        )
    }

    /// Eat one startless range pattern.
    fn eat_startless_range_pattern(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<Pattern>> {
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
    ) -> ParserResult<LocalNodeId<Pattern>> {
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
    ) -> ParserResult<Vec<LocalNodeId<PatternField>>> {
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
                return Err(ParserError::unexpected(self.peek()?.span));
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
                    return Err(ParserError::unexpected(self.peek()?.span));
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
        // wildcard fields are positional unless explicitly used as labels
        if self.peek_identifier_str_is("_") && self.token_type_at_offset(1) != TokenType::Colon {
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
    ) -> ParserResult<PatternField> {
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
        let has_named_colon_field = self.peek_name_is()
            && self.token_type_at_offset(1) == TokenType::Colon
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
    ) -> ParserResult<(PatternField, Option<Span>)> {
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
    ) -> ParserResult<PatternField> {
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
        if !self.peek_name_is() {
            return false;
        }

        matches!(
            self.token_type_at_offset(1),
            TokenType::Colon | TokenType::Assign | TokenType::Maybe
        ) || self.token_type_at_offset(1) == separator
            || self.token_type_at_offset(1) == terminator
    }

    // eat a positional pattern field with an optional default
    fn eat_positional_pattern_field(&mut self) -> ParserResult<PatternField> {
        let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
        let pattern = self.eat_pattern_assignment_maybe(pattern, self.tree.get_span(pattern))?;

        Ok(PatternField::Positional { pattern })
    }

    // wrap one pattern in an assignment pattern if `=` follows
    fn eat_pattern_assignment_maybe(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        span: Span,
    ) -> ParserResult<LocalNodeId<Pattern>> {
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
    ) -> ParserResult<Option<LocalNodeId<Pattern>>> {
        let identifier = match name {
            Name::Identifier(name) | Name::String(name) => name,
            Name::Index(_) => return Err(ParserError::unexpected(name_span)),
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
        let has_numeric_alias_head =
            self.peek_numeric_literal_is() && self.token_type_at_offset(1) == TokenType::Colon;
        let has_boolean_alias_head = self.peek_boolean_pattern_name_head()
            && self.token_type_at_offset(1) == TokenType::Colon;
        has_numeric_alias_head || has_boolean_alias_head
    }

    // check whether next token is a boolean literal key in object patterns
    fn peek_boolean_pattern_name_head(&mut self) -> bool {
        self.peek().is_ok_and(|token| {
            token.token.ty() == TokenType::Literal
                && matches!(token.token.literal(), Some(TokenLiteral::Boolean { .. }))
        })
    }

    // eat a name for object pattern fields
    fn eat_pattern_field_name_with_span(
        &mut self,
        terminator: TokenType,
    ) -> ParserResult<(Name, Span)> {
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

    // eat a numeric pattern field name as an index
    fn eat_numeric_pattern_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        let (index, span) = self.eat_index_key_with_span()?;

        Ok((Name::Index(index), span))
    }

    // eat a boolean pattern field name as Name::Identifier
    fn eat_boolean_pattern_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        let token = *self.peek()?;
        if token.token.ty() != TokenType::Literal
            || !matches!(token.token.literal(), Some(TokenLiteral::Boolean { .. }))
        {
            return Err(ParserError::unexpected(token.span));
        }

        self.bump();
        let key_name = self.get_span_str(token.span).to_owned();
        let key_name = self.strings.intern(&key_name);
        Ok((Name::Identifier(key_name), token.span))
    }
}
