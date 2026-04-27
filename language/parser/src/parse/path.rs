use destack_core::StringId;
use destack_source::{NodeSpanList, NodeSpanType, Span};
use smallvec::SmallVec;

use crate::{ParseResult, Parser};
use destack_ast::{Expression, LocalNodeId, Path, TokenType};

impl Parser {
    /// Eat a path.
    pub fn eat_path(&mut self) -> ParseResult<Path> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // single segment fast path
        if !self.peek_is(TokenType::Dot) || self.current_token_has_leading_comment() {
            return Ok(Path { segments });
        }

        // zero or more `.identifier`
        while self.peek_is(TokenType::Dot)
            && !self.current_token_has_leading_comment()
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_identifier_is() && !parser.current_token_has_leading_comment()
            })
        {
            self.bump();
            let segment = self.eat_identifier()?;
            segments.push(segment);
        }

        Ok(Path { segments })
    }

    /// Eat a path and get its span.
    pub fn eat_path_with_span(&mut self) -> ParseResult<(Path, Span)> {
        let start = self.span_start();
        let path = self.eat_path()?;
        let span = self.get_span_from(&start);
        Ok((path, span))
    }

    /// Eat a path and return the span of its last segment.
    pub fn eat_path_with_last_span(&mut self) -> ParseResult<(Path, Span)> {
        let (path, _segment_spans, last_span) = self.eat_path_with_endpoint_spans()?;
        Ok((path, last_span))
    }

    /// Eat a path and return the spans of all its segments.
    pub fn eat_path_with_segment_spans(&mut self) -> ParseResult<(Path, SmallVec<[Span; 3]>)> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();
        let mut segment_spans: SmallVec<[Span; 3]> = SmallVec::new();

        // first identifier
        let (first, first_span) = self.eat_identifier_with_span()?;
        segments.push(first);
        segment_spans.push(first_span);

        // single segment fast path
        if !self.peek_is(TokenType::Dot) || self.current_token_has_leading_comment() {
            return Ok((Path { segments }, segment_spans));
        }

        // zero or more `.identifier`
        while self.peek_is(TokenType::Dot)
            && !self.current_token_has_leading_comment()
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_identifier_is() && !parser.current_token_has_leading_comment()
            })
        {
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
        let last_span = segment_spans.last().copied().expect("path has no segments");

        Ok((path, segment_spans, last_span))
    }

    /// Eat a tree literal path.
    pub fn eat_tree_literal_path(&mut self) -> ParseResult<Path> {
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier (kebab-case supported)
        let first = self.eat_tree_literal_identifier()?;
        segments.push(first);

        // single segment fast path
        if !self.peek_is(TokenType::Dot) {
            return Ok(Path { segments });
        }

        // zero or more `.identifier`
        while self.peek_is(TokenType::Dot)
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_identifier_is()
            })
        {
            self.bump();
            let segment = self.eat_tree_literal_identifier()?;
            segments.push(segment);
        }

        Ok(Path { segments })
    }

    /// Eat a tree literal path and return the span of its last segment.
    pub fn eat_tree_literal_path_with_last_span(&mut self) -> ParseResult<(Path, Span)> {
        let (path, _segment_spans, last_span) = self.eat_tree_literal_path_with_endpoint_spans()?;
        Ok((path, last_span))
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

        // single segment fast path
        if !self.peek_is(TokenType::Dot) {
            return Ok((Path { segments }, segment_spans));
        }

        // zero or more `.identifier`
        while self.peek_is(TokenType::Dot)
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_identifier_is()
            })
        {
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
        let last_span = segment_spans.last().copied().expect("path has no segments");

        Ok((path, segment_spans, last_span))
    }

    /// Record the identifier spans for one path expression.
    pub fn set_path_expression_spans(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        segment_spans: &[Span],
    ) {
        let first_span = *segment_spans.first().expect("path has no segments");
        let last_span = *segment_spans.last().expect("path has no segments");

        self.tree.set_main_span(expression_id, last_span);
        self.tree.set_head_span(expression_id, first_span);

        // record each path segment so semantic consumers can target the exact token
        for (index, segment_span) in segment_spans.iter().copied().enumerate() {
            let segment_index =
                u16::try_from(index).expect("path expression segment index overflow");

            self.tree.set_side_span(
                expression_id,
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                segment_span,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::TokenType;

    use crate::{TestParser, assert_path};

    #[test]
    fn test_parse_simple_path_single_segment() {
        let mut test = TestParser::new("destack");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "destack");
    }

    #[test]
    fn test_parse_simple_path_multiple_segments() {
        let mut test = TestParser::new("destack.geometry.math");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "destack.geometry.math");
    }

    #[test]
    fn test_parse_simple_path_multiple_segments_with_newline() {
        let mut test = TestParser::new("destack\n.geometry\n.math\n");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "destack.geometry.math");
    }

    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let mut test = TestParser::new("ds.geometry.{Vector2}");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "ds.geometry");
        // ensure next token is the `.` for the group
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, TokenType::Dot);
    }

    #[test]
    fn test_parse_path_stops_before_angle_bracket() {
        let mut test = TestParser::new("geom.Vector<Dims: 2, float32>");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "geom.Vector");
        // ensure next token is the `<` for the generic arguments
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, TokenType::LessThan);
    }

    #[test]
    fn test_parse_path_stops_before_dot_with_leading_comment() {
        let mut test = TestParser::new("source /* hop */ .first()");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "source");
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, TokenType::Dot);
    }

    #[test]
    fn test_parse_path_stops_before_identifier_with_leading_comment_after_dot() {
        let mut test = TestParser::new("source. /* hop */ first()");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser, path, "source");
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, TokenType::Dot);
    }
}
