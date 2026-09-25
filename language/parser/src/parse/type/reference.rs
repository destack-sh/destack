use crate::parse::{ExpressionPosition, ExpressionStop, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use smallvec::SmallVec;
use tspp_core::StringId;
use tspp_dir::{
    Expression, InferForm, Keyword, LocalNodeId, NodeType, Path, TokenType, TypeExpression,
    TypeLiteral,
};
use tspp_source::{ByteRange, NodeSpanList, NodeSpanType};

impl Parser {
    /// Parse one identifier primary in type space.
    ///
    /// Examples:
    /// ```tspp
    /// T
    /// this
    /// infer U
    /// ```
    pub(super) fn parse_type_reference(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // infer hole
        if self.peek_identifier_is("_") {
            return Ok(self.parse_type_infer_hole(start));
        }

        // keyword identifiers
        if let Some(type_expression) = self.parse_type_identifier_keyword(start, stop)? {
            return Ok(type_expression);
        }

        // literal types
        if let Some(type_expression) = self.parse_type_literal_primary(start)? {
            return Ok(type_expression);
        }

        // path reference
        self.parse_type_path_reference(start)
    }

    /// Parse a TS++ infer hole.
    ///
    /// Examples:
    /// ```tspp
    /// _
    /// _[]
    /// Promise<_>
    /// ```
    pub(super) fn parse_type_infer_hole(
        &mut self,
        start: &ParseStart,
    ) -> LocalNodeId<TypeExpression> {
        let name_range = self.peek_token().range();
        self.bump();
        let id = self.insert_node(
            TypeExpression::Infer {
                form: InferForm::Hole,
                name: None,
                constraint: None,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(id, name_range);

        id
    }

    /// Parse type primary keywords that appear in identifier position.
    ///
    /// Examples:
    /// ```tspp
    /// this
    /// infer T
    /// intrinsic
    /// ```
    fn parse_type_identifier_keyword(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if self.peek_keyword() == Some(Keyword::This) {
            let keyword_range = self.peek_token().range();
            self.bump();
            let ty = self.insert_node(TypeExpression::This, self.range_since(start));
            self.tree.set_main_range(ty, keyword_range);

            return Ok(Some(ty));
        }

        if self.peek_keyword() == Some(Keyword::Infer) {
            return self.parse_type_infer(stop).map(Some);
        }

        if self.peek_identifier_is("intrinsic")
            && Self::is_type_expression_boundary_token(self.peek_next_token_type())
        {
            self.bump();
            return Ok(Some(
                self.insert_node(TypeExpression::Intrinsic, self.range_since(start)),
            ));
        }

        Ok(None)
    }

    /// Parse a literal type primary when present.
    ///
    /// Examples:
    /// ```tspp
    /// null
    /// undefined
    /// "open"
    /// ```
    fn parse_type_literal_primary(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if self.peek_type_literal().is_none() {
            return Ok(None);
        }

        let literal = self.parse_type_literal()?;
        let type_expression = self.insert_node(
            TypeExpression::Keyword { value: literal },
            self.range_since(start),
        );

        Ok(Some(type_expression))
    }

    /// Parse a path reference type.
    ///
    /// Examples:
    /// ```tspp
    /// User
    /// namespace.User
    /// Result<string, Error>
    /// ```
    fn parse_type_path_reference(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let (first, first_range) = self.eat_identifier_with_range()?;

        if !self.peek_path_continuation() {
            return self.parse_single_type_reference(start, first, first_range);
        }

        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
        let mut segment_ranges: SmallVec<[ByteRange; 3]> = SmallVec::new();
        segments.push(first);
        segment_ranges.push(first_range);

        while self.peek_path_continuation() {
            self.bump();
            let (segment, segment_range) = self.eat_identifier_with_range()?;
            segments.push(segment);
            segment_ranges.push(segment_range);
        }

        let path = Path { segments };
        let generic_arguments = if self.peek_type_generic_arguments() {
            self.parse_type_generic_arguments()?
        } else {
            Vec::new()
        };
        let id = self.insert_node(
            TypeExpression::Reference {
                path,
                generic_arguments,
            },
            self.range_since(start),
        );
        self.record_type_path(id, &segment_ranges)?;

        Ok(id)
    }

    /// Parse a single-segment path reference type.
    ///
    /// Examples:
    /// ```tspp
    /// User
    /// Result<T>
    /// Promise<string>
    /// ```
    fn parse_single_type_reference(
        &mut self,
        start: &ParseStart,
        segment: StringId,
        segment_range: ByteRange,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
        segments.push(segment);

        let generic_arguments = if self.peek_type_generic_arguments() {
            self.parse_type_generic_arguments()?
        } else {
            Vec::new()
        };

        let id = self.insert_node(
            TypeExpression::Reference {
                path: Path { segments },
                generic_arguments,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(id, segment_range);

        Ok(id)
    }

    /// Return whether type generic arguments start here.
    #[inline]
    pub(crate) fn peek_type_generic_arguments(&self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::LessThan | TokenType::ShiftLeft
        ) && !self.peek_is_on_new_line()
    }

    /// Parse type keyword dispatch.
    ///
    /// Examples:
    /// ```tspp
    /// type Name = string
    /// interface Shape { id: string }
    /// typeof value
    /// ```
    pub(super) fn parse_type_keyword_expression(
        &mut self,
        start: &ParseStart,
        keyword: Keyword,
        stop: TypeStop,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let type_expression = match keyword {
            Keyword::Type | Keyword::Newtype | Keyword::Readonly => {
                Some(self.parse_type_declaration(start, stop)?)
            }
            Keyword::Const => {
                self.bump();
                Some(self.insert_node(TypeExpression::Const, self.range_since(start)))
            }
            Keyword::This => {
                let keyword_range = self.peek_token().range();
                self.bump();
                let ty = self.insert_node(TypeExpression::This, self.range_since(start));
                self.tree.set_main_range(ty, keyword_range);

                Some(ty)
            }
            Keyword::Null => {
                self.bump();
                Some(self.insert_node(
                    TypeExpression::Keyword {
                        value: TypeLiteral::Null,
                    },
                    self.range_since(start),
                ))
            }
            Keyword::Undefined => {
                self.bump();
                Some(self.insert_node(
                    TypeExpression::Keyword {
                        value: TypeLiteral::Undefined,
                    },
                    self.range_since(start),
                ))
            }
            Keyword::Infer => Some(self.parse_type_infer(stop)?),
            Keyword::Typeof => Some(self.parse_typeof_query(start, stop)?),
            _ => None,
        };

        Ok(type_expression)
    }

    /// Parse a `typeof` type query.
    ///
    /// Examples:
    /// ```tspp
    /// typeof value
    /// typeof namespace.value
    /// typeof infer T
    /// ```
    fn parse_typeof_query(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_keyword(Keyword::Typeof)?;
        let value = self.parse_typeof_query_value(stop)?;

        Ok(self.insert_node(TypeExpression::TypeOf { value }, self.range_since(start)))
    }

    /// Parse the value operand of a `typeof` type query.
    ///
    /// Examples:
    /// ```tspp
    /// value
    /// namespace.value
    /// call().result
    /// ```
    fn parse_typeof_query_value(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_keyword() == Some(Keyword::Infer) {
            let type_expression = self.parse_type(TypePosition::Type, stop.nest())?;

            return Ok(self.insert_type_expression_value(type_expression));
        }

        if self.peek_is(TokenType::Identifier) {
            return self.parse_typeof_reference_value();
        }

        self.parse_expression_or_recover_missing(
            ExpressionPosition::TypeQuery,
            ExpressionStop::default(),
            NodeType::Expression,
        )
    }

    /// Parse an identifier or member path in a `typeof` type query.
    ///
    /// Examples:
    /// ```tspp
    /// value
    /// namespace.value
    /// namespace.value.member
    /// namespace.value<T>
    /// ```
    fn parse_typeof_reference_value(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let mut value = self.parse_identifier_expression(&start)?;

        while self.peek_typeof_reference_continuation() {
            self.bump();
            let name = self.eat_typeof_member_name()?;
            value = self.insert_node(
                Expression::Member {
                    left: value,
                    name,
                    is_optional: false,
                },
                self.range_since(&start),
            );
        }

        if self.peek_type_generic_arguments() {
            let generic_arguments = self.parse_type_generic_arguments()?;
            value = self.insert_node(
                Expression::Instantiation {
                    left: value,
                    generic_arguments,
                },
                self.range_since(&start),
            );
        }

        Ok(value)
    }

    /// Return whether the current token continues a typeof reference path.
    fn peek_typeof_reference_continuation(&self) -> bool {
        !self.peek_is_on_new_line() && self.peek_is(TokenType::Dot)
    }

    /// Eat an optional typeof member name after a dot.
    ///
    /// Examples:
    /// ```tspp
    /// name
    /// default
    /// 0
    /// ```
    fn eat_typeof_member_name(&mut self) -> ParserResult<Option<StringId>> {
        if self.peek_is(TokenType::Identifier) || self.peek_is(TokenType::Literal) {
            return self
                .eat_member_name_with_range()
                .map(|(name, _)| Some(name));
        }

        Ok(None)
    }

    /// Record path source regions for one type reference.
    pub(in crate::parse) fn record_type_path(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
        segment_ranges: &[ByteRange],
    ) -> ParserResult<()> {
        if let Some(last) = segment_ranges.last().copied() {
            self.tree.set_main_range(type_expression_id, last);
        }
        if segment_ranges.len() <= 1 {
            return Ok(());
        }

        if let Some(first) = segment_ranges.first().copied() {
            self.tree.set_head_range(type_expression_id, first);
        }

        for (index, range) in segment_ranges.iter().copied().enumerate() {
            let Ok(index) = u16::try_from(index) else {
                return Err(ParserError::unexpected(range));
            };
            self.tree.set_side_range(
                type_expression_id,
                NodeSpanType::ListItem(NodeSpanList::Segment, index),
                range,
            );
        }

        Ok(())
    }
}
