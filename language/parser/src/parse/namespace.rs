use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_ast::{
    BlockFormat, Declaration, DeclarationDescriptor, Generics, Keyword, LocalNodeId, NodeType,
    TokenType,
};

impl Parser {
    /// Eat a namespace declaration (incl. `namespace` keyword).
    pub fn eat_namespace(
        &mut self,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Namespace)?;

        // name
        let name_span = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            descriptor = descriptor.with_name(name);
            Some(span)
        } else {
            None
        };

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let generics = Generics::new(None, where_clauses);
        let namespace = {
            let expressions = if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.eat_token(TokenType::OpenBrace)?; // eat open brace
                let expressions = self
                    .eat_block_body(BlockFormat::Explicit)
                    .for_node_type(NodeType::Block)?;
                self.eat_token(TokenType::CloseBrace)
                    .for_node_type(NodeType::Declaration)?;
                expressions
            } else {
                vec![]
            };
            Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            }
        };

        let namespace_id = self.tree.insert(namespace, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(namespace_id, span);
        }

        Ok(namespace_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        BinaryOperator, Declaration, DeclarationDescriptor, DeclarationKind, Expression,
        WhereClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Declaration::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(descriptor.name.is_none());
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(generics.is_empty());
        });
    }

    #[test]
    fn test_parse_inline_namespace_with_where() {
        let mut test = TestParser::new(
            r###"
namespace Foo where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Declaration::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(!generics.is_empty());
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
    fn test_parse_forward_namespace_with_where() {
        let mut test = TestParser::new("namespace Foo where Requirement: Interface { }");
        let mut parser = test.prepare();
        let namespace_id = parser
            .eat_namespace(DeclarationDescriptor::default())
            .unwrap();

        assert_node!(parser.tree, namespace_id, Declaration::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(!generics.is_empty());
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
