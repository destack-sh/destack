use crate::{ParseResult, Parser, PathId};
use dyst_language_arena::StringId;
use dyst_language_token::TokenType;

impl<'a> Parser<'a> {
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

        let path_id = self.paths.intern(segments);
        Ok(path_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParse;

    #[test]
    fn test_parse_simple_path_single_segment() {
        let test = TestParse::new("destack");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            parser.paths.intern(vec![parser.strings.intern("destack")])
        );
    }

    #[test]
    fn test_parse_simple_path_multiple_segments() {
        let test = TestParse::new("destack.geometry.math");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            parser.paths.intern(vec![
                parser.strings.intern("destack"),
                parser.strings.intern("geometry"),
                parser.strings.intern("math")
            ])
        );
    }

    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let test = TestParse::new("ds.geometry.{Vector2}");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            parser.paths.intern(vec![
                parser.strings.intern("ds"),
                parser.strings.intern("geometry")
            ])
        );
        // ensure next token is the `.` for the group
        let next = parser.peek().unwrap();
        assert_eq!(next.token.r#type, dyst_language_token::TokenType::Dot);
    }

    #[test]
    fn test_parse_path_stops_before_angle_bracket() {
        let test = TestParse::new("geom.Vector<Dims: 2, float32>");
        let mut parser = test.parser();
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            parser.paths.intern(vec![
                parser.strings.intern("geom"),
                parser.strings.intern("Vector")
            ])
        );
        // ensure next token is the `<` for the generic arguments
        let next = parser.peek().unwrap();
        assert_eq!(next.token.r#type, dyst_language_token::TokenType::LessThan);
    }
}
