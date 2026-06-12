use crate::parse::DeclarationHeader;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_core::StringId;
use destack_dir::{
    Expression, InferForm, Keyword, LocalNodeId, NodeType, Path, TokenType, TypeExpression,
    TypeLiteral,
};
use destack_source::{NodeSpanList, NodeSpanType, Span};
use smallvec::SmallVec;

impl Parser {
    /// Parse one identifier primary in type space.
    ///
    /// Examples:
    /// ```ds
    /// T
    /// this
    /// infer U
    /// ```
    pub(super) fn eat_type_reference_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // infer hole
        if self.current_token_starts_infer_hole() {
            return Ok(self.eat_type_infer_hole(start));
        }

        // keyword identifiers
        if let Some(type_expression) = self.eat_type_identifier_keyword(start)? {
            return Ok(type_expression);
        }

        // literal types
        if let Some(type_expression) = self.eat_type_literal_primary(start)? {
            return Ok(type_expression);
        }

        // path reference
        self.eat_type_path_reference(start)
    }

    /// Return whether the current token starts a Destack infer hole.
    fn current_token_starts_infer_hole(&self) -> bool {
        self.language.is_destack() && self.current_identifier_str_is("_")
    }

    /// Eat a Destack infer hole.
    ///
    /// Examples:
    /// ```ds
    /// _
    /// _[]
    /// Promise<_>
    /// ```
    pub(super) fn eat_type_infer_hole(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<TypeExpression> {
        let name_span = self.current_token().span(self.file_id);
        self.bump();
        let id = self.insert_node(
            TypeExpression::Infer {
                form: InferForm::Hole,
                name: None,
                constraint: None,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(id, name_span);

        id
    }

    /// Eat type primary keywords that appear in identifier position.
    ///
    /// Examples:
    /// ```ds
    /// this
    /// infer T
    /// intrinsic
    /// ```
    fn eat_type_identifier_keyword(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if self.current_keyword() == Some(Keyword::This) {
            self.bump();
            return Ok(Some(
                self.insert_node(TypeExpression::This, self.get_span_from(start)),
            ));
        }

        if self.current_keyword() == Some(Keyword::Infer) {
            return self.eat_type_infer_expression().map(Some);
        }

        if self.current_identifier_str_is("intrinsic")
            && Self::is_type_expression_boundary_token(self.next_token_type())
        {
            self.bump();
            return Ok(Some(self.insert_node(
                TypeExpression::Intrinsic,
                self.get_span_from(start),
            )));
        }

        Ok(None)
    }

    /// Eat a literal type primary when present.
    ///
    /// Examples:
    /// ```ds
    /// null
    /// undefined
    /// "open"
    /// ```
    fn eat_type_literal_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let Some(literal) = self.peek_type_literal().ok() else {
            return Ok(None);
        };

        let literal = self.eat_type_literal(Some(literal))?;
        let type_expression = self.insert_node(
            TypeExpression::Literal { value: literal },
            self.get_span_from(start),
        );

        Ok(Some(type_expression))
    }

    /// Eat a path reference type.
    ///
    /// Examples:
    /// ```ds
    /// User
    /// namespace.User
    /// Result<string, Error>
    /// ```
    fn eat_type_path_reference(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let (first, first_span) = self.eat_identifier_with_span()?;

        if !self.path_continues_to_identifier(false) {
            return self.eat_single_segment_type_path_reference(start, first, first_span);
        }

        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
        let mut segment_spans: SmallVec<[Span; 3]> = SmallVec::new();
        segments.push(first);
        segment_spans.push(first_span);

        while self.path_continues_to_identifier(false) {
            self.bump();
            let (segment, segment_span) = self.eat_identifier_with_span()?;
            segments.push(segment);
            segment_spans.push(segment_span);
        }

        let path = Path { segments };
        let generic_arguments = if self.type_generic_arguments_start_here() {
            self.eat_type_generic_arguments()?
        } else {
            Vec::new()
        };
        let id = self.insert_node(
            TypeExpression::Reference {
                path,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.set_path_type_expression_spans(id, &segment_spans)?;

        Ok(id)
    }

    /// Eat a single-segment path reference type.
    ///
    /// Examples:
    /// ```ds
    /// User
    /// Result<T>
    /// Promise<string>
    /// ```
    fn eat_single_segment_type_path_reference(
        &mut self,
        start: &ParserSpanStart,
        segment: StringId,
        segment_span: Span,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
        segments.push(segment);

        let generic_arguments = if self.type_generic_arguments_start_here() {
            self.eat_type_generic_arguments()?
        } else {
            Vec::new()
        };

        let id = self.insert_node(
            TypeExpression::Reference {
                path: Path { segments },
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(id, segment_span);

        Ok(id)
    }

    /// Return whether type generic arguments start here.
    #[inline]
    pub(crate) fn type_generic_arguments_start_here(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::LessThan | TokenType::ShiftLeft
        ) && !self.current_token_is_on_new_line()
    }

    /// Parse type keyword dispatch.
    ///
    /// Examples:
    /// ```ds
    /// type Name = string
    /// interface Shape { id: string }
    /// typeof value
    /// ```
    pub(super) fn eat_type_keyword_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let header = DeclarationHeader::default();
        let type_expression = match keyword {
            Keyword::Type | Keyword::Newtype | Keyword::Readonly => {
                Some(self.eat_type(start, header)?)
            }
            Keyword::Const => {
                self.bump();
                Some(self.insert_node(TypeExpression::Const, self.get_span_from(start)))
            }
            Keyword::This => {
                self.bump();
                Some(self.insert_node(TypeExpression::This, self.get_span_from(start)))
            }
            Keyword::Null => {
                self.bump();
                Some(self.insert_node(
                    TypeExpression::Literal {
                        value: TypeLiteral::Null,
                    },
                    self.get_span_from(start),
                ))
            }
            Keyword::Undefined => {
                self.bump();
                Some(self.insert_node(
                    TypeExpression::Literal {
                        value: TypeLiteral::Undefined,
                    },
                    self.get_span_from(start),
                ))
            }
            Keyword::Infer => Some(self.eat_type_infer_expression()?),
            Keyword::Typeof => Some(self.eat_typeof_query(start)?),
            _ => None,
        };

        Ok(type_expression)
    }

    /// Parse a `typeof` type query.
    ///
    /// Examples:
    /// ```ds
    /// typeof value
    /// typeof namespace.value
    /// typeof infer T
    /// ```
    fn eat_typeof_query(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_keyword(Keyword::Typeof)?;
        let value = self.eat_typeof_query_value()?;

        Ok(self.insert_node(
            TypeExpression::TypeOfValue { value },
            self.get_span_from(start),
        ))
    }

    /// Eat the value operand of a `typeof` type query.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// namespace.value
    /// call().result
    /// ```
    fn eat_typeof_query_value(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        if self.current_keyword() == Some(Keyword::Infer) {
            let type_expression = self.eat_type_expression()?;

            return Ok(self.insert_type_expression_value(type_expression));
        }

        if self.peek_is(TokenType::Identifier) {
            return self.eat_typeof_reference_value();
        }

        let value_flags = self
            .flags
            .not_in_position()
            .with_type(false)
            .in_typeof_query();

        self.eat_expression_or_recover_missing(value_flags, NodeType::Expression)
    }

    /// Eat an identifier or member path in a `typeof` type query.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// namespace.value
    /// namespace.value.member
    /// ```
    fn eat_typeof_reference_value(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        let mut value = self.eat_identifier_expression_path(&start)?;

        while self.current_token_continues_typeof_reference() {
            self.bump();
            let name = self.eat_typeof_member_name()?;
            value = self.insert_node(
                Expression::Member { left: value, name },
                self.get_span_from(&start),
            );
        }

        if let Some(generic_arguments) = self.eat_typeof_instantiation_arguments() {
            value = self.insert_node(
                Expression::Instantiation {
                    left: value,
                    generic_arguments,
                },
                self.get_span_from(&start),
            );
        }

        Ok(value)
    }

    /// Eat type query instantiation arguments when present.
    ///
    /// Examples:
    /// ```ds
    /// <string>
    /// <string, number>
    /// <<T>() => T>
    /// ```
    fn eat_typeof_instantiation_arguments(
        &mut self,
    ) -> Option<Vec<LocalNodeId<destack_dir::GenericArgument>>> {
        if self.current_token_is_on_new_line() {
            return None;
        }

        if !matches!(
            self.peek_token_type(),
            TokenType::LessThan | TokenType::ShiftLeft
        ) {
            return None;
        }

        self.eat_generic_arguments_if_valid(true)
    }

    /// Return whether the current token continues a typeof reference path.
    fn current_token_continues_typeof_reference(&mut self) -> bool {
        !self.current_token_is_on_new_line() && self.peek_is(TokenType::Dot)
    }

    /// Eat an optional typeof member name after a dot.
    ///
    /// Examples:
    /// ```ds
    /// name
    /// default
    /// 0
    /// ```
    fn eat_typeof_member_name(&mut self) -> ParserResult<Option<destack_core::StringId>> {
        if self.peek_is(TokenType::Identifier) || self.peek_is(TokenType::Literal) {
            return self.eat_member_name_with_span().map(|(name, _)| Some(name));
        }

        Ok(None)
    }

    /// Set path spans for a type reference.
    fn set_path_type_expression_spans(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
        segment_spans: &[Span],
    ) -> ParserResult<()> {
        if let Some(last) = segment_spans.last().copied() {
            self.tree.set_main_span(type_expression_id, last);
        }
        if segment_spans.len() <= 1 {
            return Ok(());
        }

        if let Some(first) = segment_spans.first().copied() {
            self.tree.set_head_span(type_expression_id, first);
        }

        for (index, span) in segment_spans.iter().copied().enumerate() {
            let Ok(index) = u16::try_from(index) else {
                return Err(ParserError::unexpected(span));
            };
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::ListItem(NodeSpanList::Segment, index),
                span,
            );
        }

        Ok(())
    }
}
