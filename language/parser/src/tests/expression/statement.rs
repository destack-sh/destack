use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_node, assert_string,
    assert_value_expression_path,
};
use tspp_dir::{
    BinaryOperator, Block, Declaration, Declarator, ExportKind, Expression, GenericArgument,
    Literal, Pattern, TypeDeclaration, TypeExpression, TypeLiteral,
};
use tspp_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

/// Parse labeled statements when the target statement starts on a new line.
#[test]
fn test_parse_labeled_statement_with_newline_before_target() {
    let test = TestParser::new("outer:\nwhile (true) {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::While { label, .. } => {
        assert_string!(parser, label.unwrap(), "outer");
    });
}

/// Parse newline guarded parenthesized statements after continue as separate statements.
#[test]
fn test_parse_statement_newline_before_parenthesized_guard_after_continue_stays_separate() {
    let test = TestParser::new(
        "for (;;) {\n  if (condition) continue\n\n  // breaking comment\n  (possibleArray || []).sort()\n}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::For { body, .. } => {
        assert_node!(parser.tree, *body, Block { leading_expressions, tail_expression, .. } => {
            assert!(tail_expression.is_none());
            assert_eq!(leading_expressions.len(), 2);

            assert_node!(parser.tree, leading_expressions[0], Expression::If { then_expression, .. } => {
                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                        assert!(tail_expression.is_none());
                        assert_eq!(leading_expressions.len(), 1);
                        assert_node!(parser.tree, leading_expressions[0], Expression::Continue { label: None });
                    });
                });
            });

            assert_node!(parser.tree, leading_expressions[1], Expression::Call { .. });
        });
    });
}

/// Report labeled lexical declarations.
#[test]
fn test_report_labeled_lexical_declaration() {
    // source: a: let a
    let test = TestParser::new("a: let a");
    let mut parser = test.prepare();
    let _ = parser.parse_in_place();
    let diagnostic = parser
        .diagnostics()
        .to_vec()
        .into_iter()
        .find(|diagnostic| diagnostic.id == "unexpected-token")
        .expect("expected parse diagnostic");

    // the label separator rejects the following declaration
    assert_eq!(
        parser.span_str(diagnostic.primary_label().target.span().unwrap()),
        ":"
    );
}

#[test]
fn test_parse_elementwise_leading_type_expression() {
    let test = TestParser::new(
        "
type Value =
  | string
  | number
  | boolean
        ",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    // type Value = | string | number | boolean
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            // Value
            assert_string!(parser, name.string(), "Value");
            // | string | number | boolean
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_node!(parser.tree, elements[0], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
                assert_node!(parser.tree, elements[2], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Boolean);
                });
            });
        });
    });
}

/// Keep the leading separator in the container leading span, not the main span.
#[test]
fn test_parse_elementwise_leading_type_expression_keeps_root_span() {
    let test = TestParser::new(
        "
type Value =
  | string
  | number
        ",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // type Value = ...
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            // `Value`
            assert_string!(parser, name.string(), "Value");

            // `string\n  | number`
            let value_span = parser.tree.get_span(*value);
            assert_eq!(parser.span_str(value_span), "string\n  | number");

            // `| `
            let leading_span = parser
                .tree
                .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                .expect("expected leading separator span");
            assert_eq!(parser.span_str(leading_span), "| ");
        });
    });
}

/// Parse a leading elementwise operator in a type expression with doc comments.
#[test]
fn test_parse_elementwise_leading_type_expression_with_docs() {
    let test = TestParser::new(
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
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    // type Target = | "bun" | "node" | "browser"
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Target");
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                    let Literal::String(bun_id) = value else {
                        panic!("expected string literal");
                    };
                    assert_string!(parser, *bun_id, "bun");
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                    let Literal::String(node_id) = value else {
                        panic!("expected string literal");
                    };
                    assert_string!(parser, *node_id, "node");
                });
                assert_node!(parser.tree, elements[2], TypeExpression::Literal { value } => {
                    let Literal::String(browser_id) = value else {
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
    let test = TestParser::new(
        "
const value =
  | 1
  | 2
  | 3",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    // const value = | 1 | 2 | 3
    assert_node!(parser.tree, expr_id, Expression::Let { mutability: _, declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            // | 1 | 2 | 3
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                // 1 | 2
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    // 1
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::Integer(1)));
                    // 2
                    assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
                });
                // 3
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(3)));
            });
        });
    });
}

/// Preserve the leading separator inside one value expression span.
#[test]
fn test_parse_elementwise_leading_value_expression_keeps_root_span() {
    let test = TestParser::new(
        "
const value =
  | 1
  | 2
  | 3",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // const value = ...
    assert_node!(parser.tree, expression_id, Expression::Let { mutability: _, declarators, .. } => {
        // `const`
        assert_eq!(declarators.len(), 1);

        // `| 1\n  | 2\n  | 3`
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_span = parser.tree.get_span(value.unwrap());
            assert_eq!(parser.span_str(value_span), "| 1\n  | 2\n  | 3");
        });
    });
}

/// Parse a statement expression and retain its trailing boundary.
#[test]
fn test_parse_statement_expression() {
    let test = TestParser::new("a;");
    let mut parser = test.prepare();
    let expr_id = parser.parse_statement();

    assert_expression_path!(parser, parser.tree.get(expr_id), "a");
    let span = parser.tree.get_span(expr_id);
    let trailing = parser
        .tree
        .get_side_span(expr_id, NodeSpanType::Boundary(NodeSpanBoundary::Trailing))
        .expect("statement expression should own its semicolon");

    assert_eq!(parser.span_str(span), "a");
    assert_eq!(parser.span_str(trailing), ";");
}

#[test]
fn test_parse_new_type_arguments_with_spaces_in_statement() {
    let test = TestParser::new("new A < T >;");
    let mut parser = test.prepare();
    let expression_id = parser.parse_statement();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "T");
        });

        assert!(arguments.is_empty());
    });
}

/// Parse multiline logical chains with comment-only lines between operators.
#[test]
fn test_parse_multiline_logical_chain_after_comment_lines() {
    let test = TestParser::new("a == 1\n// keep chaining\n&& b == 0\n&& c == 1");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("export const type = struct");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Let { export, declarators, .. } => {
        assert_eq!(*export, Some(ExportKind::Named));
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

/// Parse a const object pattern as a destructuring binding.
#[test]
fn test_parse_const_object_pattern_binding() {
    let test = TestParser::new("const { a } = obj;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let declarator = parser.tree.get(declarators[0]);
        assert_node!(parser.tree, declarator.pattern, Pattern::Object { .. });
    });
}

/// Parse a const brace body as a const evaluation block statement.
#[test]
fn test_parse_const_block_statement() {
    let test = TestParser::new("const { compute() }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Const { body } => {
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

/// Parse a const brace body in an initializer as a const evaluation block.
#[test]
fn test_parse_const_block_initializer() {
    let test = TestParser::new("let x = const { 1 };");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser.tree.get(declarators[0]).value.expect("expected initializer");
        assert_node!(parser.tree, value, Expression::Const { body } => {
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    });
}

/// Parse a const call in an initializer as a const evaluation.
#[test]
fn test_parse_const_call_initializer() {
    let test = TestParser::new("let x = const factorial(10);");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser.tree.get(declarators[0]).value.expect("expected initializer");
        assert_node!(parser.tree, value, Expression::Const { body } => {
            assert_node!(parser.tree, *body, Expression::Call { left, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "factorial");
                assert_eq!(arguments.len(), 1);
            });
        });
    });
}

/// Keep one trailing semicolon block comment trivia entry in no-semi for-of input slices.
#[test]
fn test_parse_no_semi_for_of_slice_trailing_block_comment_is_not_duplicated() {
    let source = "for (a of b) foo\n\n// 11\n;[]\n\nfor (a of b) foo\n\n// 21\n;foo\n\n// prettier-ignore\nfor (   a of   b)   foo (   )\n\n;[]\n\nfor (a of b) foo; /* comment */\n\n// prettier-ignore\nfor (   a of   b) while   (   1)   foo (   )\n\n;[]\n";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let _ = parser.parse_in_place();

    let trailing_block_comment_count = parser
        .comments()
        .iter()
        .filter(|comment| {
            if !comment.is_block() {
                return false;
            }

            parser.span_str(comment.span) == "/* comment */"
        })
        .count();
    assert_eq!(trailing_block_comment_count, 1);
}

/// Parse a multi-line let with multi-line infix.
#[test]
fn test_parse_let_multiline_infix() {
    let test = TestParser::new(
        r"
const x =
    foo.parse()
        + 2
        + x
",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { mutability: _, declarators, .. } => {
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
                    assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
                });
                assert_node!(parser.tree, *right, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "x");
                });
            });
        });
    });
}

/// Parse labeled statements with a label span.
#[test]
fn test_parse_labeled_statement_span() {
    let test = TestParser::new("label: loop {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Loop { label, .. } => {
        assert_string!(parser, label.unwrap(), "label");
    });

    let enclosing_span = parser.tree.get_span(expr_id);
    assert_eq!(parser.span_str(enclosing_span), "label: loop {}");

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected statement label span");
    assert_eq!(parser.span_str(main_span), "label");

    let keyword_span = parser
        .tree
        .get_side_span(expr_id, NodeSpanType::Region(NodeSpanRegion::Keyword))
        .expect("expected loop keyword span");
    assert_eq!(parser.span_str(keyword_span), "loop");
}
