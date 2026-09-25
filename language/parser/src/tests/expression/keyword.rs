use crate::tests::{TestParser, block_expression_ids};
use crate::{
    ExpressionPosition, ExpressionStop, assert_comment, assert_expression_path, assert_node,
    assert_string,
};
use tspp_dir::{
    Argument, AssignOperator, AssignPattern, BinaryOperator, Block, BlockForm, ClassDeclaration,
    CommentKind, Declaration, Declarator, Decorator, DependencyBinding, DependencyItem, ExportKind,
    Expression, FunctionDeclaration, ImportAttributeClauseKind, Literal, Name, Parameter, Pattern,
    Property, TypeDeclaration, TypeExpression, TypeLiteral,
};
use tspp_source::{NodeSpanBoundary, NodeSpanType};

/// Parse explicit first-class type values beginning with symbolic prefixes.
#[test]
fn test_parse_type_keyword_symbolic_prefix_values() {
    let test = TestParser::new("(type &User, type ^User, type *User, type !User)");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression, Expression::TupleExpression { elements } => {
        assert_eq!(elements.len(), 4);

        assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { .. });
            });
        });
        assert_node!(parser.tree, elements[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::OwnedOf { .. });
            });
        });
        assert_node!(parser.tree, elements[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::PointerOf { .. });
            });
        });
        assert_node!(parser.tree, elements[3], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Not { .. });
            });
        });
    });
}

/// Parse a do block expression with a value tail.
#[test]
fn test_parse_do_block_expression() {
    let test = TestParser::new(
        r#"
let value = do {
    let base = 1;
    base + 2
};
"#,
    );
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();

    assert_eq!(roots.len(), 1);
    assert_node!(parser.tree, roots[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { form, leading_expressions, tail_expression, .. } => {
                    assert_eq!(*form, BlockForm::Do);
                    assert_eq!(leading_expressions.len(), 1);
                    assert_node!(parser.tree, leading_expressions[0], Expression::Let { declarators, .. } => {
                        assert_eq!(declarators.len(), 1);
                        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                                assert_string!(parser, *name, "base");
                            });
                            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Literal(Literal::Integer(1)));
                        });
                    });
                    assert_node!(parser.tree, tail_expression.expect("expected do block tail"), Expression::Binary { left, operator, right } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "base");
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
                    });
                });
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse an if extends condition without consuming the block.
#[test]
fn test_parse_if_extends_type_reference() {
    let test = TestParser::new(
        r#"if (x extends Foo) {
    body
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = condition.as_expression().expect("expected expression condition");
        // x extends Foo
        assert_node!(parser.tree, condition_id, Expression::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Extends { left, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                assert_expression_path!(parser, parser.tree.get(*right), "Foo");
            });
        });
        // { body }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let body_statement_id = expressions[0];
                assert_expression_path!(parser, parser.tree.get(body_statement_id), "body");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse a static conditional type as a value expression.
#[test]
fn test_parse_type_relation_ternary_value_condition() {
    let test = TestParser::new(r#"const width = Row extends string ? 4 : 2"#);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Type { value } => {
                assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "Row");
                    assert_node!(parser.tree, *extends_type, TypeExpression::Keyword { value: TypeLiteral::String });
                    assert_node!(parser.tree, *then_type, TypeExpression::Literal { value: Literal::Integer(4) });
                    assert_node!(parser.tree, *else_type, TypeExpression::Literal { value: Literal::Integer(2) });
                });
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse an if instanceof condition inside parentheses.
#[test]
fn test_parse_if_instanceof_type_reference() {
    let test = TestParser::new(
        r#"if (T instanceof Foo) {
    value
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = condition.as_expression().expect("expected expression condition");
        // T instanceof Foo
        assert_node!(parser.tree, condition_id, Expression::InstanceOf { value, target } => {
            assert_expression_path!(parser, parser.tree.get(*value), "T");
            assert_expression_path!(parser, parser.tree.get(*target), "Foo");
        });
        // { value }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let value_statement_id = expressions[0];
                assert_expression_path!(parser, parser.tree.get(value_statement_id), "value");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse `value is string` as one runtime guard expression.
#[test]
fn test_parse_if_is_type_guard() {
    let test = TestParser::new(
        r"
if (value is string) {
  value
}
",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { condition, then_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");

        // value is string
        assert_node!(parser.tree, condition_id, Expression::Is { value, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
            assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });

        // { value }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);

                let value_statement_id = expressions[0];
                assert_expression_path!(parser, parser.tree.get(value_statement_id), "value");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse runtime type guard comments into expression and target boundaries.
#[test]
fn test_parse_if_is_type_guard_comment_boundaries() {
    let test =
        TestParser::new("if (value /* checked value */ is /* expected type */ string) { value }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_eq!(parser.comments().len(), 2);

    assert_node!(parser.tree, expression_id, Expression::If { condition, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");

        // value /* checked value */ is /* expected type */ string
        assert_node!(parser.tree, condition_id, Expression::Is { value, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
            assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });

            let operator_span = parser
                .tree
                .get_main_span(condition_id)
                .expect("expected guard operator span");
            let target_leading_span = parser
                .tree
                .get_side_span(*target_type, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                .expect("expected target leading span");

            assert_eq!(parser.span_str(operator_span), "is");
            assert_eq!(parser.span_str(target_leading_span), " /* expected type */ ");
        });
    });

    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " checked value");
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " expected type");
    test.assert_no_errors(&parser);
}

/// Parse `export { bar, baz } from foo`.
#[test]
fn test_parse_export_expression_with_items_block() {
    let test = TestParser::new("export { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // export { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export { bar, baz }`.
#[test]
fn test_parse_export_expression_items_without_target() {
    let test = TestParser::new("export { bar, baz }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { target: None, items, .. } => {
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export * as baz from "foo"`.
#[test]
fn test_parse_export_expression_namespace_alias() {
    let test = TestParser::new("export * as baz from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // export * as baz from foo
    assert_node!(parser.tree, expression_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "baz");
        });
    });
}

/// Parse an export declaration of a type declaration.
#[test]
fn test_parse_export_expression_type_declaration() {
    let test = TestParser::new("export type NonNullValue = Something");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, export, .. }) => {
            assert_string!(parser, name.string(), "NonNullValue");
            assert!(export.is_some());
        });
    });
}

/// Parse export default abstract class with decorator prefixes.
#[test]
fn test_parse_export_default_abstract_class_with_decorator_prefixes() {
    let test = TestParser::new("@before\nexport default @after abstract class Foo { }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { export, name, is_abstract, .. }) => {
            assert_eq!(*export, Some(ExportKind::Default));
            assert_string!(parser, name.expect("expected class name").string(), "Foo");
            assert!(*is_abstract);

            let decorators = parser.tree.get_decorators(declaration_id.id);
            assert_eq!(decorators.len(), 2);
            assert_node!(parser.tree, decorators[0], Decorator { expression, .. } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "before");
            });
            assert_node!(parser.tree, decorators[1], Decorator { expression, .. } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "after");
            });
        });
    });
}

/// Disambiguate using `type` as a variable.
#[test]
fn test_parse_type_as_variable() {
    let test = TestParser::new(
        r"
let type = 1
type = type + 2
",
    );
    let mut parser = test.prepare();

    // let type = 1
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value: Some(value), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "type");
            });
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    });

    // type = type + 2
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_eq!(*operator, AssignOperator::Assign);

        // type + 2
        assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_eq!(*operator, BinaryOperator::Add);
            assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
        });
    });
}

/// Parse `namespace[this.dest] = values`.
#[test]
fn test_parse_namespace_as_identifier_in_index_assignment() {
    let test = TestParser::new("namespace[this.dest] = values");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, AssignPattern::Place { expression: value } => {
            assert_node!(parser.tree, *value, Expression::Index { left, index, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "namespace");

                assert_node!(parser.tree, index.expect("expected index"), Expression::Member { left, name, .. } => {
                    assert_node!(parser.tree, *left, Expression::This);
                    assert_string!(parser, *name, "dest");
                });
            });
        });

        assert_expression_path!(parser, parser.tree.get(*right), "values");
    });
}

#[test]
fn test_parse_override_as_identifier_call() {
    let test = TestParser::new("override(value)");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "override");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_parse_abstract_as_identifier_call() {
    let test = TestParser::new("abstract(value)");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "abstract");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}
#[test]
fn test_parse_abstract_newline_as_identifier_then_class() {
    let test = TestParser::new("abstract\nclass B {}");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);
    assert_expression_path!(parser, parser.tree.get(expressions[0]), "abstract");
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "B");
        });
    });
}

/// Parse `type\nFoo = string;` as `type; Foo = string`.
#[test]
fn test_parse_type_newline_as_identifier_then_assignment() {
    let test = TestParser::new("type\nFoo = string;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);
    assert_expression_path!(parser, parser.tree.get(expressions[0]), "type");
    assert_node!(parser.tree, expressions[1], Expression::Assign { left, operator, right, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Foo");
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*right), "string");
    });
}

/// Parse `type` as a callback parameter and statement identifier.
#[test]
fn test_parse_callback_parameter_named_type() {
    let test = TestParser::new(
        "avplay.setListener({
    onsubtitlechange: (duration, subtitles, type, attributes) => {
        duration // $ExpectType string
        subtitles // $ExpectType string
        type // $ExpectType string
        attributes // $ExpectType AVPlaySubtitleAttribute[]
    }
})",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "avplay.setListener");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);

                assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                    assert_string!(parser, *name, "onsubtitlechange");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.parameters.len(), 4);

                            // duration, subtitles, type, attributes
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "duration");
                            });
                            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "subtitles");
                            });
                            assert_node!(parser.tree, signature.parameters[2], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "type");
                            });
                            assert_node!(parser.tree, signature.parameters[3], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "attributes");
                            });

                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                let expressions = block_expression_ids(parser.tree.get(*block_id));
                                assert_eq!(expressions.len(), 4);
                                assert_expression_path!(parser, parser.tree.get(expressions[0]), "duration");
                                assert_expression_path!(parser, parser.tree.get(expressions[1]), "subtitles");
                                assert_expression_path!(parser, parser.tree.get(expressions[2]), "type");
                                assert_expression_path!(parser, parser.tree.get(expressions[3]), "attributes");
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_report_export_path_expression() {
    let test = TestParser::new("export foo");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "foo");
}

/// Parse `import { bar, baz } from foo`.
#[test]
fn test_parse_import_expression_with_items_block() {
    let test = TestParser::new("import { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // import { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Import { target, items, .. } => {
        assert_string!(parser, *target, "foo");
        let items = items.as_deref().expect("expected import specifier shell");
        assert_eq!(items.len(), 2);

        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });

        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `import * as baz from "foo" with { bar: true }`.
#[test]
fn test_parse_import_expression_namespace_alias_with_arguments() {
    let test = TestParser::new("import * as baz from \"foo\" with { bar: true }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // import * as baz from foo with { bar: true }
    assert_node!(parser.tree, expression_id, Expression::Import { target, items, attributes, .. } => {
        assert_string!(parser, *target, "foo");
        let items = items.as_deref().expect("expected import specifier shell");
        assert_eq!(items.len(), 1);

        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "baz");
        });

        // with { bar: true }
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        let attributes = &attributes.attributes;
        assert_eq!(attributes.len(), 1);
    });
}

/// Report `import { foo }` without a target.
#[test]
fn test_report_import_expression_items_without_target() {
    let test = TestParser::new("import { foo }");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "");
}
