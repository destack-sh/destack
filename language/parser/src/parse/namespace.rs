use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    BlockFormat, Declaration, DeclarationDescriptor, Generics, Keyword, LocalNodeId, Name,
    NodeType, TokenType,
};

impl Parser {
    /// Eat a global augmentation declaration (like `declare global { ... }`).
    pub fn eat_global(
        &mut self,
        start: ParserMark,
        descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_identifier_str("global")?;
        self.eat_newlines_maybe()?;

        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;

        let global = Declaration::Global {
            descriptor,
            expressions,
        };
        Ok(self.tree.insert(global, self.get_span_from(start)))
    }

    /// Eat a namespace declaration (incl. `namespace` or `module` keyword).
    pub fn eat_namespace(
        &mut self,
        start: ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        let is_module = if self.peek_keyword(Keyword::Namespace).is_ok() {
            self.bump(); // eat namespace
            false
        } else {
            self.eat_identifier_str("module")?;
            true
        };

        // name
        let (name, name_span) = if is_module {
            let (name_id, span) = self.eat_string_literal_with_span()?;
            (Some(Name::String(name_id)), Some(span))
        } else if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            (Some(name), Some(span))
        } else {
            (None, None)
        };
        if let Some(name) = name {
            descriptor = descriptor.with_name(name);
        }

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let generics = Generics::new(None, where_clauses);
        let namespace = {
            let expressions = if self
                .peek_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace)
                .is_ok()
            {
                self.eat_newlines_maybe()?;
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
        BinaryOperator, Declaration, DeclarationDescriptor, DeclarationKind, Expression, Name,
        WhereClause,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_declare_global_block() {
        let mut test = TestParser::new(
            r###"
declare global {
    interface Foo {
        bar(): boolean
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Global { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                assert!(descriptor.name.is_none());
                assert!(descriptor.export.is_none());
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_global_block_without_declare() {
        let mut test = TestParser::new_with_options(
            r###"
global {
    interface Foo { }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Global { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                assert!(descriptor.name.is_none());
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_declare_module_block() {
        let mut test = TestParser::new_with_options(
            r###"
declare module "foo" {
    interface Bar { }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                assert_eq!(expressions.len(), 1);
                let name = descriptor.name.expect("expected name");
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_module_block_destack_declaration() {
        let mut test = TestParser::new_with_options(
            r###"
module "foo" {
    interface Bar { }
}
"###,
            LanguageType::DestackDeclaration,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Definition);
                assert_eq!(expressions.len(), 1);
                let name = descriptor.name.expect("expected name");
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_module_block_destack() {
        let mut test = TestParser::new_with_options(
            r###"
module "foo" {
    interface Bar { }
}
"###,
            LanguageType::Destack,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Definition);
                assert_eq!(expressions.len(), 1);
                let name = descriptor.name.expect("expected name");
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(start, DeclarationDescriptor::default())
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

        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(start, DeclarationDescriptor::default())
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
        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(start, DeclarationDescriptor::default())
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
