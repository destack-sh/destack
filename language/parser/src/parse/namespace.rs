use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    BlockFormat, Declaration, DeclarationDescriptor, DeclarationKind, Expression, Generics,
    Keyword, LocalNodeId, Name, NodeType, TokenType,
};

impl Parser {
    /// Eat a global augmentation declaration (like `declare global { ... }`).
    pub fn eat_global(
        &mut self,
        start: &ParserMark,
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
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_NAMESPACE);
        // keyword
        let is_module = if self.is_keyword(Keyword::Namespace) {
            self.bump(); // eat namespace
            false
        } else {
            self.eat_identifier_str("module")?;
            true
        };

        // name
        let (names, name_span) = if is_module && self.peek_string_literal_is() {
            let (name_id, span) = self.eat_string_literal_with_span()?;
            (vec![(Name::String(name_id), span)], Some(span))
        } else if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            let mut names = vec![(name, span)];
            while self.peek_is(TokenType::Dot) {
                self.bump(); // eat dot
                let (segment, segment_span) = self.eat_identifier_with_span()?;
                names.push((Name::Identifier(segment), segment_span));
            }
            (names, Some(span))
        } else {
            (Vec::new(), None)
        };

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let generics = Generics::new(None, where_clauses);
        // propagate ambient declaration contexts into module bodies
        let body_options = if descriptor.kind == DeclarationKind::Declaration {
            self.options.in_declare_context()
        } else {
            self.options
        };

        let has_body =
            self.is_token_after_newlines(self.pos().saturating_sub(1), TokenType::OpenBrace);
        let expressions = if has_body {
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::OpenBrace)?; // eat open brace
            let expressions = self
                .with_options(body_options, |parser| {
                    parser.eat_block_body(BlockFormat::Explicit)
                })
                .for_node_type(NodeType::Block)?;
            self.eat_token(TokenType::CloseBrace)
                .for_node_type(NodeType::Declaration)?;
            expressions
        } else {
            vec![]
        };

        // reject missing bodies for identifier modules
        if is_module
            && !has_body
            && names
                .first()
                .is_some_and(|(name, _)| !matches!(name, Name::String(_)))
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // require a statement boundary after external module declarations
        if !has_body && !self.is_statement_stop() && !self.peek_is(TokenType::CloseBrace) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // build nested namespaces from dot separated names
        let mut nested_expressions = expressions;
        let mut namespace_id = None;
        let base_descriptor = descriptor;
        for (index, (name, span)) in names.into_iter().enumerate().rev() {
            let mut local_descriptor = if index == 0 {
                base_descriptor.clone()
            } else {
                let mut inner_descriptor = base_descriptor.clone();
                inner_descriptor.export = None;
                inner_descriptor
            };
            local_descriptor = local_descriptor.with_name(name);

            let namespace = Declaration::Namespace {
                descriptor: local_descriptor,
                generics: generics.clone(),
                expressions: nested_expressions,
            };
            let current_id = self.tree.insert(namespace, self.get_span_from(start));
            self.tree.set_main_span(current_id, span);
            namespace_id = Some(current_id);

            let expression_id = self.tree.insert(
                Expression::Declaration(current_id),
                self.get_span_from(start),
            );
            nested_expressions = vec![expression_id];
        }

        if let Some(span) = name_span
            && let Some(namespace_id) = namespace_id
        {
            self.tree.set_main_span(namespace_id, span);
        }

        Ok(namespace_id.unwrap_or_else(|| {
            self.tree.insert(
                Declaration::Namespace {
                    descriptor: base_descriptor,
                    generics,
                    expressions: nested_expressions,
                },
                self.get_span_from(start),
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Declaration, DeclarationDescriptor, DeclarationKind, DependencyMode, Expression, Name,
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

        let expr_id = parser.eat_expression(parser.options).unwrap();
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

        let expr_id = parser.eat_expression(parser.options).unwrap();
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

        let expr_id = parser.eat_expression(parser.options).unwrap();
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
    fn test_parse_module_newline_as_identifiers() {
        let mut test = TestParser::new_with_options("module\nFoo\n{}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 3);

        let module_id = match parser.tree.get(expressions[0]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[0],
        };
        assert_expression_path!(parser, parser.tree.get(module_id), "module");

        let name_id = match parser.tree.get(expressions[1]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[1],
        };
        assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

        let object_id = match parser.tree.get(expressions[2]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[2],
        };
        assert_node!(parser.tree, object_id, Expression::Block(_) => {});
    }

    #[test]
    fn test_parse_namespace_newline_as_identifiers() {
        let mut test = TestParser::new_with_options("namespace\nFoo\n{}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 3);

        let namespace_id = match parser.tree.get(expressions[0]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[0],
        };
        assert_expression_path!(parser, parser.tree.get(namespace_id), "namespace");

        let name_id = match parser.tree.get(expressions[1]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[1],
        };
        assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

        let object_id = match parser.tree.get(expressions[2]) {
            Expression::Statement(statement_id) => *statement_id,
            _ => expressions[2],
        };
        assert_node!(parser.tree, object_id, Expression::Block(_) => {});
    }

    /// Parse a global augmentation inside a module declaration.
    #[test]
    fn test_parse_nested_global_block_in_module() {
        let mut test = TestParser::new_with_options(
            r###"
declare module "buffer" {
    global {
        var Buffer: BufferConstructor;
    }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                let name = descriptor.name.expect("expected name");
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, name_id, "buffer");
                });
                assert_eq!(expressions.len(), 1);

                let nested_id = expressions[0];
                assert_node!(parser.tree, nested_id, Expression::Declaration(global_id) => {
                    assert_node!(parser.tree, *global_id, Declaration::Global { descriptor, expressions, .. } => {
                        assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                        assert!(descriptor.name.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_nested_global_block_in_module_typescript() {
        let mut test = TestParser::new_with_options(
            r###"
declare module "m" {
    global {
        var x: number;
    }
}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { descriptor, expressions, .. } => {
                assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                let name = descriptor.name.expect("expected name");
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, name_id, "m");
                });
                assert_eq!(expressions.len(), 1);

                let nested_id = expressions[0];
                assert_node!(parser.tree, nested_id, Expression::Declaration(global_id) => {
                    assert_node!(parser.tree, *global_id, Declaration::Global { descriptor, expressions, .. } => {
                        assert_eq!(descriptor.kind, DeclarationKind::Declaration);
                        assert!(descriptor.name.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
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

        let expr_id = parser.eat_expression(parser.options).unwrap();
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

        let expr_id = parser.eat_expression(parser.options).unwrap();
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
    fn test_reject_identifier_module_without_body_in_destack() {
        let mut test = TestParser::new_with_options("module Foo;", LanguageType::Destack);
        let mut parser = test.prepare();
        let result = parser.eat_expression(parser.options);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationDescriptor::default())
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
namespace Foo where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Declaration::Namespace { descriptor, expressions, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(descriptor.export.is_none());
            assert!(expressions.is_empty());
            assert!(!generics.is_empty());
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);

            // where Guard: Limit
            assert_node!(parser.tree, where_items[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
        });
    }

    #[test]
    fn test_parse_forward_namespace_with_where() {
        let mut test = TestParser::new("namespace Foo where Requirement: Interface { }");
        let mut parser = test.prepare();
        let start = parser.mark();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationDescriptor::default())
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
            assert_node!(parser.tree, where_items[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });
        });
    }

    #[test]
    fn test_parse_namespace_directive_then_export_with_comment_newline() {
        let mut test = TestParser::new_with_options(
            r###"
namespace M {
  /******/ 'use strict'
  /******/ export const a = 1;
}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 1);

        assert_node!(parser.tree, expressions[0], Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace { expressions, .. } => {
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
                    assert_node!(parser.tree, *statement_id, Expression::ScalarLiteral(_) => {});
                });
                assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
                    assert_node!(parser.tree, *statement_id, Expression::Let { descriptor, .. } => {
                        assert_eq!(descriptor.export, Some(DependencyMode::Item));
                    });
                });
            });
        });
    }
}
