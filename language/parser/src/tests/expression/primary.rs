use std::sync::Arc;

use crate::tests::*;
use crate::{
    Parser, ParserOptions, assert_expression_path, assert_node, assert_path,
    assert_qualified_reference_path, assert_string, assert_value_expression_path,
};
use destack_ast::*;
use destack_core::StringPool;
use destack_source::{LanguageType, NodeSpanBoundary, NodeSpanType};

/// Parse import meta as one dedicated expression root.
#[test]
fn test_parse_import_meta_expression() {
    let mut test = TestParser::new("import.meta.env");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name } => {
        assert_string!(parser, name.expect("expected member name"), "env");
        assert_node!(parser.tree, *left, Expression::ImportMeta);
    });
}

/// Parse new target as one dedicated expression root.
#[test]
fn test_parse_new_target_expression() {
    let mut test = TestParser::new("new.target.member");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name } => {
        assert_string!(parser, name.expect("expected member name"), "member");
        assert_node!(parser.tree, *left, Expression::NewTarget);
    });
}

/// Disambiguate import source phase access as a path.
#[test]
fn test_parse_import_source_as_path() {
    let mut test = TestParser::new_with_language("import.source", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_qualified_reference_path!(parser, parser.tree.get(expression_id), "import.source");
}

/// Parse import source phase calls as member calls.
#[test]
fn test_parse_import_source_call_expression() {
    let mut test = TestParser::new_with_language(
        r#"import.source("data:text/javascript,console.log(1)")"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_qualified_reference_path!(parser, parser.tree.get(*left), "import.source");
    });
}

/// Parse import source phase calls with expression arguments.
#[test]
fn test_parse_import_source_call_expression_with_template_argument() {
    let mut test = TestParser::new_with_language(
        "import.source(String.raw`data:text/javascript,console.log(1)`)",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_qualified_reference_path!(parser, parser.tree.get(*left), "import.source");
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TaggedTemplateExpression { .. });
        });
    });
}

/// Parse a bare this expression.
#[test]
fn test_parse_this_expression() {
    let mut test = TestParser::new("this");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::This);
}

/// Parse a bare identifier as an identifier expression.
#[test]
fn test_parse_identifier_expression_as_identifier() {
    let mut test = TestParser::new("value");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Identifier { name } => {
        assert_string!(parser, *name, "value");
    });
    assert_value_expression_path!(parser, parser.tree.get(expression_id), "value");
}

/// Parse contextual type literal names as values before member access.
#[test]
fn test_parse_contextual_type_literal_name_member_expression() {
    let mut test = TestParser::new("object.property");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name } => {
        assert_expression_path!(parser, parser.tree.get(*left), "object");
        assert_string!(parser, name.expect("expected member name"), "property");
    });

    test.assert_no_errors(&parser);
}

/// Parse a type unary reference as a qualified reference.
#[test]
fn test_parse_type_unary_qualified_reference_as_qualified_reference() {
    let mut test = TestParser::new("type Foo.Bar");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
            assert!(generic_arguments.is_empty());
            assert_path!(parser, *path, "Foo.Bar");
            assert_qualified_reference_path!(parser, parser.tree.get(*value), "Foo.Bar");

            let head_span = parser
                .tree
                .get_head_span(*value)
                .expect("expected qualified reference head span");
            assert_eq!(parser.get_span_str(head_span), "Foo");
        });
    });
}

/// Parse a bare super expression.
#[test]
fn test_parse_super_expression() {
    let mut test = TestParser::new_with_language("super", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Super);
}

/// Parse a bare null literal in TypeScript.
#[test]
fn test_parse_null_literal() {
    let mut test = TestParser::new_with_language("null", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::ScalarLiteral(ScalarLiteral::Null)
    );
}

/// Parse parenthesized expressions with a transparent head span.
#[test]
fn test_parse_parenthesized_expression_records_semantic_head_span() {
    let mut test = TestParser::new_with_language("(value)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        let head_span = parser
            .tree
            .get_head_span(expression_id)
            .expect("missing parenthesized head span");
        let value_head_span = parser.expression_head_span(*expression);

        assert_eq!(head_span, value_head_span);
        assert_eq!(parser.get_span_str(head_span), "value");
    });
}

/// Keep inner spans without parenthesized expression wrappers.
#[test]
fn test_parse_without_parenthesized_wrappers_keeps_inner_expression_span() {
    let test = TestParser::new_with_language("(/* keep */ value)", LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_options(
        test.file.clone(),
        test.language,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Identifier { .. });

    let expression_span = parser.tree.get_span(expression_id);
    let leading_span = parser
        .tree
        .get_side_span(
            expression_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
        )
        .expect("missing expression leading span");
    let comment = parser.tree.comments()[0];

    assert_eq!(parser.get_span_str(expression_span), "value");
    assert_eq!(parser.get_span_str(leading_span), "/* keep */ ");
    assert_eq!(comment.attached_to, expression_span.start);
}

/// Parse a private identifier used in an in expression.
#[test]
fn test_parse_private_identifier_in_expression() {
    let mut test = TestParser::new_with_language("#a in this", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    // #a in this
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::In);
        assert_node!(parser.tree, *left, Expression::PrivateIdentifier { name } => {
            assert_string!(parser, *name, "a");
        });
        assert_node!(parser.tree, *right, Expression::This);
    });
}

/// Parse typed object methods in decorator style call arguments.
#[test]
fn test_parse_typed_object_method_in_call_argument() {
    let mut test = TestParser::new_with_language(
        r#"connect({
    num(state: State) {
        return state.counter.num;
    },
    inc: "inc",
})"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body } => {
                    assert_string!(parser, *name, "num");
                    assert_eq!(signature.parameters.len(), 1);

                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                        assert_string!(parser, *name, "state");
                        assert_expression_path!(parser, parser.tree.get(*declared_type), "State");
                    });

                    assert_node!(parser.tree, body.expect("expected object method body"), Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { .. } => {
                            let expressions = block_expression_ids(parser.tree.get(*block_id));
                            assert_eq!(expressions.len(), 1);
                            assert_node!(parser.tree, expressions[0], Expression::Return { value: Some(value) } => {
                                    assert_expression_path!(parser, parser.tree.get(*value), "state.counter.num");
                            });
                        });
                    });
                });

                assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
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
    let mut test = TestParser::new_with_language(
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
        .with_flags(parser.flags.in_decorator(), |parser| {
            parser.eat_expression(parser.flags)
        })
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { key: Some(Key::Name(Name::Identifier(name))), body, .. } => {
                    assert_string!(parser, *name, "num");
                    assert_node!(parser.tree, body.expect("expected object method body"), Expression::Block(block_id) => {
                        assert_node!(parser.tree, *block_id, Block { .. } => {
                            let expressions = block_expression_ids(parser.tree.get(*block_id));
                            assert_eq!(expressions.len(), 2);
                            assert_node!(parser.tree, expressions[0], Expression::If { .. });
                            assert_node!(parser.tree, expressions[1], Expression::Return { value: Some(value) } => {
                                    assert_node!(parser.tree, *value, Expression::New { .. });
                            });
                        });
                    });
                });

                assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
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
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 25);

        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "namespace");
            assert_expression_path!(parser, parser.tree.get(*value), "namespace");
        });

        assert_node!(parser.tree, properties[10], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "constructor");
            assert_expression_path!(parser, parser.tree.get(*value), "constructor");
        });

        assert_node!(parser.tree, properties[11], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "let");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });

        assert_node!(parser.tree, properties[24], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "match");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}

/// Parse `type instanceof Foo` as an instanceof guard.
#[test]
fn test_parse_type_keyword_instanceof_expression() {
    let mut test = TestParser::new_with_language("type instanceof Foo", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::InstanceOf { value, target } => {
        assert_expression_path!(parser, parser.tree.get(*value), "type");
        assert_expression_path!(parser, parser.tree.get(*target), "Foo");
    });
}

/// Parse `extension` as an identifier in TypeScript expressions.
#[test]
fn test_parse_extension_identifier_in_ternary_expression() {
    let mut test = TestParser::new_with_language(
        r#"typeof extension === "function" ? extension(cloned) : extension"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
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

        assert_node!(parser.tree, *then_expression, Expression::Call { left, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "extension");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "cloned");
            });
        });

        let else_expression_id = else_expression.expect("expected ternary else expression");
        assert_expression_path!(parser, parser.tree.get(else_expression_id), "extension");
    });
}
