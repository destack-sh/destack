use dyst_ast::{Asynchrony, DependencyKind, DependencyTarget, ExportType, ScalarLiteral};
use dyst_source::StringId;

use crate::TokenType;

use crate::parse::prelude::*;
use crate::{DependencyItem, Expression, Keyword, NodeId, NodeType, Parser, ParserResult};

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
    /// import Default, { type Item } from `foo`
    /// import foo.{} // valid but linted
    /// import foo as baz
    /// await import("foo")
    /// await import("foo", arg1: 2, ...)
    /// ```
    pub fn eat_import(&mut self) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // asynchrony
        let asynchrony = {
            if self.peek_keyword(Keyword::Await).is_ok() {
                self.bump(); // eat await
                Asynchrony::Async
            } else {
                Asynchrony::Sync
            }
        };

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // asynchronous import (call form)
        if asynchrony == Asynchrony::Async {
            // open parenthesis
            self.eat_token(TokenType::OpenParenthesis)?;

            // target (first argument)
            let target = match self.eat_scalar_literal()? {
                ScalarLiteral::String(string) => DependencyTarget::Virtual(string),
                _ => return Err(ParserError::expected(self.peek()?.span, TokenType::Literal)),
            };

            // arguments
            let arguments = if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                let arguments = self.eat_arguments_body(TokenType::CloseParenthesis)?;
                Some(arguments)
            } else {
                None
            };

            // close parenthesis
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(self.tree.insert(
                Expression::Import {
                    kind: DependencyKind::Value,
                    alias: None,
                    items: None,
                    asynchrony,
                    target,
                    arguments,
                },
                self.get_span_from(start),
            ));
        }

        // type
        let kind = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // binding (require import target)
        let (target, alias, items) = self.eat_dependency_binding(kind)?;
        let target = match target {
            Some(target) => target,
            None => {
                return Err(ParserError::unexpected(self.get_span_from(start))
                    .for_node_type(NodeType::Expression));
            }
        };

        // arguments
        let arguments = if self.peek_keyword(Keyword::With).is_ok() {
            self.eat_keyword(Keyword::With)?;
            self.eat_token(TokenType::OpenBrace)?;
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
                asynchrony,
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
    pub fn eat_export(&mut self, mode: Option<ExportType>) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // mode
        let mode: ExportType = {
            if let Some(mode) = mode {
                mode
            } else {
                self.eat_keyword(Keyword::Export)?;
                if self.peek_keyword(Keyword::Default).is_ok() {
                    self.bump(); // eat default
                    ExportType::Default
                } else {
                    ExportType::Item
                }
            }
        };

        // type
        let ty = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // binding
        let (target, alias, items) = self.eat_dependency_binding(ty)?;

        // export
        let export_id = self.tree.insert(
            Expression::Export {
                mode,
                kind: ty.unwrap_or(DependencyKind::Value),
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

    /// Eat an dependency target.
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo.bar
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_dependency_target(&mut self) -> ParserResult<DependencyTarget> {
        // physical/string target
        if self.peek_token(TokenType::Literal).is_ok() {
            let literal = self.eat_scalar_literal()?;
            match literal {
                ScalarLiteral::String(string) => Ok(DependencyTarget::Virtual(string)),
                _ => Err(ParserError::expected(self.peek()?.span, TokenType::Literal)),
            }
        }
        // virtual target
        else {
            let path = self.eat_path()?;
            Ok(DependencyTarget::Path(path))
        }
    }

    /// Eat the body of an dependency clause and return its components.
    ///
    /// Examples:
    /// ```
    /// { foo, bar }
    /// { foo, bar } from baz
    /// Foo, { type Bar } from baz
    /// * as foo from baz
    /// foo
    /// foo as bar
    /// ```
    fn eat_dependency_binding(
        &mut self,
        ty: Option<DependencyKind>,
    ) -> ParserResult<(
        Option<DependencyTarget>,
        Option<StringId>,
        Option<Vec<NodeId<DependencyItem>>>,
    )> {
        // type
        let ty = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            ty
        };

        // `{ ... }` with optional `from`
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            let items = self
                .eat_dependency_items_block(ty)
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
            let items = self.eat_dependency_items_block(ty)?;
            // `from`
            self.eat_keyword(Keyword::From)?;
            let target = Some(self.eat_dependency_target()?);
            Ok((target, Some(alias), Some(items)))
        }
        // `foo` or `foo as bar` or `foo.{a, b}`
        else {
            // target
            let target = Some(self.eat_dependency_target()?);
            // items
            let items = if self.peek_token(TokenType::Dot).is_ok() {
                self.bump(); // eat .
                Some(self.eat_dependency_items_block(ty)?)
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
    fn eat_dependency_items_block(
        &mut self,
        ty: Option<DependencyKind>,
    ) -> ParserResult<Vec<NodeId<DependencyItem>>> {
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;

        let mut items: Vec<NodeId<DependencyItem>> = Vec::new();
        while self.peek_token(TokenType::CloseBrace).is_err() {
            let item = self
                .eat_dependency_item(ty)
                .for_node_type(NodeType::DependencyItem)?;
            items.push(item);
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
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
    pub(crate) fn eat_dependency_item(
        &mut self,
        ty: Option<DependencyKind>,
    ) -> ParserResult<NodeId<DependencyItem>> {
        let start = self.mark();

        // type
        let ty = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            ty
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
            DependencyItem {
                kind: ty.unwrap_or(DependencyKind::Value),
                name,
                alias,
            },
            self.get_span_from(start),
        );
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, Asynchrony, DependencyKind, DependencyTarget, ExportType, ScalarLiteral,
    };

    use crate::parse::tests::TestParser;
    use crate::{DependencyItem, Expression, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_import_simple() {
        // import dyst
        let mut test = TestParser::new("import dyst");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_await_import_without_arguments() {
        // await import("destack")
        let mut test = TestParser::new("await import(\"destack\")");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { kind, asynchrony, target: DependencyTarget::Virtual(target), alias, items, arguments } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert!(arguments.is_none());
            // destack
            assert_string!(parser, *target, "destack");
        });
    }

    #[test]
    fn test_parse_await_import_with_arguments() {
        // await import("./modules/core", locale: "en")
        let mut test = TestParser::new("await import(\"./modules/core\", locale: \"en\")");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { kind, asynchrony, target: DependencyTarget::Virtual(target), alias, items, arguments: Some(arguments) } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert!(alias.is_none());
            assert!(items.is_none());
            // ./modules/core
            assert_string!(parser, *target, "./modules/core");

            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: None, name, value } => {
                // locale
                assert_string!(parser, name.string(), "locale");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                    // en
                    assert_string!(parser, *string, "en");
                });
            });
        });
    }

    #[test]
    fn test_parse_import_expression_via_expression_parser() {
        let mut test = TestParser::new("import core.memory");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import core.memory
        assert_node!(parser.tree, expression_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "core.memory");
        });
    }

    #[test]
    fn test_parse_import_path_with_arguments() {
        let mut test = TestParser::new("import dyst.geometry with { bar: true }");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst.geometry with { bar: true }
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, arguments, .. } => {
            // dyst.geometry
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst.geometry");
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
    fn test_parse_import_type_with_alias() {
        let mut test = TestParser::new("import type dyst as ds");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst as ds
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_string!(parser, alias.unwrap(), "ds");
            assert!(items.is_none());
            assert_path!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_import_with_items() {
        let mut test = TestParser::new("import ds.geometry.{Vector2, type Vector3 as V3}");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import ds.geometry.{Vector2, Vector3 as V3}
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Value);
                assert_string!(parser, *name, "Vector2");
                assert_eq!(*alias, None);
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Type);
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

        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Value);
                assert_string!(parser, *name, "Vector2");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Value);
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
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Path(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
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
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Virtual(target), alias, items, .. } => {
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
        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Virtual(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert_string!(parser, *target, "./lib/object.ng");

            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Value);
                assert_string!(parser, *name, "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Type);
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

        assert_node!(parser.tree, import_id, Expression::Import { kind, target: DependencyTarget::Virtual(target), alias, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            // Default
            assert_string!(parser, alias.unwrap(), "Default");
            // { type Item }
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Type);
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

        let export_id = parser.eat_export(None).unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { mode, kind, target, alias, items } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(*mode, ExportType::Item);
            assert!(target.is_none());
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            // CreateUIMessage
            assert_node!(parser.tree, items[0], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Type);
                assert_string!(parser, *name, "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], DependencyItem { kind, name, alias } => {
                assert_eq!(*kind, DependencyKind::Type);
                assert_string!(parser, *name, "UIMessage");
                assert!(alias.is_none());
            });
        });
    }
}
