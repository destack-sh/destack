//! Parse using declarations.

use destack_language_token::TokenType;

use crate::{Keyword, ParseResult, Parser, UsingItemNode, UsingNode};

impl<'a> Parser<'a> {
    /// Eat a using declaration (including keyword and semicolon or block).
    pub fn eat_using(&mut self) -> ParseResult<UsingNode> {
        self.eat_keyword(Keyword::Using)?;
        let using = self.eat_using_header()?;
        self.eat_stop()?;
        Ok(using)
    }

    /// Eat the content of a using declaration (without the `using` keyword).
    pub fn eat_using_header(&mut self) -> ParseResult<UsingNode> {
        let path = self.eat_path()?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_next_token(TokenType::Dot).is_ok() {
            self.eat_token(TokenType::Dot)?;
            self.eat_token(TokenType::OpenBrace)?;

            // parse zero or more items
            // (empty group `.{}` is valid)
            let mut items: Vec<UsingItemNode> = Vec::new();
            if self.peek_next_token(TokenType::CloseBrace).is_err() {
                loop {
                    let item = self.eat_using_item()?;
                    items.push(item);
                    if self.peek_next_token(TokenType::Comma).is_ok() {
                        self.eat_token(TokenType::Comma)?;
                        // allow trailing comma
                        if self.peek_next_token(TokenType::CloseBrace).is_ok() {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }

            self.eat_token(TokenType::CloseBrace)?;
            Some(items)
        } else {
            None
        };

        // optional alias `as Ident` (only when no grouped items were present)
        let alias = if items.is_none() {
            if let Ok(next) = self.peek_next_token(TokenType::Identifier) {
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

        Ok(UsingNode {
            target: path,
            alias,
            items,
        })
    }

    /// Eat a using item (like `geometry` or `geometry as geom`).
    pub fn eat_using_item(&mut self) -> ParseResult<UsingItemNode> {
        let name = self.eat_identifier()?;
        let alias = if let Ok(next) = self.peek_next_token(TokenType::Identifier) {
            if self.get_token_str(*next) == Keyword::As.as_str() {
                self.eat_keyword(Keyword::As)?;
                Some(self.eat_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(UsingItemNode { name, alias })
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Parser, Path, PathSegment, UsingItemNode, UsingNode};

    #[test]
    fn test_parse_using_declaration() {
        let input = r##"
using destack
using destack.geometry
using destack as ds
using ds.geometry as geom
using ds.geometry.{Vector2, Vector3 as V3}
using destack, dyst
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // using destack
        let using = parser.eat_using().unwrap();
        assert_eq!(
            using,
            UsingNode {
                target: Path {
                    segments: vec![PathSegment {
                        name: parser.identifiers.intern("destack")
                    }],
                },
                alias: None,
                items: None,
            }
        );

        // using destack.geometry
        let using = parser.eat_using().unwrap();
        assert_eq!(
            using,
            UsingNode {
                target: Path {
                    segments: vec![
                        PathSegment {
                            name: parser.identifiers.intern("destack")
                        },
                        PathSegment {
                            name: parser.identifiers.intern("geometry")
                        }
                    ],
                },
                alias: None,
                items: None,
            }
        );

        // using destack as ds
        let using = parser.eat_using().unwrap();
        assert_eq!(
            using,
            UsingNode {
                target: Path {
                    segments: vec![PathSegment {
                        name: parser.identifiers.intern("destack")
                    }],
                },
                alias: Some(parser.identifiers.intern("ds")),
                items: None,
            }
        );

        // using ds.geometry as geom
        let using = parser.eat_using().unwrap();
        assert_eq!(
            using,
            UsingNode {
                target: Path {
                    segments: vec![
                        PathSegment {
                            name: parser.identifiers.intern("ds")
                        },
                        PathSegment {
                            name: parser.identifiers.intern("geometry")
                        }
                    ],
                },
                alias: Some(parser.identifiers.intern("geom")),
                items: None,
            }
        );

        // using ds.geometry.{Vector2, Vector3 as V3}
        let using = parser.eat_using().unwrap();
        assert_eq!(
            using,
            UsingNode {
                target: Path {
                    segments: vec![
                        PathSegment {
                            name: parser.identifiers.intern("ds")
                        },
                        PathSegment {
                            name: parser.identifiers.intern("geometry")
                        }
                    ],
                },
                alias: None,
                items: Some(vec![
                    UsingItemNode {
                        name: parser.identifiers.intern("Vector2"),
                        alias: None,
                    },
                    UsingItemNode {
                        name: parser.identifiers.intern("Vector3"),
                        alias: Some(parser.identifiers.intern("V3")),
                    }
                ]),
            }
        );
    }
}
