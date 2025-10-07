use crate::TokenType;
use crate::parse::prelude::*;
use dyst_source::StringId;

use crate::{
    AstResult, BlockFormat, Definition, Keyword, ModuleFormat, NodeId, NodeType, Parser, Visibility,
};

impl<'a> Parser<'a> {
    /// Eat a module declaration (incl. `module` keyword).
    pub fn eat_module(&mut self, visibility: Option<Visibility>) -> AstResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Module)
            .for_node_type(NodeType::Definition)?;

        // name
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

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
                    format: ModuleFormat::Inline,
                    name,
                    visibility,
                    with_clauses,
                    where_clauses,
                    expressions,
                }
            }
            // forward module
            else {
                Definition::Module {
                    format: ModuleFormat::Forward,
                    name,
                    visibility,
                    with_clauses,
                    where_clauses,
                    expressions: Vec::new(),
                }
            }
        };

        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }

    // Eat a module body (aka a module file, without `module` keyword or braces).
    pub fn eat_module_body(
        &mut self,
        visibility: Option<Visibility>,
        name: Option<StringId>,
        format: ModuleFormat,
    ) -> AstResult<NodeId<Definition>> {
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
            format,
            name,
            visibility,
            with_clauses,
            where_clauses,
            expressions,
        };
        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, ModuleFormat, WhereClause, WithClause,
        assert_expr_path, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_empty_module() {
        let mut test = TestParser::new("module { }");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(None).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { name, visibility, expressions, format, with_clauses, where_clauses } => {
            assert!(name.is_none());
            assert!(visibility.is_none());
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
        let module_id = parser.eat_module(None).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { name, visibility, expressions, format, with_clauses, where_clauses } => {
            assert_string!(parser, name.unwrap(), "x");
            assert!(visibility.is_none());
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
        let module_id = parser.eat_module(None).unwrap();
        assert_node!(parser.tree, module_id, Definition::Module { name, format, expressions, with_clauses, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert_eq!(*format, ModuleFormat::Inline);
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
        let mut test = TestParser::new("module Foo with Context where Requirement: Trait;");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(None).unwrap();

        assert_node!(parser.tree, module_id, Definition::Module { name, format, expressions, with_clauses, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert_eq!(*format, ModuleFormat::Forward);
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

            // where Requirement: Trait
            assert_node!(parser.tree, where_items[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Trait");
                });
            });
        });
    }
}
