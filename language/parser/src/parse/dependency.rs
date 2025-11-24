use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{
    DependencyItem, DependencyKind, DependencyMode, Expression, Keyword, LocalNodeId,
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
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { type Item } from "foo"
    /// import foo as baz with { bar: true }
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
        let items = if self.peek_dependency_binding().is_ok() {
            self.eat_dependency_items_block()?
        } else {
            vec![]
        };
        if !items.is_empty() {
            self.eat_keyword(Keyword::From)?;
        }
        let target = self.eat_dependency_target()?;

        // arguments
        let arguments = if self.peek_keyword(Keyword::With).is_ok() {
            self.bump(); // eat with
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
    /// export * from "foo"
    /// export * as foo from "foo"
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export default foo
    /// export = foo
    /// ```
    pub fn eat_export(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        self.eat_keyword(Keyword::Export)?;

        // export default
        if self.peek_keyword(Keyword::Default).is_ok() {
            self.bump(); // eat default
            let name = self.eat_identifier()?;
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Default,
                    kind: Some(DependencyKind::Value),
                    name: Some(name),
                    alias: None,
                    value: None,
                },
                self.get_span_from(start),
            );
            let export = self.tree.insert(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                },
                self.get_span_from(start),
            );
            return Ok(export);
        }
        // export =
        else if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            let value = self.eat_expression()?;
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Namespace,
                    kind: Some(DependencyKind::Value),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(start),
            );
            let export = self.tree.insert(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                },
                self.get_span_from(start),
            );
            return Ok(export);
        }
        // export * from
        else if self.peek_token(TokenType::Multiply).is_ok()
            && self.peek_next_keyword(Keyword::From).is_ok()
        {
            self.bump(); // eat *
            self.bump(); // eat from
            let target = self.eat_dependency_target()?;
            let item = DependencyItem {
                mode: DependencyMode::Namespace,
                kind: None,
                name: None,
                alias: None,
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(start));
            let export = self.tree.insert(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: Some(target),
                    items: vec![item_id],
                },
                self.get_span_from(start),
            );
            return Ok(export);
        }

        // kind
        let kind = if self.peek_keyword(Keyword::Type).is_ok() {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // binding
        let items = self.eat_dependency_items_block()?;
        let target = if self.peek_keyword(Keyword::From).is_ok() {
            self.bump(); // eat from
            Some(self.eat_dependency_target()?)
        } else {
            None
        };

        // export
        let export_id = self.tree.insert(
            Expression::Export {
                kind: kind.unwrap_or(DependencyKind::Value),
                target,
                items,
            },
            self.get_span_from(start),
        );
        Ok(export_id)
    }

    /// Peek a dependency binding.
    pub(crate) fn peek_dependency_binding(&mut self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Multiply).is_ok()
            || (self.peek_token(TokenType::Identifier).is_ok()
                && (self.peek_next_token(TokenType::Comma).is_ok()
                    || self.peek_next_keyword(Keyword::From).is_ok()))
        {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
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

    /// Eat a dependency items block.
    ///
    /// Examples:
    /// ```
    /// foo
    /// * as foo
    /// Default, { a, b }
    /// { a, b }
    /// ```
    fn eat_dependency_items_block(&mut self) -> ParseResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();

        // `Default,` or `foo from`
        if self.peek_token(TokenType::Identifier).is_ok()
            && (self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_keyword(Keyword::From).is_ok())
        {
            let start = self.mark();
            let alias = self.eat_identifier()?;
            if self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma (leave from)
            }
            let item = DependencyItem {
                mode: DependencyMode::Default,
                kind: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(start));
            items.push(item_id);
        }
        // `* as foo`
        else if self.peek_token(TokenType::Multiply).is_ok()
            && self.peek_next_keyword(Keyword::As).is_ok()
        {
            let start = self.mark();
            self.bump(); // eat *
            self.bump(); // eat as
            let alias = self.eat_identifier()?;
            let item = DependencyItem {
                mode: DependencyMode::Namespace,
                kind: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(start));
            items.push(item_id);
        }

        // main items
        if items.is_empty() || self.peek_token(TokenType::OpenBrace).is_ok() {
            self.eat_token(TokenType::OpenBrace)?;
            self.eat_newlines_maybe()?;
            while self.peek_token(TokenType::CloseBrace).is_err() {
                let item = self.eat_dependency_item()?;
                items.push(item);
                if self.peek_item_stop().is_ok() {
                    self.eat_item_stop_with_newlines()?;
                }
            }
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
        }

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

        // default
        if self.peek_keyword(Keyword::Default).is_ok() {
            self.bump(); // eat default

            // alias
            let alias = if self.peek_keyword(Keyword::As).is_ok()
                || self.peek_token(TokenType::Colon).is_ok()
            {
                self.bump(); // eat `as` or `:`
                Some(self.eat_identifier()?)
            } else {
                None
            };

            // item
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Default,
                    kind,
                    name: None,
                    alias,
                    value: None,
                },
                self.get_span_from(start),
            );
            Ok(item)
        }
        // item
        else {
            // name
            let name = self.eat_identifier()?;

            // alias
            let alias = {
                if self.peek_keyword(Keyword::As).is_ok()
                    || self.peek_token(TokenType::Colon).is_ok()
                {
                    self.bump(); // eat `as` or `:`
                    Some(self.eat_identifier()?)
                } else {
                    None
                }
            };

            // item
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Item,
                    kind,
                    name: Some(name),
                    alias,
                    value: None,
                },
                self.get_span_from(start),
            );
            Ok(item)
        }
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, DependencyItem, DependencyKind, DependencyMode, Expression, ScalarLiteral,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_import_simple() {
        // import dyst
        let mut test = TestParser::new("import \"dyst\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 0);
            assert_string!(parser, *target, "dyst");
        });
    }

    #[test]
    fn test_parse_import_from_expression() {
        let mut test = TestParser::new("import os from 'os'");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import os from 'os'
        assert_node!(parser.tree, expression_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "os");
            });
            assert_string!(parser, *target, "os");
        });
    }

    #[test]
    fn test_parse_import_path_with_arguments() {
        let mut test = TestParser::new("import \"dyst.geometry\" with { bar: true }");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import dyst.geometry with { bar: true }
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, arguments, .. } => {
            // dyst.geometry
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 0);
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

        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "Vector2");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name: Some(name), alias: Some(alias),.. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "Vector3");
                assert_string!(parser, *alias, "V3");
            });
            assert_string!(parser, *target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_as_alias() {
        let mut test = TestParser::new(r#"import * as geom from "ds/geometry""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * as geom from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "geom");
            });
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
        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "./lib/object.ng");

            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias,.. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name: Some(name), alias,.. } => {
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

        assert_node!(parser.tree, import_id, Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            // Default
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "Default");
            });
            // { type Item }
            assert_node!(parser.tree, items[1], DependencyItem { mode, kind, name: Some(name), alias: None,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, *name, "Item");
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

        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert!(target.is_none());
            assert_eq!(items.len(), 2);
            // CreateUIMessage
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], DependencyItem { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, *name, "UIMessage");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_with_namespace() {
        let mut test = TestParser::new("export * from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_with_namespace_alias() {
        let mut test = TestParser::new("export * as foo from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "foo");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_with_module_export() {
        let mut test = TestParser::new("export = foo");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, value: Some(value), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_expression_path!(parser, parser.tree.get(*value), "foo");
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_item() {
        let mut test = TestParser::new("export default foo");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *name, "foo");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_target() {
        let mut test = TestParser::new("export { default } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias, .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_target_with_alias_and_items() {
        let mut test = TestParser::new("export { default as bar, baz as baz, biz } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 3);
            // default as bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // baz as baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "baz");
                assert_string!(parser, *alias, "baz");
            });
            // biz
            assert_node!(parser.tree, items[2], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "biz");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_default_with_multiple_aliases() {
        let mut test = TestParser::new("export { default as bar, default as baz } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 2);
            // default as bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // default as baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "baz");
            });
        });
    }
}
