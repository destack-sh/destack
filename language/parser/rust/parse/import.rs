//! Parse import and with declarations.
use crate::TokenType;

use crate::parse::prelude::*;
use crate::{
    Expression, ImportClause, ImportItem, Keyword, NodeId, NodeType, Parser, ParserResult,
    Visibility,
};

// nocheckin: turn import into import/export (?)

impl<'a> Parser<'a> {
    /// Eat a import declaration (including the `import` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// import foo
    /// import foo, bar
    /// import foo.bar
    /// import foo.{bar, baz}
    /// import { bar, baz } from foo
    /// import foo.{} // valid but linted
    /// import foo as baz
    /// ```
    pub fn eat_import(
        &mut self,
        visibility: Option<Visibility>,
    ) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword_in(&[Keyword::Import, Keyword::Use])?;

        // clauses
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_import_clauses()
        })?;

        // import
        let import_id = self.tree.insert(
            Expression::Import {
                clauses,
                visibility,
            },
            self.get_span_from(start),
        );
        Ok(import_id)
    }

    /// Eat the header of a import declaration (without the `import` keyword).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo, bar
    /// foo.bar
    /// foo.{bar, baz}
    /// { bar, baz } from foo // equivalent
    /// foo.{} // valid but linted
    /// foo as baz
    /// ```
    fn eat_import_clauses(&mut self) -> ParserResult<Vec<NodeId<ImportClause>>> {
        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<ImportClause>> = Vec::new();
        let clause = self
            .eat_import_clause()
            .for_node_type(NodeType::ImportClause)?;
        clauses.push(clause);
        loop {
            if self.peek_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self
                    .eat_import_clause()
                    .for_node_type(NodeType::ImportClause)?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        Ok(clauses)
    }

    /// Eat a single import clause (like `foo`, `foo as bar`, `foo.{a, b}`).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo as bar
    /// foo.{a, b}
    /// { a, b } from foo
    /// * from foo
    /// * as foo from foo
    /// ```
    fn eat_import_clause(&mut self) -> ParserResult<NodeId<ImportClause>> {
        let start = self.mark();

        let (target, alias, items) = {
            // `{ ... } from ...`
            if self.peek_token(TokenType::OpenBrace).is_ok() {
                // { ... }
                let items = self
                    .eat_import_items_block()
                    .for_node_type(NodeType::ImportClause)?;
                // from
                self.eat_keyword(Keyword::From)
                    .for_node_type(NodeType::ImportClause)?;
                // target
                let target = self.eat_path().for_node_type(NodeType::Expression)?;
                (target, None, Some(items))
            }
            // `* from ...` or `* as ... from ...`
            else if self.peek_token(TokenType::Multiply).is_ok() {
                // *
                self.eat_token(TokenType::Multiply)?;
                // alias
                let alias = if self.peek_keyword(Keyword::As).is_ok()
                    || self.peek_token(TokenType::Colon).is_ok()
                {
                    self.bump(); // eat `as` or `:`
                    Some(self.eat_identifier()?)
                } else {
                    None
                };
                // from
                self.eat_keyword(Keyword::From)
                    .for_node_type(NodeType::ImportClause)?;
                let target = self.eat_path().for_node_type(NodeType::Expression)?;
                (target, alias, None)
            }
            // `foo` or `foo as bar` or `foo.{a, b}`
            else {
                // target
                let target = self.eat_path().for_node_type(NodeType::Expression)?;
                let mut items = None;
                // `.{ ... }`
                if self.peek_token(TokenType::Dot).is_ok() {
                    self.eat_token(TokenType::Dot)?;
                    let parsed_items = self
                        .eat_import_items_block()
                        .for_node_type(NodeType::ImportClause)?;
                    items = Some(parsed_items);
                }
                // alias
                let alias = if items.is_none() {
                    if self.peek_keyword(Keyword::As).is_ok()
                        || self.peek_token(TokenType::Colon).is_ok()
                    {
                        self.bump(); // eat `as` or `:`
                        Some(self.eat_identifier()?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                (target, alias, items)
            }
        };

        let span = self.get_span_from(start);
        let clause = self.tree.insert(
            ImportClause {
                target,
                alias,
                items,
            },
            span,
        );
        Ok(clause)
    }

    /// Eat a block of import items (like `{ a, b }` in `import foo.{a, b}`).
    fn eat_import_items_block(&mut self) -> ParserResult<Vec<NodeId<ImportItem>>> {
        self.eat_token(TokenType::OpenBrace)?;

        let mut items: Vec<NodeId<ImportItem>> = Vec::new();
        if self.peek_token(TokenType::CloseBrace).is_err() {
            loop {
                let item = self.eat_import_item().for_node_type(NodeType::ImportItem)?;
                items.push(item);
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.eat_token(TokenType::Comma)?;
                    if self.peek_token(TokenType::CloseBrace).is_ok() {
                        break;
                    }
                    continue;
                }
                break;
            }
        }

        self.eat_token(TokenType::CloseBrace)?;
        Ok(items)
    }

    /// Eat a import item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```
    /// geometry
    /// geometry as geom
    /// ```
    pub fn eat_import_item(&mut self) -> ParserResult<NodeId<ImportItem>> {
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
            .insert(ImportItem { name, alias }, self.get_span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Expression, ImportClause, ImportItem, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_import_simple() {
        // import dyst
        let mut test = TestParser::new("import dyst");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { visibility, clauses } => {
            assert_eq!(*visibility, None);
            assert_eq!(clauses.len(), 1);
            // import dyst
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst");
            });
        });
    }

    #[test]
    fn test_parse_import_expression_via_expression_parser() {
        let mut test = TestParser::new("import core.memory");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import core.memory
        assert_node!(parser.tree, expression_id, Expression::Import { visibility, clauses } => {
            assert!(visibility.is_none());
            assert_eq!(clauses.len(), 1);

            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "core.memory");
            });
        });
    }

    #[test]
    fn test_parse_import_path() {
        let mut test = TestParser::new("import dyst.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import dyst.geometry
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // import dyst.geometry
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst.geometry");
            });
        });
    }

    #[test]
    fn test_parse_import_with_alias() {
        let mut test = TestParser::new("import dyst as ds");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import dyst as ds
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // import dyst as ds
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert_string!(parser, alias.unwrap(), "ds");
                assert!(items.is_none());
                assert_path!(parser, *target, "dyst");
            });
        });
    }

    #[test]
    fn test_parse_import_with_items() {
        let mut test = TestParser::new("import ds.geometry.{Vector2, Vector3 as V3}");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import ds.geometry.{Vector2, Vector3 as V3}
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // import ds.geometry.{Vector2, Vector3 as V3}
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                let items = items.as_ref().expect("expected items");
                assert_eq!(items.len(), 2);
                // Vector2
                assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                    assert_string!(parser, *name, "Vector2");
                    assert_eq!(*alias, None);
                });
                // Vector3 as V3
                assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                    assert_string!(parser, *name, "Vector3");
                    assert_string!(parser, alias.unwrap(), "V3");
                });
                // ds.geometry
                assert_path!(parser, *target, "ds.geometry");
            });
        });
    }

    #[test]
    fn test_parse_import_with_javascript_style_items() {
        let mut test = TestParser::new("import { Vector2, Vector3 as V3 } from ds.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                let items = items.as_ref().expect("expected items");
                assert_eq!(items.len(), 2);
                assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                    assert_string!(parser, *name, "Vector2");
                    assert!(alias.is_none());
                });
                assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                    assert_string!(parser, *name, "Vector3");
                    assert_string!(parser, alias.unwrap(), "V3");
                });
                assert_path!(parser, *target, "ds.geometry");
            });
        });
    }

    #[test]
    fn test_parse_import_supports_star_clause() {
        let mut test = TestParser::new("import * from ds.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import * from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_path!(parser, *target, "ds.geometry");
            });
        });
    }

    #[test]
    fn test_parse_import_supports_star_clause_alias() {
        let mut test = TestParser::new("import * as geom from ds.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();

        // import * as geom from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], ImportClause { target, alias, items } => {
                assert_string!(parser, alias.unwrap(), "geom");
                assert!(items.is_none());
                assert_path!(parser, *target, "ds.geometry");
            });
        });
    }

    #[test]
    fn test_parse_import_multiple_clauses() {
        let mut test = TestParser::new("import dyst, dyst");
        let mut parser = test.prepare();
        let import_id = parser.eat_import(None).unwrap();
        // import dyst, dyst
        assert_node!(parser.tree, import_id, Expression::Import { clauses, .. } => {
            assert_eq!(clauses.len(), 2);
            // import dyst
            assert_node!(parser.tree, clauses[0], ImportClause { target, .. } => {
                assert_path!(parser, *target, "dyst");
            });
            // import dyst
            assert_node!(parser.tree, clauses[1], ImportClause { target, .. } => {
                assert_path!(parser, *target, "dyst");
            });
        });
    }
}
