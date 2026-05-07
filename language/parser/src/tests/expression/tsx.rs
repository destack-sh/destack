use crate::tests::*;
use crate::{assert_expression_path, assert_name, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_generic_arrow_with_extends_before_tree() {
    let mut test = TestParser::new_with_language(
        "<P extends object>(x: P) => <Foo />",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            let generic_parameters = &signature.generic_parameters;
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "P");
                assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Object);
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
            assert_node!(parser.tree, body_id, Expression::TreeExpression { left, arguments, elements, .. } => {
                let left_id = left.expect("expected tag");
                assert_expression_path!(parser, parser.tree.get(left_id), "Foo");
                assert!(arguments.as_ref().is_none_or(|items| items.is_empty()));
                assert!(elements.as_ref().is_none_or(|items| items.is_empty()));
            });
        });
    });
}

#[test]
fn test_parse_parenthesized_tree_callback_body() {
    let mut test = TestParser::new_with_language(
        "items.map((item) => (<option>{item}</option>))",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "items.map");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 1);

                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "item");
                    });

                    assert_node!(parser.tree, *body, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
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

#[test]
fn test_reject_generic_arrow_without_tree_disambiguator() {
    let mut test = TestParser::new_with_language("<R>(x: R) => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

#[test]
fn test_parse_generic_arrow_with_trailing_comma_disambiguator() {
    let mut test = TestParser::new_with_language("<T,>(x: T): T => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // <T,>(x: T): T => x
    let expr_id = parser.eat_expression(parser.flags).unwrap();
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

#[test]
fn test_parse_ternary_typed_arrow_function_before_tree() {
    let mut test = TestParser::new_with_language(
        r#"Math.random() > 0.5
    ? (): void => foo()
    : (): void => bar()"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
        let else_id = else_expression.expect("expected else branch");
        assert_node!(parser.tree, else_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
    });
}

#[test]
fn test_parse_ternary_parenthesized_typed_arrow_function_before_tree() {
    let mut test = TestParser::new_with_language(
        r#"Math.random() > 0.5
    ? ((): void => foo())
    : ((): void => bar())"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
        let else_id = else_expression.expect("expected else branch");
        assert_node!(parser.tree, else_id, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tree_attribute_typed_arrow_value() {
    let mut test = TestParser::new_with_language(
        "<StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} />",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
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
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    let return_type = signature.return_type.expect("expected return type");
                    assert_node!(parser.tree, return_type, TypeExpression::Object { .. });
                });
            });
        });
    });
}

#[test]
fn test_parse_ternary_tree_attribute_typed_arrow() {
    let mut test = TestParser::new_with_language(
        "disabled ? <StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} /> : null",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::If { form, then_expression, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);
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
fn test_parse_tree_attribute_direct_nested_tree_value() {
    let mut test = TestParser::new_with_language(
        "<Foo prop=<Bar><Baz /></Bar> />;",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { arguments, .. } => {
        let arguments = arguments.as_ref().expect("expected tree arguments");
        let prop_argument = arguments.iter().copied().find(|argument_id| {
            matches!(
                parser.tree.get(*argument_id),
                Argument::Named { name, .. } if parser.strings.get(name.string()) == "prop"
            )
        });
        let prop_argument = prop_argument.expect("expected prop argument");

        assert_node!(parser.tree, prop_argument, Argument::Named { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, elements, .. } => {
                let left = left.expect("expected nested tree path");
                assert_expression_path!(parser, parser.tree.get(left), "Bar");

                let elements = elements.as_ref().expect("expected nested children");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
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
    let mut test = TestParser::new_with_language("<a></a // line\n>;", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left, .. } => {
        let left = left.expect("expected tag path");
        assert_expression_path!(parser, parser.tree.get(left), "a");
    });
}

#[test]
fn test_parse_typed_arrow_parameter_with_generic_function_target_type_before_tree() {
    let mut test = TestParser::new_with_language(
        "(signal: AbortSignal, addInspectorRequest: <Data>(result: FetcherResult<Data>) => void): AutoAbortedAPMClient => signal",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
                assert_node!(parser.tree, *declared_type, TypeExpression::FunctionTypeDeclaration(function) => {
                    let nested_generic_parameters = &function.generic_parameters;
                    assert_eq!(nested_generic_parameters.len(), 1);

                    assert_eq!(function.parameters.len(), 1);
                    assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(_), .. } => {
                        assert_string!(parser, *name, "result");
                    });

                    assert_node!(parser.tree, function.return_type.expect("expected nested return type"), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
    });
}
