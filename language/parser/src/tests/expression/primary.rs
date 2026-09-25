use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::{TestParser, block_expression_ids};
use crate::{
    CommentRetention, assert_expression_path, assert_node, assert_path, assert_string,
    assert_value_expression_path,
};
use tspp_dir::{Argument, Block, Expression, Literal, Name, Parameter, Property, TypeExpression};
use tspp_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

/// Parse import meta as one dedicated expression root.
#[test]
fn test_parse_import_meta_expression() {
    let test = TestParser::new("import.meta.env");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, name.expect("expected member name"), "env");
        assert_node!(parser.tree, *left, Expression::ImportMeta);
    });
}

/// Parse import source phase access as the import source intrinsic.
#[test]
fn test_parse_import_source_as_intrinsic() {
    let test = TestParser::new("import.source");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::ImportSource);
}

/// Parse import source phase calls as member calls.
#[test]
fn test_parse_import_source_call_expression() {
    let test = TestParser::new(r#"import.source("data:text/javascript,console.log(1)")"#);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, *left, Expression::ImportSource);
    });
}

/// Parse import source phase calls with expression arguments.
#[test]
fn test_parse_import_source_call_expression_with_template_argument() {
    let test = TestParser::new("import.source(String.raw`data:text/javascript,console.log(1)`)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, *left, Expression::ImportSource);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TaggedTemplateExpression { .. });
        });
    });
}

/// Parse a bare this expression.
#[test]
fn test_parse_this_expression() {
    let test = TestParser::new("this");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::This);
    let main_span = parser
        .tree
        .get_main_span(expression_id)
        .expect("expected this main span");
    assert_eq!(parser.span_str(main_span), "this");
}

/// Parse a bare identifier as an identifier expression.
#[test]
fn test_parse_identifier_expression_as_identifier() {
    let test = TestParser::new("value");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Identifier { name } => {
        assert_string!(parser, *name, "value");
    });
    assert_value_expression_path!(parser, parser.tree.get(expression_id), "value");
}

/// Parse contextual type literal names as values before member access.
#[test]
fn test_parse_contextual_type_literal_name_member_expression() {
    let test = TestParser::new("object.property");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "object");
        assert_string!(parser, name.expect("expected member name"), "property");
    });

    test.assert_no_errors(&parser);
}

/// Parse a type unary reference as a qualified reference.
#[test]
fn test_parse_type_unary_qualified_reference_as_qualified_reference() {
    let test = TestParser::new("type Foo.Bar");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
            assert!(generic_arguments.is_empty());
            assert_path!(parser, *path, "Foo.Bar");
            assert_expression_path!(parser, parser.tree.get(*value), "Foo.Bar");

            let head_span = parser
                .tree
                .get_head_span(*value)
                .expect("expected qualified reference head span");
            assert_eq!(parser.span_str(head_span), "Foo");
        });
    });
}

/// Parse a bare super expression.
#[test]
fn test_parse_super_expression() {
    let test = TestParser::new("super");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Super);
    let main_span = parser
        .tree
        .get_main_span(expression_id)
        .expect("expected super main span");
    assert_eq!(parser.span_str(main_span), "super");
}

/// Parse a bare null literal.
#[test]
fn test_parse_null_literal() {
    let test = TestParser::new("null");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Literal(Literal::Null)
    );
}

/// Parse a bare undefined literal.
#[test]
fn test_parse_undefined_literal() {
    let test = TestParser::new("undefined");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Literal(Literal::Undefined)
    );
}

/// Record parentheses around one canonical expression.
#[test]
fn test_parse_parenthesized_expression_records_source_region() {
    let test = TestParser::new("(value)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Identifier { .. });
    crate::assert_parenthesized!(parser.tree, expression_id);

    let expression_range = parser.tree.get_range(expression_id);
    let parentheses_range = parser
        .tree
        .get_side_range(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Parentheses),
        )
        .expect("missing parentheses region");

    assert_eq!(parser.range_str(expression_range), "value");
    assert_eq!(parser.range_str(parentheses_range), "(value)");
}

/// Keep comments outside the canonical expression range.
#[test]
fn test_parse_parenthesized_expression_keeps_inner_expression_span() {
    let test = TestParser::new("(/* keep */ value)");
    let mut parser = test.prepare_with_comment_retention(CommentRetention::All);
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::Identifier { .. });

    let expression_span = parser.tree.get_span(expression_id);
    let leading_span = parser
        .tree
        .get_side_span(
            expression_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
        )
        .expect("missing expression leading span");
    let parentheses_span = parser
        .tree
        .get_side_span(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Parentheses),
        )
        .expect("missing expression parentheses span");
    let comment = parser.comments()[0];

    assert_eq!(parser.span_str(expression_span), "value");
    assert_eq!(parser.span_str(leading_span), "/* keep */ ");
    assert_eq!(parser.span_str(parentheses_span), "(/* keep */ value)");
    assert_eq!(comment.following_token_start(), Some(expression_span.start));
}

#[test]
fn test_parse_typed_object_method_in_call_argument() {
    let test = TestParser::new(
        r#"connect({
    num(state: State) {
        return state.counter.num;
    },
    inc: "inc",
})"#,
    );
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { name: Some(Name::Identifier(name)), signature, body } => {
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

                assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                    assert_string!(parser, *name, "inc");
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::String(value)) => {
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
    let test = TestParser::new(
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
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::DecoratorHead, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "connect");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_node!(parser.tree, properties[0], Property::Method { name: Some(Name::Identifier(name)), body, .. } => {
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

                assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                    assert_string!(parser, *name, "inc");
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::String(value)) => {
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
    let test = TestParser::new(
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
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 25);

        assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "namespace");
            assert_expression_path!(parser, parser.tree.get(*value), "namespace");
        });

        assert_node!(parser.tree, properties[10], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "constructor");
            assert_expression_path!(parser, parser.tree.get(*value), "constructor");
        });

        assert_node!(parser.tree, properties[11], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "let");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
        });

        assert_node!(parser.tree, properties[24], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "match");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
        });
    });
}

/// Parse `type instanceof Foo` as an instanceof guard.
#[test]
fn test_parse_type_keyword_instanceof_expression() {
    let test = TestParser::new("type instanceof Foo");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::InstanceOf { value, target } => {
        assert_expression_path!(parser, parser.tree.get(*value), "type");
        assert_expression_path!(parser, parser.tree.get(*target), "Foo");
    });
}
