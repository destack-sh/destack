use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{
    DependencyItem, DependencyKind, ExportType, Expression, Keyword, LocalNodeId, NodeType,
    ScalarLiteral, TokenType,
};
use dyst_source::StringId;

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Eat a import declaration (including the `import` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo" // same as `import "foo" as foo`
    /// import { bar, baz } from "foo"
    /// import Default, { type Item } from "foo"
    /// import foo as baz with { bar: true } // arguments
    /// ```
    pub fn eat_import(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // kind
        let kind = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // binding
        let (target, alias, items) = self.eat_dependency_binding()?;
        // binding for import needs a dependency target
        let target = match target {
            Some(target) => target,
            None => {
                return Err(ParseError::unexpected(self.get_span_from(start))
                    .for_node_type(NodeType::Expression));
            }
        };

        // arguments
        let arguments = if self.peek_keyword(Keyword::With).is_ok() {
            self.eat_keyword(Keyword::With)?;
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
            let arguments = self.with_options(self.options.nested(), |parser| {
                parser.eat_arguments_body(TokenType::CloseBrace)
            })?;
            self.eat_token(TokenType::CloseBrace)?;
            Some(arguments)
        } else {
            None
        };

        // import
        let import_id = self.tree.insert(
            Expression::Import {
                kind: kind.unwrap_or(DependencyKind::Value),
                target,
                alias,
                items,
                arguments,
            },
            self.get_span_from(start),
        );
        Ok(import_id)
    }

    /// Eat an export declaration (including the `export` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// export "foo"
    /// export * from "foo" // same as `export "foo"`
    /// export * as foo from "foo" // same as `export "foo" as foo`
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export default foo
    /// export = foo
    /// ```
    pub fn eat_export(
        &mut self,
        start: Option<ParserMark>,
        mode: Option<ExportType>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = start.unwrap_or_else(|| self.mark());

        // mode
        let mode: ExportType = {
            if let Some(mode) = mode {
                mode
            } else {
                self.eat_keyword(Keyword::Export)?;
                if self.peek_keyword(Keyword::Default).is_ok() {
                    self.bump(); // eat default
                    ExportType::Default
                } else if self.peek_token(TokenType::Assign).is_ok() {
                    self.bump(); // eat assign
                    ExportType::Namespace
                } else {
                    ExportType::Item
                }
            }
        };

        // kind
        let kind = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // value for module export
        if mode == ExportType::Namespace {
            let value = self.eat_expression()?;
            return Ok(self.tree.insert(
                Expression::Export {
                    mode,
                    kind: kind.unwrap_or(DependencyKind::Value),
                    target: None,
                    alias: None,
                    items: None,
                    value: Some(value),
                },
                self.get_span_from(start),
            ));
        }

        // binding
        let (target, alias, items) = self.eat_dependency_binding()?;

        // export
        let export_id = self.tree.insert(
            Expression::Export {
                mode,
                kind: kind.unwrap_or(DependencyKind::Value),
                target,
                alias,
                items,
                value: None,
            },
            self.get_span_from(start),
        );
        Ok(export_id)
    }

    /// Peek an import clause.
    pub(crate) fn peek_import_clause(&mut self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Multiply).is_ok()
            || self.peek_token(TokenType::Identifier).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat the body of an dependency clause and return its components.
    ///
    /// Examples:
    /// ```
    /// { foo, bar }
    /// { foo, bar } from "baz"
    /// Foo, { type Bar } from "baz"
    /// * as foo from "baz"
    /// foo
    /// foo as bar
    /// ```
    fn eat_dependency_binding(
        &mut self,
    ) -> ParseResult<(
        Option<StringId>,
        Option<StringId>,
        Option<Vec<LocalNodeId<DependencyItem>>>,
    )> {
        // `{ ... }` with optional `from`
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            let items = self
                .eat_dependency_items_block()
                .for_node_type(NodeType::Expression)?;
            // `from` target
            let target = if self.peek_keyword(Keyword::From).is_ok() {
                self.eat_keyword(Keyword::From)?;
                Some(self.eat_dependency_target()?)
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
            let target = Some(self.eat_dependency_target()?);
            Ok((target, alias, None))
        }
        // `Foo, { ... } from bar`
        else if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Comma).is_ok() {
            // `Foo`
            let alias = self.eat_identifier()?;
            self.eat_token(TokenType::Comma)?;
            // `{ ... }`
            let items = self.eat_dependency_items_block()?;
            // `from`
            self.eat_keyword(Keyword::From)?;
            let target = Some(self.eat_dependency_target()?);
            Ok((target, Some(alias), Some(items)))
        }
        // `foo from "foo"` or `foo from foo`
        else if self.peek_identifier().is_ok() && self.peek_next_keyword(Keyword::From).is_ok() {
            let alias = self.eat_identifier()?;
            self.eat_keyword(Keyword::From)?;
            let target = Some(self.eat_dependency_target()?);
            Ok((target, Some(alias), None))
        }
        // `foo` or `foo as bar` or `foo.{a, b}`
        else {
            // target
            let target = Some(self.eat_dependency_target()?);
            // items
            let items = if self.peek_token(TokenType::Dot).is_ok() {
                self.bump(); // eat .
                Some(self.eat_dependency_items_block()?)
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

    /// Eat an dependency target.
    ///
    /// Examples:
    /// ```
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_dependency_target(&mut self) -> ParseResult<StringId> {
        // physical/string target
        let literal = self.eat_scalar_literal()?;
        match literal {
            ScalarLiteral::String(string) => Ok(string),
            _ => Err(ParseError::expected(self.peek()?.span, TokenType::Literal)),
        }
    }

    /// Eat a block of import items (like `{ a, b }` in `import foo.{a, b}`).
    fn eat_dependency_items_block(&mut self) -> ParseResult<Vec<LocalNodeId<DependencyItem>>> {
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        self.eat_newlines_maybe()?;

        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        while self.peek_token(TokenType::CloseBrace).is_err() {
            let item = self
                .eat_dependency_item()
                .for_node_type(NodeType::DependencyItem)?;
            items.push(item);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
        }

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBrace)?;
        Ok(items)
    }

    /// Eat a dependency item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```
    /// geometry
    /// geometry as geom
    /// ```
    pub(crate) fn eat_dependency_item(&mut self) -> ParseResult<LocalNodeId<DependencyItem>> {
        let start = self.mark();

        // kind
        let kind = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

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

        let item = self.tree.insert(
            DependencyItem { kind, name, alias },
            self.get_span_from(start),
        );
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, DependencyItem, DependencyKind, ExportType, Expression, ScalarLiteral,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_import_simple() {
        // import dyst
        let mut test = TestParser::new("import \"dyst\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_string!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_import_from_expression() {
        let mut test = TestParser::new("import os from 'os'");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import os from 'os'
        assert_node!(parser.tree, expression_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, alias.unwrap(), "os");
            assert!(items.is_none());
            assert_string!(parser, *target, "os");
        });
    }

    #[test]
    fn test_parse_import_path_with_arguments() {
        let mut test = TestParser::new("import \"dyst.geometry\" with { bar: true }");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst.geometry with { bar: true }
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, arguments, .. } => {
            // dyst.geometry
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_string!(parser, *target, "dyst.geometry");
            // with { bar: true }
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: None, name, value } => {
                assert_string!(parser, name.string(), "bar");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
        });
    }

    #[test]
    fn test_parse_import_with_prefix_items() {
        let mut test = TestParser::new("import { Vector2, Vector3 as V3 } from \"ds.geometry\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "Vector2");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "Vector3");
                assert_string!(parser, alias.unwrap(), "V3");
            });
            assert_string!(parser, *target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_star_prefix() {
        let mut test = TestParser::new("import * from \"ds.geometry\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_string!(parser, *target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_star_prefix_alias_with_physical_target() {
        let mut test = TestParser::new(r#"import * as geom from "ds/geometry""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * as geom from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, alias.unwrap(), "geom");
            assert!(items.is_none());
            assert_string!(parser, *target, "ds/geometry");
        });
    }

    #[test]
    fn test_parse_import_with_type() {
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
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert_string!(parser, *target, "./lib/object.ng");

            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, *name, "StructuredObjectOptions");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_with_default_and_block() {
        let mut test = TestParser::new("import Default, { type Item } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { kind, target, alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            // Default
            assert_string!(parser, alias.unwrap(), "Default");
            // { type Item }
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, *name, "Item");
                assert!(alias.is_none());
            });
            // `foo`
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_with_block() {
        let mut test = TestParser::new(
            "
export type { CreateUIMessage, UIMessage }
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let export_id = parser.eat_export(None, None).unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { mode, kind, target, alias, items, value: None } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(*mode, ExportType::Item);
            assert!(target.is_none());
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            // CreateUIMessage
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "UIMessage");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_with_module_export() {
        let mut test = TestParser::new("export = foo");
        let mut parser = test.prepare();
        let export_id = parser.eat_export(None, None).unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { mode, kind, target: None, value: Some(value), .. } => {
            assert_eq!(*mode, ExportType::Namespace);
            assert_eq!(*kind, DependencyKind::Value);
            assert_expression_path!(parser, parser.tree.get(*value), "foo");
        });
    }
}
