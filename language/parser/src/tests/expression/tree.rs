use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_name, assert_node,
    assert_path, assert_string,
};
use tspp_dir::{
    Argument, BinaryOperator, Declaration, Expression, FunctionDeclaration, FunctionForm,
    GenericParameter, IfForm, Literal, NodeType, Parameter, Pattern, TreeAttribute,
    TreeAttributeValue, TreeChild, TypeExpression, TypeLiteral,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

/// Parse a constrained generic arrow whose body is a tree literal.
#[test]
fn test_parse_constrained_generic_arrow_before_tree() {
    let test = TestParser::new("<P: Model>(x: P) => <Foo />");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), .. } => {
                assert_string!(parser, *name, "P");
                assert_node!(parser.tree, *constraint, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Model");
                });
            });
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "P");
                });
            });
            let body_id = body.expect("expected body");
            assert_node!(parser.tree, body_id, Expression::TreeExpression { left, attributes, children, .. } => {
                let left_id = left.expect("expected tag");
                assert_expression_path!(parser, parser.tree.get(left_id), "Foo");
                assert!(attributes.as_ref().is_none_or(|items| items.is_empty()));
                assert!(children.as_ref().is_none_or(|items| items.is_empty()));
            });
        });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_parenthesized_tree_callback_body() {
    let test = TestParser::new("items.map((item) => (<option>{item}</option>))");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "items.map");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 1);

                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "item");
                    });

                    crate::assert_parenthesized!(parser.tree, *body, expression => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "option");
                            assert!(attributes.is_none());

                            let children = children.as_ref().expect("expected option children");
                            assert_eq!(children.len(), 1);
                            assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "item");

                                let container_span = parser
                                    .tree
                                    .get_side_span(
                                        *value,
                                        NodeSpanType::Region(NodeSpanRegion::TreeContainer),
                                    )
                                    .expect("missing tree expression container span");
                                assert_eq!(parser.span_str(container_span), "{item}");
                                assert!(
                                    parser
                                        .tree
                                        .get_side_span(
                                            *value,
                                            NodeSpanType::Region(NodeSpanRegion::Parentheses),
                                        )
                                        .is_none()
                                );
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Recover an ambiguous generic arrow as an unterminated tree literal.
#[test]
fn test_recover_generic_arrow_without_tree_disambiguator() {
    let test = TestParser::new("<R>(x: R) => x");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children: Some(children), .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "R");
        assert_eq!(children.len(), 2);
        assert_node!(parser.tree, children[0], TreeChild::Text { value } => {
            assert_string!(parser, *value, "(x: R) =");
        });
        assert_node!(parser.tree, children[1], TreeChild::Text { value } => {
            assert_string!(parser, *value, "> x");
        });
    });
}

/// Recover a malformed generic constraint without losing the next declaration.
#[test]
fn test_recover_generic_arrow_constraint_member() {
    let test = TestParser::new(
        "const broken = <T: { item: ; }>(value: T) => value;\nconst recovered = 1;",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_errors(&parser, &[(Some(NodeType::TypeMember), None, None, ";")]);
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let declarator = parser.tree.get(declarators[0]);
        assert_node!(parser.tree, declarator.pattern, Pattern::Binding { name, .. } => {
            assert_string!(parser, *name, "recovered");
        });
    });
}

#[test]
fn test_parse_generic_arrow_with_trailing_comma() {
    let test = TestParser::new("<T,>() => 1");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.generic_parameters.len(), 1);
            assert!(signature.parameters.is_empty());
            assert_node!(parser.tree, signature.generic_parameters[0], GenericParameter::Type { name, constraint: None, default: None, .. } => {
                assert_string!(parser, *name, "T");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_generic_arrow_with_constraint_disambiguator() {
    let test = TestParser::new("<T: unknown>(x) => 1");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.generic_parameters.len(), 1);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), default: None, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, *constraint, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_generic_arrow_with_default_disambiguator() {
    let test = TestParser::new("<T = unknown,>(x) => 1");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.generic_parameters.len(), 1);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.generic_parameters[0], GenericParameter::Type { name, constraint: None, default: Some(default), .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, *default, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

/// Parse a trailing comma as a generic arrow tree disambiguator.
#[test]
fn test_parse_generic_arrow_with_trailing_comma_disambiguator() {
    let test = TestParser::new("<T,>(x: T): T => x");
    let mut parser = test.prepare();

    // <T,>(x: T): T => x
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_ternary_typed_arrow_function_before_tree() {
    let test = TestParser::new(
        r#"Math.random() > 0.5
    ? (): void => foo()
    : (): void => bar()"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
        });
        assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
        let else_id = else_expression.expect("expected else branch");
        assert_node!(parser.tree, else_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
    });
}

#[test]
fn test_parse_ternary_parenthesized_typed_arrow_function_before_tree() {
    let test = TestParser::new(
        r#"Math.random() > 0.5
    ? ((): void => foo())
    : ((): void => bar())"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
        });
        crate::assert_parenthesized!(parser.tree, *then_expression, expression => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
        let else_id = else_expression.expect("expected else branch");
        crate::assert_parenthesized!(parser.tree, else_id, expression => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tree_attribute_typed_arrow_value() {
    let test = TestParser::new(
        "<StyledComponent className={({ theme }): { [key: string]: unknown } => ({ color: theme.blue })} />",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::TreeExpression { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        let class_name_attribute = attributes.iter().copied().find(|attribute_id| {
            matches!(
                parser.tree.get(*attribute_id),
                TreeAttribute::Named { name, .. } if parser.strings.get(name.string()) == "className"
            )
        });
        let class_name_attribute = class_name_attribute.expect("expected className attribute");
        assert_node!(parser.tree, class_name_attribute, TreeAttribute::Named { name, value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_name!(parser, *name, "className");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    let return_type = signature.return_type.expect("expected return type");
                    assert_node!(parser.tree, return_type, TypeExpression::Object { .. });
                });
            });
        });
    });
}

/// Parse a fixed array repeat literal inside a tree attribute expression.
#[test]
fn test_parse_tree_attribute_fixed_array_expression_value() {
    let test = TestParser::new("<Buffer data={[0; count]} />");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected tree attributes");
        assert_eq!(attributes.len(), 1);

        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name, value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_name!(parser, *name, "data");
            assert_node!(parser.tree, *value, Expression::FixedArrayExpression { value, length } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
                assert_expression_path!(parser, parser.tree.get(*length), "count");
            });
        });
    });
}

/// Recover a fixed array repeat literal inside a tree attribute expression.
#[test]
fn test_parse_tree_attribute_fixed_array_expression_value_recovers_missing_length() {
    let test = TestParser::new("<Buffer data={[0; ]} next />");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "]")]);
    assert_node!(parser.tree, expression_id, Expression::TreeExpression { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected tree attributes");
        assert_eq!(attributes.len(), 2);

        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name, value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_name!(parser, *name, "data");
            assert_node!(parser.tree, *value, Expression::FixedArrayExpression { value, length } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
                assert_node!(parser.tree, *length, Expression::Missing);
            });
        });

        assert_node!(parser.tree, attributes[1], TreeAttribute::Named { name, value: None } => {
            assert_name!(parser, *name, "next");
        });
    });
}

#[test]
fn test_parse_ternary_tree_attribute_typed_arrow() {
    let test = TestParser::new(
        "disabled ? <StyledComponent className={({ theme }): { [key: string]: unknown } => ({ color: theme.blue })} /> : null",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, then_expression, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { attributes, .. } => {
            let attributes = attributes.as_ref().expect("expected attributes");
            let class_name_attribute = attributes.iter().copied().find(|attribute_id| {
                matches!(
                    parser.tree.get(*attribute_id),
                    TreeAttribute::Named { name, .. } if parser.strings.get(name.string()) == "className"
                )
            });
            let class_name_attribute = class_name_attribute.expect("expected className attribute");
            assert_node!(parser.tree, class_name_attribute, TreeAttribute::Named { name, value: Some(TreeAttributeValue::Expression(value)) } => {
                assert_name!(parser, *name, "className");
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.form, FunctionForm::Lambda);
                        let return_type = signature.return_type.expect("expected return type");
                        assert_node!(parser.tree, return_type, TypeExpression::Object { .. });
                    });
                });
            });
        });
        assert!(else_expression.is_some());
    });
}

#[test]
fn test_parse_tree_attribute_nested_tree_expression_value() {
    let test = TestParser::new("<Foo prop={<Bar><Baz /></Bar>} />;");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected tree attributes");
        let prop_attribute = attributes.iter().copied().find(|attribute_id| {
            matches!(
                parser.tree.get(*attribute_id),
                TreeAttribute::Named { name, .. } if parser.strings.get(name.string()) == "prop"
            )
        });
        let prop_attribute = prop_attribute.expect("expected prop attribute");

        assert_node!(parser.tree, prop_attribute, TreeAttribute::Named { value: Some(TreeAttributeValue::Expression(value)), .. } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, children, .. } => {
                let left = left.expect("expected nested tree path");
                assert_expression_path!(parser, parser.tree.get(left), "Bar");

                let children = children.as_ref().expect("expected nested children");
                assert_eq!(children.len(), 1);
                assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left, .. } => {
                        let left = left.expect("expected child tree path");
                        assert_expression_path!(parser, parser.tree.get(left), "Baz");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_closing_tag_with_trailing_line_comment_before_greater_than() {
    let test = TestParser::new("<a></a // line\n>;");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left, .. } => {
        let left = left.expect("expected tag path");
        assert_expression_path!(parser, parser.tree.get(left), "a");
    });
}

#[test]
fn test_parse_typed_arrow_parameter_with_generic_function_target_type_before_tree() {
    let test = TestParser::new(
        "(signal: AbortSignal, addInspectorRequest: <Data>(result: FetcherResult<Data>) => void): AutoAbortedAPMClient => signal",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "AutoAbortedAPMClient");
            assert_expression_path!(parser, parser.tree.get(*body), "signal");

            // signal: AbortSignal
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "signal");
                assert_expression_path!(parser, parser.tree.get(*declared_type), "AbortSignal");
            });

            // addInspectorRequest: <Data>(result: FetcherResult<Data>) => void
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "addInspectorRequest");
                assert_node!(parser.tree, *declared_type, TypeExpression::Function(function) => {
                    let nested_generic_parameters = &function.generic_parameters;
                    assert_eq!(nested_generic_parameters.len(), 1);

                    assert_eq!(function.parameters.len(), 1);
                    assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "result");
                    });

                    assert_node!(parser.tree, function.return_type.expect("expected nested return type"), TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tree_text_after_comment_expression_container() {
    let test = TestParser::new(
        r#"<test>
    {/* comment */}
     some
     text
</test>"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "test");
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 2);
        assert_node!(parser.tree, children[0], TreeChild::Empty);
        assert_node!(parser.tree, children[1], TreeChild::Text { value } => {
            assert_string!(parser, *value, "\n     some\n     text\n");
        });
    });
}
