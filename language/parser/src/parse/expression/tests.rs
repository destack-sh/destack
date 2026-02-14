use destack_ast::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, BindingKind, Block, Declaration,
    DeclarationDescriptor, Declarator, DependencyItem, DependencyKind, DependencyMode, EnumField,
    EnumKind, Expression, FunctionKind, IfCondition, IfKind, ImportAliasTarget, ImportSource,
    ImportTarget, IntType, Key, Member, Mutability, Name, Parameter, Pattern, PatternField,
    PostfixPosition, Property, ScalarLiteral, TemplateLiteral, TypeBinaryOperator, TypeLiteral,
    TypePredicateSubject, TypeUnaryOperator, UnaryOperator, VarianceBound,
};
use destack_source::{DiagnosticSeverity, LanguageType};

use crate::{
    TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
};

fn assert_import_target_string(parser: &crate::Parser, target: &ImportTarget, expected: &str) {
    assert_node!(target, ImportTarget::String(target) => {
        assert_string!(parser, *target, expected);
    });
}

/// Disambiguate using import meta as a path.
#[test]
fn test_parse_import_as_path() {
    let mut test = TestParser::new("import.meta.env");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_expression_path!(parser, parser.tree.get(expression_id), "import.meta.env");
}

/// Parse a bare this expression.
#[test]
fn test_parse_this_expression() {
    let mut test = TestParser::new("this");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::This);
}

/// Parse a bare super expression.
#[test]
fn test_parse_super_expression() {
    let mut test = TestParser::new_with_options("super", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Super);
}

/// Parse super member access.
#[test]
fn test_parse_super_member_expression() {
    let mut test = TestParser::new_with_options("super.value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // super.value
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_node!(parser.tree, *left, Expression::Super);
        assert_string!(parser, *name, "value");
    });
}

/// Parse a member expression with a newline after dot in TypeScript.
#[test]
fn test_parse_member_expression_with_newline_after_dot_typescript() {
    let mut test = TestParser::new_with_options("receiver.\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // receiver.\nnext
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "next");
        assert_expression_path!(parser, parser.tree.get(*left), "receiver");
    });
}

/// Parse a call chain with a newline after dot in TypeScript.
#[test]
fn test_parse_call_chain_with_newline_after_dot_typescript() {
    let mut test =
        TestParser::new_with_options("receiver().\nthen(value)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // receiver().\nthen(value)
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });

        // receiver().then
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "then");
            assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
                assert!(dynamic_arguments.is_empty());
                assert_expression_path!(parser, parser.tree.get(*left), "receiver");
            });
        });
    });
}

/// Parse private member access with a newline before dot in TypeScript.
#[test]
fn test_parse_private_member_expression_with_newline_before_dot_typescript() {
    let mut test = TestParser::new_with_options("this\n.#value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_node!(parser.tree, expression_id, Expression::PrivateMember { left, name, static_arguments } => {
        assert_node!(parser.tree, *left, Expression::This);
        assert_string!(parser, *name, "value");
        assert!(static_arguments.is_none());
    });
}

/// Parse private member casts with a newline before dot in object property values.
#[test]
fn test_parse_object_property_private_member_cast_with_newline_before_dot_typescript() {
    let mut test = TestParser::new_with_options(
        "({ value: this\n.#javascriptTransformer as unknown as JavaScriptTransformer })",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *value, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    assert_node!(parser.tree, *left, Expression::PrivateMember { left, name, static_arguments } => {
                        assert_node!(parser.tree, *left, Expression::This);
                        assert_string!(parser, *name, "javascriptTransformer");
                        assert!(static_arguments.is_none());
                    });
                    assert_node!(parser.tree, *right, Expression::TypeBinary { operator, left, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::Unknown));
                        assert_expression_path!(parser, parser.tree.get(*right), "JavaScriptTransformer");
                    });
                });
            });
        });
    });
}

/// Reject decimal integer member access without a separator in destack.
#[test]
fn test_reject_decimal_integer_member_access_without_separator_in_destack() {
    let mut test = TestParser::new("1.foo");
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Reject decimal integer member access without a separator in typescript.
#[test]
fn test_reject_decimal_integer_member_access_without_separator_in_typescript() {
    let mut test = TestParser::new_with_options("1.foo", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse parenthesized integer member access in destack.
#[test]
fn test_parse_parenthesized_integer_member_access_in_destack() {
    let mut test = TestParser::new("(1).foo");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

/// Parse this member access in variant context.
#[test]
fn test_parse_this_member_expression_in_variant_context() {
    let mut test = TestParser::new_with_options("this.port1.onmessage", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.options.in_variant = true;
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // this.port1.onmessage
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "onmessage");
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "port1");
            assert_node!(parser.tree, *left, Expression::This);
        });
    });
}

/// Parse this member access in call arguments in variant context.
#[test]
fn test_parse_call_argument_this_member_expression_in_variant_context() {
    let mut test = TestParser::new_with_options(
        "setTimeout(this.port1.onmessage, 0)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.options.in_variant = true;
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // setTimeout(this.port1.onmessage, 0)
    assert_node!(parser.tree, expression_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 2);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            // this.port1.onmessage
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "onmessage");
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "port1");
                    assert_node!(parser.tree, *left, Expression::This);
                });
            });
        });
    });
}

/// Parse a private identifier used in an in expression.
#[test]
fn test_parse_private_identifier_in_expression() {
    let mut test = TestParser::new_with_options("#a in this", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    // #a in this
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::In);
        assert_node!(parser.tree, *left, Expression::PrivateIdentifier { name } => {
            assert_string!(parser, *name, "a");
        });
        assert_node!(parser.tree, *right, Expression::This);
    });
}

/// Disambiguate using `type` as a variable.
#[test]
fn test_parse_type_as_variable() {
    let mut test = TestParser::new(
        r"
let type = 1
type = type * 2
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();

    // let type = 1
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value: Some(value), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "type");
            });
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
    parser.eat_newline().unwrap();

    // type = type * 2
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_eq!(*operator, AssignOperator::Assign);
        // type * 2
        assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_eq!(*operator, BinaryOperator::Multiply);
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    });
    parser.eat_newline().unwrap();
}

/// Parse `namespace` as an identifier in indexed assignment expressions.
#[test]
fn test_parse_namespace_as_identifier_in_index_assignment() {
    let mut test =
        TestParser::new_with_options("namespace[this.dest] = values", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, Expression::Index { left, index, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "namespace");

            assert_node!(parser.tree, index.expect("expected index"), Expression::Member { left, name, .. } => {
                assert_node!(parser.tree, *left, Expression::This);
                assert_string!(parser, *name, "dest");
            });
        });

        assert_expression_path!(parser, parser.tree.get(*right), "values");
    });
}

/// Parse override as an identifier in call expressions.
#[test]
fn test_parse_override_as_identifier_call_in_typescript() {
    let mut test = TestParser::new_with_options("override(value)", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "override");
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

/// Parse override as an identifier in call expressions in destack mode.
#[test]
fn test_parse_override_as_identifier_call_in_destack() {
    let mut test = TestParser::new("override(value)");
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "override");
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

/// Parse abstract as an identifier in call expressions in destack mode.
#[test]
fn test_parse_abstract_as_identifier_call_in_destack() {
    let mut test = TestParser::new("abstract(value)");
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "abstract");
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

/// Parse type as an identifier in call expressions.
#[test]
fn test_parse_type_as_identifier_call_in_typescript() {
    let mut test = TestParser::new_with_options("type(123)", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(123)));
        });
    });
}

/// Parse TypeScript newline-sensitive keyword cases without parse diagnostics.
fn assert_parse_without_errors_in_typescript(source: &str) {
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let _ = parser.parse();

    let error_diagnostics: Vec<_> = parser
        .diagnostics
        .iter()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect();
    assert!(
        error_diagnostics.is_empty(),
        "unexpected parser diagnostics: {error_diagnostics:#?}"
    );
}

/// Parse `abstract` on one line and class on the next as valid TypeScript.
#[test]
fn test_parse_typescript_abstract_newline_class_without_errors() {
    assert_parse_without_errors_in_typescript("abstract\nclass B {}");
}

/// Parse `declare enum` split across lines as valid TypeScript.
#[test]
fn test_parse_typescript_declare_enum_newline_without_errors() {
    assert_parse_without_errors_in_typescript("declare enum\nE\n{}");
}

/// Parse `type` followed by a newline as valid TypeScript source.
#[test]
fn test_parse_typescript_type_newline_without_errors() {
    assert_parse_without_errors_in_typescript("type\nFoo = string;");
}

/// Parse a callback body using `type` as an identifier statement.
#[test]
fn test_parse_typescript_callback_type_identifier_without_errors() {
    assert_parse_without_errors_in_typescript(
        "avplay.setListener({
    onsubtitlechange: (duration, subtitles, type, attributes) => {
        duration // $ExpectType string
        subtitles // $ExpectType string
        type // $ExpectType string
        attributes // $ExpectType AVPlaySubtitleAttribute[]
    }
})",
    );
}

/// Parse typed object methods in decorator style call arguments.
#[test]
fn test_parse_typed_object_method_in_call_argument() {
    let mut test = TestParser::new_with_options(
        r#"connect({
    num(state: State) {
        return state.counter.num;
    },
    inc: "inc",
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(dynamic_arguments.len(), 1);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(body), .. } => {
                    assert_string!(parser, *name, "num");
                    assert_eq!(signature.dynamic_parameters.len(), 1);

                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                        assert_string!(parser, *name, "state");
                        assert_expression_path!(parser, parser.tree.get(*ty), "State");
                    });

                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                            assert_eq!(expressions.len(), 1);
                            assert_node!(parser.tree, expressions[0], Expression::Statement(statement) => {
                                assert_node!(parser.tree, *statement, Expression::Return { value: Some(value) } => {
                                    assert_expression_path!(parser, parser.tree.get(*value), "state.counter.num");
                                });
                            });
                        });
                    });
                });

                assert_node!(parser.tree, properties[1], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                    assert_string!(parser, *name, "inc");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                        assert_string!(parser, *value, "inc");
                    });
                });
            });
        });
    });
}

/// Parse typed object methods in decorator call arguments.
#[test]
fn test_parse_typed_object_method_in_decorator_argument() {
    let mut test = TestParser::new_with_options(
        r#"connect({
    num(state: State) {
        if (state.counter.num === 0) return true;
        return new Promise((resolve, reject) => {
            setTimeout(() => {
                resolve(state.counter.num !== 0);
            }, 1);
        });
    },
    inc: "inc",
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .with_options(parser.options.in_decorator(), |parser| {
            parser.eat_expression(parser.options)
        })
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(dynamic_arguments.len(), 1);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { key: Some(Key::Name(Name::Identifier(name))), body: Some(body), .. } => {
                    assert_string!(parser, *name, "num");
                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                            assert_eq!(expressions.len(), 2);
                            assert_node!(parser.tree, expressions[0], Expression::If { .. });
                            assert_node!(parser.tree, expressions[1], Expression::Statement(statement) => {
                                assert_node!(parser.tree, *statement, Expression::Return { value: Some(value) } => {
                                    assert_node!(parser.tree, *value, Expression::New { .. });
                                });
                            });
                        });
                    });
                });

                assert_node!(parser.tree, properties[1], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                    assert_string!(parser, *name, "inc");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                        assert_string!(parser, *value, "inc");
                    });
                });
            });
        });
    });
}

/// Parse keywords as fields and identifiers.
#[test]
fn test_parse_keywords_as_fields_and_identifiers() {
    let mut test = TestParser::new(
        "{
    // can be used as both fields and bindings
    namespace: namespace,
    module: module,
    struct: struct,
    class: class,
    enum: enum,
    union: union,
    interface: interface,
    type: type,
    implement: implement,
    function: function,
    constructor: constructor,
    // can only be used as fields
    let: 0,
    var: 0,
    new: 0,
    delete: 0,
    switch: 0,
    case: 0,
    default: 0,
    do: 0,
    while: 0,
    for: 0,
    loop: 0,
    break: 0,
    continue: 0,
    match: 0,
}",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 25);

        assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
            assert_string!(parser, *name, "namespace");
            assert_expression_path!(parser, parser.tree.get(*value), "namespace");
        });

        assert_node!(parser.tree, properties[10], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
            assert_string!(parser, *name, "constructor");
            assert_expression_path!(parser, parser.tree.get(*value), "constructor");
        });

        assert_node!(parser.tree, properties[11], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
            assert_string!(parser, *name, "let");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });

        assert_node!(parser.tree, properties[24], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
            assert_string!(parser, *name, "match");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}

/// Parse an if extends condition without consuming the block.
#[test]
fn test_parse_if_extends_type_reference() {
    let mut test = TestParser::new(
        r#"if x extends Foo {
    body
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        // x extends Foo
        assert_node!(parser.tree, condition_id, Expression::TypeBinary { left, operator, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            assert_eq!(*operator, TypeBinaryOperator::Extends);
            assert_expression_path!(parser, parser.tree.get(*right), "Foo");
        });
        // { body }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { format: _, expressions } => {
                assert_eq!(expressions.len(), 1);
                assert_expression_path!(parser, parser.tree.get(expressions[0]), "body");
            });
        });
    });
}

/// Parse an if instanceof condition inside parentheses.
#[test]
fn test_parse_if_instanceof_type_reference() {
    let mut test = TestParser::new(
        r#"if (T instanceof Foo) {
    value
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        // T instanceof Foo
        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "T");
            assert_eq!(*operator, BinaryOperator::InstanceOf);
            assert_expression_path!(parser, parser.tree.get(*right), "Foo");
        });
        // { value }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { format: _, expressions } => {
                assert_eq!(expressions.len(), 1);
                assert_expression_path!(parser, parser.tree.get(expressions[0]), "value");
            });
        });
    });
}

/// Parse `export { bar, baz } from foo`.
#[test]
fn test_parse_export_expression_with_items_block() {
    let mut test = TestParser::new("export { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // export { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export { bar, baz }`.
#[test]
fn test_parse_export_expression_items_without_target() {
    let mut test = TestParser::new("export { bar, baz }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export type { Foo, Bar } from "module"`.
#[test]
fn test_parse_export_expression_type_items_with_target() {
    let mut test = TestParser::new("export type { Foo, Bar } from \"module\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Type);
        assert_string!(parser, *target, "module");
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export type { Foo }`.
#[test]
fn test_parse_export_expression_type_items_without_target() {
    let mut test = TestParser::new("export type { Foo }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
        assert_eq!(*kind, DependencyKind::Type);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export * as baz from "foo"`.
#[test]
fn test_parse_export_expression_namespace_alias() {
    let mut test = TestParser::new("export * as baz from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // export * as baz from foo
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_string!(parser, *alias, "baz");
        });
    });
}

/// Parse `export = foo`.
#[test]
fn test_parse_export_expression_module_export() {
    let mut test = TestParser::new("export = foo");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        // = foo
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, value: Some(value), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_expression_path!(parser, parser.tree.get(*value), "foo");
        });
    });
}

/// Parse an export declaration of a type declaration.
#[test]
fn test_parse_export_expression_type_declaration() {
    let mut test = TestParser::new("export type NonNullValue = Something");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, export, .. }, .. } => {
            assert_string!(parser, name.unwrap().string(), "NonNullValue");
            assert!(export.is_some());
        });
    });
}

/// Parse export default abstract class with decorator prefixes.
#[test]
fn test_parse_export_default_abstract_class_with_decorator_prefixes() {
    let mut test = TestParser::new_with_options(
        "@before\nexport default @after abstract class Foo { }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);
    let expression_id = match parser.tree.get(expressions[0]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[0],
    };
    assert_node!(parser.tree, expression_id, Expression::Export { kind, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem { mode, value: Some(value), .. } => {
            assert_eq!(*mode, DependencyMode::Default);
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
                    assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
                });
            });
        });
    });
}

#[test]
fn test_reject_export_type_without_binding_or_declaration() {
    let mut test = TestParser::new("export type");
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Reject bare export path expressions in JavaScript.
#[test]
fn test_reject_export_path_expression_javascript() {
    // source: export foo
    let mut test = TestParser::new_with_options("export foo", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    // foo
    assert_eq!(parser.get_span_str(error.leaf_span()), "foo");
}

/// Reject bare export path expressions in Destack.
#[test]
fn test_reject_export_path_expression_destack() {
    // source: export foo
    let mut test = TestParser::new_with_options("export foo", LanguageType::Destack);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    // foo
    assert_eq!(parser.get_span_str(error.leaf_span()), "foo");
}

#[test]
fn test_reject_export_default_enum() {
    let mut test = TestParser::new("export default enum A { X, Y, Z }");
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse `export import foo = bar.baz`.
#[test]
fn test_parse_export_import_equals() {
    let mut test = TestParser::new("export import atob = globalThis.atob");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
            assert!(descriptor.export.is_some());
            assert_eq!(*kind, DependencyKind::Value);
            let name = descriptor.name.expect("import alias name");
            assert_string!(parser, name.string(), "atob");
            match target {
                ImportAliasTarget::Path { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "globalThis.atob");
                }
                ImportAliasTarget::Require { .. } => {
                    panic!("expected import alias path");
                }
            }
        });
    });
}

/// Parse `export import type React = require("react")`.
#[test]
fn test_parse_export_import_type_equals_require() {
    let mut test = TestParser::new(r#"export import type React = require("react")"#);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
            assert!(descriptor.export.is_some());
            assert_eq!(*kind, DependencyKind::Type);
            let name = descriptor.name.expect("import alias name");
            assert_string!(parser, name.string(), "React");
            match target {
                ImportAliasTarget::Require { target } => {
                    assert_string!(parser, *target, "react");
                }
                ImportAliasTarget::Path { .. } => {
                    panic!("expected import alias require");
                }
            }
        });
    });
}

#[test]
fn test_parse_export_import_type_equals_require_with_newlines() {
    let mut test = TestParser::new_with_options(
        r#"
export
import
type
React = require("react")
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
            assert!(descriptor.export.is_some());
            assert_eq!(*kind, DependencyKind::Type);
            let name = descriptor.name.expect("import alias name");
            assert_string!(parser, name.string(), "React");
            match target {
                ImportAliasTarget::Require { target } => {
                    assert_string!(parser, *target, "react");
                }
                ImportAliasTarget::Path { .. } => {
                    panic!("expected import alias require");
                }
            }
        });
    });
}

/// Parse `import { bar, baz } from foo`.
#[test]
fn test_parse_import_expression_with_items_block() {
    let mut test = TestParser::new("import { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // import { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
        assert_eq!(*source, ImportSource::ImportStatement);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `import * as baz from "foo" with { bar: true }`.
#[test]
fn test_parse_import_expression_namespace_alias_with_arguments() {
    let mut test = TestParser::new("import * as baz from \"foo\" with { bar: true }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // import * as baz from foo with { bar: true }
    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: Some(arguments), .. } => {
        assert_eq!(*source, ImportSource::ImportStatement);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert_eq!(items.len(), 1);
        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_string!(parser, *alias, "baz");
        });
        // with { bar: true }
        assert_eq!(arguments.len(), 1);
    });
}

/// Reject `import { foo }` without a target.
#[test]
fn test_parse_import_expression_items_without_target_error() {
    let mut test = TestParser::new("import { foo }");
    let mut parser = test.prepare();
    assert!(parser.eat_expression(parser.options).is_err());
}

/// Parse mixed prefix and postfix increment/decrement operations.
#[test]
fn test_parse_mixed_prefix_and_postfix_increment_decrement() {
    let mut test = TestParser::new("(a++ + ++a) * (b-- - --b)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // (a++ + ++a) * (b-- - --b)
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {

        // (a++ + ++a)
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                // a++
                assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PostIncrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                });
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // ++a
                assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PreIncrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                });
            });
        });

        // *
        assert_eq!(*operator, BinaryOperator::Multiply);

        // (b-- - --b)
        assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                // b--
                assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PostDecrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                });
                // -
                assert_eq!(*operator, BinaryOperator::Subtract);
                // --b
                assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PreDecrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                });
            });
        });
    });
}

/// Parse `import("foo")`.
#[test]
fn test_parse_import_call_expression() {
    let mut test = TestParser::new("import(\"foo\")");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert!(items.is_empty());
    });
}

/// Parse `import("foo", { assert: { type: "json" } })`.
#[test]
fn test_parse_import_call_with_assertions() {
    let mut test = TestParser::new("import(\"foo\", { assert: { type: \"json\" } })");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: Some(arguments), .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert!(items.is_empty());
        assert_eq!(arguments.len(), 1);
    });
}

/// Parse dynamic import calls with non-literal targets.
#[test]
fn test_parse_import_call_with_expression_target() {
    let mut test = TestParser::new(r#"import(join("file://", process.argv[2]))"#);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert!(items.is_empty());
        assert_node!(target, ImportTarget::Expression { target } => {
            assert_node!(parser.tree, *target, Expression::Call { .. });
        });
    });
}

/// Parse an empty parenthesis as a tuple literal.
#[test]
fn test_parse_empty_parenthesis_tuple() {
    let mut test = TestParser::new("()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements, .. } => {
        assert_eq!(elements.len(), 0);
    });
}

/// Parse a tuple literal with two elements.
#[test]
fn test_parse_tuple_literal() {
    let mut test = TestParser::new("(1, 2)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::TupleExpression { elements, .. } => {
            assert_eq!(elements.len(), 2);
            // 1
            assert_node!(
                parser.tree,
                elements[0],
                Argument::Positional { modifiers: _, value } => {
                    assert_node!(
                        parser.tree,
                        *value,
                        Expression::ScalarLiteral(ScalarLiteral::Integer(1))
                    );
                }
            );
            // 2
            assert_node!(
                parser.tree,
                elements[1],
                Argument::Positional { modifiers: _, value } => {
                    assert_node!(
                        parser.tree,
                        *value,
                        Expression::ScalarLiteral(ScalarLiteral::Integer(2))
                    );
                }
            );
        }
    );
}

/// Parse a tuple literal over multiple lines.
#[test]
fn test_parse_tuple_literal_multiline() {
    let mut test = TestParser::new(
        r"
const shapes = (
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
    TetrisPieceShape.S,
)",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            // shapes
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "shapes");
            });
            // (...)
            assert_node!(parser.tree, value.unwrap(), Expression::TupleExpression { elements, .. } => {
                assert_eq!(elements.len(), 5);
                // TetrisPieceShape.I
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
                });
            });
        });
    });
}

/// Parse a range literal.
#[test]
fn test_parse_range_literal() {
    let mut test = TestParser::new("1..3");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::RangeExpression { start, end, .. } => {
        assert_node!(parser.tree, *start, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
            assert_eq!(*val, 1);
        });
        assert_node!(parser.tree, *end, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
            assert_eq!(*val, 3);
        });
    });
}

/// Parse an anonymous block.
#[test]
fn test_parse_anonymous_struct_literal() {
    let mut test = TestParser::new("{ }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block { .. });
}

/// Parse a statement-position object literal with a comment.
#[test]
fn test_parse_statement_position_object_literal_with_comment() {
    let mut test = TestParser::new("{ /* key */ a: 1 }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

/// Parse a statement-position object literal with a computed key.
#[test]
fn test_parse_statement_position_object_literal_computed_key() {
    let mut test = TestParser::new("{ [key]: value }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Expression(key_id)), value: Some(value_id), default: None, .. } => {
            assert_expression_path!(parser, parser.tree.get(*key_id), "key");
            assert_expression_path!(parser, parser.tree.get(*value_id), "value");
        });
    });
}

/// Parse a statement-position block with assignments.
#[test]
fn test_parse_statement_position_block_with_assignment() {
    let mut test = TestParser::new("{ step = step + 1; return base + step; }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 2);
            assert_node!(parser.tree, expressions[0], Expression::Statement(_));
            assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
                assert_node!(parser.tree, *statement_id, Expression::Return { .. });
            });
        });
    });
}

/// Parse a statement-position block with an array literal.
#[test]
fn test_parse_statement_position_block_with_array_literal() {
    let mut test = TestParser::new("{ [] }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 1);
            let expression_id = match parser.tree.get(expressions[0]) {
                Expression::Statement(statement_id) => *statement_id,
                _ => expressions[0],
            };
            assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
                assert!(elements.is_empty());
            });
        });
    });
}

/// Prefer a block over a computed method object literal in statement position.
#[test]
fn test_parse_statement_position_computed_method_as_block() {
    let mut test = TestParser::new("{ [key]()\n{} }");
    let mut parser = test.prepare();
    parser.options.in_statement_position = true;
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(_) => {});
}

/// Parse an anonymous block with a do disambiguation.
#[test]
fn test_parse_anonymous_block_with_do_disambiguation() {
    let mut test = TestParser::new("let x = do { }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_node!(parser.tree, value.unwrap(), Expression::Block { .. });
        });
    });
}

/// Parse an object literal in parenthesis.
#[test]
fn test_parse_object_literal_in_parenthesis() {
    let mut test = TestParser::new("({ x: 1, y })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, properties[1], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
                assert_string!(parser, *name, "y");
            });
        });
    });
}

/// Parse a ternary if expression.
#[test]
fn test_parse_if_ternary() {
    let mut test = TestParser::new("true ? 1 : 2");
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Parse a ternary if expression over multiple lines.
#[test]
fn test_parse_if_ternary_multiline() {
    let mut test = TestParser::new(
        r#"true
    ? 1
    : 2"#,
    );
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Parse a ternary if expression over multiple lines with comments.
#[test]
fn test_parse_if_ternary_multiline_with_comments() {
    let mut test = TestParser::new(
        r#"
 cond
    ? // comment
      a
    : // comment
      b"#,
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();

    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // cond
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
        // a
        assert_expression_path!(parser, parser.tree.get(*then_expression), "a");
        // b
        assert_expression_path!(parser, parser.tree.get(else_expression.unwrap()), "b");
    });
}

/// Parse a ternary if expression with parenthesis (disambiguate from call expression).
#[test]
fn test_parse_if_ternary_with_parenthesis() {
    let mut test = TestParser::new("x ? () : ()");
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
            assert_path!(parser, *path, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::TupleExpression { elements, .. } => {
            assert_eq!(elements.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::TupleExpression { elements, .. } => {
            assert_eq!(elements.len(), 0);
        });
    });
}

/// Parse a ternary if expression with brackets (disambiguate from index).
#[test]
fn test_parse_if_ternary_with_brackets() {
    let mut test = TestParser::new("x ? [] : []");
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
            assert_path!(parser, *path, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 0);
        });
    });
}

/// Parse a ternary if with braces (disambiguate from block).
#[test]
fn test_parse_if_ternary_with_braces() {
    let mut test = TestParser::new("x ? {} : {}");
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
            assert_path!(parser, *path, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 0);
        });
    });
}

/// Parse a ternary if expression with a binary condition.
#[test]
fn test_parse_if_ternary_with_binary_condition() {
    let mut test = TestParser::new("x == 0 ? 1 : 2");
    let mut parser = test.prepare();
    let if_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::Equal);
            assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
        assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Parse a mixed index postfix expression (should disambiguate ternary and index/call).
#[test]
fn test_parse_mixed_index_call_postfix() {
    let mut test = TestParser::new("x?.[f]?.y<T>?.().?");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // x?.[f]?.y<T>?.().?
    // .?
    assert_node!(parser.tree, expr_id, Expression::Maybe { left, position: PostfixPosition::Indirect } => {
        // ()
        assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 0);
            // ?
            assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                // .y
                assert_node!(parser.tree, *left, Expression::Member { left, name, static_arguments: Some(static_arguments) } => {
                    // y
                    assert_string!(parser, *name, "y");
                    // <T>
                    assert_eq!(static_arguments.len(), 1);
                    // ?
                    assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                        // .[f]
                        assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Indirect } => {
                            // f
                            assert_node!(parser.tree, index.unwrap(), Expression::Path { path, static_arguments } => {
                                assert!(static_arguments.is_none());
                                assert_path!(parser, *path, "f");
                            });
                            // ?
                            assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                                // x
                                assert_expression_path!(parser, parser.tree.get(*left), "x");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse optional chaining after comment-separated newlines.
#[test]
fn test_parse_optional_chain_after_comment_newlines() {
    let input = "promise\n  .then(noop)\n  // comment\n  // comment\n  ?.catch(noop)";
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(parser.errors.is_empty());
    assert_eq!(expressions.len(), 1);

    let statement_id = match parser.tree.get(expressions[0]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[0],
    };

    assert_node!(parser.tree, statement_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "noop");
        });

        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "catch");
            assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position: PostfixPosition::Direct } => {
                assert_node!(parser.tree, *maybe_left, Expression::Call { .. });
            });
        });
    });
}

#[test]
fn test_parse_instantiation_expression_with_index() {
    let mut test = TestParser::new_with_options("f[\"g\"]<number>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Instantiation { left, static_arguments } => {
        assert_eq!(static_arguments.len(), 1);
        assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
        });
        assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Direct } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert!(static_arguments.is_none());
                assert_path!(parser, *path, "f");
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

    assert_node!(parser.tree, expr_id, Expression::Instantiation { left, static_arguments } => {
        assert_eq!(static_arguments.len(), 1);
        assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
        });
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Instantiation { left, static_arguments } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "f");
                    assert!(static_arguments.is_none());
                });
            });
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
    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_eq!(expressions.len(), 2);

    let first_expression = match parser.tree.get(expressions[0]) {
        Expression::Statement(statement) => *statement,
        _ => expressions[0],
    };
    let second_expression = match parser.tree.get(expressions[1]) {
        Expression::Statement(statement) => *statement,
        _ => expressions[1],
    };

    assert_node!(parser.tree, first_expression, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { left, static_arguments } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "BaseAttributes");
                });
                assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "addSpanAttributes");
                    assert_eq!(dynamic_arguments.len(), 2);
                });
            });
        });
    });

    assert_node!(parser.tree, second_expression, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { static_arguments, .. } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "OperationAttributes");
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
        assert_node!(parser.tree, *left, Expression::Instantiation { left, static_arguments } => {
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert!(static_arguments.is_none());
                assert_path!(parser, *path, "f");
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
        assert_node!(parser.tree, *left, Expression::Instantiation { left, static_arguments } => {
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert!(static_arguments.is_none());
                assert_path!(parser, *path, "cls.myFunc");
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

    assert_node!(parser.tree, expr_id, Expression::Member { left, name, static_arguments } => {
        assert!(static_arguments.is_none());
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Instantiation { left, static_arguments } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert!(static_arguments.is_none());
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "f");
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
    assert_eq!(expressions.len(), 14);

    // f<x>, g<y>
    let sequence_id = match parser.tree.get(expressions[0]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[0],
    };
    assert_node!(parser.tree, sequence_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Instantiation { left, static_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert_eq!(static_arguments.len(), 1);
        });
        assert_node!(parser.tree, expressions[1], Expression::Instantiation { left, static_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "g");
            assert_eq!(static_arguments.len(), 1);
        });
    });

    // [f<x>]
    let array_id = match parser.tree.get(expressions[1]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[1],
    };
    assert_node!(parser.tree, array_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
            assert_node!(parser.tree, *value, Expression::Instantiation { left, static_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_eq!(static_arguments.len(), 1);
            });
        });
    });

    // f<x> ? g<y> : h<z>
    let ternary_id = match parser.tree.get(expressions[2]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[2],
    };
    assert_node!(parser.tree, ternary_id, Expression::If { kind, .. } => {
        assert_eq!(*kind, IfKind::Ternary);
    });

    // f<x> ?? g<y>
    let coalesce_id = match parser.tree.get(expressions[9]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[9],
    };
    assert_node!(parser.tree, coalesce_id, Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
    });

    // f<x> !== g<y>
    let strict_not_equal_id = match parser.tree.get(expressions[13]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => expressions[13],
    };
    assert_node!(parser.tree, strict_not_equal_id, Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::NotEqualStrict);
    });
}

/// Parse a TypeScript call with string literal type arguments.
#[test]
fn test_parse_call_with_string_literal_type_arguments() {
    let mut test = TestParser::new_with_options(
        "accessor.getValue<\"auto\" | \"always\" | \"never\">(\"long\")",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let mut static_args = static_arguments.as_ref();
        if static_args.is_none()
            && let Expression::Member { name, static_arguments: Some(member_args), .. } =
                parser.tree.get(*left)
            {
                assert_string!(parser, *name, "getValue");
                static_args = Some(member_args);
            }
        let static_args = static_args.expect("expected static arguments on call or member");
        assert_eq!(static_args.len(), 1);
        assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: _, value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
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
    parser.options.in_before_block = true;
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "fetchListResult");
            assert_eq!(dynamic_arguments.len(), 2);

            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "complianceConfig");
            });

            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "route");
            });

            let static_arguments = static_arguments.as_ref().expect("expected static arguments");
            assert_eq!(static_arguments.len(), 1);

            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ObjectExpression { ty: None, properties } => {
                    assert_eq!(properties.len(), 2);

                    assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                        assert_string!(parser, *name, "pattern");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });

                    assert_node!(parser.tree, properties[1], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                        assert_string!(parser, *name, "script");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                });
            });
        });
    });
}

/// Parse a TypeScript call with shift-left static arguments.
#[test]
fn test_parse_call_with_shift_left_static_arguments() {
    let mut test = TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(dynamic_arguments.is_empty());
        let static_args = static_arguments.as_ref().expect("expected static arguments");
        assert_eq!(static_args.len(), 1);
        assert_node!(parser.tree, static_args[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert!(body.is_none());
                });
            });
        });
    });
}

/// Parse shift-left static arguments in decorator context.
#[test]
fn test_parse_call_with_shift_left_static_arguments_in_decorator_context() {
    let mut test = TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let options = parser
        .options
        .not_in_position()
        .in_left_precedence(u16::MAX)
        .not_in_sequence_expression()
        .in_decorator();
    let expr_id = parser.eat_expression(options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "f");
        assert!(dynamic_arguments.is_empty());
        let static_args = static_arguments.as_ref().expect("expected static arguments");
        assert_eq!(static_args.len(), 1);
    });
}

/// Parse a TypeScript arrow function parameter named `accessor`.
#[test]
fn test_parse_arrow_parameter_accessor_name() {
    let mut test = TestParser::new_with_options(
        "(accessor: ServicesAccessor) => accessor.get()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "accessor");
                assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "ServicesAccessor");
            });
        });
    });
}

/// Parse a TypeScript class expression with implements.
#[test]
fn test_parse_class_expression_with_implements() {
    let mut test =
        TestParser::new_with_options("new (class implements Foo {})()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                    assert!(heritage.implements_types.is_some());
                });
            });
        });
    });
}

/// Parse a TypeScript class expression when heritage starts on the next line.
#[test]
fn test_parse_class_expression_with_newline_implements() {
    let mut test = TestParser::new_with_options(
        "new (class\n  implements Foo\n{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                    assert!(heritage.implements_types.is_some());
                });
            });
        });
    });
}

/// Parse a TypeScript class expression with multiline extends heritage.
#[test]
fn test_parse_class_expression_with_newline_extends() {
    let mut test = TestParser::new_with_options(
        "new (class\n  extends Foo<Bar>\n{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                    assert!(heritage.extends_types.is_some());
                });
            });
        });
    });
}

/// Parse an unparenthesized class expression as a new receiver.
#[test]
fn test_parse_new_unparenthesized_class_expression_with_extends() {
    let mut test = TestParser::new_with_options(
        "new class extends TestSession {}()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::New { left, dynamic_arguments, .. } => {
        assert!(dynamic_arguments.is_empty());
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, heritage, .. } => {
                assert!(descriptor.name.is_none());
                let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                assert_eq!(extends_types.len(), 1);
                assert_expression_path!(parser, parser.tree.get(extends_types[0]), "TestSession");
            });
        });
    });
}

/// Parse a JavaScript class expression with a parenthesized sequence extends target.
#[test]
fn test_parse_javascript_class_expression_with_parenthesized_sequence_extends() {
    let mut test =
        TestParser::new_with_options("var a = class extends (b,c) {};", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class { heritage, .. } => {
                    let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                    assert_eq!(extends_types.len(), 1);
                    assert_node!(parser.tree, extends_types[0], Expression::SequenceExpression { expressions } => {
                        assert_eq!(expressions.len(), 2);
                        assert_expression_path!(parser, parser.tree.get(expressions[0]), "b");
                        assert_expression_path!(parser, parser.tree.get(expressions[1]), "c");
                    });
                });
            });
        });
    });
}

/// Parse an object property value that is a named class expression.
#[test]
fn test_parse_object_property_named_class_expression_value() {
    let mut test = TestParser::new_with_options(
        "{ useClass: class MyExampleClass {} }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
            assert_string!(parser, *name, "useClass");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, members, .. } => {
                    assert_string!(parser, descriptor.name.unwrap().string(), "MyExampleClass");
                    assert!(members.is_empty());
                });
            });
        });
    });
}

/// Parse class expression values in decorator call arguments.
#[test]
fn test_parse_decorator_object_property_named_class_expression_value() {
    let mut test = TestParser::new_with_options(
        "Component({ useClass: class MyExampleClass {} })",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .with_options(parser.options.in_decorator(), |parser| {
            parser.eat_expression(parser.options)
        })
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                    assert_string!(parser, *name, "useClass");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, members, .. } => {
                            assert_string!(parser, descriptor.name.unwrap().string(), "MyExampleClass");
                            assert!(members.is_empty());
                        });
                    });
                });
            });
        });
    });
}

/// Parse a lambda function type with empty parameters.
#[test]
fn test_parse_lambda_function_empty_type() {
    let mut test = TestParser::new("() => void");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 0);
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
        });
    });
}

/// Parse a lambda function type with parameters and return type.
#[test]
fn test_parse_lambda_function_type() {
    let mut test = TestParser::new("(a: int32) => int32");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // (a: int32)
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
            // int32
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    });
}

/// Parse a lambda function value with a body.
#[test]
fn test_parse_lambda_function_value() {
    let mut test = TestParser::new("(a) => a > 2");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.return_type.is_none());
            assert!(body.is_some());
            assert_eq!(signature.dynamic_parameters.len(), 1);
            // (a)
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
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

/// Parse a generic lambda function value with a body.
#[test]
fn test_parse_generic_lambda_function_value() {
    let mut test = TestParser::new("<T,>(x: T): T => x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default: None, .. } => {
                assert_string!(parser, *name, "T");
            });
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "T");
            });
            assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    });
}

/// Parse a generic lambda function with a newline after `<`.
#[test]
fn test_parse_generic_lambda_function_value_multiline_after_less_than() {
    let mut test = TestParser::new_with_options(
        "<\nT extends string\n>(x: T) => x",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
            });
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    });
}

/// TSX generic arrows with extends constraints should parse as functions.
#[test]
fn test_parse_tsx_generic_arrow_with_extends() {
    let mut test = TestParser::new_with_options(
        "<P extends object>(x: P) => <Foo />",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "P");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Object));
            });
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "P");
                });
            });
            let body_id = body.expect("expected body");
            assert_node!(parser.tree, body_id, Expression::TreeExpression { left, arguments, elements } => {
                let left_id = left.expect("expected tag");
                assert_node!(parser.tree, left_id, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Foo");
                });
                assert!(arguments.as_ref().is_none_or(|items| items.is_empty()));
                assert!(elements.as_ref().is_none_or(|items| items.is_empty()));
            });
        });
    });
}

/// Parse map callbacks with parenthesized TSX element bodies.
#[test]
fn test_parse_tsx_parenthesized_tree_callback_body() {
    let mut test = TestParser::new_with_options(
        "items.map((item) => (<option>{item}</option>))",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);

    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "items.map");
        assert_eq!(dynamic_arguments.len(), 1);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.dynamic_parameters.len(), 1);

                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "item");
                    });

                    assert_node!(parser.tree, *body, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "option");
                            assert!(arguments.is_none());

                            let elements = elements.as_ref().expect("expected option children");
                            assert_eq!(elements.len(), 1);
                            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "item");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse TSX generic arrows without explicit disambiguators.
#[test]
fn test_parse_tsx_generic_arrow_without_disambiguator() {
    let mut test = TestParser::new_with_options("<R>(x: R) => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // ambiguous TSX generics should not parse without disambiguators
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse TSX generic arrows with trailing comma disambiguators.
#[test]
fn test_parse_tsx_generic_arrow_with_trailing_comma() {
    let mut test = TestParser::new_with_options("<T,>(x: T): T => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // parse a generic lambda with disambiguated type parameters
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default: None, .. } => {
                assert_string!(parser, *name, "T");
            });
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "T");
            });
            assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    });
}

/// Parse ternaries with typed arrow functions in TSX context.
#[test]
fn test_parse_tsx_ternary_typed_arrow_function() {
    let mut test = TestParser::new_with_options(
        "Math.random() > 0.5 ? (): void => foo() : (): void => bar()",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
        assert_eq!(*kind, IfKind::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
            });
        });
        let else_id = else_expression.expect("expected else branch");
        assert_node!(parser.tree, else_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
            });
        });
    });
}

/// Parse TSX tree attributes with typed arrow function values.
#[test]
fn test_parse_tsx_tree_attribute_typed_arrow_value() {
    let mut test = TestParser::new_with_options(
        "<StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} />",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::TreeExpression { arguments, .. } => {
        let arguments = arguments.as_ref().expect("expected arguments");
        let class_name_argument = arguments.iter().copied().find(|argument_id| {
            matches!(
                parser.tree.get(*argument_id),
                Argument::Named { name, .. } if parser.strings.get(name.string()) == "className"
            )
        });
        let class_name_argument = class_name_argument.expect("expected className argument");
        assert_node!(parser.tree, class_name_argument, Argument::Named { name, value, .. } => {
            assert_name!(parser, *name, "className");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    let return_type = signature.return_type.expect("expected return type");
                    assert_node!(parser.tree, return_type, Expression::ObjectExpression { .. });
                });
            });
        });
    });
}

/// Parse ternaries with typed arrow functions inside TSX tree attributes.
#[test]
fn test_parse_tsx_ternary_tree_attribute_typed_arrow() {
    let mut test = TestParser::new_with_options(
        "disabled ? <StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} /> : null",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { kind, then_expression, else_expression, .. } => {
        assert_eq!(*kind, IfKind::Ternary);
        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { arguments, .. } => {
            let arguments = arguments.as_ref().expect("expected arguments");
            let class_name_argument = arguments.iter().copied().find(|argument_id| {
                matches!(
                    parser.tree.get(*argument_id),
                    Argument::Named { name, .. } if parser.strings.get(name.string()) == "className"
                )
            });
            let class_name_argument = class_name_argument.expect("expected className argument");
            assert_node!(parser.tree, class_name_argument, Argument::Named { name, value, .. } => {
                assert_name!(parser, *name, "className");
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        let return_type = signature.return_type.expect("expected return type");
                        assert_node!(parser.tree, return_type, Expression::ObjectExpression { .. });
                    });
                });
            });
        });
        assert!(else_expression.is_some());
    });
}

/// Parse a lambda function value with a body and pattern parameters.
#[test]
fn test_parse_lambda_function_value_with_pattern_parameters() {
    let mut test = TestParser::new("(_, { x, y }: T) => a");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(_), .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.return_type.is_none());
            assert_eq!(signature.dynamic_parameters.len(), 2);
            // _
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Pattern { pattern, ty: None, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });
            // { x, y }: T
            assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Pattern { pattern, ty, .. } => {
                // { x, y }
                assert_node!(parser.tree, *pattern, Pattern::Object { fields, .. } => {
                    assert_eq!(fields.len(), 2);
                    // x
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                        assert_name!(parser, *name, "x");
                    });
                    // y
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                        assert_name!(parser, *name, "y");
                    });
                });
                // T
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
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
    let mut test = TestParser::new_with_options(
        "<T, N>(): [T, (action: N) => void] => {}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // <T, N>(): [T, (action: N) => void] => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            let return_type = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, return_type, Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });

                assert_node!(parser.tree, elements[1], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(function_id) => {
                        assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                            assert_eq!(signature.dynamic_parameters.len(), 1);
                            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                                assert_string!(parser, *name, "action");
                                assert_expression_path!(parser, parser.tree.get(*ty), "N");
                            });
                            assert_node!(parser.tree, signature.return_type.expect("expected nested return type"), Expression::TypeLiteral(TypeLiteral::Void));
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
    let mut test = TestParser::new_with_options(
        "<T>(fn: T): (value: T) => T => value => value",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // <T>(fn: T): (value: T) => T => value => value
    assert_node!(parser.tree, expr_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function { signature, body: Some(body), .. } => {
            // generic head
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert!(ty.is_none());
            });

            // fn: T parameter
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "fn");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });

            // return type: (value: T) => T
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::Declaration(return_function_id) => {
                assert_node!(parser.tree, *return_function_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                        assert_string!(parser, *name, "value");
                        assert_expression_path!(parser, parser.tree.get(*ty), "T");
                    });
                    assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected nested return type")), "T");
                });
            });

            // body: value => value
            assert_node!(parser.tree, *body, Expression::Declaration(body_function_id) => {
                assert_node!(parser.tree, *body_function_id, Declaration::Function { signature, body: Some(body), .. } => {
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
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
    let mut test = TestParser::new_with_options(
        "(b): b is FormField<unknown> & { focus: (options?: FocusOptions) => void } => b.focus !== undefined",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypePredicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("b")));
                assert_node!(parser.tree, target.expect("expected type predicate target"), Expression::Binary { operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                    assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                        assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                            assert_string!(parser, *name, "focus");
                            assert_node!(parser.tree, *value, Expression::Declaration(function_id) => {
                                assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
                                    assert_eq!(signature.dynamic_parameters.len(), 1);
                                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), .. } => {
                                        assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                                        assert_string!(parser, *name, "options");
                                        assert_expression_path!(parser, parser.tree.get(*ty), "FocusOptions");
                                    });
                                    assert_node!(parser.tree, signature.return_type.expect("expected function type return"), Expression::TypeLiteral(TypeLiteral::Void));
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
fn test_parse_static_parameter_constraint_object_property_named_in() {
    // source: <V extends { in: string }>() => {}
    let mut test = TestParser::new_with_options(
        "<V extends { in: string }>() => {}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // <V extends { in: string }>() => {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            let generics = signature.generics.as_ref().expect("expected generics");
            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);

            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "V");
                assert_node!(parser.tree, *ty, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { key: Some(key), value: Some(value), .. } => {
                        assert_node!(key, Key::Name(Name::Identifier(name)) => {
                            assert_string!(parser, *name, "in");
                        });
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
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
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.return_type.is_none());
            assert_eq!(signature.dynamic_parameters.len(), 1);
            // x
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // x
            assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    });
}

/// Parse a struct literal with a path type and two fields.
#[test]
fn test_parse_struct_literal_path() {
    let mut test = TestParser::new("geom.Vector2 { x: 1, y }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
            // geom.Vector2
            assert_node!(
                parser.tree,
                *ty,
                Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "geom.Vector2");
                }
            );
            assert_eq!(properties.len(), 2);
            // x: 1
            assert_node!(
                parser.tree,
                properties[0],
                Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(
                        parser.tree,
                        *value,
                        Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                            assert_eq!(*val, 1);
                        }
                    );
                }
            );
            // y
            assert_node!(
                parser.tree,
                properties[1],
                Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
                    assert_string!(parser, *name, "y");
                }
            );
        }
    );
}

/// Parse a struct literal with static parameters and two fields.
#[test]
fn test_parse_struct_literal_path_with_static_parameters() {
    let mut test = TestParser::new(
        r##"
geom.Mesh<2, 4> {
    vertices: [1, 2],
    y,
}"##,
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
            assert_node!(
                parser.tree,
                *ty,
                Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "geom.Mesh");
                    assert!(static_arguments.is_some());
                    let params = static_arguments.as_ref().unwrap();
                    assert_eq!(params.len(), 2);
                }
            );
            assert_eq!(properties.len(), 2);
            // vertices: [1, 2]
            assert_node!(
                parser.tree,
                properties[0],
                Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                    assert_string!(parser, *name, "vertices");
                    assert_node!(
                        parser.tree,
                        *value,
                        Expression::ArrayExpression { .. }
                    );
                }
            );
            // y
            assert_node!(
                parser.tree,
                properties[1],
                Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
                    assert_string!(parser, *name, "y");
                }
            );
        }
    );
}

/// Comparison operators should not be parsed as static arguments.
#[test]
fn test_parse_static_arguments_disambiguate_relational() {
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

    // fn(x < y, x < y, x >>= y)
    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(dynamic_arguments.len(), 3);

        // x < y
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        // x < y
        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        // x >>= y
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

    // fn(x < y, x < y, x >>>= y)
    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "fn");
        assert_eq!(dynamic_arguments.len(), 3);

        // x < y
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        // x < y
        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
        });

        // x >>>= y
        assert_node!(parser.tree, dynamic_arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Assign { operator, .. } => {
                assert_eq!(*operator, AssignOperator::UnsignedShiftRightAssign);
            });
        });
    });
}

/// Parse a let binding with a type with static parameters as value.
#[test]
fn test_parse_type_with_static_parameters() {
    let mut test = TestParser::new("let Alias = A<B<C>>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // let Alias = A<B<C>>
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            // Alias
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "Alias");
            });
            // A<B<C>>
            assert_node!(parser.tree, value.unwrap(), Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "A");
                assert!(static_arguments.is_some());
                // B<C>
                assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "B");
                        assert!(static_arguments.is_some());
                        // C
                        assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "C");
                        });
                    });
                });
            });
        });
    });
}

/// Parse a dereference expression.
#[test]
fn test_parse_dereference_variable() {
    let mut test = TestParser::new("*x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // *x
    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right, .. } => {
        assert_eq!(*operator, UnaryOperator::Dereference);
        // x
        assert_expression_path!(parser, parser.tree.get(*right), "x");
    });
}

/// Dereference should fail in JavaScript compatibility mode.
#[test]
fn test_dereference_fails_in_js_mode() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options("*x", options);
    let mut parser = test.prepare();
    // Should fail to parse *x as dereference in JS mode
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse a reference expression.
#[test]
fn test_parse_reference_variable() {
    let mut test = TestParser::new("&x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // &x
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ReferenceOf { mutability: Some(mutability), right, .. } => {
            assert_eq!(*mutability, Mutability::Mutable);
            // x
            assert_expression_path!(parser, parser.tree.get(*right), "x");
        }
    );
}

/// Parse a reference to a member call.
#[test]
fn test_parse_reference_member_call() {
    let mut test = TestParser::new("&self.foo()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // &self.foo()
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ReferenceOf { mutability: Some(Mutability::Mutable), right, .. } => {
            // self.foo()
            assert_node!(
                parser.tree,
                *right,
                Expression::Call { left, .. } => {
                    // self.foo
                    assert_node!(
                        parser.tree,
                        *left,
                        Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "self.foo");
                        }
                    );
                }
            );
        }
    );
}

/// Parse a bound reference expression.
#[test]
fn test_parse_bound_reference_expression() {
    let mut test = TestParser::new("&readonly super T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ReferenceOf { mutability: Some(mutability), variance, right, .. } => {
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(*variance, Some(VarianceBound::Super));
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Parse a value expression.
#[test]
fn test_parse_value_expression() {
    let mut test = TestParser::new("^super T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ValueOf { mutability, variance, right, .. } => {
        assert_eq!(*mutability, Some(Mutability::Mutable));
        assert_eq!(*variance, Some(VarianceBound::Super));
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Parse a new constructor call.
#[test]
fn test_parse_new_constructor_call() {
    let mut test = TestParser::new("new Foo()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { left, static_arguments, dynamic_arguments } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Foo");
        assert!(static_arguments.is_none());
        assert!(dynamic_arguments.is_empty());
    });
}

/// Parse a delete expression.
#[test]
fn test_parse_delete_expression() {
    let mut test = TestParser::new("delete foo.bar");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Delete { value } => {
        assert_expression_path!(parser, parser.tree.get(*value), "foo.bar");
    });
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
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // let x = foo.parse() + 2 + x
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Let { mutability, declarators, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(
                parser.tree,
                declarators[0],
                Declarator { pattern, value, .. } => {
                    // x
                    assert_node!(
                        parser.tree,
                        *pattern,
                        Pattern::Binding { name, .. } => {
                            assert_string!(parser, *name, "x");
                        }
                    );
                    // foo.parse() + 2 + x
                    assert_node!(
                        parser.tree,
                        value.unwrap(),
                        Expression::Binary { left, operator, right, .. } => {
                            assert_eq!(*operator, BinaryOperator::Add);
                            // foo.parse() + 2
                            assert_node!(
                                parser.tree,
                                *left,
                                Expression::Binary { left, operator, right, .. } => {
                                    assert_eq!(*operator, BinaryOperator::Add);
                                    // foo.parse()
                                    assert_node!(
                                        parser.tree,
                                        *left,
                                        Expression::Call { left, .. } => {
                                            // foo.parse
                                            assert_node!(
                                                parser.tree,
                                                *left,
                                                Expression::Path { path, .. } => {
                                                    assert_path!(parser, *path, "foo.parse");
                                                }
                                            );
                                        }
                                    );
                                    // 2
                                    assert_node!(
                                        parser.tree,
                                        *right,
                                        Expression::ScalarLiteral(ScalarLiteral::Integer(2))
                                    );
                                }
                            );
                            // x
                            assert_node!(
                                parser.tree,
                                *right,
                                Expression::Path { path, .. } => {
                                    assert_path!(parser, *path, "x");
                                }
                            );
                        }
                    );
                }
            );
        }
    );
}

/// Parse multi-line member and calls.
#[test]
fn test_parse_member_multiline() {
    let mut test = TestParser::new(
        r"
self
    .foo()
    .baz()
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // self.foo().baz()
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Call { left: baz_recv, .. } => {
            // self.foo()
            assert_node!(
                parser.tree,
                *baz_recv,
                Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "baz");
                    assert_node!(parser.tree, *left, Expression::Call { left: foo_recv, .. } => {
                        // self.foo
                        assert_expression_path!(parser, parser.tree.get(*foo_recv), "self.foo");
                    })
                }
            );
        }
    );
}

/// Parse boolean IdentifierName member access in JavaScript.
#[test]
fn test_parse_member_boolean_identifier_name() {
    // source: a.true
    let mut test = TestParser::new_with_options("a.true", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // a.true
    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "true");
    });
}

/// Parse default IdentifierName member access in JavaScript.
#[test]
fn test_parse_member_default_identifier_name_after_parenthesized_await_import() {
    let mut test = TestParser::new_with_options(
        r#"(await import(join("file://", process.argv[2]))).default"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // (await import(join("file://", process.argv[2]))).default
    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "default");
    });
}

/// Parse boolean IdentifierName property keys and accessors in JavaScript.
#[test]
fn test_parse_object_boolean_identifier_name_keys() {
    // source: { true: 1, false: 2, get true() {}, set false(value) {} }
    let mut test = TestParser::new_with_options(
        "{ true: 1, false: 2, get true() {}, set false(value) {} }",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // { true: 1, false: 2, get true() {}, set false(value) {} }
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 4);

        assert_node!(parser.tree, properties[0], Property::Field { key, value, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "true");
            });
            assert_node!(parser.tree, value.expect("expected field value"), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });

        assert_node!(parser.tree, properties[1], Property::Field { key, value, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "false");
            });
            assert_node!(parser.tree, value.expect("expected field value"), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });

        assert_node!(parser.tree, properties[2], Property::Method { key, signature, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "true");
            });
            assert_eq!(signature.mode, Some(destack_ast::FunctionMode::Getter));
        });

        assert_node!(parser.tree, properties[3], Property::Method { key, signature, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "false");
            });
            assert_eq!(signature.mode, Some(destack_ast::FunctionMode::Setter));
        });
    });
}

/// Parse regex literal in export default.
#[test]
fn test_parse_export_default_regex_literal() {
    // source: export default /foo/
    let mut test = TestParser::new_with_options("export default /foo/", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // export default /foo/
    assert_node!(parser.tree, expr_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
        });
    });
}

/// Parse regex literals inside template interpolation expressions.
#[test]
fn test_parse_tagged_template_with_regex_interpolation() {
    let mut test = TestParser::new_with_options("re`/^${/^$/}$/u`", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TaggedTemplateExpression { tag, value } => {
        assert_expression_path!(parser, parser.tree.get(*tag), "re");
        match value {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                assert_string!(parser, strings[0], "/^");
                assert_string!(parser, strings[1], "$/u");
                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                });
            }
            other => panic!("expected interpolated template, got {other:?}"),
        }
    });
}

/// Parse regex literal after assign with a newline.
#[test]
fn test_parse_regex_literal_after_assign_newline() {
    // source: var match =\n/^foo$/i.exec(str)
    let mut test =
        TestParser::new_with_options("var match =\n/^foo$/i.exec(str)", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // var match =\n/^foo$/i.exec(str)
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Call { left, dynamic_arguments, .. } => {
                assert_eq!(dynamic_arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "exec");
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after an arrow.
#[test]
fn test_parse_regex_literal_after_arrow() {
    // source: () => /^foo$/.test(value)
    let mut test =
        TestParser::new_with_options("() => /^foo$/.test(value)", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let body = body.expect("expected body");
            assert_node!(parser.tree, body, Expression::Call { left, dynamic_arguments, .. } => {
                assert_eq!(dynamic_arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "test");
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after unary not.
#[test]
fn test_parse_regex_literal_after_unary_not() {
    // source: !/[A-Z]/.test(k)
    let mut test = TestParser::new_with_options("!/[A-Z]/.test(k)", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // !/[A-Z]/.test(k)
    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Not);
        assert_node!(parser.tree, *right, Expression::Call { left, dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "test");
                assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
            });
        });
    });
}

/// Parse regex literal after coalesce assignment.
#[test]
fn test_parse_regex_literal_after_coalesce_assign() {
    // source: encoded ??= /[%+]/.test(url)
    let mut test =
        TestParser::new_with_options("encoded ??= /[%+]/.test(url)", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // encoded ??= /[%+]/.test(url)
    assert_node!(parser.tree, expr_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::CoalesceAssign);
        assert_expression_path!(parser, parser.tree.get(*left), "encoded");
        assert_node!(parser.tree, *right, Expression::Call { left, dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "test");
                assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
            });
        });
    });
}

/// Parse division after a TypeScript non-null assertion.
#[test]
fn test_parse_divide_after_typescript_non_null_assertion() {
    // source: x! / 2
    let mut test = TestParser::new_with_options("x! / 2", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // x! / 2
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Divide);
        assert_node!(parser.tree, *left, Expression::Must { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "x");
        });
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Parse regex literal with slash inside a character class.
#[test]
fn test_parse_regex_literal_with_character_class_slash() {
    // source: var a = /[\]/]/
    let mut test = TestParser::new_with_options("var a = /[\\]/]/", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // var a = /[\]/]/
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
        });
    });
}

/// Reject regex unicode escapes beyond the valid unicode scalar range.
#[test]
fn test_reject_regex_unicode_escape_out_of_range() {
    // source: /\u{110000}/u
    let mut test = TestParser::new_with_options("/\\u{110000}/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    assert_eq!(error.leaf_span().start, 0);
}

/// Reject unicode regex decimal escapes without matching capture groups.
#[test]
fn test_reject_regex_unicode_invalid_decimal_escape() {
    // source: /\1/u
    let mut test = TestParser::new_with_options("/\\1/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    assert_eq!(error.leaf_span().start, 0);
}

/// Reject unicode regex literals with lone quantifier opening braces.
#[test]
fn test_reject_regex_unicode_lone_opening_quantifier_brace() {
    // source: /{*/u
    let mut test = TestParser::new_with_options("/{*/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    assert_eq!(error.leaf_span().start, 0);
}

/// Reject unicode regex literals with invalid quantified lookaheads.
#[test]
fn test_reject_regex_unicode_quantified_lookahead() {
    // source: /(?!.){0,}?/u
    let mut test = TestParser::new_with_options("/(?!.){0,}?/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    assert_eq!(error.leaf_span().start, 0);
}

/// Reject unicode regex literals with lone quantifier closing braces.
#[test]
fn test_reject_regex_unicode_lone_closing_quantifier_brace() {
    // source: /}?/u
    let mut test = TestParser::new_with_options("/}?/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();

    assert_eq!(error.leaf_span().start, 0);
}

/// Parse unicode regex property escapes.
#[test]
fn test_parse_regex_unicode_property_escape() {
    // source: /\p{Emoji}/u
    let mut test = TestParser::new_with_options("/\\p{Emoji}/u", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::ScalarLiteral(ScalarLiteral::RegexString { .. })
    );
}

/// Parse regex unicode escapes with long leading-zero code point forms.
#[test]
fn test_parse_regex_unicode_escape_with_long_leading_zeros() {
    // source: /[\u{0000000000000061}-\u{7A}]/u
    let mut test = TestParser::new_with_options(
        "/[\\u{0000000000000061}-\\u{7A}]/u",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // /[\u{0000000000000061}-\u{7A}]/u
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ScalarLiteral(ScalarLiteral::RegexString { .. })
    );
}

/// Parse string literal with long leading-zero code point escapes.
#[test]
fn test_parse_string_unicode_escape_with_long_leading_zeros() {
    // source: "\u{00000000034}"
    let mut test = TestParser::new_with_options("\"\\u{00000000034}\"", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // "\u{00000000034}"
    assert_node!(parser.tree, expr_id, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
        assert_string!(parser, *string_id, "\\u{00000000034}");
    });
}

/// Parse a less-than comparison.
#[test]
fn test_parse_comparison_less_than() {
    let mut test = TestParser::new("x < y");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // x < y
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            // x
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            // y
            assert_expression_path!(parser, parser.tree.get(*right), "y");
        }
    );
}

/// Addition is left associative.
#[test]
fn test_parse_precedence_addition_left_associative() {
    let mut test = TestParser::new("a + b + c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b + c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    assert_eq!(*operator, BinaryOperator::Add);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Infix operators work across lines.
#[test]
fn test_parse_precedence_addition_across_lines() {
    let mut test = TestParser::new(
        r#"a +
 b +
 c"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b + c (across lines)
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    assert_eq!(*operator, BinaryOperator::Add);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Multiplication has higher precedence than addition.
#[test]
fn test_parse_precedence_multiply_before_addition() {
    let mut test = TestParser::new("a + b * c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b * c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");
            // b * c
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Multiply);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*left), "b");
                    // c
                    assert_expression_path!(parser, parser.tree.get(*right), "c");
                }
            );
        }
    );
}

/// Mixed precedence chain with addition and multiplication.
#[test]
fn test_parse_precedence_chain_mixed() {
    let mut test = TestParser::new("a + b * c + d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b * c + d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b * c
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b * c
                    assert_node!(
                        parser.tree,
                        *right,
                        Expression::Binary { left, operator, right, .. } => {
                            assert_eq!(*operator, BinaryOperator::Multiply);
                            // b
                            assert_expression_path!(parser, parser.tree.get(*left), "b");
                            // c
                            assert_expression_path!(parser, parser.tree.get(*right), "c");
                        }
                    );
                }
            );
            // d
            assert_expression_path!(parser, parser.tree.get(*right), "d");
        }
    );
}

/// Type casts bind to the left side before addition.
#[test]
fn test_parse_precedence_cast_before_addition() {
    let mut test = TestParser::new("a as number + b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a as number + b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a as number
            assert_node!(
                parser.tree,
                *left,
                Expression::TypeBinary { left, operator, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // number
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Parse a TypeScript angle bracket type assertion.
#[test]
fn test_parse_typescript_angle_type_assertion_expression() {
    let mut test = TestParser::new_with_options("<any>value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        assert_expression_path!(parser, parser.tree.get(*left), "value");
        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Any));
    });
}

/// Parse a TypeScript const assertion in angle bracket form.
#[test]
fn test_parse_typescript_angle_const_assertion_expression() {
    let mut test = TestParser::new_with_options("<const>[1, 2, 3]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
        assert_eq!(*operator, TypeUnaryOperator::AsConst);
        assert_node!(parser.tree, *right, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 3);
        });
    });
}

/// Type casts bind to the full addition expression on the left.
#[test]
fn test_parse_precedence_cast_after_addition() {
    let mut test = TestParser::new("a + b as number");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b as number
    assert_node!(
        parser.tree,
        expr_id,
        Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // number
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
        }
    );
}

/// Type casts bind to the left side before multiplication.
#[test]
fn test_parse_precedence_cast_before_multiply() {
    let mut test = TestParser::new("a as boolean * b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a as boolean * b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);
            // a as boolean
            assert_node!(
                parser.tree,
                *left,
                Expression::TypeBinary { left, operator, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // boolean
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Type casts bind to the full multiplication expression on the left.
#[test]
fn test_parse_precedence_cast_after_multiply() {
    let mut test = TestParser::new("a * b as boolean");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a * b as boolean
    assert_node!(
        parser.tree,
        expr_id,
        Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            // a * b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Multiply);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // boolean
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
        }
    );
}

/// Type casts bind tighter than comparisons.
#[test]
fn test_parse_precedence_cast_before_comparison() {
    let mut test = TestParser::new("a >= b as number");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a >= b as number
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");
            // b as number
            assert_node!(
                parser.tree,
                *right,
                Expression::TypeBinary { left, operator, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*left), "b");
                    // number
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                }
            );
        }
    );
}

/// Addition has higher precedence than elementwise or.
#[test]
fn test_parse_precedence_elementwise_vs_addition() {
    let mut test = TestParser::new("a + b | c + d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b | c + d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::ElementwiseOr);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c + d
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // c
                    assert_expression_path!(parser, parser.tree.get(*left), "c");
                    // d
                    assert_expression_path!(parser, parser.tree.get(*right), "d");
                }
            );
        }
    );
}

/// Comparison has higher precedence than logical and.
#[test]
fn test_parse_precedence_comparison_vs_logical() {
    let mut test = TestParser::new("a == b && c == d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a == b && c == d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
            // a == b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c == d
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // c
                    assert_expression_path!(parser, parser.tree.get(*left), "c");
                    // d
                    assert_expression_path!(parser, parser.tree.get(*right), "d");
                }
            );
        }
    );
}

/// Unary prefix has higher precedence than multiplication.
#[test]
fn test_parse_precedence_unary_before_multiply() {
    let mut test = TestParser::new("-a * b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // -a * b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);
            // -a
            assert_node!(
                parser.tree,
                *left,
                Expression::Unary { right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Unary operator spans point at the operator token.
#[test]
fn test_parse_unary_operator_span() {
    let mut test = TestParser::new("-value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary operator span");
    assert_eq!(parser.get_span_str(main_span), "-");
}

#[test]
fn test_parse_unary_postfix_operator_span() {
    let mut test = TestParser::new("value++");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::PostIncrement);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary postfix operator span");
    assert_eq!(parser.get_span_str(main_span), "++");
}

#[test]
fn test_parse_unary_keyword_operators() {
    let mut test = TestParser::new("typeof foo; void 0");
    let mut parser = test.prepare();

    let typeof_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, typeof_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Typeof);
        assert_expression_path!(parser, parser.tree.get(*right), "foo");
    });
    parser.eat_statement_stop_with_newlines().unwrap();

    let void_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, void_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Void);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
    });
}

/// Binary operator spans point at the operator token.
#[test]
fn test_parse_binary_operator_span() {
    let mut test = TestParser::new("left + right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
        assert_eq!(*operator, BinaryOperator::Add);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "+");
}

#[test]
fn test_parse_binary_operator_multichar_span() {
    let mut test = TestParser::new("left === right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
        assert_eq!(*operator, BinaryOperator::EqualStrict);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "===");
}

#[test]
fn test_parse_binary_operator_logical_span() {
    let mut test = TestParser::new("left && right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
        assert_eq!(*operator, BinaryOperator::And);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "&&");
}

#[test]
fn test_parse_binary_operator_coalesce_span() {
    let mut test = TestParser::new("left ?? right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "??");
}

#[test]
fn test_parse_assign_operator_span() {
    let mut test = TestParser::new("left += right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Assign { operator, left, right } => {
        assert_eq!(*operator, AssignOperator::AddAssign);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected assign operator span");
    assert_eq!(parser.get_span_str(main_span), "+=");
}

/// Postfix call has higher precedence than addition.
#[test]
fn test_parse_precedence_postfix_call_before_add() {
    let mut test = TestParser::new("a() + b() / c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a() + b() / c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a()
            assert_node!(
                parser.tree,
                *left,
                Expression::Call { left, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                }
            );
            // b() / c
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Divide);
                    // b()
                    assert_node!(
                        parser.tree,
                        *left,
                        Expression::Call { left, .. } => {
                            // b
                            assert_expression_path!(parser, parser.tree.get(*left), "b");
                        }
                    );
                    // c
                    assert_expression_path!(parser, parser.tree.get(*right), "c");
                }
            );
        }
    );
}

/// Combine postfix member access and call with coalesce.
#[test]
fn test_parse_precedence_postfix_call_before_coalesce() {
    let mut test = TestParser::new("y.sqrt() ?? 0");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // y.sqrt() ?? 0
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Coalesce);
            // y.sqrt()
            assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                // y.sqrt
                assert_expression_path!(parser, parser.tree.get(*left), "y.sqrt");
            });
            // 0
            assert_node!(
                parser.tree,
                *right,
                Expression::ScalarLiteral(ScalarLiteral::Integer(0))
            );
        }
    );
}

/// Parse type prefix operators and infer bindings.
#[test]
fn test_parse_type_unary_prefix_expression() {
    let mut test = TestParser::new("keyof typeof infer Value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // keyof typeof infer Value
    assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
        // keyof
        assert_eq!(*operator, TypeUnaryOperator::Keyof);
        assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
            // typeof
            assert_eq!(*operator, TypeUnaryOperator::Typeof);
            assert_node!(parser.tree, *right, Expression::TypeInfer { name, constraint } => {
                // infer
                assert_string!(parser, *name, "Value");
                assert!(constraint.is_none());
            });
        });
    });
}

/// Parse type unary postfix as const operation.
#[test]
fn test_parse_type_unary_postfix_as_const_expression() {
    let mut test = TestParser::new("Value as const");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // Value as const
    assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
        assert_eq!(*operator, TypeUnaryOperator::AsConst);
        assert_expression_path!(parser, parser.tree.get(*right), "Value");
    });
}

/// Parse type unary postfix as comptime operation.
#[test]
fn test_parse_type_unary_postfix_as_comptime_expression() {
    let mut test = TestParser::new("Value as comptime");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // Value as comptime
    assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
        assert_eq!(*operator, TypeUnaryOperator::AsComptime);
        assert_expression_path!(parser, parser.tree.get(*right), "Value");
    });
}

/// Parse comparisons against members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_member_access() {
    let mut test = TestParser::new_with_options("i > as.length", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // i > as.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_expression_path!(parser, parser.tree.get(*right), "as.length");
    });
}

/// Parse comparisons against members on an identifier named `satisfies`.
#[test]
fn test_parse_comparison_with_satisfies_identifier_member_access() {
    let mut test = TestParser::new_with_options("i > satisfies.length", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // i > satisfies.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_expression_path!(parser, parser.tree.get(*right), "satisfies.length");
    });
}

/// Parse comparisons against optional members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_optional_member_access() {
    let mut test = TestParser::new_with_options("i > as?.length", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // i > as?.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "length");
            assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                assert_expression_path!(parser, parser.tree.get(*left), "as");
            });
        });
    });
}

/// Parse typed arrow bodies that reference a parameter named `as`.
#[test]
fn test_parse_typed_arrow_body_with_as_parameter_member_access() {
    let mut test = TestParser::new_with_options(
        "(as: Array<number>) => i > as.length",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // (as: Array<number>) => i > as.length
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 1);

            // (as: Array<number>)
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "as");
                assert_node!(parser.tree, *ty, Expression::Path { path, static_arguments: Some(static_arguments) } => {
                    assert_path!(parser, *path, "Array");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });

            // i > as.length
            assert_node!(parser.tree, body.expect("expected body"), Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expression_path!(parser, parser.tree.get(*left), "i");
                assert_expression_path!(parser, parser.tree.get(*right), "as.length");
            });
        });
    });
}

/// Parse nested typed arrows with an `as` parameter used in a ternary condition.
#[test]
fn test_parse_typed_arrow_with_as_parameter_in_ternary_condition() {
    let source = r#"<A>(i: number, a: A) =>
  (as: Array<A>): Option<NonEmptyArray<A>> =>
    i < 0 || i > as.length ? _.none : _.some(unsafeInsertAt(i, a, as))"#;
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // outer typed lambda
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 2);

            // (i: number, a: A)
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "i");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Number));
            });
            assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "a");
                assert_expression_path!(parser, parser.tree.get(*ty), "A");
            });

            // inner typed lambda
            assert_node!(parser.tree, body.expect("expected body"), Expression::Declaration(inner_declaration_id) => {
                assert_node!(parser.tree, *inner_declaration_id, Declaration::Function { signature, body, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "as");
                    });

                    // i < 0 || i > as.length ? _.none : _.some(...)
                    assert_node!(parser.tree, body.expect("expected body"), Expression::If { condition, then_expression, else_expression, .. } => {
                        let condition_id = match condition {
                            IfCondition::Expression { condition } => *condition,
                            IfCondition::Let { .. } => panic!("expected expression condition"),
                        };
                        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                            assert_eq!(*operator, BinaryOperator::Or);
                            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::LessThan);
                                assert_expression_path!(parser, parser.tree.get(*left), "i");
                                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                            });
                            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::GreaterThan);
                                assert_expression_path!(parser, parser.tree.get(*left), "i");
                                assert_expression_path!(parser, parser.tree.get(*right), "as.length");
                            });
                        });

                        assert_expression_path!(parser, parser.tree.get(*then_expression), "_.none");
                        assert_node!(parser.tree, else_expression.expect("expected else expression"), Expression::Call { left, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "_.some");
                        });
                    });
                });
            });
        });
    });
}

/// Parse async identifiers with `as` casts.
#[test]
fn test_parse_async_as_cast() {
    let mut test = TestParser::new_with_options("async as any", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        assert_expression_path!(parser, parser.tree.get(*left), "async");
        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Any));
    });
}

/// Parse async arrows with a parameter named `as`.
#[test]
fn test_parse_async_arrow_with_as_parameter() {
    let mut test = TestParser::new_with_options("async as => {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "as");
            });
            assert!(body.is_some());
        });
    });
}

/// Parse async arrows with a newline before a return type annotation.
#[test]
fn test_parse_async_arrow_with_newline_before_return_type() {
    let mut test = TestParser::new_with_options("async (f)\n: t => { }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "f");
            });
            assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "t");
            assert!(body.is_some());
        });
    });
}

/// Parse async comparisons and generic calls without async function false positives.
#[test]
fn test_parse_async_generic_false_positive_in_typescript() {
    let mut test =
        TestParser::new_with_options("async < 1;\nasync<T>() == 0;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    let error_diagnostics: Vec<_> = parser
        .diagnostics
        .iter()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect();
    assert!(
        error_diagnostics.is_empty(),
        "unexpected parser diagnostics: {error_diagnostics:#?}"
    );
    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            assert_expression_path!(parser, parser.tree.get(*left), "async");
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::Equal);
            assert_node!(parser.tree, *left, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "async");
                assert!(dynamic_arguments.is_empty());

                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });
            });
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}
/// Parse `type as string` as a cast expression.
#[test]
fn test_parse_type_keyword_as_cast_expression() {
    let mut test = TestParser::new_with_options("type as string", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::String));
    });
}

#[test]
fn test_parse_module_identifier_as_cast_expression() {
    let mut test =
        TestParser::new_with_options("module as DynamicModule", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        assert_expression_path!(parser, parser.tree.get(*left), "module");
        assert_expression_path!(parser, parser.tree.get(*right), "DynamicModule");
    });
}

#[test]
fn test_parse_namespace_identifier_as_cast_call_argument() {
    let mut test = TestParser::new_with_options(
        "render(cloned, rootContainer, namespace as ElementNamespace)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "render");
        assert_eq!(dynamic_arguments.len(), 3);
        assert_node!(parser.tree, dynamic_arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeBinary { left, operator, right } => {
                assert_eq!(*operator, TypeBinaryOperator::Cast);
                assert_expression_path!(parser, parser.tree.get(*left), "namespace");
                assert_expression_path!(parser, parser.tree.get(*right), "ElementNamespace");
            });
        });
    });
}

#[test]
fn test_parse_cast_with_keyof_typeof_type_argument() {
    let mut test = TestParser::new_with_options(
        "Object.keys(touchedFields) as Array<keyof typeof touchedFields>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Object.keys");
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "touchedFields");
            });
        });
        assert_node!(parser.tree, *right, Expression::Path { path, static_arguments: Some(static_arguments) } => {
            assert_path!(parser, *path, "Array");
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeUnary { operator, right } => {
                    assert_eq!(*operator, TypeUnaryOperator::Keyof);
                    assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
                        assert_eq!(*operator, TypeUnaryOperator::Typeof);
                        assert_expression_path!(parser, parser.tree.get(*right), "touchedFields");
                    });
                });
            });
        });
    });
}

/// Parse casts whose type target is a conditional type.
#[test]
fn test_parse_cast_with_conditional_type_target() {
    let mut test = TestParser::new_with_options(
        "value as Flag extends true ? Selected : Rejected",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // value as ...
    assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
        assert_eq!(*operator, TypeBinaryOperator::Cast);
        // value
        assert_expression_path!(parser, parser.tree.get(*left), "value");
        // Flag extends true ? Selected : Rejected
        assert_node!(parser.tree, *right, Expression::TypeConditional { left, right, then_type, else_type } => {
            // Flag
            assert_expression_path!(parser, parser.tree.get(*left), "Flag");
            // true
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // Selected
            assert_expression_path!(parser, parser.tree.get(*then_type), "Selected");
            // Rejected
            assert_expression_path!(parser, parser.tree.get(*else_type), "Rejected");
        });
    });
}

/// Parse ternary expressions after an `as` cast.
#[test]
fn test_parse_cast_followed_by_ternary_expression() {
    let mut test = TestParser::new_with_options(
        "perFileCache === resolvedModuleNames as unknown ? resolved : fallback",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // perFileCache === (resolvedModuleNames as unknown) ? resolved : fallback
    assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
        assert_eq!(*kind, IfKind::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::EqualStrict);
                assert_expression_path!(parser, parser.tree.get(*left), "perFileCache");
                assert_node!(parser.tree, *right, Expression::TypeBinary { operator, left, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                    assert_expression_path!(parser, parser.tree.get(*left), "resolvedModuleNames");
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Unknown));
                });
            });
        });
        assert_expression_path!(parser, parser.tree.get(*then_expression), "resolved");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "fallback");
    });
}

/// Parse ternary expressions after `satisfies`.
#[test]
fn test_parse_satisfies_followed_by_ternary_expression() {
    let mut test = TestParser::new_with_options(
        "value satisfies SomeType ? yes : no",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
        assert_eq!(*kind, IfKind::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::TypeBinary { operator, left, right } => {
                assert_eq!(*operator, TypeBinaryOperator::Satisfies);
                assert_expression_path!(parser, parser.tree.get(*left), "value");
                assert_expression_path!(parser, parser.tree.get(*right), "SomeType");
            });
        });
        assert_expression_path!(parser, parser.tree.get(*then_expression), "yes");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "no");
    });
}

#[test]
fn test_parse_parenthesized_cast_followed_by_flat_map_call() {
    let mut test = TestParser::new_with_options(
        "(Object.keys(touchedFields) as Array<keyof typeof touchedFields>).flatMap((topLevelKey) => topLevelKey)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "flatMap");
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::TypeBinary { operator, .. } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                });
            });
        });
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                });
            });
        });
    });
}

/// Parse relational arrow values without swallowing following object properties.
#[test]
fn test_parse_call_argument_object_relational_arrow_then_typed_block_arrow() {
    let mut test = TestParser::new_with_options(
        r#"morgan({
  skip: (req, res) => res.statusCode < 400,
  write: (str: string) => {
    str;
  },
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "morgan");
        assert_eq!(dynamic_arguments.len(), 1);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                // skip: (req, res) => res.statusCode < 400
                assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                    assert_string!(parser, *name, "skip");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                            assert_eq!(signature.dynamic_parameters.len(), 2);
                            assert_node!(parser.tree, *body, Expression::Binary { operator, .. } => {
                                assert_eq!(*operator, BinaryOperator::LessThan);
                            });
                        });
                    });
                });

                // write: (str: string) => { str }
                assert_node!(parser.tree, properties[1], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                    assert_string!(parser, *name, "write");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                            assert_eq!(signature.dynamic_parameters.len(), 1);
                            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                                assert_string!(parser, *name, "str");
                                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
                            });
                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::Statement(statement) => {
                                        assert_expression_path!(parser, parser.tree.get(*statement), "str");
                                    });
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
fn test_parse_function_parameter_readonly_tuple_type_annotation() {
    let mut test = TestParser::new_with_options(
        r#"function flattenPairs(pair: readonly [string, number], acc: Array<string | number>): Array<string | number> {
  return acc.concat(pair);
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 2);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: Some(ty), .. } => {
                assert_string!(parser, *name, "pair");
                assert_node!(parser.tree, *ty, Expression::TypeUnary { operator, right } => {
                    assert_eq!(*operator, TypeUnaryOperator::Readonly);
                    assert_node!(parser.tree, *right, Expression::ArrayExpression { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

/// Parse function expressions in decorator call arguments.
#[test]
fn test_parse_decorator_call_with_function_expression_argument() {
    let mut test = TestParser::new_with_options(
        r#"computed("fullName", function(this: Foo) {
  return this.fullName.toUpperCase();
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .with_options(parser.options.in_decorator(), |parser| {
            parser.eat_expression(parser.options)
        })
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "computed");
        assert_eq!(dynamic_arguments.len(), 2);

        assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(body), .. } => {
                    assert!(signature.this_parameter.is_some());

                    assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                            assert_eq!(expressions.len(), 1);
                        });
                    });
                });
            });
        });
    });
}

/// Parse new class expressions with generic implements clauses.
#[test]
fn test_parse_new_class_expression_with_generic_implements_clause() {
    let mut test = TestParser::new_with_options(
        r#"new class implements Iterable<string> {
  *[Symbol.iterator]() {
    yield "value";
  }
}()"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 0);
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, members, .. } => {
                let implements_types = heritage.implements_types.as_ref().expect("expected implements types");
                assert_eq!(implements_types.len(), 1);
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.cardinality, destack_ast::FunctionCardinality::Generator);
                });
            });
        });
    });
}

/// Parse type aliases named `as` and `satisfies`.
#[test]
fn test_parse_type_alias_named_as_or_satisfies() {
    let mut test = TestParser::new_with_options(
        "type as = 0;\ntype satisfies = 0;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "as");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "satisfies");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    });
}

/// Reject angle bracket assertions in disallow ambiguous mode.
#[test]
fn test_reject_type_assertion_when_disallow_ambiguous_tree_literal() {
    let mut test = TestParser::new_with_options("<T>x", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.options.disallow_ambiguous_tree_literal = true;

    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Reject ambiguous generic arrows in disallow ambiguous mode.
#[test]
fn test_reject_generic_arrow_when_disallow_ambiguous_tree_literal() {
    let mut test = TestParser::new_with_options("<T>() => 1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.options.disallow_ambiguous_tree_literal = true;

    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse `new` calls with generic receivers and const assertion arguments.
#[test]
fn test_parse_new_expression_with_generic_receiver_and_const_assertion_argument() {
    let mut test = TestParser::new_with_options(
        "new Set<keyof A | keyof B>(<const>[\"connect\"])",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments: Some(static_arguments), dynamic_arguments } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Set");
        assert_eq!(static_arguments.len(), 1);
        assert_eq!(dynamic_arguments.len(), 1);

        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeUnary { operator, right } => {
                assert_eq!(*operator, TypeUnaryOperator::AsConst);
                assert_node!(parser.tree, *right, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                });
            });
        });
    });
}

/// Reject angle bracket assertions in `new` receivers.
#[test]
fn test_reject_type_assertion_in_new_receiver() {
    let mut test = TestParser::new_with_options("new <any>Test2();", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse await parenthesized `new` with generic receiver and `void` type argument.
#[test]
fn test_parse_await_parenthesized_new_expression_with_void_type_argument() {
    let mut test = TestParser::new_with_options(
        "await (new Promise<void>(resolve => setTimeout(() => resolve(), delay)))",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // await (new Promise<void>(...))
    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        assert_node!(parser.tree, *expression, Expression::Parenthesized { expression: parenthesized_expression } => {
            assert_node!(parser.tree, *parenthesized_expression, Expression::New { left, static_arguments: Some(static_arguments), dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "Promise");
                assert_eq!(static_arguments.len(), 1);
                assert_eq!(dynamic_arguments.len(), 1);
            });
        });
    });
}

/// Parse `type instanceof Foo` as a binary expression.
#[test]
fn test_parse_type_keyword_instanceof_expression() {
    let mut test = TestParser::new_with_options("type instanceof Foo", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::InstanceOf);
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_expression_path!(parser, parser.tree.get(*right), "Foo");
    });
}

/// Parse `extension` as an identifier in TypeScript expressions.
#[test]
fn test_parse_extension_identifier_in_typescript_ternary_expression() {
    let mut test = TestParser::new_with_options(
        r#"typeof extension === "function" ? extension(cloned) : extension"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
        assert_eq!(*kind, IfKind::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::EqualStrict);
                assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::Typeof);
                    assert_expression_path!(parser, parser.tree.get(*right), "extension");
                });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                    assert_string!(parser, *string, "function");
                });
            });
        });

        assert_node!(parser.tree, *then_expression, Expression::Call { left, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "extension");
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "cloned");
            });
        });

        let else_expression_id = else_expression.expect("expected ternary else expression");
        assert_expression_path!(parser, parser.tree.get(else_expression_id), "extension");
    });
}

/// Parse a type asserts expression.
#[test]
fn test_parse_type_unary_postfix_asserts_expression() {
    let mut test = TestParser::new(
        r"
function isStringy(value: any): asserts value is string {
    // ...
}
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();

    let expr_id = parser.eat_expression(parser.options).unwrap();
    // function isStringy(value: any): asserts value is string { .. }
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { descriptor, signature, .. } => {
            // isStringy
            assert_string!(parser, descriptor.name.unwrap().string(), "isStringy");
            assert_eq!(signature.dynamic_parameters.len(), 1);
            // value: any
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
            });
            // asserts value is string
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypePredicate { asserts, subject, target } => {
                // asserts value is string
                assert!(*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                assert_node!(parser.tree, target.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
            });

            let predicate_id = signature.return_type.unwrap();
            let main_span = parser
                .tree
                .get_main_span(predicate_id)
                .expect("expected predicate main span");
            assert_eq!(parser.get_span_str(main_span), "value");
        });
    });
}

#[test]
fn test_parse_type_predicate_in_before_block_context() {
    let mut test = TestParser::new_with_options(
        "module is DynamicModule { value: true }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .with_options(parser.options.in_type().in_before_block(), |parser| {
            parser.eat_expression(parser.options)
        })
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::TypePredicate { asserts, subject, target } => {
        assert!(!asserts);
        assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("module")));
        assert_expression_path!(parser, parser.tree.get(target.unwrap()), "DynamicModule");
    });
}

/// Parse a predicate return type with a parenthesized union target.
#[test]
fn test_parse_arrow_return_type_predicate_with_parenthesized_union_target() {
    let mut test = TestParser::new_with_options(
        "(item): item is (IChatRequestViewModel | IChatResponseViewModel) => isRequestVM(item)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypePredicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("item")));
                assert_node!(parser.tree, target.expect("expected predicate target"), Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    });
                });
            });
        });
    });
}

/// Parse a predicate return type with a parenthesized intersection target.
#[test]
fn test_parse_arrow_return_type_predicate_with_parenthesized_intersection_target() {
    let mut test = TestParser::new_with_options(
        "(p: unknown): p is (TentativeBoundary & { inner: CharacterPrediction }) => p instanceof TentativeBoundary",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function { signature, .. } => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypePredicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("p")));
                assert_node!(parser.tree, target.expect("expected predicate target"), Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                    });
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
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Labelled { label, .. } => {
        assert_string!(parser, *label, "label");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected label main span");
    assert_eq!(parser.get_span_str(main_span), "label");
}

/// Parse labelled statements when the target statement starts on a new line.
#[test]
fn test_parse_labelled_statement_with_newline_before_target() {
    let mut test = TestParser::new("outer:\nwhile (true) {}");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Labelled { label, body } => {
        assert_string!(parser, *label, "outer");
        assert_node!(parser.tree, *body, Expression::While { .. });
    });
}

/// Reject labelled lexical declarations in javascript.
#[test]
fn test_reject_labelled_lexical_declaration_javascript() {
    // source: a: let a
    let mut test = TestParser::new_with_options("a: let a", LanguageType::JavaScript);
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
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // type Value = | string | number | boolean
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, .. }, value, .. } => {
            // value
            assert_string!(parser, name.unwrap().string(), "Value");
            // | string | number | boolean
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                // string | number
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    // string
                    assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::String));
                    // number
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                });
                // boolean
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
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
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // type Target = | "bun" | "node" | "browser"
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, .. }, value, .. } => {
            assert_string!(parser, name.unwrap().string(), "Target");
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::String(bun_id)) => {
                        assert_string!(parser, *bun_id, "bun");
                    });
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(node_id)) => {
                        assert_string!(parser, *node_id, "node");
                    });
                });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(browser_id)) => {
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
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();
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

/// Parse a statement expression.
#[test]
fn test_parse_statement_expression() {
    let mut test = TestParser::new("a;");
    let mut parser = test.prepare();
    let expr_id = parser.try_eat_statement_expression().unwrap();
    // a;
    assert_node!(parser.tree, expr_id, Expression::Statement(expression_id) => {
        assert_expression_path!(parser, parser.tree.get(*expression_id), "a");
    });
}

#[test]
fn test_parse_new_without_parenthesized_type_arguments_in_statement() {
    let mut test = TestParser::new_with_options("new A < T;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.try_eat_statement_expression().unwrap();

    assert_node!(parser.tree, expression_id, Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            assert_node!(parser.tree, *left, Expression::New { left, static_arguments, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert!(static_arguments.is_none());
                assert!(dynamic_arguments.is_empty());
            });
            assert_expression_path!(parser, parser.tree.get(*right), "T");
        });
    });
}

/// Comma in parentheses parses as sequence expression.
#[test]
fn test_parse_sequence_expression() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options("(a, b, c)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // (a, b, c)
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 3);
        // a
        assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
        // b
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
        // c
        assert_expression_path!(parser, parser.tree.get(expressions[2]), "c");
    });
}

/// Comma operator parses as sequence expression in JS/TS.
#[test]
fn test_parse_sequence_expression_without_parens() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("a, b", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a, b
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        // a
        assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
        // b
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
    });
}

/// Sequence expressions should parse inside lambda block bodies in TS.
#[test]
fn test_parse_sequence_expression_in_lambda_block_body() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        "() => { (lastIndex = history.state?.index), (lastY = scrollY), (lastX = scrollX); }",
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // () => { ... }
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body: Some(body), .. } => {
            // { ... }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.expressions.len(), 1);

                // ((lastIndex = ...), (lastY = ...), (lastX = ...));
                assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
                    assert_node!(parser.tree, *statement_id, Expression::SequenceExpression { expressions } => {
                        assert_eq!(expressions.len(), 3);
                        assert_node!(parser.tree, expressions[0], Expression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, Expression::Assign { .. });
                        });
                        assert_node!(parser.tree, expressions[1], Expression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, Expression::Assign { .. });
                        });
                        assert_node!(parser.tree, expressions[2], Expression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, Expression::Assign { .. });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_sequence_expression_with_ternary_tail() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("a && (b = 1, c = 2), d ? e : f", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a && (b = 1, c = 2), d ? e : f
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
        });
    });
}

#[test]
fn test_parse_sequence_expression_with_nested_ternary() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        "l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)",
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
        });
    });
}

/// Parse multiline logical chains with comment-only lines between operators.
#[test]
fn test_parse_multiline_logical_chain_after_comment_lines() {
    let options = LanguageType::TypeScript;
    let mut test =
        TestParser::new_with_options("a == 1\n// keep chaining\n&& b == 0\n&& c == 1", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
fn test_parse_export_const_ternary_object_literal_arrow_value() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        r#"export const reproValue = true ? {} : {
    reproFunc: (_: any): any => { },
};"#,
        options,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Let { descriptor, declarators, .. } => {
        assert_eq!(descriptor.export, Some(DependencyMode::Item));
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern, .. } => {
                assert_string!(parser, *name, "reproValue");
                assert!(pattern.is_none());
            });
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::If { kind, then_expression, else_expression, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
                assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                    assert!(properties.is_empty());
                });
                let else_expression = else_expression.expect("expected else branch");
                assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { key, value, default, .. } => {
                        assert!(default.is_none());
                        assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                            assert_string!(parser, *name, "reproFunc");
                        });
                        let value_id = value.expect("expected property value");
                        assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                assert_eq!(signature.kind, FunctionKind::Lambda);
                                let body_id = body.expect("expected function body");
                                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                        assert!(expressions.is_empty());
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_export_const_type_identifier_with_struct_value() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("export const type = struct", options);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Let { descriptor, declarators, .. } => {
        assert_eq!(descriptor.export, Some(DependencyMode::Item));
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

#[test]
fn test_parse_ternary_object_literal_arrow_value_expression() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        r#"true ? {} : {
    reproFunc: (_: any): any => { },
}"#,
        options,
    );
    let mut parser = test.prepare();
    let result = parser.with_options(
        parser
            .options
            .not_in_position()
            .not_in_sequence_expression(),
        |parser| parser.eat_expression(parser.options),
    );
    match result {
        Ok(expr_id) => {
            assert_node!(parser.tree, expr_id, Expression::If { kind, then_expression, else_expression, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
                assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                    assert!(properties.is_empty());
                });
                let else_expression = else_expression.expect("expected else branch");
                assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { key, value, default, .. } => {
                        assert!(default.is_none());
                        assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                            assert_string!(parser, *name, "reproFunc");
                        });
                        let value_id = value.expect("expected property value");
                        assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                assert_eq!(signature.kind, FunctionKind::Lambda);
                                let body_id = body.expect("expected function body");
                                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                        assert!(expressions.is_empty());
                                    });
                                });
                            });
                        });
                    });
                });
            });
        }
        Err(err) => panic!("unexpected error: {err:?}"),
    }
}

#[test]
fn test_parse_object_literal_with_typed_arrow_value() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        r#"{
    reproFunc: (_: any): any => { },
}"#,
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
            assert_string!(parser, *name, "reproFunc");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(_), .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                });
            });
        });
    });
}

/// Comma in parentheses parses as tuple expression.
#[test]
fn test_parse_tuple_expression() {
    let options = LanguageType::Destack;
    let mut test = TestParser::new_with_options("(a, b, c)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // (a, b, c)
    assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements } => {
        assert_eq!(elements.len(), 3);
        // a
        assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "a");
        });
        // b
        assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "b");
        });
        // c
        assert_node!(parser.tree, elements[2], Argument::Positional { modifiers: _, value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "c");
        });
    });
}

/// Sequence expression with unary void.
#[test]
fn test_parse_sequence_expression_with_unary_void() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options("(a, void 0, 1)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // (a, void 0, 1)
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 3);
        // a
        assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
        // void 0
        assert_node!(parser.tree, expressions[1], Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Void);
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
        // 1
        assert_node!(parser.tree, expressions[2], Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

/// Test const enum declaration.
#[test]
fn test_parse_const_enum() {
    let mut test = TestParser::new("const enum Foo { A, B }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // const enum Foo { A, B }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Enum { descriptor, kind, fields, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
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

#[test]
fn test_parse_arrow_body_with_anonymous_class_expression() {
    let mut test = TestParser::new_with_options(
        r###"<P extends Props>(
  wrapped: ComponentType<P>
) => class extends Component<Omit<P, keyof A> & Partial<B>, C> {
  static displayName = `x`;
}"###,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // lambda with class expression body
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function { signature, body: Some(body), .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);

            // anonymous class extends generic component
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class { descriptor, heritage, members, .. } => {
                    assert!(descriptor.name.is_none());
                    assert_eq!(members.len(), 1);

                    let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                    assert_eq!(extends_types.len(), 1);

                    // Component<Omit<...>, C>
                    assert_node!(parser.tree, extends_types[0], Expression::Instantiation { left, static_arguments } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Component");
                        assert_eq!(static_arguments.len(), 2);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_arrow_body_with_multiline_class_heritage_static_arguments() {
    let mut test = TestParser::new_with_options(
        r###"<P extends Props>(
  wrapped: React.ComponentType<P>
) => class extends React.Component<
  Omit<P, keyof Props> & Partial<Props>,
  Props
> {
  static displayName = `x`;
}"###,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // lambda with multiline class heritage static arguments
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function { signature, body: Some(body), .. } => {
            assert_eq!(signature.kind, FunctionKind::Lambda);

            // class extends React.Component<...>
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class { heritage, .. } => {
                    let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                    assert_eq!(extends_types.len(), 1);

                    // React.Component<Omit<...>, Props>
                    assert_node!(parser.tree, extends_types[0], Expression::Instantiation { left, static_arguments } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "React.Component");
                        assert_eq!(static_arguments.len(), 2);
                    });
                });
            });
        });
    });
}
