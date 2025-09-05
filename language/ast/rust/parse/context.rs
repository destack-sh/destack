//! Parse use declarations.
use destack_language_token::TokenType;

use crate::{
    Expression, Keyword, NodeId, ParseResult, Parser, Use, UseClause, UseItem, With, WithClause,
};

impl<'a> Parser<'a> {
    /// Eat a use declaration (including keyword and semicolon or block).
    pub fn eat_use(&mut self) -> ParseResult<NodeId<Use>> {
        self.eat_keyword(Keyword::Use)?;
        let using = self.eat_use_header()?;
        self.eat_statement_stop()?;
        Ok(using)
    }

    /// Eat the content of a use declaration (without the `use` keyword).
    pub fn eat_use_header(&mut self) -> ParseResult<NodeId<Use>> {
        let start = self.mark();

        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<UseClause>> = Vec::new();
        let clause = self.eat_use_clause()?;
        clauses.push(clause);
        loop {
            if self.peek_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self.eat_use_clause()?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        let using = self.tree.allocate(
            Use {
                clauses,
                body: None,
            },
            self.get_span_from(start),
        );
        Ok(using)
    }

    /// Eat a single use clause (like `foo`, `foo as bar`, `foo.{a, b}`).
    pub fn eat_use_clause(&mut self) -> ParseResult<NodeId<UseClause>> {
        let start = self.mark();
        let path = self.eat_path()?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_token(TokenType::Dot).is_ok() {
            self.eat_token(TokenType::Dot)?;
            self.eat_token(TokenType::OpenBrace)?;

            // parse zero or more items
            // (empty group `.{}` is valid)
            let mut items: Vec<NodeId<UseItem>> = Vec::new();
            if self.peek_token(TokenType::CloseBrace).is_err() {
                loop {
                    let item = self.eat_use_item()?;
                    items.push(item);
                    if self.peek_token(TokenType::Comma).is_ok() {
                        self.eat_token(TokenType::Comma)?;
                        // allow trailing comma
                        if self.peek_token(TokenType::CloseBrace).is_ok() {
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
            if let Ok(next) = self.peek_token(TokenType::Identifier) {
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
        let span = self.get_span_from(start);
        let target = self.tree.allocate(Expression::Path { path }, span);

        let clause = self.tree.allocate(
            UseClause {
                target,
                alias,
                items,
            },
            span,
        );
        Ok(clause)
    }

    /// Eat a use item (like `geometry` or `geometry as geom`).
    pub fn eat_use_item(&mut self) -> ParseResult<NodeId<UseItem>> {
        let start = self.mark();
        let name = self.eat_identifier()?;
        let alias = if let Ok(next) = self.peek_token(TokenType::Identifier) {
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
            .allocate(UseItem { name, alias }, self.get_span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, UseItem};

    #[test]
    fn test_parse_using_declaration() {
        let input = r##"
use dyst
use dyst.geometry
use dyst as ds
use ds.geometry as geom
use ds.geometry.{Vector2, Vector3 as V3}
use dyst, dyst
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // use dyst
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")])
            ),
            _ => panic!("expected path expression"),
        }

        // use dyst.geometry
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![
                    parser.strings.intern("dyst"),
                    parser.strings.intern("geometry")
                ])
            ),
            _ => panic!("expected path expression"),
        }

        // use dyst as ds
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("ds")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }

        // use ds.geometry as geom
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("geom")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![
                    parser.strings.intern("ds"),
                    parser.strings.intern("geometry")
                ]),
            ),
            _ => panic!("expected path expression"),
        }

        // use ds.geometry.{Vector2, Vector3 as V3}
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        let items = clause.items.as_ref().expect("expected items");
        assert_eq!(items.len(), 2);
        let item0 = parser.tree.get(items[0]);
        assert_eq!(
            *item0,
            UseItem {
                name: parser.strings.intern("Vector2"),
                alias: None,
            }
        );
        let item1 = parser.tree.get(items[1]);
        assert_eq!(
            *item1,
            UseItem {
                name: parser.strings.intern("Vector3"),
                alias: Some(parser.strings.intern("V3")),
            }
        );

        // use dyst, dyst
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 2);
        let clause0 = parser.tree.get(using.clauses[0]);
        match parser.tree.get(clause0.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }
        let clause1 = parser.tree.get(using.clauses[1]);
        match parser.tree.get(clause1.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }
    }
}
