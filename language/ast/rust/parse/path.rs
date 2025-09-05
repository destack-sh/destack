use crate::{ParseResult, Parser, PathId};
use destack_language_arena::StringId;
use destack_language_token::TokenType;

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
    use destack_language_token::{SourceFile, TokenType, tokenize_semantic};

    use crate::Parser;
    use crate::parse::tests::TestParse;

    /// Test that the parser can parse a path with a single segment.
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

    /// Test that the parser can parse a path with multiple segments.
    #[test]
    fn test_parse_simple_path_multiple_segments() {
        let input = "destack.geometry.math";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
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
    /// Test that the parser stops before non-path items (like for Use items).
    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let input = "ds.geometry.{Vector2}";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
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
        assert_eq!(next.token.r#type, TokenType::Dot);
    }

    /// Test that the parser stops before non-path items (like for generic arguments).
    #[test]
    fn test_parse_path_stops_before_angle_bracket() {
        let input = "geom.Vector<Dims: 2, float32>";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
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
        assert_eq!(next.token.r#type, TokenType::LessThan);
    }
}
