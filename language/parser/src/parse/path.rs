use destack_core::StringId;
use destack_source::Span;
use smallvec::SmallVec;

use crate::{Parser, ParserError, ParserResult};
use destack_dir::{Expression, LocalNodeId, Path, TokenSpan, TokenType};

impl Parser {
    /// Eat a path.
    pub fn eat_path(&mut self) -> ParserResult<Path> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();

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
    pub fn eat_path_with_segment_spans(&mut self) -> ParserResult<(Path, SmallVec<[Span; 3]>)> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
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
    ) -> ParserResult<(Path, SmallVec<[Span; 3]>, Span)> {
        let (path, segment_spans) = self.eat_path_with_segment_spans()?;
        let Some(last_span) = segment_spans.last().copied() else {
            return Err(ParserError::unexpected(self.anchor_span_here()));
        };

        Ok((path, segment_spans, last_span))
    }

    /// Eat a tree literal path.
    pub fn eat_tree_literal_path(&mut self) -> ParserResult<Path> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();

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
    ) -> ParserResult<(Path, SmallVec<[Span; 3]>)> {
        let mut segments: SmallVec<[StringId; 1]> = SmallVec::new();
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
    ) -> ParserResult<(Path, SmallVec<[Span; 3]>, Span)> {
        let (path, segment_spans) = self.eat_tree_literal_path_with_segment_spans()?;
        let Some(last_span) = segment_spans.last().copied() else {
            return Err(ParserError::unexpected(self.anchor_span_here()));
        };

        Ok((path, segment_spans, last_span))
    }

    /// Build a nested member chain expression from path segments and their spans.
    pub(in crate::parse) fn build_member_chain(
        &mut self,
        segments: &[StringId],
        segment_spans: &[Span],
    ) -> LocalNodeId<Expression> {
        // the path root is a bare identifier
        let root_span = segment_spans[0];
        let mut node = self.insert_node(Expression::Identifier { name: segments[0] }, root_span);
        self.tree.set_main_span(node, root_span);

        // extend through each member segment, spanning from the path start to the segment
        for (name, span) in segments[1..].iter().zip(&segment_spans[1..]) {
            let combined = Span::new(root_span.file, root_span.start, span.end);
            node = self.insert_node(
                Expression::Member {
                    left: node,
                    name: Some(*name),
                },
                combined,
            );
            self.tree.set_main_span(node, *span);
        }

        node
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

        let dot_end = self.current_token().end();
        let next = self.token_at_offset(1);
        if next.ty() != TokenType::Identifier {
            return false;
        }

        let next = TokenSpan::new(next, self.file_id);
        allows_leading_comment || !self.token_has_leading_comment_after(dot_end, next)
    }
}
