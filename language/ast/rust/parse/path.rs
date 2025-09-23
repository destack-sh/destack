use crate::{ParseError, ParseResult, Parser, PathId};
use dyst_language_source::StringId;
use dyst_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Peek a path.
    /// NOTE :Performance: peek_path uses :UnboundedLookahead
    #[inline]
    pub fn peek_path(&self) -> ParseResult<(usize, usize, usize)> {
        // first name can't be a keyword
        if self.peek_any_keyword().is_ok() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        let mut pos = self.pos() as usize;
        let start = pos;

        // identifier .identifier*
        // like `geom.Mesh`
        while let Some(token) = self.tokens.get(pos)
            && token.token.r#type == TokenType::Identifier
        {
            pos += 1;
            // keep going if there's a dot
            if let Some(token) = self.tokens.get(pos)
                && token.token.r#type == TokenType::Dot
            {
                pos += 1;
                continue;
            }
        }

        if pos > start {
            Ok((start, pos, pos - start))
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a Path.
    pub fn eat_path(&mut self) -> ParseResult<PathId> {
        let mut segments: Vec<StringId> = Vec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // zero or more `.identifier`
        // (but stop any non-[identifier/dot] token)
        loop {
            if self.peek_token(TokenType::Dot).is_ok()
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.r#type == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let seg = self.eat_identifier()?;
                segments.push(seg);
                continue;
            } else {
                break;
            }
        }

        let path_id = self.intern_path(segments);
        Ok(path_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_path;
    use crate::parse::tests::TestParser;

    #[test]
    fn test_parse_simple_path_single_segment() {
        let mut test = TestParser::new("destack");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "destack");
    }

    #[test]
    fn test_parse_simple_path_multiple_segments() {
        let mut test = TestParser::new("destack.geometry.math");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "destack.geometry.math");
    }

    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let mut test = TestParser::new("ds.geometry.{Vector2}");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "ds.geometry");
        // ensure next token is the `.` for the group
        let next = parser.peek().unwrap();
        assert_eq!(next.token.r#type, dyst_language_token::TokenType::Dot);
    }

    #[test]
    fn test_parse_path_stops_before_angle_bracket() {
        let mut test = TestParser::new("geom.Vector<Dims: 2, float32>");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "geom.Vector");
        // ensure next token is the `<` for the generic arguments
        let next = parser.peek().unwrap();
        assert_eq!(next.token.r#type, dyst_language_token::TokenType::LessThan);
    }
}
