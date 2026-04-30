use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_string};
use destack_ast::*;
use destack_source::{LanguageType, NodeSpanBoundary, NodeSpanType};

/// Parse labelled statements when the target statement starts on a new line.
#[test]
fn test_parse_labelled_statement_with_newline_before_target() {
    let mut test = TestParser::new("outer:\nwhile (true) {}");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Labelled { label, body } => {
        assert_string!(parser, *label, "outer");
        assert_node!(parser.tree, *body, Expression::While { .. });
    });
}

/// Parse newline guarded parenthesized statements after continue as separate statements.
#[test]
fn test_parse_statement_newline_before_parenthesized_guard_after_continue_stays_separate() {
    let mut test = TestParser::new_with_language(
        "for (;;) {\n  if (condition) continue\n\n  // breaking comment\n  (possibleArray || []).sort()\n}",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::For { body, .. } => {
        assert_node!(parser.tree, *body, Block { leading_expressions, tail_expression, .. } => {
            assert!(tail_expression.is_none());
            let expressions = leading_expressions;
            assert_eq!(expressions.len(), 2);

            assert_node!(parser.tree, expressions[0], Expression::If { then_expression, .. } => {
                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                        assert!(tail_expression.is_none());
                        assert_eq!(leading_expressions.len(), 1);
                        assert_node!(parser.tree, leading_expressions[0], Expression::Continue { label: None });
                    });
                });
            });
        });
    });
}

/// Reject labelled lexical declarations.
#[test]
fn test_reject_labelled_lexical_declaration() {
    // source: a: let a
    let mut test = TestParser::new_with_language("a: let a", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let _ = parser.parse();
    let diagnostic = parser
        .diagnostics
        .iter()
        .into_iter()
        .find(|diagnostic| diagnostic.code.starts_with("EP"))
        .expect("expected parse diagnostic");

    // let a
    assert_eq!(parser.get_span_str(diagnostic.primary_span.span), "let a");
}

/// Parse a leading elementwise operator in a type expression.
#[test]
fn test_parse_elementwise_leading_type_expression() {
    let mut test = TestParser::new(
        "
type Value =
  | string
  | number
  | boolean
        ",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type Value = | string | number | boolean
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            // Value
            assert_string!(parser, name.string(), "Value");
            // | string | number | boolean
            assert_node!(parser.tree, *value, destack_ast::TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
                assert_node!(parser.tree, elements[2], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Boolean);
                });
            });
        });
    });
}

/// Keep the leading separator in the container leading span, not the main span.
#[test]
fn test_parse_elementwise_leading_type_expression_keeps_root_span() {
    let mut test = TestParser::new(
        "
type Value =
  | string
  | number
        ",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // type Value = ...
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            // `Value`
            assert_string!(parser, name.string(), "Value");

            // `string\n  | number`
            let value_span = parser.tree.get_span(*value);
            assert_eq!(parser.get_span_str(value_span), "string\n  | number");

            // `| `
            let leading_span = parser
                .tree
                .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                .expect("expected leading separator span");
            assert_eq!(parser.get_span_str(leading_span), "| ");
        });
    });
}

/// Parse a leading elementwise operator in a type expression with doc comments.
#[test]
fn test_parse_elementwise_leading_type_expression_with_docs() {
    let mut test = TestParser::new(
        r###"
type Target =
  /**
   * bun
   */
  | "bun"
  /**
   * node
   */
  | "node"
  /**
   * browser
   */
  | "browser"
            "###,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type Target = | "bun" | "node" | "browser"
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Target");
            assert_node!(parser.tree, *value, destack_ast::TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_node!(parser.tree, elements[0], TypeExpression::ScalarLiteral { value } => {
                    let ScalarLiteral::String(bun_id) = value else {
                        panic!("expected string literal");
                    };
                    assert_string!(parser, *bun_id, "bun");
                });
                assert_node!(parser.tree, elements[1], TypeExpression::ScalarLiteral { value } => {
                    let ScalarLiteral::String(node_id) = value else {
                        panic!("expected string literal");
                    };
                    assert_string!(parser, *node_id, "node");
                });
                assert_node!(parser.tree, elements[2], TypeExpression::ScalarLiteral { value } => {
                    let ScalarLiteral::String(browser_id) = value else {
                        panic!("expected string literal");
                    };
                    assert_string!(parser, *browser_id, "browser");
                });
            });
        });
    });
}

/// Parse a leading elementwise operator in a value expression.
#[test]
fn test_parse_elementwise_leading_value_expression() {
    let mut test = TestParser::new(
        "
const value =
  | 1
  | 2
  | 3",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // const value = | 1 | 2 | 3
    assert_node!(parser.tree, expr_id, Expression::Let { mutability, declarators, .. } => {
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            // | 1 | 2 | 3
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                // 1 | 2
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    // 1
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    // 2
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                // 3
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    });
}

/// Preserve the leading separator inside one value expression span.
#[test]
fn test_parse_elementwise_leading_value_expression_keeps_root_span() {
    let mut test = TestParser::new(
        "
const value =
  | 1
  | 2
  | 3",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // const value = ...
    assert_node!(parser.tree, expression_id, Expression::Let { mutability, declarators, .. } => {
        // `const`
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(declarators.len(), 1);

        // `| 1\n  | 2\n  | 3`
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_span = parser.tree.get_span(value.unwrap());
            assert_eq!(parser.get_span_str(value_span), "| 1\n  | 2\n  | 3");
        });
    });
}

/// Parse a statement expression.
#[test]
fn test_parse_statement_expression() {
    let mut test = TestParser::new("a;");
    let mut parser = test.prepare();
    let expr_id = parser.try_eat_statement_expression().unwrap();
    // a;
    assert_expression_path!(parser, parser.tree.get(expr_id), "a");
}

#[test]
fn test_parse_new_without_parenthesized_type_arguments_in_statement() {
    let mut test = TestParser::new_with_language("new A < T;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.try_eat_statement_expression().unwrap();

    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::LessThan);
        assert_node!(parser.tree, *left, Expression::New { left, generic_arguments, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
        });
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Parse multiline logical chains with comment-only lines between operators.
#[test]
fn test_parse_multiline_logical_chain_after_comment_lines() {
    let language = LanguageType::TypeScript;
    let mut test =
        TestParser::new_with_language("a == 1\n// keep chaining\n&& b == 0\n&& c == 1", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // a == 1 && b == 0 && c == 1
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::And);
        assert_node!(parser.tree, *right, Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::Equal);
        });
        assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::And);
            assert_node!(parser.tree, *left, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::Equal);
            });
            assert_node!(parser.tree, *right, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::Equal);
            });
        });
    });
}

#[test]
fn test_parse_export_const_type_identifier_with_struct_value() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language("export const type = struct", language);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Let { export, declarators, .. } => {
        assert_eq!(*export, Some(ExportMode::Named));
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "type");
            });
            let value_id = value.expect("expected initializer");
            assert_expression_path!(parser, parser.tree.get(value_id), "struct");
        });
    });
}

/// Test const enum declaration.
#[test]
fn test_parse_const_enum() {
    let mut test = TestParser::new("const enum Foo { A, B }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // const enum Foo { A, B }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Enum(EnumDeclaration { name, kind, fields, .. }) => {
            assert_string!(parser, name.unwrap().string(), "Foo");
            assert_eq!(*kind, EnumKind::Const);
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "A");
                assert!(value.is_none());
            });
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "B");
                assert!(value.is_none());
            });
        });
    });
}

/// Keep one trailing semicolon block comment trivia entry in no-semi for-of input slices.
#[test]
fn test_parse_no_semi_for_of_slice_trailing_block_comment_is_not_duplicated() {
    let source = "for (a of b) foo\n\n// 11\n;[]\n\nfor (a of b) foo\n\n// 21\n;foo\n\n// prettier-ignore\nfor (   a of   b)   foo (   )\n\n;[]\n\nfor (a of b) foo; /* comment */\n\n// prettier-ignore\nfor (   a of   b) while   (   1)   foo (   )\n\n;[]\n";
    let mut test = TestParser::new_with_language(source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let _ = parser.parse();
    parser.attach_comments();

    let trailing_block_comment_count = parser
        .tree
        .comments()
        .iter()
        .filter(|comment| {
            if !comment.is_block() {
                return false;
            }

            parser.get_span_str(comment.span) == "/* comment */"
        })
        .count();
    assert_eq!(trailing_block_comment_count, 1);
}

/// Parse a multi-line let with multi-line infix.
#[test]
fn test_parse_let_multiline_infix() {
    let mut test = TestParser::new(
        r"
const x =
    foo.parse()
        + 2
        + x
",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { mutability, declarators, .. } => {
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                            assert_string!(parser, *name, "parse");
                            assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                                assert_string!(parser, *name, "foo");
                            });
                        });
                    });
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                assert_node!(parser.tree, *right, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "x");
                });
            });
        });
    });
}

/// Parse labelled statements with a label span.
#[test]
fn test_parse_labelled_statement_span() {
    let mut test = TestParser::new("label: loop {}");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Labelled { label, .. } => {
        assert_string!(parser, *label, "label");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected label main span");
    assert_eq!(parser.get_span_str(main_span), "label");
}
