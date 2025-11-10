use dyst_source::{SmallVec, StringId};

use crate::{Parser, ParserResult};
use dyst_ast::{Path, TokenType};

impl<'a> Parser<'a> {
    /// Eat a Path.
    pub fn eat_path(&mut self) -> ParserResult<Path> {
        let mut segments: SmallVec<StringId, 3> = SmallVec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // zero or more `.identifier` (ignoring newlines)
        loop {
            // dot followed by identifier
            if self.peek_token(TokenType::Dot).is_ok()
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let seg = self.eat_identifier()?;
                segments.push(seg);
            }
            // newline followed by dot
            else if self.peek_token(TokenType::Newline).is_ok()
                && self
                    .peek_token_after_newlines(self.pos(), TokenType::Dot)
                    .is_ok()
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
}

#[cfg(test)]
mod tests {
    use dyst_ast::TokenType;

    use crate::assert_path;
    use crate::parse::tests::TestParser;

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
