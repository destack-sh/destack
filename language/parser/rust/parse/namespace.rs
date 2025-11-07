use crate::TokenType;
use crate::parse::prelude::*;

use crate::{
    BlockFormat, Definition, DefinitionMeta, Keyword, NodeId, NodeType, Parser, ParserResult,
};

impl<'a> Parser<'a> {
    /// Eat a namespace declaration (incl. `namespace` keyword).
    pub fn eat_namespace(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Namespace)?;

        // name
        meta = meta.with_name_or_key_maybe(self.eat_name_or_key_maybe()?);

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let namespace = {
            let expressions = if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.eat_token(TokenType::OpenBrace)?; // eat open brace
                let expressions = self
                    .eat_block_body(BlockFormat::Explicit)
                    .for_node_type(NodeType::Block)?;
                self.eat_token(TokenType::CloseBrace)
                    .for_node_type(NodeType::Definition)?;
                expressions
            } else {
                vec![]
            };
            Definition::Namespace {
                meta,
                with_clauses,
                where_clauses,
                expressions,
            }
        };

        let namespace_id = self.tree.insert(namespace, self.get_span_from(start));
        Ok(namespace_id)
    }

    // Eat a namespace body (aka a namespace file, without `namespace` keyword or braces).
    pub fn eat_namespace_body(&mut self, meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let expressions = self
            .eat_block_body(BlockFormat::Implicit)
            .for_node_type(NodeType::Block)?;
        let namespace = Definition::Namespace {
            meta,
            with_clauses,
            where_clauses,
            expressions,
        };
        let namespace_id = self.tree.insert(namespace, self.get_span_from(start));
        Ok(namespace_id)
    }

    /// Eat an implicit namespace body (without `namespace` keyword or braces).
    pub fn eat_implicit_namespace_with_recovery(
        &mut self,
        meta: DefinitionMeta,
    ) -> Option<NodeId<Definition>> {
        self.with_recovery(
            self.mark(),
            |parser| parser.eat_namespace_body(meta).map(Some),
            None,
            TokenType::End,
        )
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::DefinitionMeta;

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, WhereClause, WithClause, assert_expr_path,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let namespace_id = parser.eat_namespace(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, namespace_id, Definition::Namespace { meta, expressions, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(meta.visibility.is_none());
            assert!(meta.export.is_none());
            assert!(expressions.is_empty());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_inline_namespace_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
namespace Foo with Context where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // namespace Foo with Context where Guard > Limit { }
        let namespace_id = parser.eat_namespace(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, namespace_id, Definition::Namespace { meta, expressions, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
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
    fn test_parse_forward_namespace_with_with_and_where() {
        let mut test =
            TestParser::new("namespace Foo with Context where Requirement: Interface { }");
        let mut parser = test.prepare();
        let namespace_id = parser.eat_namespace(DefinitionMeta::default()).unwrap();

        assert_node!(parser.tree, namespace_id, Definition::Namespace { meta, expressions, with_clauses, where_clauses, .. } => {
            assert_eq!(meta.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
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
