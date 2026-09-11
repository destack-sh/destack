use crate::parse::error::ParserResultExt;
use crate::parse::{ExpressionPosition, ExpressionStop, RangedPath};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    Expression, Keyword, Literal, LocalNodeId, Name, NodeType, OperatorPrecedence, Pattern,
    PatternField, RangeEnd, TokenType, TypeExpression,
};
use destack_source::ByteRange;

/// Token ownership for one pattern operand.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
struct PatternStop(u8);

impl PatternStop {
    /// A type annotation owned by the enclosing declaration.
    const ANNOTATION: Self = Self(1 << 0);
    /// A union separator owned by the enclosing pattern.
    const UNION: Self = Self(1 << 1);

    /// Add one enclosing token.
    const fn add(self, stop: Self) -> Self {
        Self(self.0 | stop.0)
    }

    /// Return whether one token belongs to the enclosing pattern.
    const fn has(self, stop: Self) -> bool {
        self.0 & stop.0 != 0
    }
}

impl Parser {
    /// Parse one standalone pattern fragment.
    ///
    /// Examples:
    /// ```ds
    /// Vector2 { x: 0, y }
    /// ```
    pub fn parse_pattern_fragment(&mut self) -> ParserResult<LocalNodeId<Pattern>> {
        self.parse_pattern()
    }

    /// Parse a pattern.
    ///
    /// Examples:
    /// ```ds
    /// _
    /// 1
    /// 2 | 3
    /// (x, 0, ...)
    /// { a: 2 }
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ...] }
    /// ```
    pub(crate) fn parse_pattern(&mut self) -> ParserResult<LocalNodeId<Pattern>> {
        self.parse_pattern_until(PatternStop::default())
    }

    /// Parse a pattern followed by a type annotation.
    pub(crate) fn parse_pattern_before_type(&mut self) -> ParserResult<LocalNodeId<Pattern>> {
        self.parse_pattern_until(PatternStop::ANNOTATION)
    }

    /// Parse a pattern with its immediate owner.
    fn parse_pattern_until(&mut self, stops: PatternStop) -> ParserResult<LocalNodeId<Pattern>> {
        self.with_recursive_descent(NodeType::Pattern, |parser| {
            parser.parse_pattern_after_descent(stops)
        })
    }

    /// Parse a pattern after checking the recursion depth.
    fn parse_pattern_after_descent(
        &mut self,
        stops: PatternStop,
    ) -> ParserResult<LocalNodeId<Pattern>> {
        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();

        // preserve contextually reserved bindings for recovery while reporting them
        self.report_forbidden_binding_identifier();

        // parse the primary pattern
        let mut pattern_id = {
            // startless range pattern
            let startless_range_end = match self.peek_token_type() {
                TokenType::Range => Some(RangeEnd::Open),
                TokenType::RangeInclusive => Some(RangeEnd::Inclusive),
                _ => None,
            };
            if let Some(end_kind) = startless_range_end {
                self.parse_startless_range_pattern(&start, end_kind)?
            }
            // wildcard
            else if self.peek_identifier_is("_") {
                self.bump();
                self.insert_node(Pattern::Wildcard, self.range_since(&start))
            }
            // reference of
            else if matches!(
                self.peek_token_type(),
                TokenType::ElementwiseAnd | TokenType::LogicalAnd
            ) {
                self.eat_reference_prefix_operator()?;
                let access = Some(self.parse_borrow_access()?);
                let right_id = self.parse_pattern_until(stops).in_node(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::BorrowOf {
                        access,
                        right: right_id,
                    },
                    self.range_since(&start),
                )
            }
            // value of
            else if self.peek_is(TokenType::ElementwiseXor) {
                self.bump();
                let mutability = Some(self.parse_reference_mutability());
                let right_id = self.parse_pattern_until(stops).in_node(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::MoveOf {
                        mutability,
                        right: right_id,
                    },
                    self.range_since(&start),
                )
            }
            // dereference
            else if self.peek_is(TokenType::Multiply) {
                self.bump();
                let right_id = self.parse_pattern_until(stops).in_node(NodeType::Pattern)?;
                self.insert_node(
                    Pattern::DereferenceOf { right: right_id },
                    self.range_since(&start),
                )
            }
            // tuple (without type, no struct tuples)
            else if self.peek_is(TokenType::OpenParenthesis) {
                self.bump();
                let fields =
                    self.parse_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)?;
                let pattern = Pattern::Tuple { fields };
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseParenthesis,
                    NodeType::Pattern,
                )?;
                self.insert_node(pattern, self.range_since(&start))
            }
            // struct (without type)
            else if self.peek_is(TokenType::OpenBrace) {
                self.bump();
                let fields =
                    self.parse_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)?;
                let pattern = Pattern::Object { fields };
                self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Pattern)?;
                self.insert_node(pattern, self.range_since(&start))
            }
            // array or slice
            else if self.peek_is(TokenType::OpenBracket) {
                self.bump();
                let fields =
                    self.parse_pattern_field_list(TokenType::Comma, TokenType::CloseBracket)?;
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseBracket,
                    NodeType::Pattern,
                )?;
                self.insert_node(Pattern::Sequence { fields }, self.range_since(&start))
            }
            // scalar literal expression
            else if matches!(self.peek_token_type(), TokenType::Add | TokenType::Subtract)
                || self.peek_scalar_literal_start()
            {
                let scalar_literal = match self.peek_token_type() {
                    TokenType::Add | TokenType::Subtract => self
                        .parse_signed_numeric_literal()
                        .in_node(NodeType::Pattern)?,
                    _ => self.parse_scalar_literal().in_node(NodeType::Pattern)?,
                };
                let expression_id = self.insert_node(
                    Expression::Literal(scalar_literal),
                    self.range_since(&start),
                );
                if let Some(end_kind) = self.peek_range_end() {
                    self.parse_range_pattern(&start, expression_id, end_kind)?
                } else {
                    self.insert_node(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.range_since(&start),
                    )
                }
            }
            // nullish literals
            else if let Some(keyword @ (Keyword::Null | Keyword::Undefined)) = self.peek_keyword()
            {
                let literal = if keyword == Keyword::Null {
                    Literal::Null
                } else {
                    Literal::Undefined
                };
                self.bump();

                let expression_id =
                    self.insert_node(Expression::Literal(literal), self.range_since(&start));
                let pattern = Pattern::Expression {
                    value: expression_id,
                };
                self.insert_node(pattern, self.range_since(&start))
            }
            // binding with expression or pattern
            else if !stops.has(PatternStop::ANNOTATION)
                && self.peek_is(TokenType::Identifier)
                && self.peek_token_type_at(1) == TokenType::Colon
            {
                let (name, name_range) = self.eat_binding_identifier_with_range()?;
                self.bump();
                let nested_pattern = self.parse_pattern()?;
                let pattern_id = self.insert_node(
                    Pattern::Binding {
                        name,
                        pattern: Some(nested_pattern),
                    },
                    self.range_since(&start),
                );
                self.tree.set_main_range(pattern_id, name_range);
                pattern_id
            }
            // path or identifier
            else {
                let path = self.parse_ranged_path().in_node(NodeType::Pattern)?;
                let last_range = path
                    .last_range()
                    .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;
                let RangedPath {
                    path,
                    segment_ranges,
                } = path;
                let range_end_kind = self.peek_range_end();
                // tuple with path
                if self.peek_is(TokenType::OpenParenthesis) {
                    self.bump();
                    let fields = self
                        .parse_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                        .in_node(NodeType::Pattern)?;
                    let expression_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.range_since(&start),
                    );
                    self.record_type_path(expression_id, &segment_ranges)?;
                    let pattern = Pattern::NominalTuple {
                        ty: expression_id,
                        fields,
                    };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseParenthesis,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.range_since(&start))
                }
                // struct with path
                else if self.peek_is(TokenType::OpenBrace) {
                    self.bump();
                    let fields = self
                        .parse_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                        .in_node(NodeType::Pattern)?;
                    let ty_id = self.insert_node(
                        TypeExpression::Reference {
                            path,
                            generic_arguments: vec![],
                        },
                        self.range_since(&start),
                    );
                    self.record_type_path(ty_id, &segment_ranges)?;
                    let pattern = Pattern::NominalObject { ty: ty_id, fields };
                    self.eat_close_token_or_recover_missing(
                        TokenType::CloseBrace,
                        NodeType::Pattern,
                    )?;
                    self.insert_node(pattern, self.range_since(&start))
                }
                // range with path or identifier start
                else if let Some(end_kind) = range_end_kind {
                    let expression_id =
                        self.insert_member_chain(&path.segments, &segment_ranges)?;

                    self.parse_range_pattern(&start, expression_id, end_kind)?
                }
                // path
                else if path.segments.len() > 1 {
                    let expression_id =
                        self.insert_member_chain(&path.segments, &segment_ranges)?;

                    self.insert_node(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.range_since(&start),
                    )
                }
                // identifier
                else {
                    let pattern_id = self.insert_node(
                        Pattern::Binding {
                            name: path.segments[0],
                            pattern: None,
                        },
                        self.range_since(&start),
                    );
                    self.tree.set_main_range(pattern_id, last_range);
                    pattern_id
                }
            }
        };

        // apply the postfix assertion
        if self.peek_is(TokenType::Not) {
            self.bump();
            let pattern = Pattern::Must(pattern_id);
            pattern_id = self.insert_node(pattern, self.range_since(&start));
        }
        // collect a union at the current level
        let pattern_id = if self.peek_is(TokenType::ElementwiseOr) && !stops.has(PatternStop::UNION)
        {
            // eat all union "fields" (just unnamed patterns)
            let mut patterns: Vec<LocalNodeId<Pattern>> = vec![pattern_id];
            while self.peek_is(TokenType::ElementwiseOr) {
                self.bump();
                let field_pattern_id = self.parse_pattern_until(stops.add(PatternStop::UNION))?;
                patterns.push(field_pattern_id);
            }
            let pattern = Pattern::Union { patterns };
            self.insert_node(pattern, self.range_since(&start))
        }
        // return the single pattern
        else {
            pattern_id
        };
        self.attach_documentation(pattern_id, documentation);

        Ok(pattern_id)
    }

    /// Return the range end kind when the next token continues a pattern range.
    fn peek_range_end(&self) -> Option<RangeEnd> {
        if self.peek_is_on_new_line() {
            return None;
        }

        match self.peek_token_type() {
            TokenType::Range => Some(RangeEnd::Open),
            TokenType::RangeInclusive => Some(RangeEnd::Inclusive),
            _ => None,
        }
    }

    /// Return whether the current range pattern end is omitted.
    fn peek_range_pattern_end_omitted(&self) -> bool {
        if self.peek_is_on_new_line() {
            return true;
        }

        matches!(
            self.peek_token_type(),
            TokenType::ArrowWide | TokenType::ElementwiseOr
        ) || Self::is_expression_slot_boundary_token(self.peek_token_type())
    }

    /// Parse one range endpoint after its operator.
    fn parse_range_end(
        &mut self,
        end_kind: RangeEnd,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        let is_omitted = self.peek_range_pattern_end_omitted();

        // open-ended ranges may omit the right endpoint
        if is_omitted && end_kind == RangeEnd::Open {
            return Ok(None);
        }

        // inclusive ranges require a syntactic right endpoint
        if is_omitted {
            let missing_id = self.recover_missing_expression_here(NodeType::Pattern);

            return Ok(Some(missing_id));
        }

        let end_id = self.parse_range_end_expression()?;

        Ok(Some(end_id))
    }

    /// Parse one range endpoint expression.
    fn parse_range_end_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // fold explicit signs into numeric scalar bounds
        if matches!(self.peek_token_type(), TokenType::Add | TokenType::Subtract)
            && self.peek_token_type_at(1) == TokenType::Literal
        {
            let value = self
                .parse_signed_numeric_literal()
                .in_node(NodeType::Pattern)?;

            return Ok(self.insert_node(Expression::Literal(value), self.range_since(&start)));
        }

        // match arrows terminate symbolic endpoints
        if self.peek_is(TokenType::Identifier)
            && self.peek_next_token_type() == TokenType::ArrowWide
        {
            return self.parse_identifier_expression(&start);
        }

        self.parse_expression_at(
            ExpressionPosition::Value,
            ExpressionStop::default(),
            OperatorPrecedence::Range,
        )
    }

    /// Parse one startless range pattern.
    fn parse_startless_range_pattern(
        &mut self,
        start: &ParseStart,
        end_kind: RangeEnd,
    ) -> ParserResult<LocalNodeId<Pattern>> {
        self.bump();

        let mut end = self.parse_range_end(end_kind)?;
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
            self.range_since(start),
        ))
    }

    /// Parse one range pattern after its start expression.
    fn parse_range_pattern(
        &mut self,
        start: &ParseStart,
        start_id: LocalNodeId<Expression>,
        end_kind: RangeEnd,
    ) -> ParserResult<LocalNodeId<Pattern>> {
        self.bump();
        let end = self.parse_range_end(end_kind)?;

        Ok(self.insert_node(
            Pattern::Range {
                start: Some(start_id),
                end,
                end_kind,
            },
            self.range_since(start),
        ))
    }

    /// Parse a pattern field list such as `x, y, z`.
    fn parse_pattern_field_list(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
    ) -> ParserResult<Vec<LocalNodeId<PatternField>>> {
        let mut fields = Vec::new();
        let is_object_pattern = terminator == TokenType::CloseBrace;

        while self.has_more_tokens() {
            // stop at the pattern terminator
            if self.peek_is(terminator) {
                break;
            }

            // parse one field
            let documentation = self.parse_documentation();
            let field_start = self.mark_parse_start();
            let (pattern_field, name_range) =
                self.parse_pattern_field(separator, terminator, is_object_pattern, field_start)?;
            let pattern_field_id = self.insert_node(pattern_field, self.range_since(&field_start));
            if let Some(name_range) = name_range {
                self.tree.set_main_range(pattern_field_id, name_range);
            }
            self.attach_documentation(pattern_field_id, documentation);
            fields.push(pattern_field_id);

            // consume an explicit separator
            if self.peek_is(separator) {
                self.bump();
            }
            // continue only across a nonterminal newline separator
            else if !self.peek_is_on_new_line() || self.peek_is(terminator) {
                break;
            }
        }

        Ok(fields)
    }

    /// Parse one pattern field according to its surrounding delimiter.
    fn parse_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        is_object_pattern: bool,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        // array and tuple elisions are empty fields before a separator
        if !is_object_pattern && self.peek_is(separator) {
            return Ok((PatternField::Elision, None));
        }

        // object patterns use property shaped fields
        if is_object_pattern {
            return self.parse_object_pattern_field(separator, terminator, field_start);
        }

        self.parse_list_pattern_field(separator, terminator)
    }

    /// Parse an object pattern property field.
    fn parse_object_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        // computed property
        if self.peek_is(TokenType::OpenBracket) {
            let (pattern_field, key_range) = self.parse_computed_pattern_field(field_start)?;
            return Ok((pattern_field, Some(key_range)));
        }

        // named property or rest property
        if self.peek_is(TokenType::Spread)
            || self.peek_name_start()
            || self.peek_numeric_pattern_name()
        {
            return self.parse_named_or_rest_pattern_field(separator, terminator, field_start);
        }

        Err(ParserError::unexpected(self.peek_token_span()))
    }

    /// Parse a tuple or array pattern field.
    fn parse_list_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        // wildcard fields are positional unless explicitly used as labels
        if self.peek_identifier_is("_") && self.peek_token_type_at(1) != TokenType::Colon {
            let pattern_field = self.parse_positional_pattern_field()?;
            return Ok((pattern_field, None));
        }

        // rest fields belong to the list element
        if self.peek_is(TokenType::Spread) {
            let pattern_field = self.parse_rest_pattern_field(separator, terminator)?;
            return Ok((pattern_field, None));
        }

        // otherwise the element is a binding pattern
        let pattern_field = self.parse_positional_pattern_field()?;

        Ok((pattern_field, None))
    }

    /// Parse a computed object pattern property.
    fn parse_computed_pattern_field(
        &mut self,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, ByteRange)> {
        self.eat_token(TokenType::OpenBracket)?;
        let key = self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseBracket,
            NodeType::PatternField,
            |_, token_type| {
                Self::is_close_delimiter_boundary_token(token_type)
                    || token_type == TokenType::Colon
            },
        )?;

        // retain the complete computed key as the field's main range
        let key_range = self.range_since(&field_start);
        self.eat_token(TokenType::Colon)?;
        let pattern = self.parse_pattern().in_node(NodeType::Pattern)?;
        let pattern = self.parse_pattern_default(pattern, self.range_since(&field_start))?;

        Ok((PatternField::Computed { key, pattern }, key_range))
    }

    /// Parse either a named object property or an object rest property.
    fn parse_named_or_rest_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        if self.peek_is(TokenType::Spread) {
            let pattern_field = self.parse_rest_pattern_field(separator, terminator)?;
            return Ok((pattern_field, None));
        }

        self.parse_named_pattern_field(terminator, field_start)
    }

    /// Parse a named pattern field.
    fn parse_named_pattern_field(
        &mut self,
        terminator: TokenType,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        let has_named_colon_field = self.peek_name_start()
            && self.peek_token_type_at(1) == TokenType::Colon
            || terminator == TokenType::CloseBrace && self.peek_numeric_pattern_name();
        if has_named_colon_field {
            return self.parse_named_colon_pattern_field(terminator, field_start);
        }

        // shorthand property names also declare bindings
        self.report_forbidden_binding_identifier();

        let (name, range) = self.eat_pattern_field_name_with_range(terminator)?;
        let shorthand_pattern =
            self.parse_shorthand_pattern_default(name, range, self.range_since(&field_start))?;
        let pattern_field = PatternField::Named {
            name,
            is_shorthand: true,
            pattern: shorthand_pattern,
        };

        Ok((pattern_field, Some(range)))
    }

    /// Parse a named field with an explicit nested pattern.
    fn parse_named_colon_pattern_field(
        &mut self,
        terminator: TokenType,
        field_start: ParseStart,
    ) -> ParserResult<(PatternField, Option<ByteRange>)> {
        let (name, name_range) = self.eat_pattern_field_name_with_range(terminator)?;
        self.bump();

        let pattern = self.parse_pattern().in_node(NodeType::Pattern)?;
        let pattern = self.parse_pattern_default(pattern, self.range_since(&field_start))?;
        let pattern_field = PatternField::Named {
            name,
            is_shorthand: false,
            pattern: Some(pattern),
        };

        Ok((pattern_field, Some(name_range)))
    }

    /// Parse a rest pattern field with an optional target.
    fn parse_rest_pattern_field(
        &mut self,
        separator: TokenType,
        terminator: TokenType,
    ) -> ParserResult<PatternField> {
        self.bump();

        // omitted targets are allowed before separators and terminators
        let has_omitted_target = self.peek_token_type() == separator || self.peek_is(terminator);
        let has_line_omitted_target = self.peek_is_on_new_line() && {
            let peek_next_token_type = self.peek_next_token_type();
            peek_next_token_type == separator || peek_next_token_type == terminator
        };
        let pattern = if has_omitted_target || has_line_omitted_target {
            None
        } else {
            let pattern = self.parse_pattern().in_node(NodeType::Pattern)?;
            Some(pattern)
        };

        Ok(PatternField::Rest { pattern })
    }

    /// Parse a positional pattern field with an optional default.
    fn parse_positional_pattern_field(&mut self) -> ParserResult<PatternField> {
        let pattern = self.parse_pattern().in_node(NodeType::Pattern)?;
        let pattern = self.parse_pattern_default(pattern, self.tree.get_range(pattern))?;

        Ok(PatternField::Positional { pattern })
    }

    /// Wrap one pattern in a default pattern when `=` follows.
    fn parse_pattern_default(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        range: ByteRange,
    ) -> ParserResult<LocalNodeId<Pattern>> {
        if !self.peek_is(TokenType::Assign) {
            return Ok(pattern_id);
        }

        self.bump();
        let value = self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
        let pattern = Pattern::Default {
            pattern: pattern_id,
            value,
        };

        Ok(self.insert_node(pattern, range))
    }

    /// Create one shorthand default pattern when `=` follows.
    fn parse_shorthand_pattern_default(
        &mut self,
        name: Name,
        name_range: ByteRange,
        field_range: ByteRange,
    ) -> ParserResult<Option<LocalNodeId<Pattern>>> {
        let identifier = match name {
            Name::Identifier(name) => name,
            Name::String(_) | Name::Index(_) => {
                return Err(ParserError::unexpected(name_range));
            }
        };

        let binding_pattern = self.insert_node(
            Pattern::Binding {
                name: identifier,
                pattern: None,
            },
            name_range,
        );
        let pattern = self.parse_pattern_default(binding_pattern, field_range)?;

        if pattern == binding_pattern {
            Ok(None)
        } else {
            Ok(Some(pattern))
        }
    }

    /// Return whether an integer pattern name starts here.
    fn peek_numeric_pattern_name(&self) -> bool {
        self.peek_numeric_literal_start() && self.peek_token_type_at(1) == TokenType::Colon
    }

    /// Eat one object-pattern field name.
    fn eat_pattern_field_name_with_range(
        &mut self,
        terminator: TokenType,
    ) -> ParserResult<(Name, ByteRange)> {
        let is_numeric_object_name =
            terminator == TokenType::CloseBrace && self.peek_numeric_literal_start();
        if is_numeric_object_name {
            return self.eat_numeric_pattern_name_with_range();
        }
        self.eat_name_with_range()
    }

    /// Eat one numeric pattern field name as an index.
    fn eat_numeric_pattern_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        let (index, range) = self.eat_index_name_with_range()?;

        Ok((Name::Index(index), range))
    }
}
