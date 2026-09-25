use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::{TestParser, block_expression_ids};
use crate::{
    TypePosition, TypeStop, assert_expression_path, assert_name, assert_node, assert_path,
    assert_string,
};
use tspp_dir::{
    Argument, BinaryOperator, Block, Declaration, Declarator, Expression, FunctionDeclaration,
    FunctionForm, GenericParameter, IfForm, IntegerType, Literal, MappedTypeModifier, Name,
    Parameter, Pattern, PatternField, Property, TokenType, TupleElement, TypeExpression,
    TypeLiteral, TypeMember,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

/// Parse a lambda function type with empty parameters.
#[test]
fn test_parse_lambda_function_empty_type() {
    let test = TestParser::new("() => void");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::Function(function) => {
        assert_eq!(function.parameters.len(), 0);
        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Void);
        });
    });
}

/// Parse a lambda function type with parameters and return type.
#[test]
fn test_parse_lambda_function_type() {
    let test = TestParser::new("(a: int32) => int32");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::Function(function) => {
        // a: int32
        assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                assert_eq!(
                    *value,
                    TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                    })
                );
            });
        });

        // int32
        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Keyword { value } => {
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
    let test = TestParser::new("(value: /* arg */ string) /* fn-tail */ => void");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, type_expression_id, TypeExpression::Function(function) => {
        let parameter_type_span = parser
            .tree
            .get_side_span(function.parameters[0], NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("missing lambda parameter type span");
        assert_eq!(parser.span_str(parameter_type_span), ": /* arg */ string");

        let parameter_span = parser
            .tree
            .get_side_span(type_expression_id, NodeSpanType::Region(NodeSpanRegion::Parameters))
            .expect("missing lambda parameter span");
        assert_eq!(parser.span_str(parameter_span), "(value: /* arg */ string)");

        let return_type_span = parser
            .tree
            .get_side_span(type_expression_id, NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("missing lambda return type span");
        assert_eq!(parser.span_str(return_type_span), "=> void");
    });
}

/// Parse a lambda function value with a body.
#[test]
fn test_parse_lambda_function_value() {
    let test = TestParser::new("(a) => a > 2");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
            });
        });
    });
}

/// Parse a lambda body that returns a parenthesized struct literal.
#[test]
fn test_parse_lambda_struct_literal_body() {
    let test = TestParser::new("() => (Node { parent: this })");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.return_type.is_none());
            assert!(signature.parameters.is_empty());

            assert_node!(parser.tree, body.expect("expected lambda body"), Expression::StructExpression { ty, properties } => {
                assert_expression_path!(parser, parser.tree.get(*ty), "Node");
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { name, value, .. } => {
                    let Name::Identifier(name) = name else {
                        panic!("expected parent field name");
                    };
                    assert_string!(parser, *name, "parent");
                    assert_node!(parser.tree, *value, Expression::This);
                });
            });
        });
    });
}

/// Parse a typed lambda with multiple parameters and a return annotation.
#[test]
fn test_parse_typed_lambda_value_with_multiple_parameters() {
    let test = TestParser::new(
        r#"const add = (a: number, b: number): number => a + b;
add satisfies (a: number, b: number) => number;"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);

    // const add = (a: number, b: number): number => a + b
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 2);
                    assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                    assert_node!(parser.tree, body.expect("expected lambda body"), Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                    });
                });
            });
        });
    });

    // add satisfies (a: number, b: number) => number
    assert_node!(parser.tree, expressions[1], Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "add");
        assert_node!(parser.tree, *target_type, TypeExpression::Function(function) => {
            assert_eq!(function.parameters.len(), 2);
            assert_node!(parser.tree, function.return_type.expect("expected return type"), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });
    });
}

/// Parse function expression callbacks with a newline before the body block.
#[test]
fn test_parse_call_with_function_expression_newline_before_body() {
    let test = TestParser::new(
        r"defer(function nextTick_callback()
{
  callback(err, result);
});",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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

/// Recover anonymous function arguments without abandoning their call or following statement.
#[test]
fn test_recover_anonymous_function_argument() {
    let test = TestParser::new("consume(function () {});\nconst stable = 1;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Error);
    });
    assert_node!(parser.tree, expressions[1], Expression::Let { .. });
    test.assert_errors(
        &parser,
        &[(None, Some(TokenType::Identifier), None, "function")],
    );
}

/// Parse a generic lambda function value with a body.
#[test]
fn test_parse_generic_lambda_function_value() {
    let test = TestParser::new("<T,>(x: T): T => x");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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

/// Parse a generic lambda function with a newline after `<`.
#[test]
fn test_parse_generic_lambda_function_value_multiline_after_less_than() {
    let test = TestParser::new("<\nT: string\n>(x: T) => x");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Keyword { value } => {
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
    let test = TestParser::new(
        r#"shouldAssert(AssertionLevel.Normal)
    ? (nodes: Node[], test: (node: Node) => boolean, message?: string): void => assert(
        test === undefined || every(nodes, test),
        message || "Unexpected node.",
        () => `Node array did not pass test '${getFunctionName(test)}'.`,
        assertEachNode,
    )
    : noop"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { form, then_expression, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);

        // (nodes: Node[], test: (node: Node) => boolean, message?: string): void => assert(...)
        assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_eq!(signature.parameters.len(), 3);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
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
    let test = TestParser::new("(_, { x, y }: T) => a");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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

/// Parse a lambda parameter whose type is a mapped object type.
#[test]
fn test_parse_lambda_parameter_with_mapped_object_type() {
    let test = TestParser::new("(expected: { [T in TestFilterTerm]?: boolean; }) => {}");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { declared_type: Some(declared_type), .. } => {
                assert_node!(parser.tree, *declared_type, TypeExpression::Mapped { parameter, readonly, optional, value: Some(value) } => {
                    assert_eq!(*readonly, MappedTypeModifier::None);
                    assert_eq!(*optional, MappedTypeModifier::Present);
                    assert_string!(parser, parser.tree.get(*parameter).name, "T");
                    assert_expression_path!(parser, parser.tree.get(parser.tree.get(*parameter).source_type), "TestFilterTerm");
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Boolean);
                    });
                });
            });
        });
    });
}

/// Parse a destructured lambda parameter whose type is an object type.
#[test]
fn test_parse_lambda_pattern_parameter_with_object_type() {
    let test = TestParser::new(
        r#"(
  options,
  { log, logger, messenger }: {
    log: LogFun;
    logger: Logger;
    messenger: Messenger;
  }) => {}
"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            assert_node!(parser.tree, signature.parameters[1], Parameter::Pattern { declared_type: Some(declared_type), .. } => {
                assert_node!(parser.tree, *declared_type, TypeExpression::Object { members } => {
                    let [log_member, logger_member, messenger_member] = members.as_slice() else {
                        panic!("expected exactly three object type members");
                    };

                    assert_node!(parser.tree, *log_member, TypeMember::Field { name: Name::Identifier(name), declared_type: Some(field_type), .. } => {
                        assert_string!(parser, *name, "log");
                        assert_expression_path!(parser, parser.tree.get(*field_type), "LogFun");
                    });
                    assert_node!(parser.tree, *logger_member, TypeMember::Field { name: Name::Identifier(name), declared_type: Some(field_type), .. } => {
                        assert_string!(parser, *name, "logger");
                        assert_expression_path!(parser, parser.tree.get(*field_type), "Logger");
                    });
                    assert_node!(parser.tree, *messenger_member, TypeMember::Field { name: Name::Identifier(name), declared_type: Some(field_type), .. } => {
                        assert_string!(parser, *name, "messenger");
                        assert_expression_path!(parser, parser.tree.get(*field_type), "Messenger");
                    });
                });
            });
        });
    });
}

/// Parse nested lambda types inside arrow return tuple types.
#[test]
fn test_parse_lambda_return_type_tuple_with_nested_lambda_type() {
    // source: <T, N>(): ((T, (action: N) => void)) => {}
    let test = TestParser::new("<T, N>(): ((T, (action: N) => void)) => {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <T, N>(): ((T, (action: N) => void)) => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            let return_type = signature.return_type.expect("expected return type");
            crate::assert_parenthesized!(parser.tree, return_type, expression => {
                assert_node!(parser.tree, *expression, TypeExpression::Tuple { elements, .. } => {
                    assert_eq!(elements.len(), 2);

                    assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "T");
                    });

                    assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                        assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                            assert_eq!(function.parameters.len(), 1);
                            assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                                assert_string!(parser, *name, "action");
                                assert_expression_path!(parser, parser.tree.get(*ty), "N");
                            });
                            assert_node!(parser.tree, function.return_type.expect("expected nested return type"), TypeExpression::Keyword { value } => {
                                assert_eq!(*value, TypeLiteral::Void);
                            });
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
    let test = TestParser::new("<T,>(fn: T): (value: T) => T => value => value");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <T,>(fn: T): (value: T) => T => value => value
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
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Function(function) => {
                assert_eq!(function.parameters.len(), 1);
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(*ty), "T");
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

/// Parse generic parameter constraints with object keys named `in`.
#[test]
fn test_parse_generic_parameter_constraint_object_property_named_in() {
    // source: <V: { in: string }>() => {}
    let test = TestParser::new("<V: { in: string }>() => {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <V: { in: string }>() => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);

            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), .. } => {
                assert_string!(parser, *name, "V");
                assert_node!(parser.tree, *constraint, TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], TypeMember::Field { name, declared_type, .. } => {
                        assert_node!(name, Name::Identifier(name) => {
                            assert_string!(parser, *name, "in");
                        });
                        assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Keyword { value } => {
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
    let test = TestParser::new("x => x");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new(
        r#"morgan({
  skip: (req, res) => res.statusCode < 400,
  write: (str: string) => {
    str;
  },
})"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "morgan");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                // skip: (req, res) => res.statusCode < 400
                assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
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
                assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                    assert_string!(parser, *name, "write");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.parameters.len(), 1);
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                                assert_string!(parser, *name, "str");
                                assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value } => {
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
    let test = TestParser::new(
        r#"function flattenPairs(pair: readonly (string, number), acc: Array<string | number>): Array<string | number> {
  return acc.concat(pair);
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "pair");
                assert_node!(parser.tree, *declared_type, TypeExpression::Readonly { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Tuple { elements, .. } => {
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
    let test = TestParser::new(
        r#"computed("fullName", function fullName(this: Foo) {
  return this.fullName.toUpperCase();
})"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::DecoratorHead, ExpressionStop::default())
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
