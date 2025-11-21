use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{
    BlockFormat, DeclarationDescriptor, Definition, Generics, Keyword, LocalNodeId, NodeType, TokenType,
};

impl<'a> Parser<'a> {
    /// Eat a namespace declaration (incl. `namespace` keyword).
    pub fn eat_namespace(
        &mut self,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Namespace)?;

        // name
        descriptor = descriptor.with_name_maybe(self.eat_name_maybe()?);

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let generics = Generics::new(None, with_clauses, where_clauses);
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
                descriptor,
                generics,
                expressions,
            }
        };

        let namespace_id = self.tree.insert(namespace, self.get_span_from(start));
        Ok(namespace_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        BinaryOperator, DeclarationDescriptor, Definition, Expression, WhereClause, WithClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Definition::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, dyst_ast::DeclarationKind::Definition);
            assert!(descriptor.name.is_none());
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(generics.is_empty());
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
        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Definition::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(!generics.is_empty());
            let with_items = generics.with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);

            // with Context
            assert_node!(parser.tree, with_items[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);

            // where Guard > Limit
            assert_node!(parser.tree, where_items[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expression_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expression_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
        });
    }

    #[test]
    fn test_parse_forward_namespace_with_with_and_where() {
        let mut test =
            TestParser::new("namespace Foo with Context where Requirement: Interface { }");
        let mut parser = test.prepare();
        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, namespace_id, Definition::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, dyst_ast::DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(!generics.is_empty());
            let with_items = generics.with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);

            // with Context
            assert_node!(parser.tree, with_items[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
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
