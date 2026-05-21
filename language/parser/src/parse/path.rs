use destack_core::StringId;
use destack_source::{NodeSpanList, NodeSpanType, Span};
use smallvec::SmallVec;

use crate::{ParseError, ParseResult, Parser};
use destack_dir::{Expression, LocalNodeId, Path, TokenType};

impl Parser {
    /// Eat a path.
    pub fn eat_path(&mut self) -> ParseResult<Path> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // zero or more `.identifier`
        while self.path_continues_to_identifier(false) {
            self.bump();
            let segment = self.eat_identifier()?;
            segments.push(segment);
        }

        Ok(Path { segments })
    }

    /// Eat a path and return the spans of all its segments.
    pub fn eat_path_with_segment_spans(&mut self) -> ParseResult<(Path, SmallVec<[Span; 3]>)> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();
        let mut segment_spans: SmallVec<[Span; 3]> = SmallVec::new();

        // first identifier
        let (first, first_span) = self.eat_identifier_with_span()?;
        segments.push(first);
        segment_spans.push(first_span);

        // zero or more `.identifier`
        while self.path_continues_to_identifier(false) {
            self.bump();
            let (segment, segment_span) = self.eat_identifier_with_span()?;
            segments.push(segment);
            segment_spans.push(segment_span);
        }

        Ok((Path { segments }, segment_spans))
    }

    /// Eat a path and return the spans of its first and last segments.
    pub fn eat_path_with_endpoint_spans(
        &mut self,
    ) -> ParseResult<(Path, SmallVec<[Span; 3]>, Span)> {
        let (path, segment_spans) = self.eat_path_with_segment_spans()?;
        let Some(last_span) = segment_spans.last().copied() else {
            return Err(ParseError::unexpected(self.anchor_span_here()));
        };

        Ok((path, segment_spans, last_span))
    }

    /// Eat a tree literal path.
    pub fn eat_tree_literal_path(&mut self) -> ParseResult<Path> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier (kebab-case supported)
        let first = self.eat_tree_literal_identifier()?;
        segments.push(first);

        // zero or more `.identifier`
        while self.path_continues_to_identifier(true) {
            self.bump();
            let segment = self.eat_tree_literal_identifier()?;
            segments.push(segment);
        }

        Ok(Path { segments })
    }

    /// Eat a tree literal path and return the spans of all its segments.
    pub fn eat_tree_literal_path_with_segment_spans(
        &mut self,
    ) -> ParseResult<(Path, SmallVec<[Span; 3]>)> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();
        let mut segment_spans: SmallVec<[Span; 3]> = SmallVec::new();

        // first identifier (kebab-case supported)
        let (first, first_span) = self.eat_tree_literal_identifier_with_span()?;
        segments.push(first);
        segment_spans.push(first_span);

        // zero or more `.identifier`
        while self.path_continues_to_identifier(true) {
            self.bump();
            let (segment, segment_span) = self.eat_tree_literal_identifier_with_span()?;
            segments.push(segment);
            segment_spans.push(segment_span);
        }

        Ok((Path { segments }, segment_spans))
    }

    /// Eat a tree literal path and return the spans of its first and last segments.
    pub fn eat_tree_literal_path_with_endpoint_spans(
        &mut self,
    ) -> ParseResult<(Path, SmallVec<[Span; 3]>, Span)> {
        let (path, segment_spans) = self.eat_tree_literal_path_with_segment_spans()?;
        let Some(last_span) = segment_spans.last().copied() else {
            return Err(ParseError::unexpected(self.anchor_span_here()));
        };

        Ok((path, segment_spans, last_span))
    }

    /// Record the identifier spans for one path expression.
    pub fn set_path_expression_spans(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        segment_spans: &[Span],
    ) -> ParseResult<()> {
        let Some(first_span) = segment_spans.first().copied() else {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        };
        let Some(last_span) = segment_spans.last().copied() else {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        };

        self.tree.set_main_span(expression_id, last_span);

        if segment_spans.len() <= 1 {
            return Ok(());
        }

        self.tree.set_head_span(expression_id, first_span);

        // record each path segment so semantic consumers can target the exact token
        for (index, segment_span) in segment_spans.iter().copied().enumerate() {
            let Ok(segment_index) = u16::try_from(index) else {
                return Err(ParseError::unexpected(segment_span));
            };

            self.tree.set_side_span(
                expression_id,
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                segment_span,
            );
        }

        Ok(())
    }

    /// Return whether the current dot continues a static path.
    pub(in crate::parse) fn path_continues_to_identifier(
        &mut self,
        allows_leading_comment: bool,
    ) -> bool {
        if !self.peek_is(TokenType::Dot) {
            return false;
        }

        if !allows_leading_comment && self.current_token_has_leading_comment() {
            return false;
        }

        let dot_end = self.current_token().span.end;
        let next = self.token_at_offset(1);
        if next.token.ty != TokenType::Identifier {
            return false;
        }

        allows_leading_comment || !self.token_has_leading_comment_after(dot_end, next)
    }
}
