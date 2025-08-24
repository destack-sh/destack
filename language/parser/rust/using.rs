use destack_language_lexer::TokenType;

use crate::{Keyword, ParseResult, Parser, Using, UsingItem};

impl<'a> Parser<'a> {
    /// Eat a using declaration (including keyword and semicolon).
    pub fn eat_using(&mut self) -> ParseResult<Using> {
        self.eat_keyword(Keyword::Using)?;
        let using = self.eat_using_content()?;
        self.eat_semicolon()?;
        Ok(using)
    }

    /// Eat the content of a using declaration (without the `using` keyword).
    pub fn eat_using_content(&mut self) -> ParseResult<Using> {
        let path = self.eat_path()?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_token_type(TokenType::Dot).is_ok() {
            self.eat_token_type(TokenType::Dot)?;
            self.eat_token_type(TokenType::OpenBrace)?;

            // parse zero or more items
            let mut items: Vec<UsingItem> = Vec::new();
            // empty group `.{}` is valid
            if self.peek_token_type(TokenType::CloseBrace).is_err() {
                loop {
                    let item = self.eat_using_item()?;
                    items.push(item);
                    if self.peek_token_type(TokenType::Comma).is_ok() {
                        self.eat_token_type(TokenType::Comma)?;
                        // allow trailing comma
                        if self.peek_token_type(TokenType::CloseBrace).is_ok() {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }

            self.eat_token_type(TokenType::CloseBrace)?;
            Some(items)
        } else {
            None
        };

        // optional alias `as Ident` (only when no grouped items were present)
        let alias = if items.is_none() {
            if let Ok(next) = self.peek_token_type(TokenType::Identifier) {
                if self.get_token_str(*next) == Keyword::As.as_str() {
                    self.eat_keyword(Keyword::As)?;
                    Some(self.eat_identifier()?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Using { path, alias, items })
    }

    /// Eat a using item (like `geometry` or `geometry as geom`).
    pub fn eat_using_item(&mut self) -> ParseResult<UsingItem> {
        let name = self.eat_identifier()?;
        let alias = if let Ok(next) = self.peek_token_type(TokenType::Identifier) {
            if self.get_token_str(*next) == Keyword::As.as_str() {
                self.eat_keyword(Keyword::As)?;
                Some(self.eat_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(UsingItem { name, alias })
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::{SourceFile, tokenize_semantic};

    use crate::{Parser, Path, PathSegment, Using, UsingItem};

    #[test]
    fn test_using() {
        let input = r##"
using destack;
using destack.geometry;
using destack as ds;
using ds.geometry as geom;
using ds.geometry.{Vector2, Vector3 as V3};
"##;
        let tokens = tokenize_semantic(input);
        println!("{tokens:?}");
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        // using destack;
        let u1 = parser.eat_using().unwrap();
        assert_eq!(
            u1,
            Using {
                path: Path {
                    segments: vec![PathSegment {
                        name: "destack".to_string()
                    }],
                },
                alias: None,
                items: None,
            }
        );

        // using destack.geometry;
        let u2 = parser.eat_using().unwrap();
        assert_eq!(
            u2,
            Using {
                path: Path {
                    segments: vec![
                        PathSegment {
                            name: "destack".to_string()
                        },
                        PathSegment {
                            name: "geometry".to_string()
                        },
                    ],
                },
                alias: None,
                items: None,
            }
        );

        // using destack as ds;
        let u3 = parser.eat_using().unwrap();
        assert_eq!(
            u3,
            Using {
                path: Path {
                    segments: vec![PathSegment {
                        name: "destack".to_string()
                    }],
                },
                alias: Some("ds".to_string()),
                items: None,
            }
        );

        // using ds.geometry as geom;
        let u4 = parser.eat_using().unwrap();
        assert_eq!(
            u4,
            Using {
                path: Path {
                    segments: vec![
                        PathSegment {
                            name: "ds".to_string()
                        },
                        PathSegment {
                            name: "geometry".to_string()
                        },
                    ],
                },
                alias: Some("geom".to_string()),
                items: None,
            }
        );

        // using ds.geometry.{Vector2, Vector3 as V3};
        let u5 = parser.eat_using().unwrap();
        assert_eq!(
            u5,
            Using {
                path: Path {
                    segments: vec![
                        PathSegment {
                            name: "ds".to_string()
                        },
                        PathSegment {
                            name: "geometry".to_string()
                        },
                    ],
                },
                alias: None,
                items: Some(vec![
                    UsingItem {
                        name: "Vector2".to_string(),
                        alias: None
                    },
                    UsingItem {
                        name: "Vector3".to_string(),
                        alias: Some("V3".to_string())
                    },
                ]),
            }
        );
    }
}
