use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_instantiation_expression_with_index() {
    let mut test = TestParser::new_with_options("f[\"g\"]<number>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    let mut test = TestParser::new_with_options("(f<number>)<number>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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

/// Parse optional-chain generic argument calls in value positions.
#[test]
fn test_parse_optional_chain_generic_argument_call() {
    let mut test = TestParser::new_with_options("fn?.<number>();", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert!(dynamic_arguments.is_empty());

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
    let mut test = TestParser::new_with_options(
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
                assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "addSpanAttributes");
                    assert_eq!(dynamic_arguments.len(), 2);
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

/// Instantiation expressions can appear as assignment targets in parse output.
#[test]
fn test_parse_instantiation_expression_assignment() {
    let mut test = TestParser::new_with_options("f<T> = g", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Assign { left, right, .. } => {
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
            });
            assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                assert_string!(parser, *name, "f");
            });
        });
        assert_expression_path!(parser, parser.tree.get(*right), "g");
    });
}

/// Instantiation expressions with members remain assignable targets in parse output.
#[test]
fn test_parse_instantiation_expression_member_assignment() {
    let mut test = TestParser::new_with_options("cls.myFunc<T> = g", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Assign { left, right, .. } => {
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
            });
            assert_node!(parser.tree, *left, Expression::Member { left, name, generic_arguments } => {
                assert!(generic_arguments.is_empty());
                assert_string!(parser, *name, "myFunc");
                assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "cls");
                });
            });
        });
        assert_expression_path!(parser, parser.tree.get(*right), "g");
    });
}

/// Parse parenthesized instantiation receivers before member access.
#[test]
fn test_parse_instantiation_expression_member_access_with_parentheses() {
    let mut test = TestParser::new_with_options("(f<T>).x", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { left, name, generic_arguments } => {
        assert!(generic_arguments.is_empty());
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
    let mut test = TestParser::new_with_options(
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
    assert_node!(parser.tree, expressions[2], Expression::If { kind, .. } => {
        assert_eq!(*kind, IfKind::Ternary);
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
    let mut test = TestParser::new_with_options(
        "accessor.getValue<\"auto\" | \"always\" | \"never\">(\"long\")",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert!(generic_arguments.is_empty());
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, *left, Expression::Member { left, name, generic_arguments: member_arguments } => {
                assert!(member_arguments.is_empty());
                assert_string!(parser, *name, "getValue");
                assert_expression_path!(parser, parser.tree.get(*left), "accessor");
            });
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 3);
                    });
            });
        });
    });
}

/// Parse await call expressions with object type arguments in before block contexts.
#[test]
fn test_parse_await_call_with_object_type_argument_in_before_block_context() {
    let mut test = TestParser::new_with_options(
        r#"
await fetchListResult<{
    pattern: string;
    script: string;
}>(complianceConfig, route)
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    parser.options.set_in_before_block(true);
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "fetchListResult");
            assert_eq!(dynamic_arguments.len(), 2);

            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "complianceConfig");
            });

            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "route");
            });

            let generic_arguments = generic_arguments.as_slice();
            assert_eq!(generic_arguments.len(), 1);

            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                        assert_eq!(properties.len(), 2);

                        assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                            assert_string!(parser, *name, "pattern");
                            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::String);
                            });
                        });

                        assert_node!(parser.tree, properties[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                            assert_string!(parser, *name, "script");
                            assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value } => {
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
    let mut test = TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(dynamic_arguments.is_empty());
        let generic_arguments = generic_arguments.as_slice();
        assert_eq!(generic_arguments.len(), 1);
        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                assert_node!(parser.tree, *value, TypeExpression::Declaration { declaration: declaration_id } => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        assert!(body.is_none());
                    });
                });
        });
    });
}

/// Parse shift-left generic arguments in decorator context.
#[test]
fn test_parse_call_with_shift_left_generic_arguments_in_decorator_context() {
    let mut test = TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let options = parser
        .options
        .not_in_position()
        .in_left_precedence(u16::MAX)
        .not_in_sequence_expression()
        .in_decorator();
    let expr_id = parser.eat_expression(options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(dynamic_arguments.is_empty());
        let generic_arguments = generic_arguments.as_slice();
        assert_eq!(generic_arguments.len(), 1);
    });
}

/// Comparison operators should not be parsed as generic arguments.
#[test]
fn test_parse_generic_arguments_disambiguate_relational() {
    let mut test = TestParser::new("fn(x < y, x > y)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(dynamic_arguments.len(), 2);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });
        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
            });
        });
    });
}

/// Relational call arguments should stay relational before shift right assign.
#[test]
fn test_parse_call_arguments_relational_then_shift_right_assign() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("fn(x < y, x < y, x >>= y)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(dynamic_arguments.len(), 3);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, dynamic_arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Assign { operator, .. } => {
                assert_eq!(*operator, AssignOperator::ShiftRightAssign);
            });
        });
    });
}

/// Relational call arguments should stay relational before unsigned shift right assign.
#[test]
fn test_parse_call_arguments_relational_then_unsigned_shift_right_assign() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("fn(x < y, x < y, x >>>= y)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(dynamic_arguments.len(), 3);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        assert_node!(parser.tree, dynamic_arguments[2], Argument::Positional { value, .. } => {
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
    let expr_id = parser.eat_expression(parser.options).unwrap();
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
        TestParser::new_with_options("foo/* marker */<string>(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_call_with_instantiation_callee_and_newline_block_comment_before_type_arguments() {
    let mut test =
        TestParser::new_with_options("foo/* marker */\n<string>(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_empty_call_with_instantiation_callee_and_newline_block_comment_before_type_arguments()
{
    let mut test =
        TestParser::new_with_options("foo/* marker */\n<string>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments, dynamic_arguments, .. } => {
        assert!(dynamic_arguments.is_empty());
        assert_expression_path!(parser, parser.tree.get(*left), "foo");
        assert_eq!(generic_arguments.len(), 1);
    });
}

#[test]
fn test_parse_call_with_instantiation_callee_and_line_comment_before_arguments() {
    let mut test =
        TestParser::new_with_options("foo<string>// marker\n(1)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, generic_arguments: _, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_eq!(generic_arguments.len(), 1);
        });
    });
}
