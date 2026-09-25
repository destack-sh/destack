use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use tspp_dir::{
    Argument, ClassDeclaration, Declaration, Expression, FunctionDeclaration, FunctionForm,
    GenericArgument, Member, Name, Property, TypeExpression,
};

/// Parse a class expression with implements.
#[test]
fn test_parse_class_expression_with_implements() {
    let test = TestParser::new("class implements Foo {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, .. }) => {
            assert_eq!(implements_types.len(), 1);
            assert_expression_path!(parser, parser.tree.get(implements_types[0]), "Foo");
        });
    });
}

/// Parse a final class expression.
#[test]
fn test_parse_final_class_expression() {
    let test = TestParser::new("final class Service {}");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, is_final, .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "Service");
            assert!(*is_final);
        });
    });
}

/// Parse a class expression when heritage starts on the next line.
#[test]
fn test_parse_class_expression_with_newline_implements() {
    let test = TestParser::new("class\n  implements Foo\n{}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, .. }) => {
            assert_eq!(implements_types.len(), 1);
            assert_expression_path!(parser, parser.tree.get(implements_types[0]), "Foo");
        });
    });
}

/// Parse a class expression with multiline extends heritage.
#[test]
fn test_parse_class_expression_with_newline_extends() {
    let test = TestParser::new("class\n  extends Foo<Bar>\n{}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_type, .. }) => {
            let extends_type = extends_type.expect("expected extends type");
            assert_node!(parser.tree, extends_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Foo");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "Bar");
                });
            });
        });
    });
}

/// Parse an unparenthesized class expression with extends.
#[test]
fn test_parse_unparenthesized_class_expression_with_extends() {
    let test = TestParser::new("class extends TestRepository {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_type, .. }) => {
            let extends_type = extends_type.expect("expected extends type");
            assert_expression_path!(parser, parser.tree.get(extends_type), "TestRepository");
        });
    });
}

/// Parse an object property value that is a named class expression.
#[test]
fn test_parse_object_property_named_class_expression_value() {
    let test = TestParser::new("{ useClass: class MyExampleClass {} }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
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
    let test = TestParser::new("Component({ useClass: class MyExampleClass {} })");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::DecoratorHead, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
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

/// Parse class expressions with generic implements clauses.
#[test]
fn test_parse_class_expression_with_generic_implements_clause() {
    let test = TestParser::new(
        r#"class implements Iterable<string> {
  *iterator() {
    yield "value";
  }
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, members, .. }) => {
            assert_eq!(implements_types.len(), 1);
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert!(signature.is_generator);
            });
        });
    });
}

#[test]
fn test_parse_arrow_body_with_anonymous_class_expression() {
    let test = TestParser::new(
        r###"<P: Props>(
  wrapped: ComponentType<P>
) => class extends Component<Omit<P, keyof A> & Partial<B>, C> {
  static displayName = `x`;
}"###,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <P: Props>(wrapped: ComponentType<P>) => class extends Component<...> { ... }
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);

            // class extends Component<Omit<P, keyof A> & Partial<B>, C>
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_type, members, .. }) => {
                    assert_eq!(members.len(), 1);

                    // Component<Omit<...>, C>
                    let extends_type = extends_type.expect("expected extends type");
                    assert_node!(parser.tree, extends_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert_path!(parser, *path, "Component");
                        assert_eq!(generic_arguments.len(), 2);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_arrow_body_with_multiline_class_heritage_generic_arguments() {
    let test = TestParser::new(
        r###"<P: Props>(
  wrapped: React.ComponentType<P>
) => class extends React.Component<
  Omit<P, keyof Props> & Partial<Props>,
  Props
> {
  static displayName = `x`;
}"###,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <P: Props>(wrapped: React.ComponentType<P>) => class extends React.Component<...> { ... }
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);

            // class extends React.Component<Omit<P, keyof Props> & Partial<Props>, Props>
            assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_type, .. }) => {
                    // React.Component<Omit<P, keyof Props> & Partial<Props>, Props>
                    let extends_type = extends_type.expect("expected extends type");
                    assert_node!(parser.tree, extends_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert_path!(parser, *path, "React.Component");
                        assert_eq!(generic_arguments.len(), 2);
                    });
                });
            });
        });
    });
}
