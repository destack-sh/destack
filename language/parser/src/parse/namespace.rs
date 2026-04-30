use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserSpanStart};

use destack_ast::{
    Ambientness, BlockContext, BlockFormat, Declaration, Expression, GlobalDeclaration, Keyword,
    LocalNodeId, Name, NamespaceDeclaration, NamespaceKind, NodeType, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::expression::common::DeclarationHeader;

impl Parser {
    /// Eat a global augmentation declaration (like `declare global { ... }`).
    pub(crate) fn eat_global(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_identifier_str("global")?;

        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .eat_block_body_in_context(BlockFormat::Explicit, BlockContext::Statement)
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let global = Declaration::Global(GlobalDeclaration {
            ambient: if header.ambient == Ambientness::Ambient {
                Ambientness::Ambient
            } else {
                Ambientness::Concrete
            },
            expressions,
        });
        let global_id = self.insert_node(global, self.get_span_from(start));

        if let Some(span) = header.declare_span {
            self.tree.set_side_span(
                global_id,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
                span,
            );
        }

        Ok(global_id)
    }

    /// Eat a namespace declaration (incl. `namespace` or `module` keyword).
    pub(crate) fn eat_namespace(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_NAMESPACE);
        // keyword
        let namespace_kind = if self.is_keyword(Keyword::Namespace) {
            self.bump(); // eat namespace
            NamespaceKind::Namespace
        } else {
            self.eat_identifier_str("module")?;
            NamespaceKind::Module
        };

        // name
        let (names, name_span) =
            if namespace_kind == NamespaceKind::Module && self.peek_string_literal_is() {
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

        // namespaces always need a name
        if names.is_empty() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        // propagate ambient declaration contexts into module bodies
        let body_flags = if header.ambient == Ambientness::Ambient {
            self.flags.in_declare_context()
        } else {
            self.flags
        };

        let has_body = self.peek_is(TokenType::OpenBrace);
        let expressions = if has_body {
            self.eat_token(TokenType::OpenBrace)?; // eat open brace
            let expressions_result = self.with_flags(body_flags, |parser| {
                parser.eat_block_body_in_context(BlockFormat::Explicit, BlockContext::Statement)
            })?;
            let expressions = expressions_result;
            self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;
            expressions
        } else {
            vec![]
        };

        // reject missing bodies for identifier modules
        if namespace_kind == NamespaceKind::Module
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
        let base_header = header;
        for (index, (name, span)) in names.into_iter().enumerate().rev() {
            let local_header = if index == 0 {
                base_header
            } else {
                let mut inner_header = base_header;
                inner_header.export = None;
                inner_header
            };

            let namespace = Declaration::Namespace(NamespaceDeclaration {
                name,
                export: local_header.export,
                ambient: local_header.ambient,
                kind: namespace_kind,
                generic_parameters: vec![],
                where_clauses: where_clauses.clone().unwrap_or_default(),
                expressions: nested_expressions,
            });
            let current_id = self.insert_node(namespace, self.get_span_from(start));
            self.tree.set_main_span(current_id, span);
            namespace_id = Some(current_id);

            let expression_id = self.insert_node(
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

        Ok(namespace_id.expect("namespace name was validated above"))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Ambientness, Declaration, ExportMode, Expression, GlobalDeclaration, Name,
        NamespaceDeclaration, NamespaceKind, TypeExpression, WhereClause,
    };
    use destack_source::LanguageType;

    use crate::parse::expression::common::DeclarationHeader;
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

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Global(GlobalDeclaration { ambient, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Ambient);
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_global_block_without_declare() {
        let mut test = TestParser::new_with_language(
            r###"
global {
    interface Foo { }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Global(GlobalDeclaration { ambient, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Ambient);
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_declare_module_block() {
        let mut test = TestParser::new_with_language(
            r###"
declare module "foo" {
    interface Bar { }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { name, ambient, kind, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Ambient);
                assert_eq!(*kind, NamespaceKind::Module);
                assert_eq!(expressions.len(), 1);
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, *name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_namespace_with_missing_close_brace() {
        let mut test = TestParser::new("namespace Foo { interface Bar { }");
        let mut parser = test.prepare();

        let start = parser.span_start();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationHeader::default())
            .unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, namespace_id, Declaration::Namespace(NamespaceDeclaration { name, expressions, .. }) => {
            assert_string!(parser, name.string(), "Foo");
            assert_eq!(expressions.len(), 1);
        });
    }

    #[test]
    fn test_parse_declare_module_body_recovers_statement_like_object_properties() {
        // declare module A { "name": ..., "typings": ..., "version": ... }
        let mut test = TestParser::new_with_language(
            r##"
declare module A {
    "name": "troublesome-lib",
    "typings": "lib/index.d.ts",
    "version": "0.0.1"
}
"##,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // declare module A { ... }
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Namespace(NamespaceDeclaration { expressions, .. }) => {
                assert_eq!(expressions.len(), 3);

                // "name": "troublesome-lib",
                let first_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                assert_node!(parser.tree, first_statement_id, Expression::ScalarLiteral(_));

                // "typings": "lib/index.d.ts",
                let second_statement_id = parser.unwrap_labelled_expression(expressions[1]);
                assert_node!(parser.tree, second_statement_id, Expression::ScalarLiteral(_));

                // "version": "0.0.1"
                let third_statement_id = parser.unwrap_labelled_expression(expressions[2]);
                assert_node!(parser.tree, third_statement_id, Expression::ScalarLiteral(_));
            });
        });
    }

    #[test]
    fn test_parse_module_newline_as_identifiers() {
        let mut test = TestParser::new_with_language("module\nFoo\n{}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 3);

        let module_id = expressions[0];
        assert_expression_path!(parser, parser.tree.get(module_id), "module");

        let name_id = expressions[1];
        assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

        let object_id = expressions[2];
        assert_node!(parser.tree, object_id, Expression::Block(_) => {});
    }

    #[test]
    fn test_parse_namespace_newline_as_identifiers() {
        let mut test =
            TestParser::new_with_language("namespace\nFoo\n{}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 3);

        let namespace_id = expressions[0];
        assert_expression_path!(parser, parser.tree.get(namespace_id), "namespace");

        let name_id = expressions[1];
        assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

        let object_id = expressions[2];
        assert_node!(parser.tree, object_id, Expression::Block(_) => {});
    }

    /// Parse a global augmentation inside a module declaration.
    #[test]
    fn test_parse_nested_global_block_in_string_module() {
        let mut test = TestParser::new_with_language(
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

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { name, ambient, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Ambient);
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, *name_id, "buffer");
                });
                assert_eq!(expressions.len(), 1);

                let nested_id = expressions[0];
                assert_node!(parser.tree, nested_id, Expression::Declaration(global_id) => {
                    assert_node!(parser.tree, *global_id, Declaration::Global(GlobalDeclaration { ambient, expressions, .. }) => {
                        assert_eq!(*ambient, Ambientness::Ambient);
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_nested_global_block_in_module() {
        let mut test = TestParser::new_with_language(
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

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { name, ambient, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Ambient);
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, *name_id, "m");
                });
                assert_eq!(expressions.len(), 1);

                let nested_id = expressions[0];
                assert_node!(parser.tree, nested_id, Expression::Declaration(global_id) => {
                    assert_node!(parser.tree, *global_id, Declaration::Global(GlobalDeclaration { ambient, expressions, .. }) => {
                        assert_eq!(*ambient, Ambientness::Ambient);
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_module_block_declaration() {
        let mut test = TestParser::new_with_language(
            r###"
module "foo" {
    interface Bar { }
}
"###,
            LanguageType::DestackDeclaration,
        );
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { name, ambient, kind, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Concrete);
                assert_eq!(*kind, NamespaceKind::Module);
                assert_eq!(expressions.len(), 1);
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, *name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_module_block() {
        let mut test = TestParser::new_with_language(
            r###"
module "foo" {
    interface Bar { }
}
"###,
            LanguageType::Destack,
        );
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { name, ambient, expressions, .. }) => {
                assert_eq!(*ambient, Ambientness::Concrete);
                assert_eq!(expressions.len(), 1);
                assert_node!(name, Name::String(name_id) => {
                    assert_string!(parser, *name_id, "foo");
                });
            });
        });
    }

    #[test]
    fn test_reject_identifier_module_without_body() {
        let mut test = TestParser::new_with_language("module Foo;", LanguageType::Destack);
        let mut parser = test.prepare();
        let result = parser.eat_expression(parser.flags);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_namespace() {
        let mut test = TestParser::new("namespace { }");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let result = parser.eat_namespace(&start, DeclarationHeader::default());
        assert!(result.is_err());
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

        let start = parser.span_start();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, namespace_id, Declaration::Namespace(NamespaceDeclaration { name, export, ambient, kind, expressions, where_clauses, .. }) => {
            assert_eq!(*ambient, Ambientness::Concrete);
            assert_eq!(*kind, NamespaceKind::Namespace);
            assert_string!(parser, name.string(), "Foo");
            assert!(export.is_none());
            assert!(expressions.is_empty());
            assert_eq!(where_clauses.len(), 1);

            // where Guard: Limit
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
        });
    }

    #[test]
    fn test_parse_forward_namespace_with_where() {
        let mut test = TestParser::new("namespace Foo where Requirement: Interface { }");
        let mut parser = test.prepare();
        let start = parser.span_start();
        let namespace_id = parser
            .eat_namespace(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, namespace_id, Declaration::Namespace(NamespaceDeclaration { name, export, ambient, kind, expressions, where_clauses, .. }) => {
            assert_eq!(*ambient, Ambientness::Concrete);
            assert_eq!(*kind, NamespaceKind::Namespace);
            assert_string!(parser, name.string(), "Foo");
            assert!(export.is_none());
            assert!(expressions.is_empty());
            assert_eq!(where_clauses.len(), 1);

            // where Requirement: Interface
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });
        });
    }

    #[test]
    fn test_parse_namespace_directive_then_export_with_comment_newline() {
        let mut test = TestParser::new_with_language(
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
            assert_node!(parser.tree, *decl_id, Declaration::Namespace(NamespaceDeclaration { expressions, .. }) => {
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(_) => {});
                assert_node!(parser.tree, expressions[1], Expression::Let { export, .. } => {
                    assert_eq!(*export, Some(ExportMode::Named));
                });
            });
        });
    }
}
