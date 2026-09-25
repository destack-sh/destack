use crate::{ExpressionPosition, ExpressionStop};
use std::fmt::Write;
use tspp_dir::{
    Argument, BinaryOperator, Declaration, Expression, FunctionDeclaration, GenericArgument,
    InferForm, Literal, LocalNodeId, NodeType, PostfixPosition, TokenType, TypeExpression,
};

use crate::{
    Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string,
    assert_value_expression_path,
};

fn make_receiver(parser: &mut Parser) -> LocalNodeId<Expression> {
    let receiver_str = parser.strings.intern("receiver");
    let span = parser.peek_token_span().span;
    parser.insert_node(Expression::Identifier { name: receiver_str }, span.range())
}

#[test]
fn test_parse_index_postfix_explicit() {
    // [1]
    let test = TestParser::new("[1]");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);

    let index_id = parser
        .parse_index(recv, PostfixPosition::Direct, Default::default(), false)
        .unwrap();
    let main_range = parser
        .tree
        .get_main_range(index_id)
        .expect("missing index main range");
    assert_eq!(parser.range_str(main_range), "[");

    assert_node!(parser.tree, index_id, Expression::Index { position, left, index, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);
        assert_node!(parser.tree, index.unwrap(), Expression::Literal(Literal::Integer(1)));
    });
}

#[test]
fn test_parse_index_postfix_multiline() {
    // [
    //   1
    // ]
    let test = TestParser::new("[\n  1\n]");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);

    let index_id = parser
        .parse_index(recv, PostfixPosition::Direct, Default::default(), false)
        .unwrap();
    assert_node!(parser.tree, index_id, Expression::Index { position, left, index, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);
        assert_node!(parser.tree, index.unwrap(), Expression::Literal(Literal::Integer(1)));
    });
}

#[test]
fn test_parse_call_postfix() {
    // (1, 2)
    let test = TestParser::new("(1, 2)");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);
    let call_id = parser
        .parse_call(
            recv,
            Vec::new(),
            PostfixPosition::Direct,
            Default::default(),
            false,
        )
        .unwrap();

    assert_node!(parser.tree, call_id, Expression::Call { position, left, generic_arguments: _, arguments, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);

        // (1, 2)
        assert_eq!(arguments.len(), 2);

        // 1
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });

        // 2
        assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        });
    });
}

#[test]
fn test_parse_member_postfix_missing_name() {
    // foo.
    let test = TestParser::new("foo.");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // foo.
    assert_node!(parser.tree, expression_id, Expression::Member { left, name: None, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
    });
}

#[test]
fn test_parse_optional_member_postfix_missing_name() {
    // foo?.
    let test = TestParser::new("foo?.");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // foo?.
    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Member { left, name: None, is_optional, .. } => {
            assert!(*is_optional);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    });
}

#[test]
fn test_parse_parenthesized_member_postfix_missing_name_preserves_outer_close() {
    // (foo.)
    let test = TestParser::new("(foo.)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, ")")]);

    // (foo.)
    crate::assert_parenthesized!(parser.tree, expression_id, expression => {
        assert_node!(parser.tree, *expression, Expression::Member { left, name: None, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    });
}

#[test]
fn test_parse_call_postfix_missing_close_parenthesis() {
    // foo(
    let test = TestParser::new("foo(");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );

    // foo(
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_call_postfix_missing_close_parenthesis_after_argument() {
    // foo(1
    let test = TestParser::new("foo(1");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );

    // foo(1
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    });
}

#[test]
fn test_parse_indirect_call_postfix_missing_close_parenthesis_after_argument() {
    // foo.(1
    let test = TestParser::new("foo.(1");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );

    // foo.(1
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments, .. } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    });
}

#[test]
fn test_parse_optional_call_postfix_missing_close_parenthesis_after_argument() {
    // foo?.(1
    let test = TestParser::new("foo?.(1");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );

    // foo?.(1
    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { position, left, generic_arguments: _, arguments, is_optional, .. } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert!(*is_optional);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
        });
    });
}

/// Build one deeply nested call source.
fn nested_call_source(depth: usize) -> String {
    let mut source = String::with_capacity(depth * 72);
    source.push_str("const deepCall = ");

    // open calls from outermost to innermost
    for index in (0..depth).rev() {
        write!(source, "wrap{index}(").unwrap();
    }

    source.push_str("input");

    // close calls from innermost to outermost
    for index in 0..depth {
        write!(
            source,
            ", value{index}, () => fallback{index}, {{ index: {index} }})"
        )
        .unwrap();
    }

    source.push(';');

    source
}

/// Parse a deeply nested call chain without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_call_expression() {
    let source = nested_call_source(1024);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_index_postfix_missing_expression() {
    // foo[
    let test = TestParser::new("foo[");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // foo[
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index), .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::Missing);
    });
}

#[test]
fn test_parse_index_postfix_missing_close_bracket() {
    // foo[1
    let test = TestParser::new("foo[1");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseBracket),
            "",
        )],
    );

    // foo[1
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index), .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::Literal(Literal::Integer(1)));
    });
}

#[test]
fn test_parse_indirect_index_postfix_missing_expression() {
    // foo.[
    let test = TestParser::new("foo.[");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // foo.[
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index), .. } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::Missing);
    });
}

#[test]
fn test_parse_optional_index_postfix_missing_expression() {
    // foo?.[
    let test = TestParser::new("foo?.[");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // foo?.[
    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Index { position, left, index: Some(index), is_optional, .. } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert!(*is_optional);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    });
}

#[test]
fn test_parse_parenthesized_index_postfix_missing_expression_preserves_outer_close() {
    // (foo[)
    let test = TestParser::new("(foo[)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, ")")]);

    // (foo[)
    crate::assert_parenthesized!(parser.tree, expression_id, expression => {
        assert_node!(parser.tree, *expression, Expression::Index { position, left, index: Some(index), .. } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    });
}

#[test]
fn test_parse_call_expression_with_generic_arguments() {
    // foo<T>(1, 2)
    let test = TestParser::new("foo<T>(1, 2)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // foo<T>(1, 2)
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments, arguments, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        // foo
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        // <T>
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "T");
                    assert!(generic_arguments.is_empty());
                });
        });
        // (1, 2)
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
        assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        });
    });
}

#[test]
fn test_parse_call_with_inferred_callee() {
    let test = TestParser::new("_(1, 2)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        // _
        assert_node!(parser.tree, *left, Expression::Infer { form, name } => {
            assert_eq!(*form, InferForm::Hole);
            assert!(name.is_none());
        });
        // (1, 2)
        assert_eq!(arguments.len(), 2);
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_call_with_inferred_callee_object_argument() {
    let test = TestParser::new("_({ x: 1, y: 2 })");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        // _
        assert_node!(parser.tree, *left, Expression::Infer { form, name } => {
            assert_eq!(*form, InferForm::Hole);
            assert!(name.is_none());
        });
        // ({ x: 1, y: 2 })
        assert_eq!(arguments.len(), 1);
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_without_parentheses() {
    // new Foo without parentheses
    let test = TestParser::new("new Foo");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        // Foo
        assert_value_expression_path!(parser, parser.tree.get(*left), "Foo");
        assert!(arguments.is_empty());
    });
}

/// Constructor indexing and type arguments precede instance member access.
#[test]
fn test_parse_indexed_constructor() {
    let test = TestParser::new("new constructors[0]<Item>(value).member");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_eq!(parser.peek_token_type(), TokenType::End);
    assert_node!(parser.tree, expression, Expression::Member { left, name, .. } => {
        assert_string!(parser, name.unwrap(), "member");
        assert_node!(parser.tree, *left, Expression::New { left, generic_arguments, arguments } => {
            assert_node!(parser.tree, *left, Expression::Index { left, index, .. } => {
                assert_value_expression_path!(parser, parser.tree.get(*left), "constructors");
                assert_node!(parser.tree, index.unwrap(), Expression::Literal(Literal::Integer(0)));
            });
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Item");
            });
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_value_expression_path!(parser, parser.tree.get(*value), "value");
            });
        });
    });
}

/// A parenthesized call can return the constructor to invoke.
#[test]
fn test_parse_constructor_returned_by_call() {
    let test = TestParser::new("new (selectConstructor())(value)");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_eq!(parser.peek_token_type(), TokenType::End);
    assert_node!(parser.tree, expression, Expression::New { left, generic_arguments, arguments } => {
        assert!(generic_arguments.is_empty());
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_value_expression_path!(parser, parser.tree.get(*value), "value");
        });
        assert_node!(parser.tree, *left, Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_value_expression_path!(parser, parser.tree.get(*left), "selectConstructor");
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_parse_new_with_empty_parentheses() {
    // new Foo() (with empty parentheses)
    let test = TestParser::new("new Foo()");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        // Foo
        assert_value_expression_path!(parser, parser.tree.get(*left), "Foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_with_infer_hole() {
    let test = TestParser::new("new _()");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Infer { form, name } => {
            assert_eq!(*form, InferForm::Hole);
            assert!(name.is_none());
        });
        assert!(arguments.is_empty());
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_with_infer_hole_type_argument() {
    let test = TestParser::new("new Box<_>(value)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "Box");
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Hole);
                assert!(name.is_none());
                assert!(constraint.is_none());
            });
        });

        assert_eq!(arguments.len(), 1);
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_with_generic_member_constructor_name() {
    let test = TestParser::new("new ns.Box<_>.Inner<T>(value)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "T");
        });
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, name.unwrap(), "Inner");
            assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
                assert_value_expression_path!(parser, parser.tree.get(*left), "ns.Box");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Infer { form, .. } => {
                        assert_eq!(*form, InferForm::Hole);
                    });
                });
            });
        });
        assert_eq!(arguments.len(), 1);
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_without_receiver_recovers_missing_constructor() {
    let test = TestParser::new("new");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // new
    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Missing);
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_with_arguments() {
    // new Foo(1, 2)
    let test = TestParser::new("new Foo(1, 2)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        // Foo
        assert_value_expression_path!(parser, parser.tree.get(*left), "Foo");
        // (1, 2)
        assert_eq!(arguments.len(), 2);
    });
}

#[test]
fn test_parse_new_type_arguments_before_if_keyword() {
    // new A<T> if (0);
    let test = TestParser::new("new A<T> if (0);");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // new A<T>
    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "T");
                assert!(generic_arguments.is_empty());
            });
        });

        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_type_arguments_without_parenthesized_call() {
    // new A<T>
    let test = TestParser::new("new A<T>");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // new A<T>
    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "T");
                assert!(generic_arguments.is_empty());
            });
        });

        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_type_arguments_with_spaces() {
    // new A < T >
    let test = TestParser::new("new A < T >");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // new A < T >
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

#[test]
fn test_parse_new_multiple_type_arguments_with_spaces() {
    // new A < B, C >
    let test = TestParser::new("new A < B, C >");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // new A < B, C >
    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(generic_arguments.len(), 2);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "B");
        });
        assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "C");
        });

        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_shift_left_comparison_not_type_arguments_like_babel() {
    // f<< T > (()=>T) > T
    let test = TestParser::new("f<< T > (()=>T) > T");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    // f<< T > (()=>T) > T
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*right), "T");

        // f<< T > (()=>T)
        assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);

            // f<< T
            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::ShiftLeft);
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_expression_path!(parser, parser.tree.get(*right), "T");
            });

            // (()=>T)
            crate::assert_parenthesized!(parser.tree, *right, expression => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                        assert!(signature.parameters.is_empty());
                        assert!(signature.return_type.is_none());
                        assert_expression_path!(parser, parser.tree.get(body.unwrap()), "T");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_new_with_type_identifier_receiver_and_spread_argument() {
    // new Type(...instances)
    let test = TestParser::new("new Type(...instances)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // new Type(...instances)
    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "Type");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Spread { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "instances");
        });
    });
}

#[test]
fn test_parse_new_parenthesized_cast_receiver_with_generic_arguments() {
    let test = TestParser::new("new Promise<Foo>((resolve, reject) => {})");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "Promise");
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Foo");
                assert!(generic_arguments.is_empty());
            });
        });

        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(_));
        });
    });
}
