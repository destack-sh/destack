use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse a class expression with implements.
#[test]
fn test_parse_class_expression_with_implements() {
    let mut test =
        TestParser::new_with_options("new (class implements Foo {})()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, .. }) => {
                    assert_eq!(implements_types.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(implements_types[0]), "Foo");
                });
            });
        });
    });
}

/// Parse a class expression when heritage starts on the next line.
#[test]
fn test_parse_class_expression_with_newline_implements() {
    let mut test = TestParser::new_with_options(
        "new (class\n  implements Foo\n{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, .. }) => {
                    assert_eq!(implements_types.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(implements_types[0]), "Foo");
                });
            });
        });
    });
}

/// Parse a class expression with multiline extends heritage.
#[test]
fn test_parse_class_expression_with_newline_extends() {
    let mut test = TestParser::new_with_options(
        "new (class\n  extends Foo<Bar>\n{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_expression, .. }) => {
                    let extends_expression = extends_expression.expect("expected extends expression");
                    assert_node!(parser.tree, extends_expression, Expression::Instantiation { left, generic_arguments } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Foo");
                        assert_eq!(generic_arguments.len(), 1);
                        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                assert_expression_path!(parser, parser.tree.get(*value), "Bar");
                        });
                    });
                });
            });
        });
    });
}

/// Parse an unparenthesized class expression as a new receiver.
#[test]
fn test_parse_new_unparenthesized_class_expression_with_extends() {
    let mut test = TestParser::new_with_options(
        "new class extends TestRepository {}()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::New { left, arguments, .. } => {
        assert!(arguments.is_empty());
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_expression, .. }) => {
                let extends_expression = extends_expression.expect("expected extends expression");
                assert_expression_path!(parser, parser.tree.get(extends_expression), "TestRepository");
            });
        });
    });
}

/// Parse a class expression with a parenthesized sequence extends target.
#[test]
fn test_parse_class_expression_with_parenthesized_sequence_extends() {
    let mut test =
        TestParser::new_with_options("var a = class extends (b,c) {};", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // var a = class extends (b, c) {};
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
                    assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::SequenceExpression { expressions } => {
                            assert_eq!(expressions.len(), 2);
                            assert_expression_path!(parser, parser.tree.get(expressions[0]), "b");
                            assert_expression_path!(parser, parser.tree.get(expressions[1]), "c");
                        });
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

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value } => {
            assert_string!(parser, *name, "useClass");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, members, .. }) => {
                    assert_string!(parser, name.expect("expected class name").string(), "MyExampleClass");
                    assert!(members.is_empty());
                });
            });
        });
    });
}

/// Parse class expression values in decorator call arguments.
#[test]
fn test_eat_decorator_object_property_named_class_expression_value() {
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
    assert_node!(parser.tree, expr_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value } => {
                    assert_string!(parser, *name, "useClass");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, members, .. }) => {
                            assert_string!(parser, name.expect("expected class name").string(), "MyExampleClass");
                            assert!(members.is_empty());
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

    assert_node!(parser.tree, expression_id, Expression::New { left, arguments, .. } => {
        assert_eq!(arguments.len(), 0);
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, members, .. }) => {
                assert_eq!(implements_types.len(), 1);
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.cardinality, FunctionCardinality::Generator);
                });
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

    // <P extends Props>(wrapped: ComponentType<P>) => class extends Component<...> { ... }
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.kind, FunctionKind::Lambda);

            // class extends Component<Omit<P, keyof A> & Partial<B>, C>
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_expression, members, .. }) => {
                    assert_eq!(members.len(), 1);

                    // Component<Omit<...>, C>
                    let extends_expression =
                        extends_expression.expect("expected extends expression");
                    assert_node!(parser.tree, extends_expression, Expression::Instantiation { left, generic_arguments } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Component");
                        assert_eq!(generic_arguments.len(), 2);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_arrow_body_with_multiline_class_heritage_generic_arguments() {
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

    // <P extends Props>(wrapped: React.ComponentType<P>) => class extends React.Component<...> { ... }
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.kind, FunctionKind::Lambda);

            // class extends React.Component<Omit<P, keyof Props> & Partial<Props>, Props>
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_expression, .. }) => {
                    // React.Component<Omit<P, keyof Props> & Partial<Props>, Props>
                    let extends_expression =
                        extends_expression.expect("expected extends expression");
                    assert_node!(parser.tree, extends_expression, Expression::Instantiation { left, generic_arguments } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "React.Component");
                        assert_eq!(generic_arguments.len(), 2);
                    });
                });
            });
        });
    });
}
