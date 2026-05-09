use crate::tests::*;
use crate::{assert_expression_path, assert_name, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

/// Parse a lambda function type with empty parameters.
#[test]
fn test_parse_lambda_function_empty_type() {
    let mut test = TestParser::new("() => void");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::FunctionTypeDeclaration(function) => {
        assert_eq!(function.parameters.len(), 0);
        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Void);
        });
    });
}

/// Parse a lambda function type with parameters and return type.
#[test]
fn test_parse_lambda_function_type() {
    let mut test = TestParser::new("(a: int32) => int32");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::FunctionTypeDeclaration(function) => {
        // a: int32
        assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(
                    *value,
                    TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                    })
                );
            });
        });

        // int32
        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                })
            );
        });
    });
}

/// Parse lambda function type container spans with separator comments.
#[test]
fn test_parse_lambda_function_type_container_spans_with_comments() {
    let mut test = TestParser::new_with_language(
        "(value: /* arg */ string) /* fn-tail */ => void",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::FunctionTypeDeclaration(function) => {
        let parameter_type_span = parser
            .tree
            .get_side_span(function.parameters[0], NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("missing lambda parameter type span");
        assert_eq!(parser.get_span_str(parameter_type_span), ": /* arg */ string");

        let parameter_span = parser
            .tree
            .get_side_span(type_expression_id, NodeSpanType::Region(NodeSpanRegion::Parameters))
            .expect("missing lambda parameter span");
        assert_eq!(parser.get_span_str(parameter_span), "(value: /* arg */ string)");

        let return_type_span = parser
            .tree
            .get_side_span(type_expression_id, NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("missing lambda return type span");
        assert_eq!(parser.get_span_str(return_type_span), "=> void");
    });
}

/// Parse a lambda function value with a body.
#[test]
fn test_parse_lambda_function_value() {
    let mut test = TestParser::new("(a) => a > 2");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.return_type.is_none());
            assert!(body.is_some());
            assert_eq!(signature.parameters.len(), 1);
            // (a)
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                assert_string!(parser, *name, "a");
            });
            // a > 2
            assert_node!(parser.tree, body.unwrap(), Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expression_path!(parser, parser.tree.get(*left), "a");
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    });
}

/// Parse function expression callbacks with a newline before the body block.
#[test]
fn test_parse_call_with_function_expression_newline_before_body() {
    let mut test = TestParser::new_with_language(
        r"defer(function nextTick_callback()
{
  callback(err, result);
});",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    // defer(function nextTick_callback() { ... });
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "defer");
        assert_eq!(arguments.len(), 1);

        // function nextTick_callback() { ... }
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, signature, body, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Function);
                    assert_string!(parser, name.unwrap().string(), "nextTick_callback");
                    assert_eq!(signature.parameters.len(), 0);

                    // callback(err, result)
                    let body_id = body.expect("expected function body");
                    assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                        let block = parser.tree.get(*block_id);
                        let expressions = block_expression_ids(block);
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "callback");
                            assert_eq!(arguments.len(), 2);
                            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "err");
                            });
                            assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "result");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse a generic lambda function value with a body.
#[test]
fn test_parse_generic_lambda_function_value() {
    let mut test = TestParser::new("<T,>(x: T): T => x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: None, default: None, .. } => {
                assert_string!(parser, *name, "T");
            });
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "T");
            });
            assert_node!(parser.tree, body.unwrap(), Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
    });
}

/// Generic parameter constraints should reject `implements`.
#[test]
fn test_parse_generic_lambda_function_rejects_implements_constraint() {
    let mut test = TestParser::new("<T implements Foo>(x: T): T => x");
    let mut parser = test.prepare();
    let _ = parser.eat_expression(parser.flags);

    assert!(!parser.errors.is_empty(), "expected parse errors");
    assert_eq!(parser.get_span_str(parser.errors[0].span), "implements");
}

/// Parse a generic lambda function with a newline after `<`.
#[test]
fn test_parse_generic_lambda_function_value_multiline_after_less_than() {
    let mut test = TestParser::new_with_language(
        "<\nT extends string\n>(x: T) => x",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, body.unwrap(), Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
    });
}

/// Parse ternary typed arrows whose parameter annotations include function types.
#[test]
fn test_parse_ternary_typed_arrow_with_function_type_parameter() {
    let mut test = TestParser::new_with_language(
        r#"shouldAssert(AssertionLevel.Normal)
    ? (nodes: Node[], test: (node: Node) => boolean, message?: string): void => assert(
        test === undefined || every(nodes, test),
        message || "Unexpected node.",
        () => `Node array did not pass test '${getFunctionName(test)}'.`,
        assertEachNode,
    )
    : noop"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { form, then_expression, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);

        // (nodes: Node[], test: (node: Node) => boolean, message?: string): void => assert(...)
        assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_eq!(signature.parameters.len(), 3);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });

        // noop
        let else_id = else_expression.expect("expected else branch");
        assert_expression_path!(parser, parser.tree.get(else_id), "noop");
    });
}

/// Parse a lambda function value with a body and pattern parameters.
#[test]
fn test_parse_lambda_function_value_with_pattern_parameters() {
    let mut test = TestParser::new("(_, { x, y }: T) => a");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(_), .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.return_type.is_none());
            assert_eq!(signature.parameters.len(), 2);
            // _
            assert_node!(parser.tree, signature.parameters[0], Parameter::Pattern { pattern, declared_type: None, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });
            // { x, y }: T
            assert_node!(parser.tree, signature.parameters[1], Parameter::Pattern { pattern, declared_type, .. } => {
                // { x, y }
                assert_node!(parser.tree, *pattern, Pattern::Object { fields, .. } => {
                    assert_eq!(fields.len(), 2);
                    // x
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                        assert_name!(parser, *name, "x");
                    });
                    // y
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                        assert_name!(parser, *name, "y");
                    });
                });
                // T
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
        });
    });
}

/// Parse nested lambda types inside arrow return tuple types.
#[test]
fn test_parse_lambda_return_type_tuple_with_nested_lambda_type() {
    // source: <T, N>(): [T, (action: N) => void] => {}
    let mut test = TestParser::new_with_language(
        "<T, N>(): [T, (action: N) => void] => {}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // <T, N>(): [T, (action: N) => void] => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            let return_type = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, return_type, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });

                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                        assert_eq!(function.parameters.len(), 1);
                        assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                            assert_string!(parser, *name, "action");
                            assert_expression_path!(parser, parser.tree.get(*declared_type), "N");
                        });
                        assert_node!(parser.tree, function.return_type.expect("expected nested return type"), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Void);
                        });
                    });
                });
            });
        });
    });
}

/// Parse generic arrow functions with function type return annotations.
#[test]
fn test_parse_generic_arrow_with_function_type_return_annotation() {
    let mut test = TestParser::new_with_language(
        "<T>(fn: T): (value: T) => T => value => value",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // <T>(fn: T): (value: T) => T => value => value
    assert_node!(parser.tree, expr_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            // <T>
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                assert!(constraint.is_none());
            });

            // fn: T
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "fn");
                assert_expression_path!(parser, parser.tree.get(*declared_type), "T");
            });

            // (value: T) => T
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::FunctionTypeDeclaration(function) => {
                assert_eq!(function.parameters.len(), 1);
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "T");
                });
                assert_expression_path!(parser, parser.tree.get(function.return_type.expect("expected nested return type")), "T");
            });

            // value => value
            assert_node!(parser.tree, *body, Expression::Declaration(body_function_id) => {
                assert_node!(parser.tree, *body_function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                        assert_string!(parser, *name, "value");
                    });
                    assert_expression_path!(parser, parser.tree.get(*body), "value");
                });
            });
        });
    });
}

/// Parse a typed arrow predicate with a nested optional-parameter function type.
#[test]
fn test_parse_arrow_return_type_predicate_with_nested_optional_parameter_function_type() {
    let mut test = TestParser::new_with_language(
        "(b): b is FormField<unknown> & { focus: (options?: FocusOptions) => void } => b.focus !== undefined",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("b")));
                assert_node!(parser.tree, target.expect("expected type predicate target"), TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                        assert_eq!(properties.len(), 1);
                        assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                            assert_string!(parser, *name, "focus");
                            assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::FunctionTypeDeclaration(function) => {
                                assert_eq!(function.parameters.len(), 1);
                                assert_node!(parser.tree, function.parameters[0], Parameter::Named { is_optional, name, declared_type: Some(declared_type), .. } => {
                                    assert!(*is_optional);
                                    assert_string!(parser, *name, "options");
                                    assert_expression_path!(parser, parser.tree.get(*declared_type), "FocusOptions");
                                });
                                assert_node!(parser.tree, function.return_type.expect("expected function type return"), TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::Void);
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse static parameter constraints with object keys named `in`.
#[test]
fn test_parse_generic_parameter_constraint_object_property_named_in() {
    // source: <V extends { in: string }>() => {}
    let mut test = TestParser::new_with_language(
        "<V extends { in: string }>() => {}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // <V extends { in: string }>() => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);

            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), .. } => {
                assert_string!(parser, *name, "V");
                assert_node!(parser.tree, *constraint, TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], TypeMember::Field { key, declared_type, .. } => {
                        assert_node!(key, Key::Name(Name::Identifier(name)) => {
                            assert_string!(parser, *name, "in");
                        });
                        assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                });
            });
        });
    });
}

/// Parse a lambda function value with a shorthand argument.
#[test]
fn test_parse_lambda_function_value_shorthand() {
    let mut test = TestParser::new("x => x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.return_type.is_none());
            assert_eq!(signature.parameters.len(), 1);
            // x
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // x
            assert_node!(parser.tree, body.unwrap(), Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
    });
}

/// Parse relational arrow values without swallowing following object properties.
#[test]
fn test_parse_call_argument_object_relational_arrow_then_typed_block_arrow() {
    let mut test = TestParser::new_with_language(
        r#"morgan({
  skip: (req, res) => res.statusCode < 400,
  write: (str: string) => {
    str;
  },
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "morgan");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                // skip: (req, res) => res.statusCode < 400
                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                    assert_string!(parser, *name, "skip");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.parameters.len(), 2);
                            assert_node!(parser.tree, *body, Expression::Binary { operator, .. } => {
                                assert_eq!(*operator, BinaryOperator::LessThan);
                            });
                        });
                    });
                });

                // write: (str: string) => { str }
                assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                    assert_string!(parser, *name, "write");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.parameters.len(), 1);
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                                assert_string!(parser, *name, "str");
                                assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::String);
                                });
                            });
                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                                    assert_eq!(leading_expressions.len(), 1);
                                    assert!(tail_expression.is_none());
                                    assert_expression_path!(parser, parser.tree.get(leading_expressions[0]), "str");
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse readonly tuple type annotations in function parameters.
#[test]
fn test_parse_function_parameter_readonly_tuple_target_type() {
    let mut test = TestParser::new_with_language(
        r#"function flattenPairs(pair: readonly [string, number], acc: Array<string | number>): Array<string | number> {
  return acc.concat(pair);
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "pair");
                assert_node!(parser.tree, *declared_type, TypeExpression::Readonly { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::ArrayTuple { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

/// Parse function expressions in decorator call arguments.
#[test]
fn test_eat_decorator_call_with_function_expression_argument() {
    let mut test = TestParser::new_with_language(
        r#"computed("fullName", function(this: Foo) {
  return this.fullName.toUpperCase();
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .with_flags(parser.flags.in_decorator(), |parser| {
            parser.eat_expression(parser.flags)
        })
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "computed");
        assert_eq!(arguments.len(), 2);

        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                    assert!(signature.this_parameter.is_some());
                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                            assert_eq!(leading_expressions.len(), 1);
                            assert!(tail_expression.is_none());
                            assert_node!(parser.tree, leading_expressions[0], Expression::Return { value: Some(value) } => {
                                assert_node!(parser.tree, *value, Expression::Call { .. });
                            });
                        });
                    });
                });
            });
        });
    });
}
