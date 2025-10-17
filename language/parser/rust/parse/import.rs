//! Parse import and with declarations.
use dyst_ast::{ExportMode, ImportTarget, ScalarLiteral};
use dyst_source::StringId;

use crate::TokenType;

use crate::parse::prelude::*;
use crate::{Expression, ImportItem, Keyword, NodeId, NodeType, Parser, ParserResult};

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Eat a import declaration (including the `import` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// import foo
    /// import foo.bar
    /// import foo.{bar, baz}
    /// import { bar, baz } from foo
    /// import foo.{} // valid but linted
    /// import foo as baz
    /// ```
    pub fn eat_import(&mut self) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Import)?;
        // eat type (doesn't do anything, but is allowed for #Leniency)
        if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
        }

        // binding (require import target)
        let (target, alias, items) = self.eat_import_binding()?;
        let target = match target {
            Some(target) => target,
            None => {
                return Err(ParserError::unexpected(self.get_span_from(start))
                    .for_node_type(NodeType::Expression));
            }
        };

        // import
        let import_id = self.tree.insert(
            Expression::Import {
                target,
                alias,
                items,
            },
            self.get_span_from(start),
        );
        Ok(import_id)
    }

    /// Eat an export declaration (including the `export` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// export foo
    /// export foo.bar
    /// export foo.{bar, baz}
    /// export * from foo // same as `export foo`
    /// export * as foo from foo // same as `export foo as foo`
    /// export { bar, baz } from foo
    /// export { bar, baz }
    /// export foo.{} // valid but linted
    /// export foo as baz
    /// ```
    pub fn eat_export(&mut self, mode: Option<ExportMode>) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // mode
        let mode: ExportMode = {
            if let Some(mode) = mode {
                mode
            } else {
                self.eat_keyword(Keyword::Export)?;
                if self.peek_keyword(Keyword::Default).is_ok() {
                    self.bump(); // eat default
                    ExportMode::Default
                } else {
                    ExportMode::Item
                }
            }
        };

        // eat type (doesn't do anything, but is allowed for #Leniency)
        if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
        }

        // binding
        let (target, alias, items) = self.eat_import_binding()?;

        // export
        let export_id = self.tree.insert(
            Expression::Export {
                mode,
                target,
                alias,
                items,
            },
            self.get_span_from(start),
        );
        Ok(export_id)
    }

    /// Peek an import clause.
    pub(crate) fn peek_import_clause(&mut self) -> ParserResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Multiply).is_ok()
            || self.peek_token(TokenType::Identifier).is_ok()
        {
            Ok(())
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat an import target.
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo.bar
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_import_target(&mut self) -> ParserResult<ImportTarget> {
        // physical/string target
        if self.peek_token(TokenType::Literal).is_ok() {
            let literal = self.eat_scalar_literal()?;
            match literal {
                ScalarLiteral::String(string) => Ok(ImportTarget::Physical(string)),
                _ => Err(ParserError::expected(self.peek()?.span, TokenType::Literal)),
            }
        }
        // virtual target
        else {
            let path = self.eat_path()?;
            Ok(ImportTarget::Virtual(path))
        }
    }

    /// Eat the body of an import/export clause and return its components.
    ///
    /// Examples:
    /// ```
    /// { foo, bar }
    /// { foo, bar } from baz
    /// * as foo from baz
    /// foo
    /// foo as bar
    /// ```
    fn eat_import_binding(
        &mut self,
    ) -> ParserResult<(
        Option<ImportTarget>,
        Option<StringId>,
        Option<Vec<NodeId<ImportItem>>>,
    )> {
        // eat type (doesn't do anything, but is allowed for #Leniency)
        if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
        }

        // `{ ... }` with optional `from`
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            let items = self
                .eat_import_items_block()
                .for_node_type(NodeType::Expression)?;
            // `from` target
            let target = if self.peek_keyword(Keyword::From).is_ok() {
                self.eat_keyword(Keyword::From)?;
                Some(self.eat_import_target()?)
            } else {
                None
            };
            Ok((target, None, Some(items)))
        }
        // `* from ...` or `* as ... from ...`
        else if self.peek_token(TokenType::Multiply).is_ok() {
            self.bump(); // eat *
            // `as` alias
            let alias = if self.peek_keyword(Keyword::As).is_ok()
                || self.peek_token(TokenType::Colon).is_ok()
            {
                self.bump(); // eat `as` or `:`
                Some(self.eat_identifier()?)
            } else {
                None
            };
            // `from` target
            self.eat_keyword(Keyword::From)?;
            let target = Some(self.eat_import_target()?);
            Ok((target, alias, None))
        }
        // `foo` or `foo as bar` or `foo.{a, b}`
        else {
            // target
            let target = Some(self.eat_import_target()?);
            // items
            let items = if self.peek_token(TokenType::Dot).is_ok() {
                self.bump(); // eat .
                Some(self.eat_import_items_block()?)
            } else {
                None
            };
            // `as` alias
            let alias = if items.is_none()
                && (self.peek_keyword(Keyword::As).is_ok()
                    || self.peek_token(TokenType::Colon).is_ok())
            {
                self.bump(); // eat `as` or `:`
                Some(self.eat_identifier()?)
            } else {
                None
            };
            Ok((target, alias, items))
        }
    }

    /// Eat a block of import items (like `{ a, b }` in `import foo.{a, b}`).
    fn eat_import_items_block(&mut self) -> ParserResult<Vec<NodeId<ImportItem>>> {
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;

        let mut items: Vec<NodeId<ImportItem>> = Vec::new();
        while self.peek_token(TokenType::CloseBrace).is_err() {
            let item = self.eat_import_item().for_node_type(NodeType::ImportItem)?;
            items.push(item);
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
        }

        self.eat_newlines_maybe()?;
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
    pub(crate) fn eat_import_item(&mut self) -> ParserResult<NodeId<ImportItem>> {
        let start = self.mark();

        // eat type (doesn't do anything, but is allowed for #Leniency)
        if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
        }

        // name
        let name = self.eat_identifier()?;

        // alias
        let alias = {
            if self.peek_keyword(Keyword::As).is_ok() || self.peek_token(TokenType::Colon).is_ok() {
                self.bump(); // eat `as` or `:`
                Some(self.eat_identifier()?)
            } else {
                None
            }
        };

        let item = self
            .tree
            .insert(ImportItem { name, alias }, self.get_span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{ExportMode, ImportTarget};

    use crate::parse::tests::TestParser;
    use crate::{Expression, ImportItem, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_import_simple() {
        // import dyst
        let mut test = TestParser::new("import dyst");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_import_expression_via_expression_parser() {
        let mut test = TestParser::new("import core.memory");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import core.memory
        assert_node!(parser.tree, expression_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "core.memory");
        });
    }

    #[test]
    fn test_parse_import_path() {
        let mut test = TestParser::new("import dyst.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst.geometry
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst.geometry");
        });
    }

    #[test]
    fn test_parse_import_with_alias() {
        let mut test = TestParser::new("import dyst as ds");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst as ds
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert_string!(parser, alias.unwrap(), "ds");
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_import_with_items() {
        let mut test = TestParser::new("import ds.geometry.{Vector2, Vector3 as V3}");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import ds.geometry.{Vector2, Vector3 as V3}
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "Vector2");
                assert_eq!(*alias, None);
            });
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "Vector3");
                assert_string!(parser, alias.unwrap(), "V3");
            });
            assert_path!(parser, *target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_with_prefix_items() {
        let mut test = TestParser::new("import { Vector2, Vector3 as V3 } from ds.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
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
    }

    #[test]
    fn test_parse_import_star_prefix() {
        let mut test = TestParser::new("import * from ds.geometry");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_star_prefix_alias_with_physical_target() {
        let mut test = TestParser::new(r#"import * as geom from "ds/geometry""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * as geom from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Physical(target), alias, items } => {
            assert_string!(parser, alias.unwrap(), "geom");
            assert!(items.is_none());
            assert_string!(parser, *target, "ds/geometry");
        });
    }

    #[test]
    fn test_parse_import_with_lenient_type() {
        let mut test = TestParser::new(
            "
import {
  StructuredObject,
  type StructuredObjectOptions,
} from './lib/object.ng';
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let import_id = parser.eat_import().unwrap();
        assert_node!(parser.tree, import_id, Expression::Import { target: ImportTarget::Physical(target), alias, items } => {
            assert!(alias.is_none());
            assert_string!(parser, *target, "./lib/object.ng");

            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "StructuredObjectOptions");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_with_block() {
        let mut test = TestParser::new(
            "
export { CreateUIMessage, UIMessage }
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let export_id = parser.eat_export(None).unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { mode, target, alias, items } => {
            assert_eq!(*mode, ExportMode::Item);
            assert!(target.is_none());
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            // CreateUIMessage
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "UIMessage");
                assert!(alias.is_none());
            });
        });
    }
}
