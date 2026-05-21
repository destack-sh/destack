use destack_dir::{
    Argument, Asynchrony, ClassDeclaration, CommentKind, Declaration, Decorator, DecoratorPosition,
    Expression, FunctionDeclaration, FunctionRole, GenericArgument, GenericParameter, IfForm,
    IntegerType, Key, Keyword, Member, Name, NodeType, Parameter, Pattern, PatternField, Property,
    ScalarLiteral, TokenType, TupleElement, TypeExpression, TypeLiteral, Visibility,
};
use destack_source::{LanguageType, NodeSpanBoundary, NodeSpanType};

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
    let mut test = TestParser::new("T");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "T");
        assert!(declared_type.is_none());
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_parameter_with_type() {
    // x: int32
    let mut test = TestParser::new("x: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_parameter_with_maybe_type() {
    // x?: int32
    let mut test = TestParser::new("x?: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, is_optional, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert!(*is_optional);
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_parameter_with_default() {
    // validate: boolean = false
    let mut test = TestParser::new("validate: boolean = false");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "validate");
        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value: TypeLiteral::Boolean });
        assert!(default.is_some());
    });
}

#[test]
fn test_parse_parameter_missing_type_expression() {
    // x:
    let mut test = TestParser::new("x:");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Parameter), None, "")]);

    // x:
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *declared_type, TypeExpression::Missing);
    });
}

#[test]
fn test_parse_parameter_default_async_lambda_with_await_body() {
    let mut test = TestParser::new_with_language(
        "loadFonts: () => Promise<void> = async () => { await Fonts.loadElementsFonts(elements); }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();

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
    let mut test = TestParser::new("{ x = 4 }: boolean = false");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, declared_type: Some(declared_type), default: Some(default), .. } => {
        // { x = 4 }
        assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern) } => {
                // x
                assert_name!(parser, *name, "x");

                // x = 4
                assert_node!(parser.tree, *pattern, Pattern::Assign { pattern, value } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                        assert_string!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });
        });
        // boolean
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Boolean });
        // = false
        assert_node!(parser.tree, *default, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
    });
}

#[test]
fn test_parse_parameter_optional_pattern() {
    // []? optional pattern parameter
    let mut test = TestParser::new_with_language("[]?", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Pattern { pattern, is_optional, .. } => {
        assert!(*is_optional);
        assert_node!(parser.tree, *pattern, Pattern::Sequence { .. } => {});
    });
}

#[test]
fn test_parse_parameter_underscore_name() {
    // _ in TypeScript parameters is a normal name
    let mut test = TestParser::new_with_language("_", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type, default, .. } => {
        assert_string!(parser, *name, "_");
        assert!(declared_type.is_none());
        assert!(default.is_none());
    });
}

#[test]
fn test_parse_parameter_variadic() {
    // ...args
    let mut test = TestParser::new("...args");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
        assert_string!(parser, *name, "args");
        assert!(declared_type.is_none());
    });
}

#[test]
fn test_parse_parameter_variadic_with_type() {
    // ...args: int32[]
    let mut test = TestParser::new("...args: int32[]");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, declared_type, .. } => {
        assert_string!(parser, *name, "args");
        assert!(declared_type.is_some());
    });
}

#[test]
fn test_parse_parameter_optional_variadic() {
    // ...args? optional rest parameter
    let mut test = TestParser::new_with_language("...args?", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicNamed { name, .. } => {
        assert_string!(parser, *name, "args");
    });
}

/// Parse bracketed rest parameters in type position as sequence patterns.
#[test]
fn test_parse_parameter_variadic_tuple_name() {
    let mut test = TestParser::new("...[value]: [] | [TNext]");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                assert_name!(parser, *name, "value");
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
    let mut test = TestParser::new_with_language("...[first, second]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
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
    let mut test = TestParser::new_with_language(
        "...[body, init]: ConstructorParameters<typeof Response>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type, .. } => {
        // [body, init]
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields, .. } => {
            assert_eq!(fields.len(), 2);

            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                assert_name!(parser, *name, "body");
            });

            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                assert_name!(parser, *name, "init");
            });
        });

        // ConstructorParameters<typeof Response>
        let declared_type = declared_type.expect("expected variadic tuple type annotation");
        assert_node!(parser.tree, declared_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "ConstructorParameters");
            assert_eq!(generic_arguments.len(), 1);

            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::TypeOfValue { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "Response");
                    });
            });
        });
    });
}

#[test]
fn test_parse_parameter_variadic_array_pattern_with_nested_object_and_defaults() {
    // ...[src, { id, systemId, input, syncSnapshot = false } = {} as any]: SpawnArguments<...>
    let mut test = TestParser::new_with_language(
        r#"...[
    src,
    { id, systemId, input, syncSnapshot = false } = {} as any
]: SpawnArguments<TContext, TExpressionEvent, TEvent, TActor>"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::VariadicPattern { pattern, declared_type: Some(declared_type), .. } => {
        // [src, { ... } = {} as any]
        assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);

            // src
            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None, .. } => {
                assert_name!(parser, *name, "src");
            });

            // { id, systemId, input, syncSnapshot = false } = {} as any
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Assign { pattern, value } => {
                    assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                        assert_eq!(fields.len(), 4);

                        assert_node!(parser.tree, fields[3], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern), .. } => {
                            assert_name!(parser, *name, "syncSnapshot");

                            assert_node!(parser.tree, *pattern, Pattern::Assign { pattern, value } => {
                                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                                    assert_string!(parser, *name, "syncSnapshot");
                                });
                                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                            });
                        });
                    });

                    assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
                            assert!(properties.is_empty());
                        });
                        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value: TypeLiteral::Any });
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
    let mut test = TestParser::new_with_language("...{ value: alias }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
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
    let mut test = TestParser::new("x:\n\tint32");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_parameter_with_modifiers() {
    // private readonly const x: 1
    let mut test = TestParser::new("private readonly const x: 1");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();

    assert_node!(parser.tree, parameter_id, Parameter::Named { visibility, is_readonly, declared_type: Some(declared_type), .. } => {
        assert_eq!(*visibility, Some(Visibility::Private));
        assert!(*is_readonly);
        assert_node!(parser.tree, *declared_type, TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(1) });
    });
}

#[test]
fn test_parse_generic_parameters_multiline_union_constraint_with_default() {
    let mut test = TestParser::new_with_language(
        r#"<
  Return extends ReturnType<onRequestHookHandler<RawServer>>
    | ReturnType<onRequestAsyncHookHandler<RawServer>>
    = ReturnType<onRequestHookHandler<RawServer>>
>"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let generic_parameters = parser.eat_generic_parameters(true).unwrap();

    // Return extends ReturnType<onRequestHookHandler<RawServer>> | ReturnType<onRequestAsyncHookHandler<RawServer>> = ReturnType<onRequestHookHandler<RawServer>>
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
    let mut test = TestParser::new_with_language(
        "<Union, LastElement = LastOf<Union>>",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let generic_parameters = parser.eat_generic_parameters(true).unwrap();

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
    let mut test = TestParser::new("<\n  T>");
    let mut parser = test.prepare();
    let generic_parameters = parser.eat_generic_parameters(true).unwrap();

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
    let mut test = TestParser::new("<T");
    let mut parser = test.prepare();
    let parameters = parser.eat_generic_parameters(true).unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // <T
    assert_eq!(parameters.len(), 1);
    assert_node!(parser.tree, parameters[0], GenericParameter::Type { name, constraint: None, default: None, .. } => {
        assert_string!(parser, *name, "T");
    });
}

#[test]
fn test_parse_generic_arguments_missing_close_angle_in_type_context() {
    // <string, number
    let mut test = TestParser::new("<string, number");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // <string, number
    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::String });
    });
    assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_generic_arguments_explicit_type_argument() {
    // <type {}>
    let mut test = TestParser::new("<type {}>");
    let mut parser = test.prepare();
    let arguments = parser.eat_generic_arguments().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Object { members } => {
            assert!(members.is_empty());
        });
    });
}

#[test]
fn test_parse_generic_arguments_object_literal_stays_value_in_type_context() {
    // <{ name: "alpha"; count: 1 }>
    let mut test =
        TestParser::new_with_language(r#"<{ name: "alpha"; count: 1 }>"#, LanguageType::Destack);
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();

    test.assert_no_errors(&parser);
    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Value { value } => {
        assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);

            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "name");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                    assert_string!(parser, *value, "alpha");
                });
            });

            assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    });
}

#[test]
fn test_parse_generic_arguments_empty_in_type_context_recovers_error_slot() {
    // <>
    let mut test = TestParser::new("<>");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, Some(TokenType::Identifier), "<")]);

    // <>
    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Error);
}

#[test]
fn test_parse_generic_arguments_first_value_with_boundary_comment() {
    let source = "<\n  // first-type-arg\n  string | number\n>";
    let mut test = TestParser::new_with_language(source, LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();
    parser.attach_comments();

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Union { .. });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "first-type-arg");
}

#[test]
fn test_parse_generic_arguments_following_value_with_boundary_comment() {
    let source = "<string,\n  // second-type-arg\n  number>";
    let mut test = TestParser::new_with_language(source, LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();
    parser.attach_comments();

    assert_eq!(arguments.len(), 2);
    assert_node!(parser.tree, arguments[1], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "second-type-arg");
}

#[test]
fn test_reject_generic_arguments_missing_close_angle_in_value_context() {
    // <string, number
    let mut test = TestParser::new_with_language("<string, number", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let result = parser.eat_generic_arguments();

    assert!(result.is_err());
}

#[test]
fn test_parse_spread_type_generic_argument() {
    // <...T>
    let mut test = TestParser::new_with_language("<...T>", LanguageType::Destack);
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();

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
fn test_parse_spread_value_generic_argument() {
    // <...values()>
    let mut test = TestParser::new_with_language("<...values()>", LanguageType::Destack);
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);
    let arguments = parser.eat_generic_arguments().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(arguments.len(), 1);
    assert_node!(parser.tree, arguments[0], GenericArgument::SpreadValue { value } => {
        assert_node!(parser.tree, *value, Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "values");
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_parse_variadic_type_generic_parameter() {
    // <...Parameters, Return>
    let mut test = TestParser::new_with_language("<...Parameters, Return>", LanguageType::Destack);
    let mut parser = test.prepare();
    let parameters = parser.eat_generic_parameters(true).unwrap();

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
fn test_parse_variadic_value_generic_parameter() {
    // <comptime ...Shape: readonly usize[]>
    let mut test = TestParser::new_with_language(
        "<comptime ...Shape: readonly usize[]>",
        LanguageType::Destack,
    );
    let mut parser = test.prepare();
    let parameters = parser.eat_generic_parameters(true).unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(parameters.len(), 1);
    assert_node!(parser.tree, parameters[0], GenericParameter::VariadicValue { name, declared_type, default, is_comptime } => {
        assert_string!(parser, *name, "Shape");
        assert!(*is_comptime);
        assert!(default.is_none());
        assert_node!(parser.tree, declared_type.expect("expected value variadic type"), TypeExpression::Readonly { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::Array { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Integer(IntegerType::Pointer { is_signed: false }));
                });
            });
        });
    });
}

#[test]
fn test_parse_parameter_with_readonly_public_modifier_order_reports_error() {
    // readonly public x: number
    let mut test =
        TestParser::new_with_language("readonly public x: number", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, "public")]);

    // readonly public x: number
    assert_node!(parser.tree, parameter_id, Parameter::Named { visibility, is_readonly, name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_eq!(*visibility, Some(Visibility::Public));
        assert!(*is_readonly);
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_constructor_parameter_with_readonly_public_modifier_order_reports_error() {
    // class D { constructor(readonly public x: number) {} }
    let mut test = TestParser::new_with_language(
        "class D { constructor(readonly public x: number) {} }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, "public")]);

    // class D { constructor(readonly public x: number) {} }
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);

            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert_eq!(signature.role, Some(FunctionRole::Constructor));
                assert_eq!(signature.parameters.len(), 1);

                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { visibility, is_readonly, name, declared_type: Some(declared_type), default: None, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_eq!(*visibility, Some(Visibility::Public));
                    assert!(*is_readonly);
                    assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Number });
                });
            });
        });
    });
}

#[test]
fn test_parse_parameter_readonly_name() {
    // readonly: int32
    let mut test = TestParser::new("readonly: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, declared_type: Some(declared_type), default: None, .. } => {
        assert_string!(parser, *name, "readonly");
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_parameter_comptime() {
    // comptime n: int32
    let mut test = TestParser::new("comptime n: int32");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    assert_node!(parser.tree, parameter_id, Parameter::Named { name, is_comptime, declared_type: Some(declared_type), .. } => {
        assert_string!(parser, *name, "n");
        assert!(*is_comptime);
        assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true
        }) });
    });
}

#[test]
fn test_parse_comptime_modifier_target_requires_same_line() {
    let mut test = TestParser::new(
        r#"comptime
n"#,
    );
    let mut parser = test.prepare();
    let modifiers = parser
        .eat_binding_modifiers_prefix_maybe(true, true, true, true, true, true)
        .unwrap();

    assert!(modifiers.is_none());
    assert!(parser.is_keyword(Keyword::Comptime));
}

#[test]
fn test_parse_comptime_modifier_allows_block_line_break() {
    let mut test = TestParser::new(
        r#"comptime
{}"#,
    );
    let mut parser = test.prepare();
    let modifiers = parser
        .eat_binding_modifiers_prefix_maybe(true, true, true, true, true, true)
        .unwrap()
        .unwrap();

    assert!(modifiers.is_comptime);
    assert!(parser.current_token_is_on_new_line());
    assert!(parser.peek_is(TokenType::OpenBrace));
}

/// Parse TypeScript parameter decorators in constructors and methods.
#[test]
fn test_parse_parameter_decorators() {
    let input = r#"
class Test {
    constructor(@p1 t1, @p2 private t2, @p3 ...t3) {}

    method(@p1 t1, @p1 @p2 ...t2) {}
}
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    // class Test { ... }
    let expression_id = parser.unwrap_label_expression(expressions[0]);
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
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { visibility, name, declared_type: None, default: None, .. } => {
                    assert_string!(parser, *name, "t2");
                    assert_eq!(*visibility, Some(Visibility::Private));
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
}

#[test]
fn test_parse_named_argument() {
    // x: 1
    let mut test = TestParser::new("x: 1");
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        // x
        assert_string!(parser, *name, "x");
        // 1
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });

    let main_span = parser
        .tree
        .get_main_span(argument_id)
        .expect("expected argument name span");
    assert_eq!(parser.get_span_str(main_span), "x");
}

#[test]
fn test_parse_named_argument_string_span() {
    let mut test = TestParser::new("\"Content-Type\": 1");
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::String(name), value } => {
        assert_string!(parser, *name, "Content-Type");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });

    let main_span = parser
        .tree
        .get_main_span(argument_id)
        .expect("expected argument name span");
    assert_eq!(parser.get_span_str(main_span), "\"Content-Type\"");
}

#[test]
fn test_parse_named_argument_string_literal_value() {
    let mut test = TestParser::new("title=\"hello\"");
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "title");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "hello");
        });
    });
}

/// Decode valid HTML entities in quoted tree attribute strings.
#[test]
fn test_parse_tree_attribute_string_decodes_html_entities() {
    let mut test = TestParser::new_with_language(
        "title=\"A&nbsp;&amp;&#160;&#xA0;B\"",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();

    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "title");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "A\u{a0}&\u{a0}\u{a0}B");
        });
    });
}

/// Preserve invalid HTML entities in quoted tree attribute strings.
#[test]
fn test_parse_tree_attribute_string_preserves_invalid_html_entities() {
    let mut test =
        TestParser::new_with_language("title=\"A&missing;B&amp;C\"", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();

    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "title");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "A&missing;B&C");
        });
    });
}

#[test]
fn test_parse_named_argument_with_newline_before_assign_before_tree() {
    let mut test = TestParser::new_with_language(
        "onBroadcastSelected\n    = { this._onYouTubeBroadcastIDSelected }",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();

    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "onBroadcastSelected");
        assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
            assert_node!(parser.tree, *left, Expression::This);
            assert_string!(parser, *name, "_onYouTubeBroadcastIDSelected");
        });
    });
}

#[test]
fn test_parse_named_argument_with_numeric_kebab_segment() {
    let mut test = TestParser::new_with_language("panose-1=\"test\"", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "panose1");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "test");
        });
    });
}

#[test]
fn test_parse_named_argument_with_double_hyphen_kebab_segment() {
    let mut test = TestParser::new_with_language(
        "data-nextjs-container-errors-pseudo-html--diff={sign === '+' ? 'add' : 'remove'}",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let argument_id = parser.eat_tree_literal_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Named { name: Name::Identifier(name), value } => {
        assert_string!(parser, *name, "dataNextjsContainerErrorsPseudoHtmlDiff");
        assert_node!(parser.tree, *value, Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

#[test]
fn test_parse_positional_argument() {
    // 3
    let mut test = TestParser::new("3");
    let mut parser = test.prepare();
    let argument_id = parser.eat_positional_argument().unwrap();

    assert_node!(parser.tree, argument_id, Argument::Positional { value } => {
        // 3
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
    });
}

#[test]
fn test_parse_dynamic_argument_span_trims_before_delayed_comma() {
    let source = r#"(
  a

  ,
  b
)"#;
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

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
    let mut test = TestParser::new("(x, =, y)");
    let mut parser = test.prepare();
    let parameters = parser.eat_dynamic_parameters().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Parameter), None, "=")]);

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
    let mut test = TestParser::new("(1, , 3)");
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, ",")]);

    // (1, , 3)
    assert_eq!(arguments.len(), 3);
    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
    assert_node!(parser.tree, arguments[1], Argument::Error);
    assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
    });
}

#[test]
fn test_parse_dynamic_arguments_recover_missing_close_before_next_statement() {
    // (a,b const
    let source = "(a,b const";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

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
    let mut test = TestParser::new("(a, ...)");
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, ")")]);

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
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ";")]);

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
    let mut test = TestParser::new("(,,b)");
    let mut parser = test.prepare();
    let arguments = parser.eat_dynamic_arguments().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, ","), (None, None, ",")]);

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
    let mut test = TestParser::new_with_language("foo(a,b;", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ";")]);

    // foo(a,b;
    assert_eq!(expressions.len(), 1);
    let call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });
}

#[test]
fn test_parse_malformed_call_statement_before_const_keeps_call_shape() {
    let mut test = TestParser::new_with_language("foo(a,b const;", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[
            (Some(NodeType::Expression), None, "const"),
            (None, None, "const"),
            (Some(NodeType::Expression), None, ";"),
        ],
    );

    // foo(a,b const;
    assert_eq!(expressions.len(), 2);
    let call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });
    assert_node!(parser.tree, expressions[1], Expression::Error);
}

#[test]
fn test_parse_malformed_call_statement_with_leading_empty_slots_keeps_call_shape() {
    let mut test = TestParser::new_with_language("foo (,,b);", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, ","), (None, None, ",")]);

    // foo (,,b);
    assert_eq!(expressions.len(), 1);
    let call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 3);
        assert_node!(parser.tree, arguments[0], Argument::Error);
        assert_node!(parser.tree, arguments[1], Argument::Error);
    });
}

#[test]
fn test_parse_malformed_call_statement_with_trailing_spread_keeps_call_shape() {
    let mut test = TestParser::new_with_language("foo (a, ...);", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(None, None, ")")]);

    // foo (a, ...);
    assert_eq!(expressions.len(), 1);
    let call_id = parser.unwrap_label_expression(expressions[0]);
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
    let mut test = TestParser::new_with_language(source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[
            (Some(NodeType::Expression), None, "const"),
            (None, None, "const"),
            (Some(NodeType::Expression), None, ";"),
            (None, None, ","),
            (None, None, ","),
        ],
    );

    // foo(a,b const;
    // Error
    // foo (,,b);
    assert_eq!(expressions.len(), 3);

    let first_call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });

    assert_node!(parser.tree, expressions[1], Expression::Error);

    let second_call_id = parser.unwrap_label_expression(expressions[2]);
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
    let mut test = TestParser::new_with_language(source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[
            (Some(NodeType::Expression), None, "const"),
            (None, None, "const"),
            (Some(NodeType::Expression), None, ";"),
            (None, None, ")"),
        ],
    );

    assert_eq!(expressions.len(), 3);

    // foo(a,b const;
    let first_call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
    });

    // Error
    assert_node!(parser.tree, expressions[1], Expression::Error);

    // foo (a, ...);
    let second_call_id = parser.unwrap_label_expression(expressions[2]);
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
    let mut test = TestParser::new_with_language(source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[(None, None, ","), (Some(NodeType::Expression), None, "bar")],
    );

    assert_eq!(expressions.len(), 2);

    // foo(,
    let first_call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Error);
    });

    // bar();
    let second_call_id = parser.unwrap_label_expression(expressions[1]);
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
    let mut test = TestParser::new_with_language(source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[
            (None, None, ","),
            (Some(NodeType::Expression), None, "const"),
        ],
    );

    assert_eq!(expressions.len(), 2);

    // foo(,
    let first_call_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_call_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Error);
    });

    // const value = 1;
    let second_expression_id = parser.unwrap_label_expression(expressions[1]);
    assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_parse_spread_argument() {
    // ...args
    let mut test = TestParser::new("...args");
    let mut parser = test.prepare();
    let argument_id = parser.eat_positional_argument().unwrap();
    assert_node!(parser.tree, argument_id, Argument::Spread { label, value } => {
        // ...args
        assert!(label.is_none());
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "args");
        });
    });
}

#[test]
fn test_parse_spread_argument_with_doc_block_comment_newline() {
    // .../** comment */\nargs
    let mut test =
        TestParser::new_with_language(".../** comment */\nargs", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let argument_id = parser.eat_positional_argument().unwrap();

    assert_node!(parser.tree, argument_id, Argument::Spread { label, value } => {
        assert!(label.is_none());
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "args");
        });
    });
}

#[test]
fn test_parse_type_tuple_spread_label_element() {
    let mut test = TestParser::new("[...args: number]");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenBracket).unwrap();
    let elements = parser
        .eat_type_tuple_elements_body(TokenType::CloseBracket)
        .unwrap();

    assert_eq!(elements.len(), 1);
    assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
        assert_string!(parser, label.unwrap(), "args");
        assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_type_tuple_label_element_span() {
    let mut test = TestParser::new("[label: number]");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenBracket).unwrap();
    let elements = parser
        .eat_type_tuple_elements_body(TokenType::CloseBracket)
        .unwrap();

    assert_eq!(elements.len(), 1);
    assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
        assert!(!*is_optional);
        assert!(!*is_readonly);
        assert_string!(parser, label.unwrap(), "label");
        assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Number });
    });
}

#[test]
fn test_parse_type_tuple_label_element_multiline_union_type() {
    let mut test = TestParser::new("[options?:\n  | SkipToken\n  | OtherOption]");
    let mut parser = test.prepare();
    parser.eat_token(TokenType::OpenBracket).unwrap();
    let elements = parser
        .eat_type_tuple_elements_body(TokenType::CloseBracket)
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
    let mut test = TestParser::new_with_language(
        "<keyof ServerReservedEventsMap<never, never, never, never> | keyof NamespaceReservedEventsMap<never, never, never, never>>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let generic_arguments = parser.eat_generic_arguments().unwrap();

    assert_eq!(generic_arguments.len(), 1);
    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert!(matches!(parser.tree.get(elements[0]), TypeExpression::KeyOf { .. }));
                assert!(matches!(parser.tree.get(elements[1]), TypeExpression::KeyOf { .. }));
            });
    });
}
