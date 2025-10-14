//! Parse use and with declarations.
use crate::TokenType;

use crate::parse::prelude::*;
use crate::{
    Expression, Keyword, NodeId, NodeType, Parser, ParserResult, UseClause, UseItem, Visibility,
};

// nocheckin: turn use into import/export (?)

impl<'a> Parser<'a> {
    /// Eat a use declaration (including the `use` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// use foo
    /// use foo, bar
    /// use foo.bar
    /// use foo.{bar, baz} {
    ///   ...
    /// }
    /// ```
    pub fn eat_use(&mut self, visibility: Option<Visibility>) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Use)?;

        // clauses
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_use_clauses()
        })?;

        // use
        let use_id = self.tree.insert(
            Expression::Use {
                clauses,
                visibility,
            },
            self.get_span_from(start),
        );
        Ok(use_id)
    }

    /// Eat the header of a use declaration (without the `use` keyword).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo, bar
    /// foo.bar
    /// foo.{bar, baz}
    /// foo.{} // valid but linted
    /// foo as baz
    /// ```
    fn eat_use_clauses(&mut self) -> ParserResult<Vec<NodeId<UseClause>>> {
        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<UseClause>> = Vec::new();
        let clause = self.eat_use_clause().for_node_type(NodeType::UseClause)?;
        clauses.push(clause);
        loop {
            if self.peek_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self.eat_use_clause().for_node_type(NodeType::UseClause)?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        Ok(clauses)
    }

    /// Eat a single use clause (like `foo`, `foo as bar`, `foo.{a, b}`).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo as bar
    /// foo.{a, b}
    /// ```
    fn eat_use_clause(&mut self) -> ParserResult<NodeId<UseClause>> {
        let start = self.mark();
        let path = self.eat_path().for_node_type(NodeType::Expression)?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_token(TokenType::Dot).is_ok() {
            self.eat_token(TokenType::Dot)?;
            self.eat_token(TokenType::OpenBrace)?;

            // parse zero or more items
            // (empty group `.{}` is valid)
            let mut items: Vec<NodeId<UseItem>> = Vec::new();
            if self.peek_token(TokenType::CloseBrace).is_err() {
                loop {
                    let item = self.eat_use_item().for_node_type(NodeType::UseItem)?;
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
            if self.peek_identifier().is_ok() || self.peek_token(TokenType::Colon).is_ok() {
                // handle both `as` and `:`
                if self.peek_identifier().is_ok() || self.peek_token(TokenType::Colon).is_ok() {
                    self.bump(); // eat `as` or `:`
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

        // clause
        let span = self.get_span_from(start);
        let clause = self.tree.insert(
            UseClause {
                target: path,
                alias,
                items,
            },
            span,
        );
        Ok(clause)
    }

    /// Eat a use item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```
    /// geometry
    /// geometry as geom
    /// ```
    pub fn eat_use_item(&mut self) -> ParserResult<NodeId<UseItem>> {
        let start = self.mark();
        let name = self.eat_identifier()?;
        let alias = if self.peek_identifier().is_ok() || self.peek_token(TokenType::Colon).is_ok() {
            if self.peek_identifier().is_ok() || self.peek_token(TokenType::Colon).is_ok() {
                self.bump(); // eat `as` or `:`
                Some(self.eat_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        let item = self
            .tree
            .insert(UseItem { name, alias }, self.get_span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Expression, UseClause, UseItem, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_use_simple() {
        // use dyst
        let mut test = TestParser::new("use dyst");
        let mut parser = test.prepare();
        let use_id = parser.eat_use(None).unwrap();

        // use
        assert_node!(parser.tree, use_id, Expression::Use { visibility, clauses } => {
            assert_eq!(*visibility, None);
            assert_eq!(clauses.len(), 1);
            // use dyst
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst");
            });
        });
    }

    #[test]
    fn test_parse_use_expression_via_expression_parser() {
        let mut test = TestParser::new("use core.memory");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // use core.memory
        assert_node!(parser.tree, expression_id, Expression::Use { visibility, clauses } => {
            assert!(visibility.is_none());
            assert_eq!(clauses.len(), 1);

            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "core.memory");
            });
        });
    }

    #[test]
    fn test_parse_use_path() {
        let mut test = TestParser::new("use dyst.geometry");
        let mut parser = test.prepare();
        let use_id = parser.eat_use(None).unwrap();

        // use dyst.geometry
        assert_node!(parser.tree, use_id, Expression::Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use dyst.geometry
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst.geometry");
            });
        });
    }

    #[test]
    fn test_parse_use_with_alias() {
        let mut test = TestParser::new("use dyst as ds");
        let mut parser = test.prepare();
        let use_id = parser.eat_use(None).unwrap();

        // use dyst as ds
        assert_node!(parser.tree, use_id, Expression::Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use dyst as ds
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert_string!(parser, alias.unwrap(), "ds");
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst");
            });
        });
    }

    #[test]
    fn test_parse_use_with_items() {
        let mut test = TestParser::new("use ds.geometry.{Vector2, Vector3 as V3}");
        let mut parser = test.prepare();
        let use_id = parser.eat_use(None).unwrap();

        // use ds.geometry.{Vector2, Vector3 as V3}
        assert_node!(parser.tree, use_id, Expression::Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use ds.geometry.{Vector2, Vector3 as V3}
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                let items = items.as_ref().expect("expected items");
                assert_eq!(items.len(), 2);
                // Vector2
                assert_node!(parser.tree, items[0], UseItem { name, alias } => {
                    assert_string!(parser, *name, "Vector2");
                    assert_eq!(*alias, None);
                });
                // Vector3 as V3
                assert_node!(parser.tree, items[1], UseItem { name, alias } => {
                    assert_string!(parser, *name, "Vector3");
                    assert_string!(parser, alias.unwrap(), "V3");
                });
                // ds.geometry
                assert_path!(parser, *target, "ds.geometry");
            });
        });
    }

    #[test]
    fn test_parse_use_multiple_clauses() {
        let mut test = TestParser::new("use dyst, dyst");
        let mut parser = test.prepare();
        let use_id = parser.eat_use(None).unwrap();
        // use dyst, dyst
        assert_node!(parser.tree, use_id, Expression::Use { clauses, .. } => {
            assert_eq!(clauses.len(), 2);
            // use dyst
            assert_node!(parser.tree, clauses[0], UseClause { target, .. } => {
                assert_path!(parser, *target, "dyst");
            });
            // use dyst
            assert_node!(parser.tree, clauses[1], UseClause { target, .. } => {
                assert_path!(parser, *target, "dyst");
            });
        });
    }
}
