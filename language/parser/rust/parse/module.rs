use dyst_ast::ModuleStyle;

use crate::TokenType;
use crate::parse::prelude::*;

use crate::{
    BlockFormat, Definition, DefinitionMeta, Keyword, ModuleFormat, NodeId, NodeType, Parser,
    ParserResult,
};

impl<'a> Parser<'a> {
    /// Eat a module declaration (incl. `module` keyword).
    pub fn eat_module(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        let keyword = self
            .eat_keyword_in(&[Keyword::Module, Keyword::Namespace])
            .for_node_type(NodeType::Definition)?;
        let style = match keyword {
            Keyword::Module => ModuleStyle::Module,
            Keyword::Namespace => ModuleStyle::Namespace,
            _ => unreachable!(),
        };

        // name
        meta.name = self.eat_name_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let module = {
            // inline module
            if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.bump(); // eat open brace
                let expressions = self
                    .eat_block_body(BlockFormat::Explicit)
                    .for_node_type(NodeType::Block)?;
                self.eat_token(TokenType::CloseBrace)
                    .for_node_type(NodeType::Definition)?;
                Definition::Module {
                    meta,
                    format: ModuleFormat::Inline,
                    style,
                    with_clauses,
                    where_clauses,
                    expressions,
                }
            }
            // forward module
            else {
                Definition::Module {
                    meta,
                    format: ModuleFormat::Forward,
                    style,
                    with_clauses,
                    where_clauses,
                    expressions: Vec::new(),
                }
            }
        };

        let module_id = self.tree.insert(module, self.get_span_from(start));
        Ok(module_id)
    }

    // Eat a module body (aka a module file, without `module` keyword or braces).
    pub fn eat_module_body(
        &mut self,
        meta: DefinitionMeta,
        format: ModuleFormat,
        style: ModuleStyle,
    ) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let expressions = self
            .eat_block_body(BlockFormat::Implicit)
            .for_node_type(NodeType::Block)?;
        let module = Definition::Module {
            meta,
            format,
            style,
            with_clauses,
            where_clauses,
            expressions,
        };
        let module_id = self.tree.insert(module, self.get_span_from(start));
        Ok(module_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::DefinitionMeta;

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, ModuleFormat, WhereClause, WithClause,
        assert_expr_path, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_empty_module() {
        let mut test = TestParser::new("module { }");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { meta, expressions, format, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(meta.visibility.is_none());
            assert!(meta.export.is_none());
            assert_eq!(*format, ModuleFormat::Inline);
            assert!(expressions.is_empty());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_forward_module() {
        let mut test = TestParser::new("module x;");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { meta, expressions, format, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "x");
            assert!(meta.visibility.is_none());
            assert!(meta.export.is_none());
            assert_eq!(*format, ModuleFormat::Forward);
            assert!(expressions.is_empty());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_inline_module_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
module Foo with Context where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // module Foo with Context where Guard > Limit { }
        let module_id = parser.eat_module(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { meta, format, expressions, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert_eq!(*format, ModuleFormat::Inline);
            assert!(meta.export.is_none());
            assert!(expressions.is_empty());

            let with_items = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);

            // with Context
            assert_node!(parser.tree, with_items[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            let where_items = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);

            // where Guard > Limit
            assert_node!(parser.tree, where_items[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
        });
    }

    #[test]
    fn test_parse_forward_module_with_with_and_where() {
        let mut test = TestParser::new("module Foo with Context where Requirement: Interface;");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(DefinitionMeta::default()).unwrap();

        assert_node!(parser.tree, module_id, Definition::Module { meta, format, expressions, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert_eq!(*format, ModuleFormat::Forward);
            assert!(meta.export.is_none());
            assert!(expressions.is_empty());
            let with_items = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);

            // with Context
            assert_node!(parser.tree, with_items[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            let where_items = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);

            // where Requirement: Interface
            assert_node!(parser.tree, where_items[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });
        });
    }
}
