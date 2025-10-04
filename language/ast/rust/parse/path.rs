use crate::{AstResult, Parser, PathId};
use dyst_source::StringId;
use dyst_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat a Path.
    pub fn eat_path(&mut self) -> AstResult<PathId> {
        let mut segments: Vec<StringId> = Vec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(first);

        // zero or more `.identifier`
        // (but stop any non-[identifier/dot] token)
        loop {
            if self.peek_token(TokenType::Dot).is_ok()
                && let Ok(after_dot) = self.peek_next()
                && after_dot.token.ty == TokenType::Identifier
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
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "destack");
    }

    #[test]
    fn test_parse_simple_path_multiple_segments() {
        let mut test = TestParser::new("destack.geometry.math");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "destack.geometry.math");
    }

    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let mut test = TestParser::new("ds.geometry.{Vector2}");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "ds.geometry");
        // ensure next token is the `.` for the group
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, dyst_token::TokenType::Dot);
    }

    #[test]
    fn test_parse_path_stops_before_angle_bracket() {
        let mut test = TestParser::new("geom.Vector<Dims: 2, float32>");
        let mut parser = test.prepare();
        let path = parser.eat_path().unwrap();
        assert_path!(parser.session, path, "geom.Vector");
        // ensure next token is the `<` for the generic arguments
        let next = parser.peek().unwrap();
        assert_eq!(next.token.ty, dyst_token::TokenType::LessThan);
    }
}
