use tspp_dir::{
    Argument, Asynchrony, BinaryOperator, ClassDeclaration, CommentKind, Declaration, Decorator,
    DecoratorPosition, Expression, FunctionDeclaration, FunctionRole, GenericArgument,
    GenericParameter, IfForm, IntegerType, InterfaceDeclaration, Keyword, Literal, Member, Name,
    NodeType, Parameter, Pattern, PatternField, TokenType, TreeAttribute, TreeAttributeValue,
    TupleElement, TypeExpression, TypeLiteral, TypeMember,
};
use tspp_source::{NodeSpanBoundary, NodeSpanType};

use crate::parse::BindingPosition;
use crate::{
    TestParser, assert_comment, assert_expression_path, assert_name, assert_node, assert_path,
    assert_string,
};

/// Return the source text covered by one parser node span.
fn span_text(source: &str, start: u32, end: u32) -> &str {
    &source[start as usize..end as usize]
}

#[test]
fn test_parse_parameter_type_only() {
    // T
    let test = TestParser::new("T");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "T");
        assert!(declared_type.is_none());
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_parameter_with_type() {
    // x: int32
    let test = TestParser::new("x: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_parameter_with_maybe_type() {
    // x?: int32
    let test = TestParser::new("x?: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, is_optional, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert!(*is_optional);
        assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_parameter_with_default() {
    // validate: boolean = false
    let test = TestParser::new("validate: boolean = false");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "validate");
        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value: TypeLiteral::Boolean });
        assert!(default.is_some());
    });
}

#[test]
fn test_parse_parameter_missing_type_expression() {
    // x:
    let test = TestParser::new("x:");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(&parser, &[(Some(NodeType::Parameter), None, None, "")]);

    // x:
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *declared_type, TypeExpression::Missing);
    });
}

#[test]
fn test_parse_parameter_default_async_lambda_with_await_body() {
    let test = TestParser::new(
        "loadFonts: () => Promise<void> = async () => { await Fonts.loadElementsFonts(elements); }",
    );
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();

    // parameter default should parse as an async lambda value
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(_), default: Some(default), .. } => {
        assert_string!(parser, *name, "loadFonts");
        assert_node!(parser.tree, *default, Expression::Declaration(default_declaration_id) => {
            assert_node!(parser.tree, *default_declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                assert_eq!(signature.asynchrony, Asynchrony::Async);
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Await { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { .. });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_parameter_with_pattern_and_defaults() {
    // { x }: T = false
    let test = TestParser::new("{ x = 4 }: boolean = false");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, declared_type: Some(declared_type), default: Some(default), .. } => {
        // { x = 4 }
        assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern) } => {
                // x
                assert_name!(parser, *name, "x");

                // x = 4
                assert_node!(parser.tree, *pattern, Pattern::Default { pattern, value } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                        assert_string!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(4)));
                });
            });
        });
        // boolean
        assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value: TypeLiteral::Boolean });
        // = false
        assert_node!(parser.tree, *default, Expression::Literal(Literal::Boolean(false)));
    });
}

#[test]
fn test_parse_parameter_optional_pattern() {
    // []? optional pattern parameter
    let test = TestParser::new("[]?");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, is_optional, .. } => {
        assert!(*is_optional);
        assert_node!(parser.tree, *pattern, Pattern::Sequence { .. } => {});
    });
}

#[test]
fn test_parse_parameter_variadic() {
    // ...args
    let test = TestParser::new("...args");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
        assert_string!(parser, *name, "args");
        assert!(declared_type.is_none());
    });
}

#[test]
fn test_parse_parameter_variadic_with_type() {
    // ...args: int32[]
    let test = TestParser::new("...args: int32[]");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
        assert_string!(parser, *name, "args");
        assert!(declared_type.is_some());
    });
}

#[test]
fn test_parse_parameter_optional_variadic() {
    // ...args? optional rest parameter
    let test = TestParser::new("...args?");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, .. } => {
        assert_string!(parser, *name, "args");
    });
}

/// Parse bracketed rest parameters in type position as sequence patterns.
#[test]
fn test_parse_parameter_variadic_tuple_name() {
    let test = TestParser::new("...[value]: [] | [TNext]");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });

        assert_node!(parser.tree, declared_type.expect("expected type annotation"), TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 2);
        });
    });
}

#[test]
fn test_parse_parameter_variadic_array_pattern() {
    // ...[first, second]
    let test = TestParser::new("...[first, second]");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        assert!(declared_type.is_none());
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields, .. } => {
            assert_eq!(fields.len(), 2);
        });
    });
}

#[test]
fn test_parse_parameter_variadic_array_pattern_with_type() {
    // ...[body, init]: ConstructorParameters<typeof Response>
    let test = TestParser::new("...[body, init]: ConstructorParameters<typeof Response>");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        // [body, init]
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields, .. } => {
            assert_eq!(fields.len(), 2);

            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "body");
                });
            });

            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "init");
                });
            });
        });

        // ConstructorParameters<typeof Response>
        let declared_type = declared_type.expect("expected variadic tuple type annotation");
        assert_node!(parser.tree, declared_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "ConstructorParameters");
            assert_eq!(generic_arguments.len(), 1);

            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::TypeOf { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "Response");
                    });
            });
        });
    });
}

#[test]
fn test_parse_parameter_variadic_array_pattern_with_nested_object_and_defaults() {
    // ...[src, { id, systemId, input, syncSnapshot = false } = {} as unknown]: SpawnArguments<...>
    let test = TestParser::new(
        r#"...[
    src,
    { id, systemId, input, syncSnapshot = false } = {} as unknown
]: SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>"#,
    );
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type: Some(declared_type), .. } => {
        // [src, { ... } = {} as unknown]
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);

            // src
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "src");
                });
            });

            // { id, systemId, input, syncSnapshot = false } = {} as unknown
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Default { pattern, value } => {
                    assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                        assert_eq!(fields.len(), 4);

                        assert_node!(parser.tree, fields[3], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern), .. } => {
                            assert_name!(parser, *name, "syncSnapshot");

                            assert_node!(parser.tree, *pattern, Pattern::Default { pattern, value } => {
                                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                                    assert_string!(parser, *name, "syncSnapshot");
                                });
                                assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(false)));
                            });
                        });
                    });

                    assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
                            assert!(properties.is_empty());
                        });
                        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value: TypeLiteral::Unknown });
                    });
                });
            });
        });

        // SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>
        assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "SpawnArguments");
            assert_eq!(generic_arguments.len(), 4);
        });
    });
}

#[test]
fn test_parse_parameter_variadic_object_pattern() {
    // ...{ value: alias }
    let test = TestParser::new("...{ value: alias }");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        assert!(declared_type.is_none());
        assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
        });
    });
}

#[test]
fn test_parse_parameter_multiline() {
    // x: int32
    let test = TestParser::new("x:\n\tint32");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_generic_parameters_multiline_union_constraint_with_default() {
    let test = TestParser::declaration(
        r#"<
  Return: ReturnType<onRequestHookHandler<RawServer>>
    | ReturnType<onRequestAsyncHookHandler<RawServer>>
    = ReturnType<onRequestHookHandler<RawServer>>
>"#,
    );
    let mut parser = test.prepare();
    let generic_parameters = parser.parse_generic_parameter_list(true).unwrap();

    // Return: ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>> = ReturnType<onRequestHookHandler<RawServer>>
    assert_eq!(generic_parameters.len(), 1);
    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), default: Some(default), .. } => {
        assert_string!(parser, *name, "Return");

        // ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>>
        assert_node!(parser.tree, *constraint, TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 2);

            // ReturnType<onRequestHookHandler<RawServer>>
            assert_node!(parser.tree, elements[0], TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "ReturnType");
                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "onRequestHookHandler");
                            assert_eq!(generic_arguments.len(), 1);

                            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                        assert_path!(parser, *path, "RawServer");
                                        assert!(generic_arguments.is_empty());
                                    });
                            });
                        });
                });
            });

            // ReturnType<onRequestAsyncHookHandler<RawServer>>
            assert_node!(parser.tree, elements[1], TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "ReturnType");
                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            assert_path!(parser, *path, "onRequestAsyncHookHandler");
                            assert_eq!(generic_arguments.len(), 1);

                            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                        assert_path!(parser, *path, "RawServer");
                                        assert!(generic_arguments.is_empty());
                                    });
                            });
                        });
                });
            });
        });

        // ReturnType<onRequestHookHandler<RawServer>>
        assert_node!(parser.tree, *default, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "ReturnType");
            assert_eq!(generic_arguments.len(), 1);

            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert_path!(parser, *path, "onRequestHookHandler");
                        assert_eq!(generic_arguments.len(), 1);
                        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                    assert_path!(parser, *path, "RawServer");
                                    assert!(generic_arguments.is_empty());
                                });
                        });
                    });
            });
        });
    });
}

#[test]
fn test_parse_generic_parameters_default_before_shifted_close() {
    let test = TestParser::declaration("<Union, LastElement = LastOf<Union>>");
    let mut parser = test.prepare();
    let generic_parameters = parser.parse_generic_parameter_list(true).unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(generic_parameters.len(), 2);
    assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { default: Some(default), .. } => {
        assert_node!(parser.tree, *default, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "LastOf");
            assert_eq!(generic_arguments.len(), 1);
        });
    });
}

#[test]
fn test_parse_generic_parameters_record_first_parameter_container_leading_span() {
    let test = TestParser::new("<\n  T>");
    let mut parser = test.prepare();
    let generic_parameters = parser.parse_generic_parameter_list(true).unwrap();

    assert_eq!(generic_parameters.len(), 1);

    let leading_span = parser
        .tree
        .get_side_span(
            generic_parameters[0],
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
        )
        .expect("first generic parameter should record its container leading span");

    assert_eq!(parser.file.span_str(leading_span), "<\n  ");
}

#[test]
fn test_parse_generic_parameters_missing_close_angle() {
    // <T
    let test = TestParser::new("<T");
    let mut parser = test.prepare();
    let parameters = parser.parse_generic_parameter_list(true).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::GreaterThan),
            "",
        )],
    );

    // <T
    assert_eq!(parameters.len(), 1);
    assert_node!(parser.tree, parameters[0], GenericParameter::Type { name, constraint: None, default: None, .. } => {
        assert_string!(parser, *name, "T");
    });
}

#[test]
fn test_parse_generic_arguments_missing_close_angle_in_type_context() {
    // <string, number
    let test = TestParser::new("<string, number");
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::GreaterThan),
            "",
        )],
    );

    // <string, number
    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::String });
    });
    assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_generic_arguments_explicit_type_argument() {
    // <type {}>
    let test = TestParser::new("<type {}>");
    let mut parser = test.prepare();
    let arguments = parser
        .parse_generic_argument_list(Default::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Object { members } => {
            assert!(members.is_empty());
        });
    });
}

#[test]
fn test_parse_generic_arguments_object_shape_prefers_type_in_type_context() {
    // <{ name: "alpha"; count: 1 }>
    let test = TestParser::new(r#"<{ name: "alpha"; count: 1 }>"#);
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();

    test.assert_no_errors(&parser);
    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Object { members } => {
            assert_eq!(members.len(), 2);

            assert_node!(parser.tree, members[0], TypeMember::Field { name: Name::Identifier(name), declared_type, .. } => {
                assert_string!(parser, *name, "name");
                assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value: Literal::String(value) } => {
                    assert_string!(parser, *value, "alpha");
                });
            });

            assert_node!(parser.tree, members[1], TypeMember::Field { name: Name::Identifier(name), declared_type, .. } => {
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value: Literal::Integer(1) });
            });
        });
    });
}

#[test]
fn test_parse_generic_arguments_empty_in_type_context_recovers_error_slot() {
    // <>
    let test = TestParser::new("<>");
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();

    // diagnostics
    test.assert_errors(&parser, &[(None, None, Some(TokenType::Identifier), "<")]);

    // <>
    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Error);
}

#[test]
fn test_parse_generic_arguments_first_value_with_boundary_comment() {
    let source = "<\n  // first-type-arg\n  string | number\n>";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();
    parser.finalize_comments();

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Union { .. });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "first-type-arg");
}

#[test]
fn test_parse_generic_arguments_following_value_with_boundary_comment() {
    let source = "<string,\n  // second-type-arg\n  number>";
    let test = TestParser::declaration(source);
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();
    parser.finalize_comments();

    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Number });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "second-type-arg");
}

#[test]
fn test_report_generic_arguments_missing_close_angle_in_value_context() {
    // <string, number
    let test = TestParser::new("<string, number");
    let mut parser = test.prepare();
    let error = parser
        .parse_generic_argument_list(Default::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "");
}

#[test]
fn test_parse_spread_type_generic_argument() {
    // <...T>
    let test = TestParser::new("<...T>");
    let mut parser = test.prepare();
    let arguments = parser.parse_type_generic_arguments().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::SpreadType { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "T");
            assert!(generic_arguments.is_empty());
        });
    });
}

#[test]
fn test_parse_spread_static_generic_argument() {
    // <...1 + 2>
    let test = TestParser::new("<...1 + 2>");
    let mut parser = test.prepare();
    let arguments = parser
        .parse_generic_argument_list(Default::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::SpreadType { value } => {
        assert_node!(parser.tree, *value, TypeExpression::StaticValue { expression } => {
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(parser.tree, *left, Expression::Literal(Literal::Integer(value)) => {
                    assert_eq!(*value, 1);
                });
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(value)) => {
                    assert_eq!(*value, 2);
                });
            });
        });
    });
}

#[test]
fn test_parse_variadic_type_generic_parameter() {
    // <...Parameters, Return>
    let test = TestParser::new("<...Parameters, Return>");
    let mut parser = test.prepare();
    let parameters = parser.parse_generic_parameter_list(true).unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(parameters.len(), 2);
    assert_node!(parser.tree, parameters[0], GenericParameter::VariadicType { name, constraint, default, .. } => {
        assert_string!(parser, *name, "Parameters");
        assert!(constraint.is_none());
        assert!(default.is_none());
    });
    assert_node!(parser.tree, parameters[1], GenericParameter::Type { name, constraint, default, .. } => {
        assert_string!(parser, *name, "Return");
        assert!(constraint.is_none());
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_variadic_const_generic_parameter() {
    // <const ...Shape: readonly usize[]>
    let test = TestParser::new("<const ...Shape: readonly usize[]>");
    let mut parser = test.prepare();
    let parameters = parser.parse_generic_parameter_list(true).unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(parameters.len(), 1);
    assert_node!(parser.tree, parameters[0], GenericParameter::VariadicType { name, constraint, default, is_const, .. } => {
        assert_string!(parser, *name, "Shape");
        assert!(*is_const);
        assert!(default.is_none());
        assert_node!(parser.tree, constraint.expect("expected variadic constraint"), TypeExpression::Readonly { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::Array { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Integer(IntegerType::Pointer { is_signed: false }));
                });
            });
        });
    });
}

#[test]
fn test_parse_parameter_readonly_name() {
    // readonly: int32
    let test = TestParser::new("readonly: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.parse_parameter(Default::default()).unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "readonly");
        assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_const_modifier_target_requires_same_line() {
    let test = TestParser::new(
        r#"const
n"#,
    );
    let mut parser = test.prepare();
    let modifiers = parser.parse_binding_modifiers(BindingPosition::Member);

    assert!(modifiers.is_empty());
    assert!(parser.peek_is_keyword(Keyword::Const));
}

#[test]
fn test_parse_const_modifier_allows_block_line_break() {
    let test = TestParser::new(
        r#"const
{}"#,
    );
    let mut parser = test.prepare();
    let modifiers = parser.parse_binding_modifiers(BindingPosition::Member);

    assert!(modifiers.is_const_block);
    assert!(parser.peek_is_on_new_line());
    assert!(parser.peek_is(TokenType::OpenBrace));
}

/// Parse parameter decorators across callable declarations.
#[test]
fn test_parse_parameter_decorators() {
    let input = r#"
class Test {
    constructor(@p1 t1, @p2 t2, @p3 ...t3) {}

    method(@p1 t1, @p1 @p2 ...t2) {}
}

interface Reader {
    read(@tracked value: string): string;
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    test.assert_no_errors(&parser);

    // class Test { ... }
    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 2);

            // constructor(@p1 t1, @p2 t2, @p3 ...t3)
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert_eq!(signature.role, Some(FunctionRole::Constructor));
                assert_eq!(signature.parameters.len(), 3);

                // @p1 t1
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
                    assert_string!(parser, *name, "t1");
                });
                let t1_annotations = parser.tree.get_decorators(signature.parameters[0].id);
                assert_eq!(t1_annotations.len(), 1);
                assert_node!(parser.tree, t1_annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                });

                // @p2 t2
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type: None, default: None, .. } => {
                    assert_string!(parser, *name, "t2");
                });
                let t2_annotations = parser.tree.get_decorators(signature.parameters[1].id);
                assert_eq!(t2_annotations.len(), 1);
                assert_node!(parser.tree, t2_annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p2");
                });

                // @p3 ...t3
                assert_node!(parser.tree, signature.parameters[2], Parameter::VariadicNamed { name, declared_type: None, .. } => {
                    assert_string!(parser, *name, "t3");
                });
                let t3_annotations = parser.tree.get_decorators(signature.parameters[2].id);
                assert_eq!(t3_annotations.len(), 1);
                assert_node!(parser.tree, t3_annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p3");
                });
            });

            // method(@p1 t1, @p2 ...t2)
            assert_node!(parser.tree, members[1], Member::Method { signature, .. } => {
                assert_eq!(signature.role, None);
                assert_eq!(signature.parameters.len(), 2);

                // @p1 t1
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
                    assert_string!(parser, *name, "t1");
                });
                let method_t1_annotations = parser.tree.get_decorators(signature.parameters[0].id);
                assert_eq!(method_t1_annotations.len(), 1);
                assert_node!(parser.tree, method_t1_annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                });

                // @p2 ...t2
                assert_node!(parser.tree, signature.parameters[1], Parameter::VariadicNamed { name, declared_type: None, .. } => {
                    assert_string!(parser, *name, "t2");
                });
                let method_t2_annotations = parser.tree.get_decorators(signature.parameters[1].id);
                assert_eq!(method_t2_annotations.len(), 2);
                assert_node!(parser.tree, method_t2_annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p1");
                });
                assert_node!(parser.tree, method_t2_annotations[1], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "p2");
                });
            });
        });
    });

    // interface Reader { read(@tracked value: string): string; }
    let expression = expressions[1];
    assert_node!(parser.tree, expression, Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_node!(parser.tree, members[0], TypeMember::Method { signature, .. } => {
                let parameter = signature.parameters[0];
                let name = parser
                    .tree
                    .get_main_span(parameter)
                    .expect("missing parameter name span");

                assert_eq!(parser.span_str(name), "value");
                let decorators = parser.tree.get_decorators(parameter.id);
                assert_eq!(decorators.len(), 1);
                assert_node!(parser.tree, decorators[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                    assert_expression_path!(parser, parser.tree.get(*expression), "tracked");
                });
            });
        });
    });
}

/// Parse ordinary constructor parameters without parameter-property shorthand.
#[test]
fn test_parse_constructor_parameters() {
    let input = r#"
class Test {
    constructor(value: string, count = 0, ...items: Item[]) {}
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);

            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                let main_range = parser
                    .tree
                    .get_main_range(members[0])
                    .expect("missing constructor main range");
                assert_eq!(parser.range_str(main_range), "constructor");

                assert_eq!(signature.role, Some(FunctionRole::Constructor));
                assert_eq!(signature.parameters.len(), 3);

                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });

                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type: None, default: Some(default), .. } => {
                    assert_string!(parser, *name, "count");
                    assert_node!(parser.tree, *default, Expression::Literal(Literal::Integer(0)));
                });

                assert_node!(parser.tree, signature.parameters[2], Parameter::VariadicNamed { name, declared_type: Some(_), .. } => {
                    assert_string!(parser, *name, "items");
                });
            });
        });
    });
}

#[test]
fn test_parse_tree_attribute_string_literal_value() {
    let test = TestParser::new("title=\"hello\"");
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();
    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(string)) } => {
        assert_string!(parser, *name, "title");
        assert_string!(parser, *string, "hello");
    });
}

/// Decode valid HTML entities in quoted tree attribute strings.
#[test]
fn test_parse_tree_attribute_string_decodes_html_entities() {
    let test = TestParser::new("title=\"A&nbsp;&amp;&#160;&#xA0;B\"");
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();

    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(string)) } => {
        assert_string!(parser, *name, "title");
        assert_string!(parser, *string, "A\u{a0}&\u{a0}\u{a0}B");
    });
}

/// Preserve invalid HTML entities in quoted tree attribute strings.
#[test]
fn test_parse_tree_attribute_string_preserves_invalid_html_entities() {
    let test = TestParser::new("title=\"A&missing;B&amp;C\"");
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();

    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(string)) } => {
        assert_string!(parser, *name, "title");
        assert_string!(parser, *string, "A&missing;B&C");
    });
}

#[test]
fn test_parse_tree_attribute_with_newline_before_assign() {
    let test = TestParser::new("onBroadcastSelected\n    = { this._onYouTubeBroadcastIDSelected }");
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();

    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
        assert_string!(parser, *name, "onBroadcastSelected");
        assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
            assert_node!(parser.tree, *left, Expression::This);
            assert_string!(parser, *name, "_onYouTubeBroadcastIDSelected");
        });
    });
}

#[test]
fn test_parse_tree_attribute_with_numeric_kebab_segment() {
    let test = TestParser::new("panose-1=\"test\"");
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();
    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(string)) } => {
        assert_string!(parser, *name, "panose1");
        assert_string!(parser, *string, "test");
    });
}

#[test]
fn test_parse_tree_attribute_with_double_hyphen_kebab_segment() {
    let test = TestParser::new(
        r#"data-nextjs-container-errors-pseudo-html--diff={sign === '+' ? "add" : "remove"}"#,
    );
    let mut parser = test.prepare();
    let argument_id = parser.parse_tree_attribute().unwrap();
    assert_node!(parser.tree, argument_id, TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
        assert_string!(parser, *name, "dataNextjsContainerErrorsPseudoHtmlDiff");
        assert_node!(parser.tree, *value, Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

#[test]
fn test_parse_positional_argument() {
    // 3
    let test = TestParser::new("3");
    let mut parser = test.prepare();
    let argument_id = parser
        .parse_positional_argument(Default::default())
        .unwrap();

    assert_node!(parser.tree, argument_id, Argument::Positional { value } => {
        // 3
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
    });
}

#[test]
fn test_parse_dynamic_argument_span_trims_before_delayed_comma() {
    let source = r#"(
  a

  ,
  b
)"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // first argument and value span should both end at the separator
    assert_eq!(arguments.len(), 2);
    let first_argument_id = arguments[0];
    let first_value_id = match parser.tree.get(first_argument_id) {
        Argument::Positional { value, .. } => *value,
        _ => panic!("expected first positional argument"),
    };
    let first_argument_span = parser.tree.get_span(first_argument_id);
    let first_value_span = parser.tree.get_span(first_value_id);
    assert_eq!(first_argument_span.end, first_value_span.end);

    // verify spans do not cross the separator token
    let separator_offset = source.find(',').expect("expected comma separator") as u32;
    assert!(first_argument_span.end <= separator_offset);
    assert!(first_value_span.end <= separator_offset);
}

#[test]
fn test_parse_dynamic_parameters_recover_error_slot() {
    // (x, =, y)
    let test = TestParser::new("(x, =, y)");
    let mut parser = test.prepare();
    let parameters = parser.parse_dynamic_parameters(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Parameter),
            Some(TokenType::Assign),
            Some(TokenType::Identifier),
            "=",
        )],
    );

    // (x, =, y)
    assert_eq!(parameters.len(), 3);
    assert_node!(parser.tree, parameters[0], Parameter::Named { name, declared_type: None, default: None, .. } => {
        assert_string!(parser, *name, "x");
    });
    assert_node!(parser.tree, parameters[1], Parameter::Error);
    assert_node!(parser.tree, parameters[2], Parameter::Named { name, declared_type: None, default: None, .. } => {
        assert_string!(parser, *name, "y");
    });
}

#[test]
fn test_parse_dynamic_arguments_recover_error_slot() {
    // (1, , 3)
    let test = TestParser::new("(1, , 3)");
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(&parser, &[(None, Some(TokenType::Comma), None, ",")]);

    // (1, , 3)
    assert_eq!(arguments.len(), 3);
    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
    });
    assert_node!(parser.tree, arguments[1], Argument::Error);
    assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
    });
}

#[test]
fn test_parse_dynamic_arguments_recover_missing_close_before_next_statement() {
    // (a,b const
    let source = "(a,b const";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Identifier),
            Some(TokenType::CloseParenthesis),
            "const",
        )],
    );

    // (a,b const
    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "a");
    });
    assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "b");

        // `b`
        let argument_span = parser.tree.get_span(arguments[1]);
        let value_span = parser.tree.get_span(*value);

        assert_eq!(span_text(source, argument_span.start, argument_span.end), "b");
        assert_eq!(span_text(source, value_span.start, value_span.end), "b");
    });

    // the next statement starter stays for the caller
    assert!(parser.peek_is(TokenType::Identifier));
}

#[test]
fn test_parse_dynamic_arguments_recover_trailing_spread_error_slot() {
    // (a, ...)
    let test = TestParser::new("(a, ...)");
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(None, Some(TokenType::CloseParenthesis), None, ")")],
    );

    // (a, ...)
    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "a");
    });
    assert_node!(parser.tree, arguments[1], Argument::Error);
}

#[test]
fn test_parse_dynamic_arguments_recover_missing_close_before_semicolon() {
    // (a,b;
    let source = "(a,b;";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Semicolon),
            Some(TokenType::CloseParenthesis),
            ";",
        )],
    );

    // (a,b;
    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "a");
    });
    assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "b");

        // `b`
        let argument_span = parser.tree.get_span(arguments[1]);
        let value_span = parser.tree.get_span(*value);

        assert_eq!(span_text(source, argument_span.start, argument_span.end), "b");
        assert_eq!(span_text(source, value_span.start, value_span.end), "b");
    });

    // the semicolon stays for the caller
    assert!(parser.peek_is(TokenType::Semicolon));
}

#[test]
fn test_parse_dynamic_arguments_recover_leading_empty_slots() {
    // (,,b)
    let test = TestParser::new("(,,b)");
    let mut parser = test.prepare();
    let arguments = parser.parse_argument_list(Default::default()).unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (None, Some(TokenType::Comma), None, ","),
            (None, Some(TokenType::Comma), None, ","),
        ],
    );

    // (,,b)
    assert_eq!(arguments.len(), 3);
    assert_node!(parser.tree, arguments[0], Argument::Error);
    assert_node!(parser.tree, arguments[1], Argument::Error);
    assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "b");
    });
}

#[test]
fn test_parse_malformed_call_statement_missing_close_keeps_call_shape() {
    let test = TestParser::new("foo(a,b;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Semicolon),
            Some(TokenType::CloseParenthesis),
            ";",
        )],
    );

    // foo(a,b;
    assert_eq!(expressions.len(), 1);
    let call_id = expressions[0];
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });
}

#[test]
fn test_parse_malformed_call_statement_before_const_keeps_call_shape() {
    let test = TestParser::new("foo(a,b const;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (
                Some(NodeType::Expression),
                Some(TokenType::Identifier),
                Some(TokenType::CloseParenthesis),
                "const",
            ),
            (
                Some(NodeType::Expression),
                Some(TokenType::Semicolon),
                Some(TokenType::Identifier),
                ";",
            ),
        ],
    );

    // foo(a,b const;
    assert_eq!(expressions.len(), 2);
    let call_id = expressions[0];
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });
    assert_node!(parser.tree, expressions[1], Expression::Error);
}

#[test]
fn test_parse_malformed_call_statement_with_leading_empty_slots_keeps_call_shape() {
    let test = TestParser::new("foo (,,b);");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (None, Some(TokenType::Comma), None, ","),
            (None, Some(TokenType::Comma), None, ","),
        ],
    );

    // foo (,,b);
    assert_eq!(expressions.len(), 1);
    let call_id = expressions[0];
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 3);
        assert_node!(parser.tree, arguments[0], Argument::Error);
        assert_node!(parser.tree, arguments[1], Argument::Error);
    });
}

#[test]
fn test_parse_malformed_call_statement_with_trailing_spread_keeps_call_shape() {
    let test = TestParser::new("foo (a, ...);");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(None, Some(TokenType::CloseParenthesis), None, ")")],
    );

    // foo (a, ...);
    assert_eq!(expressions.len(), 1);
    let call_id = expressions[0];
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[1], Argument::Error);
    });
}

#[test]
fn test_parse_malformed_call_before_empty_slots_call_preserves_following_statement_shape() {
    let source = r#"
foo(a,b const;
foo (,,b);
"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (
                Some(NodeType::Expression),
                Some(TokenType::Identifier),
                Some(TokenType::CloseParenthesis),
                "const",
            ),
            (
                Some(NodeType::Expression),
                Some(TokenType::Semicolon),
                Some(TokenType::Identifier),
                ";",
            ),
            (None, Some(TokenType::Comma), None, ","),
            (None, Some(TokenType::Comma), None, ","),
        ],
    );

    // foo(a,b const;
    // Error
    // foo (,,b);
    assert_eq!(expressions.len(), 3);

    let first_call_id = expressions[0];
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });

    assert_node!(parser.tree, expressions[1], Expression::Error);

    let second_call_id = expressions[2];
    assert_node!(parser.tree, second_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 3);
    });
}

#[test]
fn test_parse_malformed_call_before_trailing_spread_call_preserves_following_statement_shape() {
    let source = r#"
foo(a,b const;
foo (a, ...);
"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (
                Some(NodeType::Expression),
                Some(TokenType::Identifier),
                Some(TokenType::CloseParenthesis),
                "const",
            ),
            (
                Some(NodeType::Expression),
                Some(TokenType::Semicolon),
                Some(TokenType::Identifier),
                ";",
            ),
            (None, Some(TokenType::CloseParenthesis), None, ")"),
        ],
    );

    assert_eq!(expressions.len(), 3);

    // foo(a,b const;
    let first_call_id = expressions[0];
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });

    // Error
    assert_node!(parser.tree, expressions[1], Expression::Error);

    // foo (a, ...);
    let second_call_id = expressions[2];
    assert_node!(parser.tree, second_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });
}

#[test]
fn test_parse_malformed_call_with_empty_slot_before_following_call_keeps_statement_shape() {
    let source = r#"
foo(,
bar();
"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (None, Some(TokenType::Comma), None, ","),
            (
                Some(NodeType::Expression),
                Some(TokenType::Identifier),
                Some(TokenType::CloseParenthesis),
                "bar",
            ),
        ],
    );

    assert_eq!(expressions.len(), 2);

    // foo(,
    let first_call_id = expressions[0];
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Error);
    });

    // bar();
    let second_call_id = expressions[1];
    assert_node!(parser.tree, second_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 0);
    });
}

#[test]
fn test_parse_malformed_call_with_empty_slot_before_following_const_keeps_statement_shape() {
    let source = r#"
foo(,
const value = 1;
"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (None, Some(TokenType::Comma), None, ","),
            (
                Some(NodeType::Expression),
                Some(TokenType::Identifier),
                Some(TokenType::CloseParenthesis),
                "const",
            ),
        ],
    );

    assert_eq!(expressions.len(), 2);

    // foo(,
    let first_call_id = expressions[0];
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Error);
    });

    // const value = 1;
    let second_expression_id = expressions[1];
    assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_parse_spread_argument() {
    // ...args
    let test = TestParser::new("...args");
    let mut parser = test.prepare();
    let argument_id = parser
        .parse_positional_argument(Default::default())
        .unwrap();
    assert_node!(parser.tree, argument_id, Argument::Spread { value } => {
        // ...args
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "args");
        });
    });
}

#[test]
fn test_parse_spread_argument_with_doc_block_comment_newline() {
    // .../** comment */\nargs
    let test = TestParser::new(".../** comment */\nargs");
    let mut parser = test.prepare();
    let argument_id = parser
        .parse_positional_argument(Default::default())
        .unwrap();

    assert_node!(parser.tree, argument_id, Argument::Spread { value } => {
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "args");
        });
    });
}

#[test]
fn test_parse_type_tuple_spread_label_element() {
    let test = TestParser::new("(...args: number)");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenParenthesis).unwrap();
    let elements = parser
        .parse_type_tuple_elements(Default::default(), TokenType::CloseParenthesis)
        .unwrap();

    assert_eq!(elements.len(), 1);
    assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
        assert_string!(parser, label.unwrap(), "args");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_type_tuple_label_element_span() {
    let test = TestParser::new("(label: number)");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenParenthesis).unwrap();
    let elements = parser
        .parse_type_tuple_elements(Default::default(), TokenType::CloseParenthesis)
        .unwrap();

    assert_eq!(elements.len(), 1);
    assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
        assert!(!*is_optional);
        assert!(!*is_readonly);
        assert_string!(parser, label.unwrap(), "label");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_type_tuple_label_element_multiline_union_type() {
    let test = TestParser::new("(options?:\n  | SkipToken\n  | OtherOption)");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenParenthesis).unwrap();
    let elements = parser
        .parse_type_tuple_elements(Default::default(), TokenType::CloseParenthesis)
        .unwrap();

    assert_eq!(elements.len(), 1);
    assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
        assert!(*is_optional);
        assert!(!*is_readonly);
        assert_string!(parser, label.unwrap(), "options");
        assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 2);
            assert_expression_path!(parser, parser.tree.get(elements[0]), "SkipToken");
            assert_expression_path!(parser, parser.tree.get(elements[1]), "OtherOption");
        });
    });
}

#[test]
fn test_parse_generic_arguments_with_nested_generics_and_union() {
    let test = TestParser::new(
        "<keyof ServerReservedEventsMap<never, never, never, never> | keyof NamespaceReservedEventsMap<never, never, never, never>>",
    );
    let mut parser = test.prepare();
    let generic_arguments = parser
        .parse_generic_argument_list(Default::default())
        .unwrap();

    assert_eq!(generic_arguments.len(), 1);
    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert!(matches!(parser.tree.get(elements[0]), TypeExpression::KeyOf { .. }));
                assert!(matches!(parser.tree.get(elements[1]), TypeExpression::KeyOf { .. }));
            });
    });
}

#[test]
fn test_parse_associated_generic_refinements() {
    let test = TestParser::new("<type Item = uint8, const Width = 16>");
    let mut parser = test.prepare();
    let generic_arguments = parser
        .parse_generic_argument_list(Default::default())
        .unwrap();

    assert_eq!(generic_arguments.len(), 2);
    assert_node!(parser.tree, generic_arguments[0], GenericArgument::AssociatedType { name, value } => {
        assert_string!(parser, *name, "Item");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width: 8, is_signed: false }) });
    });
    assert_node!(parser.tree, generic_arguments[1], GenericArgument::AssociatedConst { name, value } => {
        assert_string!(parser, *name, "Width");
        assert_node!(parser.tree, *value, TypeExpression::Literal { value: Literal::Integer(16) });
    });
}
