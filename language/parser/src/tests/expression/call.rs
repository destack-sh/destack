use destack_dir::{
    Argument, BinaryOperator, Declaration, Expression, FunctionDeclaration, GenericArgument,
    InferForm, LocalNodeId, NodeType, PostfixPosition, ScalarLiteral, TypeExpression,
};
use destack_source::LanguageType;
use std::fmt::Write;

use crate::{Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string};

fn make_receiver(parser: &mut Parser) -> LocalNodeId<Expression> {
    let receiver_str = parser.strings.intern("receiver");
    let span = parser.peek().unwrap().span;
    parser.insert_node(Expression::Identifier { name: receiver_str }, span)
}

#[test]
fn test_parse_index_postfix_explicit() {
    // [1]
    let mut test = TestParser::new("[1]");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);

    let index_id = parser.eat_index(recv, PostfixPosition::Direct).unwrap();
    assert_node!(parser.tree, index_id, Expression::Index { position, left, index } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);
        assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_index_postfix_multiline() {
    // [
    //   1
    // ]
    let mut test = TestParser::new("[\n  1\n]");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);

    let index_id = parser.eat_index(recv, PostfixPosition::Direct).unwrap();
    assert_node!(parser.tree, index_id, Expression::Index { position, left, index } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);
        assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_call_postfix() {
    // (1, 2)
    let mut test = TestParser::new("(1, 2)");
    let mut parser = test.prepare();
    let recv = make_receiver(&mut parser);
    let call_id = parser
        .eat_call(recv, None, PostfixPosition::Direct)
        .unwrap();

    assert_node!(parser.tree, call_id, Expression::Call { position, left, generic_arguments: _, arguments } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_eq!(*left, recv);

        // (1, 2)
        assert_eq!(arguments.len(), 2);

        // 1
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });

        // 2
        assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    });
}

#[test]
fn test_parse_member_postfix_missing_name() {
    // foo.
    let mut test = TestParser::new("foo.");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo.
    assert_node!(parser.tree, expression_id, Expression::Member { left, name: None } => {
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
    });
}

#[test]
fn test_parse_private_member_postfix_missing_name() {
    // foo.#
    let mut test = TestParser::new_with_language("foo.#", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo.#
    assert_node!(parser.tree, expression_id, Expression::PrivateMember { left, name: None } => {
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
    });
}

#[test]
fn test_parse_optional_member_postfix_missing_name() {
    // foo?.
    let mut test = TestParser::new("foo?.");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo?.
    assert_node!(parser.tree, expression_id, Expression::Member { left, name: None } => {
        assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    });
}

#[test]
fn test_parse_parenthesized_member_postfix_missing_name_preserves_outer_close() {
    // (foo.)
    let mut test = TestParser::new("(foo.)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ")")]);

    // (foo.)
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::Member { left, name: None } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    });
}

#[test]
fn test_parse_call_postfix_missing_close_parenthesis() {
    // foo(
    let mut test = TestParser::new("foo(");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo(
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_call_postfix_missing_close_parenthesis_after_argument() {
    // foo(1
    let mut test = TestParser::new("foo(1");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo(1
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_indirect_call_postfix_missing_close_parenthesis_after_argument() {
    // foo.(1
    let mut test = TestParser::new("foo.(1");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo.(1
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_optional_call_postfix_missing_close_parenthesis_after_argument() {
    // foo?.(1
    let mut test = TestParser::new("foo?.(1");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo?.(1
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments: _, arguments } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
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
    let mut test = TestParser::new_with_language(&source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_index_postfix_missing_expression() {
    // foo[
    let mut test = TestParser::new("foo[");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo[
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::Missing);
    });
}

#[test]
fn test_parse_index_postfix_missing_close_bracket() {
    // foo[1
    let mut test = TestParser::new("foo[1");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo[1
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_indirect_index_postfix_missing_expression() {
    // foo.[
    let mut test = TestParser::new("foo.[");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo.[
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_node!(parser.tree, *index, Expression::Missing);
    });
}

#[test]
fn test_parse_optional_index_postfix_missing_expression() {
    // foo?.[
    let mut test = TestParser::new("foo?.[");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // foo?.[
    assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
        assert_node!(parser.tree, *index, Expression::Missing);
    });
}

#[test]
fn test_parse_parenthesized_index_postfix_missing_expression_preserves_outer_close() {
    // (foo[)
    let mut test = TestParser::new("(foo[)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ")")]);

    // (foo[)
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::Index { position, left, index: Some(index) } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    });
}

#[test]
fn test_parse_call_expression_with_generic_arguments() {
    // foo<T>(1, 2)
    let mut test = TestParser::new("foo<T>(1, 2)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // foo<T>(1, 2)
    assert_node!(parser.tree, expression_id, Expression::Call { position, left, generic_arguments, arguments } => {
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
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    });
}

#[test]
fn test_parse_new_without_parentheses() {
    // new Foo without parentheses
    let mut test = TestParser::new("new Foo");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        // Foo
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_with_empty_parentheses() {
    // new Foo() (with empty parentheses)
    let mut test = TestParser::new("new Foo()");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        // Foo
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_maybe_with_empty_parentheses() {
    // new? Foo()
    let mut test = TestParser::new("new? Foo()");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::NewMaybe { ty, arguments } => {
        // constructor
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_maybe_without_receiver_recovers_missing_constructor() {
    // new?
    let mut test = TestParser::new("new?");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    assert_node!(parser.tree, expression_id, Expression::NewMaybe { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Missing);
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_with_infer_hole() {
    let mut test = TestParser::new("new _()");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Infer { form, name, constraint } => {
            assert_eq!(*form, InferForm::Hole);
            assert!(name.is_none());
            assert!(constraint.is_none());
        });
        assert!(arguments.is_empty());
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_with_infer_hole_type_argument() {
    let mut test = TestParser::new("new Box<_>(value)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Box");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                    assert_eq!(*form, InferForm::Hole);
                    assert!(name.is_none());
                    assert!(constraint.is_none());
                });
            });
        });
        assert_eq!(arguments.len(), 1);
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_new_with_generic_member_constructor_name() {
    let mut test = TestParser::new("new ns.Box<_>.Inner<T>(value)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Member { left, name, generic_arguments } => {
            assert_string!(parser, *name, "Inner");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "ns.Box");
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
    let mut test = TestParser::new("new");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // new
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Missing);
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_with_arguments() {
    // new Foo(1, 2)
    let mut test = TestParser::new("new Foo(1, 2)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        // Foo
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        // (1, 2)
        assert_eq!(arguments.len(), 2);
    });
}

#[test]
fn test_parse_new_type_arguments_before_if_keyword() {
    // new A<T> if (0);
    let mut test = TestParser::new_with_language("new A<T> if (0);", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // new A<T>
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "A");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "T");
                    assert!(generic_arguments.is_empty());
                });
            });
        });
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_type_arguments_without_parenthesized_call() {
    // new A<T>
    let mut test = TestParser::new_with_language("new A<T>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // new A<T>
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "A");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "T");
                    assert!(generic_arguments.is_empty());
                });
            });
        });
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_type_arguments_with_spaces() {
    // new A < T >
    let mut test = TestParser::new_with_language("new A < T >", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // new A < T >
    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "A");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
        });
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_multiple_type_arguments_with_spaces() {
    // new A < B, C >
    let mut test = TestParser::new_with_language("new A < B, C >", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // new A < B, C >
    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "A");
            assert_eq!(generic_arguments.len(), 2);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "B");
            });
            assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "C");
            });
        });
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_shift_left_comparison_not_type_arguments_like_babel() {
    // f<< T > (()=>T) > T
    let mut test = TestParser::new_with_language("f<< T > (()=>T) > T", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
            assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
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
    let mut test =
        TestParser::new_with_language("new Type(...instances)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // new Type(...instances)
    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_expression_path!(parser, parser.tree.get(*ty), "Type");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Spread { label, value } => {
            assert!(label.is_none());
            assert_expression_path!(parser, parser.tree.get(*value), "instances");
        });
    });
}

#[test]
fn test_parse_new_parenthesized_cast_receiver_with_generic_arguments() {
    let mut test = TestParser::new_with_language(
        "new Promise<Foo>((resolve, reject) => {})",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { ty, arguments } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Promise");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Foo");
                    assert!(generic_arguments.is_empty());
                });
            });
        });
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(_));
        });
    });
}
