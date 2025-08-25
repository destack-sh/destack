use crate::{ParseResult, Parser, Path, PathSegment};
use destack_language_lexer::TokenType;

impl<'a> Parser<'a> {
    /// Eat a Path.
    pub fn eat_path(&mut self) -> ParseResult<Path> {
        let mut segments = Vec::new();

        // first identifier
        let first = self.eat_identifier()?;
        segments.push(PathSegment { name: first });

        // zero or more `.identifier`
        // (but stop before `.{` used by grouped using)
        loop {
            if self.peek_next_token(TokenType::Dot).is_ok()
                && let Ok(after_dot) = self.peek_next_next()
                && after_dot.token.r#type == TokenType::Identifier
            {
                self.eat_token(TokenType::Dot)?;
                let seg = self.eat_identifier()?;
                segments.push(PathSegment { name: seg });
                continue;
            } else {
                break;
            }
        }

        Ok(Path { segments })
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::{SourceFile, TokenType, tokenize_semantic};

    use crate::{Parser, Path, PathSegment};

    /// Test that the parser can parse a path with a single segment.
    #[test]
    fn test_parse_simple_path_single_segment() {
        let input = "destack";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            Path {
                segments: vec![PathSegment {
                    name: "destack".to_string()
                }]
            }
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
            Path {
                segments: vec![
                    PathSegment {
                        name: "destack".to_string()
                    },
                    PathSegment {
                        name: "geometry".to_string()
                    },
                    PathSegment {
                        name: "math".to_string()
                    },
                ],
            }
        );
    }

    /// Test that the parser stops before non-path items (like for Using items).
    #[test]
    fn test_parse_path_stops_before_group_brace() {
        let input = "ds.geometry.{Vector2}";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let path = parser.eat_path().unwrap();
        assert_eq!(
            path,
            Path {
                segments: vec![
                    PathSegment {
                        name: "ds".to_string()
                    },
                    PathSegment {
                        name: "geometry".to_string()
                    },
                ],
            }
        );
        // ensure next token is the `.` for the group
        let next = parser.peek_next().unwrap();
        assert_eq!(next.token.r#type, TokenType::Dot);
    }
}
