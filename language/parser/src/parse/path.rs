use destack_base::StringId;
use destack_source::Span;
use smallvec::SmallVec;

use crate::parse::timing::tags;
use crate::{ParseResult, Parser};
use destack_ast::{Path, TokenType};

impl Parser {
    /// Eat a path.
    pub fn eat_path(&mut self) -> ParseResult<Path> {
        let _timing = self.timing_scope(tags::PARSE_PATH);
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // zero or more `.identifier` (ignoring newlines)
        while self.has_more_tokens() {
            // dot followed by identifier
            if self.peek_is(TokenType::Dot)
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let seg = self.eat_identifier()?;
                segments.push(seg);
            }
            // newline followed by dot
            else if self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Dot)
            {
                self.eat_newlines_maybe()?;
            }
            // end of path
            else {
                break;
            }
        }

        let path = Path { segments };
        Ok(path)
    }

    /// Eat a path and get its span.
    pub fn eat_path_with_span(&mut self) -> ParseResult<(Path, Span)> {
        let start = self.mark();
        let path = self.eat_path()?;
        let span = self.get_span_from(&start);
        Ok((path, span))
    }

    /// Eat a path and return the span of its last segment.
    pub fn eat_path_with_last_span(&mut self) -> ParseResult<(Path, Span)> {
        let _timing = self.timing_scope(tags::PARSE_PATH);
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier
        let (first, first_span) = self.eat_identifier_with_span()?;
        segments.push(first);
        let mut last_span = first_span;

        // zero or more `.identifier` (ignoring newlines)
        while self.has_more_tokens() {
            // dot followed by identifier
            if self.peek_is(TokenType::Dot)
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let (seg, seg_span) = self.eat_identifier_with_span()?;
                segments.push(seg);
                last_span = seg_span;
            }
            // newline followed by dot
            else if self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Dot)
            {
                self.eat_newlines_maybe()?;
            }
            // end of path
            else {
                break;
            }
        }

        // build the path
        let path = Path { segments };
        Ok((path, last_span))
    }

    /// Eat a tree literal path.
    pub fn eat_tree_literal_path(&mut self) -> ParseResult<Path> {
        let _timing = self.timing_scope(tags::PARSE_PATH);
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier (kebab-case supported)
        let first = self.eat_tree_literal_identifier()?;
        segments.push(first);

        // zero or more `.identifier` (ignoring newlines)
        while self.has_more_tokens() {
            // dot followed by identifier
            if self.peek_is(TokenType::Dot)
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let seg = self.eat_tree_literal_identifier()?;
                segments.push(seg);
            }
            // newline followed by dot
            else if self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Dot)
            {
                self.eat_newlines_maybe()?;
            }
            // end of path
            else {
                break;
            }
        }

        let path = Path { segments };
        Ok(path)
    }

    /// Eat a tree literal path and return the span of its last segment.
    pub fn eat_tree_literal_path_with_last_span(&mut self) -> ParseResult<(Path, Span)> {
        let _timing = self.timing_scope(tags::PARSE_PATH);
        let mut segments: SmallVec<[StringId; 3]> = SmallVec::new();

        // first identifier (kebab-case supported)
        let (first, first_span) = self.eat_tree_literal_identifier_with_span()?;
        segments.push(first);
        let mut last_span = first_span;

        // zero or more `.identifier` (ignoring newlines)
        while self.has_more_tokens() {
            // dot followed by identifier
            if self.peek_is(TokenType::Dot)
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let (seg, seg_span) = self.eat_tree_literal_identifier_with_span()?;
                segments.push(seg);
                last_span = seg_span;
            }
            // newline followed by dot
            else if self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), TokenType::Dot)
            {
                self.eat_newlines_maybe()?;
            }
            // end of path
            else {
                break;
            }
        }

        let path = Path { segments };
        Ok((path, last_span))
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
}
