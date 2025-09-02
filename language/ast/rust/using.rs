//! Parse using declarations.
use destack_language_token::TokenType;

use crate::{Expression, Keyword, NodeId, ParseResult, Parser, Using, UsingClause, UsingItem};

impl<'a> Parser<'a> {
    /// Eat a using declaration (including keyword and semicolon or block).
    pub fn eat_using(&mut self) -> ParseResult<NodeId<Using>> {
        self.eat_keyword(Keyword::Using)?;
        let using = self.eat_using_header()?;
        self.eat_statement_stop()?;
        Ok(using)
    }

    /// Eat the content of a using declaration (without the `using` keyword).
    pub fn eat_using_header(&mut self) -> ParseResult<NodeId<Using>> {
        let start = self.mark();

        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<UsingClause>> = Vec::new();
        let clause = self.eat_using_clause()?;
        clauses.push(clause);
        loop {
            if self.peek_next_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self.eat_using_clause()?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        let using = self.tree.allocate(
            Using {
                clauses,
                body: None,
            },
            self.span_from(start),
        );
        Ok(using)
    }

    /// Eat a single using clause (like `foo`, `foo as bar`, `foo.{a, b}`).
    pub fn eat_using_clause(&mut self) -> ParseResult<NodeId<UsingClause>> {
        let start = self.mark();
        let path = self.eat_path()?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_next_token(TokenType::Dot).is_ok() {
            self.eat_token(TokenType::Dot)?;
            self.eat_token(TokenType::OpenBrace)?;

            // parse zero or more items
            // (empty group `.{}` is valid)
            let mut items: Vec<NodeId<UsingItem>> = Vec::new();
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

        // allocate the path expression for the target
        let span = self.span_from(start);
        let target = self.tree.allocate(Expression::Path(path), span);

        let clause = self.tree.allocate(
            UsingClause {
                target,
                alias,
                items,
            },
            span,
        );
        Ok(clause)
    }

    /// Eat a using item (like `geometry` or `geometry as geom`).
    pub fn eat_using_item(&mut self) -> ParseResult<NodeId<UsingItem>> {
        let start = self.mark();
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

        let item = self
            .tree
            .allocate(UsingItem { name, alias }, self.span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, Path, PathSegment, UsingItem};

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
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![PathSegment {
                        name: parser.strings.intern("destack")
                    }]
                }
            ),
            _ => panic!("expected path expression"),
        }

        // using destack.geometry
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![
                        PathSegment {
                            name: parser.strings.intern("destack")
                        },
                        PathSegment {
                            name: parser.strings.intern("geometry")
                        }
                    ]
                }
            ),
            _ => panic!("expected path expression"),
        }

        // using destack as ds
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("ds")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![PathSegment {
                        name: parser.strings.intern("destack")
                    }]
                }
            ),
            _ => panic!("expected path expression"),
        }

        // using ds.geometry as geom
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("geom")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![
                        PathSegment {
                            name: parser.strings.intern("ds")
                        },
                        PathSegment {
                            name: parser.strings.intern("geometry")
                        }
                    ]
                }
            ),
            _ => panic!("expected path expression"),
        }

        // using ds.geometry.{Vector2, Vector3 as V3}
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        let items = clause.items.as_ref().expect("expected items");
        assert_eq!(items.len(), 2);
        let item0 = parser.tree.get(items[0]);
        assert_eq!(
            *item0,
            UsingItem {
                name: parser.strings.intern("Vector2"),
                alias: None,
            }
        );
        let item1 = parser.tree.get(items[1]);
        assert_eq!(
            *item1,
            UsingItem {
                name: parser.strings.intern("Vector3"),
                alias: Some(parser.strings.intern("V3")),
            }
        );

        // using destack, dyst
        let using_id = parser.eat_using().unwrap();
        let using = parser.tree.get(using_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 2);
        let clause0 = parser.tree.get(using.clauses[0]);
        match parser.tree.get(clause0.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![PathSegment {
                        name: parser.strings.intern("destack")
                    }]
                }
            ),
            _ => panic!("expected path expression"),
        }
        let clause1 = parser.tree.get(using.clauses[1]);
        match parser.tree.get(clause1.target) {
            Expression::Path(path) => assert_eq!(
                *path,
                Path {
                    segments: vec![PathSegment {
                        name: parser.strings.intern("dyst")
                    }]
                }
            ),
            _ => panic!("expected path expression"),
        }
    }
}
