use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_dir::*;
use destack_source::LanguageType;

/// Assert that one instantiation assignment target is rejected.
fn assert_instantiation_assignment_rejects_at(input: &str, expected_leaf: &str) {
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScript);
    let mut parser = test.prepare();

    let error = parser
        .eat_expression(parser.flags)
        .expect_err("expected instantiation assignment target to fail");
    let (span, node_type, token_type) = error.leaf_content();
    let leaf = parser.get_span_str(span);

    assert_eq!(node_type, None);
    assert_eq!(token_type, None);
    assert_eq!(leaf, expected_leaf);
}

#[test]
fn test_parse_instantiation_expression_with_index() {
    let mut test = TestParser::new_with_language("f[\"g\"]<number>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Instantiation { left, generic_arguments } => {
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
        });
        assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Direct } => {
            assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                assert_string!(parser, *name, "f");
            });
            let index = index.expect("expected index expression");
            assert_node!(parser.tree, index, Expression::ScalarLiteral(ScalarLiteral::String(name)) => {
                assert_string!(parser, *name, "g");
            });
        });
    });
}

#[test]
fn test_parse_instantiation_expression_parenthesized() {
    let mut test = TestParser::new_with_language("(f<number>)<number>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Instantiation { left, generic_arguments } => {
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
        });
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Instantiation { left, generic_arguments } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "f");
                });
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                });
            });
        });
    });
}

/// Parse calls with a parenthesized instantiation callee and outer type arguments.
#[test]
fn test_parse_generic_call_with_parenthesized_instantiation_callee() {
    let mut test = TestParser::new("(getContainer().map<string>)<number>(1)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_eq!(generic_arguments.len(), 1);
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Instantiation { left, generic_arguments } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Member { .. });
            });
        });
    });
}

/// Parse optional-chain generic argument calls in value positions.
#[test]
fn test_parse_optional_chain_generic_argument_call() {
    let mut test = TestParser::new_with_language("fn?.<number>();", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Call { left, generic_arguments, arguments, .. } => {
        assert!(arguments.is_empty());

        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
        });

        assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
            assert_expression_path!(parser, parser.tree.get(*left), "fn");
        });
    });
}

/// Parse instantiation expressions that end a statement before the next declaration.
#[test]
fn test_parse_instantiation_expression_before_next_statement_keyword() {
    let mut test = TestParser::new_with_language(
        r#"const addSpanBaseAttributes = addSpanAttributes("gen_ai", String.camelToSnake)<BaseAttributes>
const addSpanOperationAttributes = addSpanAttributes("gen_ai.operation", String.camelToSnake)<OperationAttributes>"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { left, generic_arguments } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "BaseAttributes");
                            assert!(generic_arguments.is_empty());
                        });
                });
                assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "addSpanAttributes");
                    assert_eq!(arguments.len(), 2);
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { generic_arguments, .. } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "OperationAttributes");
                            assert!(generic_arguments.is_empty());
                        });
                });
            });
        });
    });
}

/// Reject instantiation expressions as assignment targets.
#[test]
fn test_reject_instantiation_expression_assignment() {
    assert_instantiation_assignment_rejects_at("f<T> = g", "f<T>");
}

/// Reject member instantiation expressions as assignment targets.
#[test]
fn test_reject_instantiation_expression_member_assignment() {
    assert_instantiation_assignment_rejects_at("cls.myFunc<T> = g", "cls.myFunc<T>");
}

/// Parse parenthesized instantiation receivers before member access.
#[test]
fn test_parse_instantiation_expression_member_access_with_parentheses() {
    let mut test = TestParser::new_with_language("(f<T>).x", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { left, name } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Instantiation { left, generic_arguments } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "T");
                        });
                });
                assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "f");
                });
            });
        });
    });
}

/// Instantiation expressions should parse in mixed operator contexts.
#[test]
fn test_parse_instantiation_expression_more_exprs() {
    let mut test = TestParser::new_with_language(
        r#"
f<x>, g<y>;
[f<x>];
f<x> ? g<y> : h<z>;
f<x> ^ g<y>;
f<x> & g<y>;
f<x> | g<y>;
f<x> && g<y>;
f<x> || g<y>;
{ f<x> }
f<x> ?? g<y>;
f<x> == g<y>;
f<x> === g<y>;
f<x> != g<y>;
f<x> !== g<y>;
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 14);

    // f<x>, g<y>
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(generic_arguments.len(), 1);
        });
        assert_node!(parser.tree, expressions[1], Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "g");
            assert_eq!(generic_arguments.len(), 1);
        });
    });

    // [f<x>]
    assert_node!(parser.tree, expressions[1], Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { left, generic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_eq!(generic_arguments.len(), 1);
            });
        });
    });

    // f<x> ? g<y> : h<z>
    assert_node!(parser.tree, expressions[2], Expression::If { form, .. } => {
        assert_eq!(*form, IfForm::Ternary);
    });

    // f<x> ?? g<y>
    assert_node!(parser.tree, expressions[9], Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
    });

    // f<x> !== g<y>
    assert_node!(parser.tree, expressions[13], Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::NotEqualStrict);
    });
}

/// Parse a call with string literal type arguments.
#[test]
fn test_parse_call_with_string_literal_type_arguments() {
    let mut test = TestParser::new_with_language(
        "accessor.getValue<\"auto\" | \"always\" | \"never\">(\"long\")",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, *left, Expression::Member { left, name } => {
            assert_string!(parser, *name, "getValue");
            assert_expression_path!(parser, parser.tree.get(*left), "accessor");
        });
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 3);
                });
        });
    });
}

/// Parse await call expressions with object type arguments in before block contexts.
#[test]
fn test_parse_await_call_with_object_type_argument_in_before_block_context() {
    let mut test = TestParser::new_with_language(
        r#"
await fetchListResult<{
    pattern: string;
    script: string;
}>(complianceConfig, route)
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.flags.set_in_before_block(true);
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "fetchListResult");
            assert_eq!(arguments.len(), 2);

            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "complianceConfig");
            });

            assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "route");
            });

            let generic_arguments = generic_arguments.as_slice();
            assert_eq!(generic_arguments.len(), 1);

            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                        assert_eq!(properties.len(), 2);

                        assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                            assert_string!(parser, *name, "pattern");
                            assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::String);
                            });
                        });

                        assert_node!(parser.tree, properties[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                            assert_string!(parser, *name, "script");
                            assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::String);
                            });
                        });
                    });
            });
        });
    });
}

/// Parse a call with shift-left generic arguments.
#[test]
fn test_parse_call_with_shift_left_generic_arguments() {
    let mut test =
        TestParser::new_with_language("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(arguments.is_empty());
        let generic_arguments = generic_arguments.as_slice();
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 1);
                });
        });
    });
}

/// Parse shift-left generic arguments in decorator context.
#[test]
fn test_parse_call_with_shift_left_generic_arguments_in_decorator_context() {
    let mut test =
        TestParser::new_with_language("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let flags = parser
        .flags
        .not_in_position()
        .in_left_precedence(u16::MAX)
        .not_in_sequence_expression()
        .in_decorator();
    let expr_id = parser.eat_expression(flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(arguments.is_empty());
        let generic_arguments = generic_arguments.as_slice();
        assert_eq!(generic_arguments.len(), 1);
    });
}

/// Comparison operators should not be parsed as generic arguments.
#[test]
fn test_parse_generic_arguments_disambiguate_relational() {
    let mut test = TestParser::new("fn(x < y, x > y)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });
        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
            });
        });
    });
}

/// Relational call arguments should stay relational before shift right assign.
#[test]
fn test_parse_call_arguments_relational_then_shift_right_assign() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language("fn(x < y, x < y, x >>= y)", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(arguments.len(), 3);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Assign { operator, .. } => {
                assert_eq!(*operator, AssignOperator::ShiftRightAssign);
            });
        });
    });
}

/// Relational call arguments should stay relational before unsigned shift right assign.
#[test]
fn test_parse_call_arguments_relational_then_unsigned_shift_right_assign() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language("fn(x < y, x < y, x >>>= y)", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(arguments.len(), 3);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Assign { operator, .. } => {
                assert_eq!(*operator, AssignOperator::UnsignedShiftRightAssign);
            });
        });
    });
}

/// Parse a let binding with a type with generic arguments as value.
#[test]
fn test_parse_type_with_generic_arguments() {
    let mut test = TestParser::new("let Alias = A<B<C>>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "Alias");
            });
            assert_node!(parser.tree, value.unwrap(), Expression::Instantiation { left, generic_arguments } => {
                assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "A");
                });

                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "B");
                            assert_eq!(generic_arguments.len(), 1);
                            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                        assert_path!(parser, *path, "C");
                                        assert!(generic_arguments.is_empty());
                                    });
                            });
                        });
                });
            });
        });
    });
}

#[test]
fn test_parse_call_with_instantiation_callee_and_inline_block_comment() {
    let mut test =
        TestParser::new_with_language("foo/* marker */<string>(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_call_with_instantiation_callee_and_newline_block_comment_before_type_arguments() {
    let mut test =
        TestParser::new_with_language("foo/* marker */\n<string>(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_empty_call_with_instantiation_callee_and_newline_block_comment_before_type_arguments()
{
    let mut test =
        TestParser::new_with_language("foo/* marker */\n<string>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert!(arguments.is_empty());
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_call_with_instantiation_callee_and_line_comment_before_arguments() {
    let mut test =
        TestParser::new_with_language("foo<string>// marker\n(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_instantiation_expression_unparenthesized_index_access() {
    let mut test = TestParser::new_with_language("f<number>[\"g\"]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Index { left, index, position } => {
        assert_eq!(*position, PostfixPosition::Direct);

        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(generic_arguments.len(), 1);
        });

        assert_node!(parser.tree, index.expect("expected index"), Expression::ScalarLiteral(ScalarLiteral::String(name)) => {
            assert_string!(parser, *name, "g");
        });
    });
}

#[test]
fn test_parse_instantiation_expression_unparenthesized_member_access() {
    let mut test = TestParser::new_with_language("f<number>.value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Member { left, name } => {
        assert_string!(parser, *name, "value");

        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(generic_arguments.len(), 1);
        });
    });
}

#[test]
fn test_parse_optional_call_after_instantiation_expression() {
    let mut test = TestParser::new_with_language("f<number>?.()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, position } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert!(generic_arguments.is_empty());
        assert!(arguments.is_empty());

        assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_eq!(generic_arguments.len(), 1);
            });
        });
    });
}

#[test]
fn test_parse_optional_call_after_function_type_instantiation_expression() {
    let mut test = TestParser::new_with_language("f<<T>() => T>?.()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, arguments, position } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert!(generic_arguments.is_empty());
        assert!(arguments.is_empty());

        assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                            assert_eq!(function.generic_parameters.len(), 1);
                        });
                });
            });
        });
    });
}

#[test]
fn test_parse_instantiation_expression_before_newline_binary_operator() {
    let mut test = TestParser::new_with_language("f<T>\n?? 1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(generic_arguments.len(), 1);
        });
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_instantiation_expression_before_newline_division_operator() {
    let mut test = TestParser::new_with_language("f<T>\n/ 1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Divide);
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(generic_arguments.len(), 1);
        });
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_relational_expression_before_newline_prefix_expression() {
    let mut test = TestParser::new_with_language("f <T>\n+1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_expression_path!(parser, parser.tree.get(*right), "T");
        });
        assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Plus);
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}
