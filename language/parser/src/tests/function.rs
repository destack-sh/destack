use crate::{ExpressionPosition, ExpressionStop, TypePosition, TypeStop};
use destack_dir::{
    Access, Argument, Asynchrony, BinaryOperator, BlockContext, BlockForm, CommentKind,
    Declaration, Declarator, Expression, FunctionDeclaration, FunctionForm, FunctionPhase,
    GenericArgument, GenericParameter, IntegerType, Literal, NodeType, Parameter, Pattern,
    PatternField, ThisForm, TokenType, TypeDeclaration, TypeExpression, TypeLiteral, UnaryOperator,
    VarianceModifier, WhereClause, YieldCardinality,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{
    CommentRetention, TestParser, assert_comment, assert_expression_path, assert_name, assert_node,
    assert_path, assert_string,
};

#[test]
fn test_parse_function_lambda_with_newlines() {
    let test = TestParser::new(
        r#"(x: number):
    number =>
    x"#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    // (x: number): number => x
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body: Some(body), .. }) => {
        assert_eq!(*name, None);
        assert_eq!(signature.form, FunctionForm::Lambda);
        // x: number
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });
        // number
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
        // x
        assert_node!(parser.tree, *body, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
    });
}

#[test]
fn test_parse_optional_arrow_parameter_without_type() {
    let test = TestParser::new("(value?) => value");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, is_optional, .. } => {
                assert_string!(parser, *name, "value");
                assert!(*is_optional);
            });
        });
    });
    test.assert_no_errors(&parser);
}

/// Parse relational and shift expressions in arrow parameter defaults.
#[test]
fn test_parse_arrow_parameter_defaults_with_angle_operators() {
    let cases = [
        ("(value = left < right) => value", BinaryOperator::LessThan),
        (
            "(value = left > right) => value",
            BinaryOperator::GreaterThan,
        ),
        (
            "(value = left << right) => value",
            BinaryOperator::ShiftLeft,
        ),
        (
            "(value = (left < right)) => value",
            BinaryOperator::LessThan,
        ),
    ];

    for (source, expected_operator) in cases {
        let test = TestParser::new(source);
        let mut parser = test.prepare();
        let expression_id = parser
            .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
            .unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { default: Some(default), .. } => {
                    assert_node!(parser.tree, *default, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, expected_operator);
                    });
                });
            });
        });
        test.assert_no_errors(&parser);
    }
}

/// Parse a generic type annotation followed by an arrow parameter default.
#[test]
fn test_parse_typed_arrow_parameter_default() {
    let test = TestParser::new("(value: Box<Item> = input) => value");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { declared_type: Some(declared_type), default: Some(default), .. } => {
                assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Box");
                    assert_eq!(generic_arguments.len(), 1);
                });
                assert_expression_path!(parser, parser.tree.get(*default), "input");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_parenthesized_optional_arrow_parameter_without_type_call() {
    let test = TestParser::new("((value?) => value)()");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        crate::assert_parenthesized!(parser.tree, *left, expression => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, is_optional, .. } => {
                        assert_string!(parser, *name, "value");
                        assert!(*is_optional);
                    });
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_function_missing_close_paren_keeps_following_declaration() {
    let test = TestParser::new(
        r#"
export function broken( {}
export function stableLater(): void {}
"#,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // export function broken( {}
    let first_declaration_id = expressions[0];
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // export function stableLater(): void {}
    let second_declaration_id = expressions[1];
    assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "stableLater");
        });
    });
}

#[test]
fn test_parse_const_function() {
    let test = TestParser::new("const function layout<T>(value: T): usize {}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_name!(parser, name.unwrap(), "layout");
        assert_eq!(signature.phase, FunctionPhase::Const);
        assert_eq!(signature.form, FunctionForm::Function);
        assert_eq!(signature.parameters.len(), 1);

        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
            assert_string!(parser, *name, "value");
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse the const function declaration through the statement path.
#[test]
fn test_parse_const_function_through_statement_path() {
    let test = TestParser::new("const function f(): usize {}");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            assert_name!(parser, name.unwrap(), "f");
            assert_eq!(signature.phase, FunctionPhase::Const);
            assert_eq!(signature.form, FunctionForm::Function);
            assert!(signature.parameters.is_empty());
        });
    });
}

#[test]
fn test_parse_function_missing_close_paren_before_following_function_keeps_declaration() {
    let test = TestParser::new(
        r#"
function broken(
function stableLater(): void {}
"#,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // function broken(
    let first_declaration_id = expressions[0];
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // function stableLater(): void {}
    let second_declaration_id = expressions[1];
    assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "stableLater");
        });
    });
}

#[test]
fn test_parse_function_missing_close_paren_before_following_const_keeps_statement() {
    let test = TestParser::new(
        r#"
function broken(
const value = 1
"#,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // function broken(
    let first_declaration_id = expressions[0];
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // const value = 1
    let second_expression_id = expressions[1];
    assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "value");
            });
        });
    });
}

#[test]
fn test_parse_function_parameter_named_type_after_newline() {
    let test = TestParser::new(
        r#"
function configure(
    type: string,
): void {}
"#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    // function configure(type: string): void {}
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_name!(parser, name.unwrap(), "configure");
        assert_eq!(signature.parameters.len(), 1);

        // type: string
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "type");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    });
}

#[test]
fn test_parse_function_parameter_named_namespace_after_newline() {
    let test = TestParser::new(
        r#"
function setns(
    namespace: ProcessNamespaceKind,
): void {}
"#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    // function setns(namespace: ProcessNamespaceKind): void {}
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_name!(parser, name.unwrap(), "setns");
        assert_eq!(signature.parameters.len(), 1);

        // namespace: ProcessNamespaceKind
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "namespace");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "ProcessNamespaceKind");
            });
        });
    });
}

#[test]
fn test_parse_plain_parenthesized_lambda_with_newlines() {
    let test = TestParser::new("(\nvalue\n) => value");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "value");
            assert!(declared_type.is_none());
            assert!(default.is_none());
        });
        assert!(signature.return_type.is_none());
        assert_node!(parser.tree, body.expect("expected body"), Expression::Identifier { name } => {
            assert_string!(parser, *name, "value");
        });
    });

    let parameter_container_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Parameters),
        )
        .unwrap();
    assert_eq!(parser.span_str(parameter_container_span), "(\nvalue\n)");

    let body_container_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.span_str(body_container_span), "value");
}

#[test]
fn test_parse_plain_identifier_lambda_parameters_span() {
    let test = TestParser::new("value => value");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    let parameters_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Parameters),
        )
        .unwrap();
    assert_eq!(parser.span_str(parameters_span), "value");

    let body_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.span_str(body_span), "value");
}

#[test]
fn test_parse_plain_lambda_parenthesized_body_span() {
    let test = TestParser::new("value => ({ key: value })");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    let body_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.span_str(body_span), "({ key: value })");
}

#[test]
fn test_parse_plain_parenthesized_typed_lambda() {
    let test = TestParser::new("(value: number) => value");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "value");
            assert!(default.is_none());
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });
        assert_node!(parser.tree, body.expect("expected body"), Expression::Identifier { name } => {
            assert_string!(parser, *name, "value");
        });
    });
}

/// Parse a function type with an explicit this parameter.
#[test]
fn test_parse_function_type_with_this_parameter() {
    let test = TestParser::new("type T = (this: Foo, value: Bar) => Baz");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // type T = (this: Foo, value: Bar) => Baz
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                assert!(function.this_parameter.is_some());
                assert_eq!(function.this_form, Some(ThisForm::Explicit));
                assert_eq!(function.parameters.len(), 1);
                // value: Bar
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(*ty), "Bar");
                });
            });
        });
    });
}

/// Parse receiver shorthand in function types.
#[test]
fn test_parse_function_type_with_unqualified_this_parameter() {
    let test = TestParser::new("type T = (this, value: Bar) => Baz");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // type T = (this, value: Bar) => Baz
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                assert_eq!(function.this_form, Some(ThisForm::Implicit));
                assert_eq!(function.parameters.len(), 1);

                let this_parameter = function.this_parameter.expect("expected this parameter");
                assert_node!(parser.tree, this_parameter, Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "this");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::This);
                });
            });
        });
    });
}

/// Parse borrowed receiver shorthand in function types.
#[test]
fn test_parse_function_type_with_borrowed_this_parameter() {
    for (prefix, expected) in [
        ("&", Access::Mutable),
        ("&readonly ", Access::Readonly),
        ("&exclusive ", Access::Exclusive),
        ("&immutable ", Access::Immutable),
        ("&'a immutable ", Access::Immutable),
        ("&'a readonly ", Access::Readonly),
    ] {
        let source = format!("type T = ({prefix}this, value: Bar) => Baz");
        let test = TestParser::new(&source);
        let mut parser = test.prepare();
        let expr_id = parser
            .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
            .unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
                assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                    assert_eq!(function.this_form, Some(ThisForm::Implicit));
                    assert_eq!(function.parameters.len(), 1);

                    let this_parameter = function.this_parameter.expect("expected this parameter");
                    assert_node!(parser.tree, this_parameter, Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "this");
                        let name_range = parser.tree.get_main_range(this_parameter).unwrap();
                        assert_eq!(parser.range_str(name_range), "this");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::BorrowedOf { access, target_type, .. } => {
                            assert_eq!(*access, Some(expected));
                            assert_node!(parser.tree, *target_type, TypeExpression::This);
                        });
                    });
                });
            });
        });
        test.assert_no_errors(&parser);
    }
}

/// Parse readonly receiver shorthand in function types.
#[test]
fn test_parse_function_type_with_readonly_this_parameter() {
    let test = TestParser::new("type T = (readonly this, value: Bar) => Baz");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // type T = (readonly this, value: Bar) => Baz
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                assert_eq!(function.this_form, Some(ThisForm::Implicit));
                assert_eq!(function.parameters.len(), 1);

                let this_parameter = function.this_parameter.expect("expected this parameter");
                assert_node!(parser.tree, this_parameter, Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "this");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Readonly { target_type } => {
                        assert_node!(parser.tree, *target_type, TypeExpression::This);
                    });
                });
            });
        });
    });
}

/// Parse an arrow function with an explicit this parameter.
#[test]
fn test_parse_arrow_function_with_this_parameter() {
    let test = TestParser::new("(this: string) => {}");
    let mut parser = test.prepare();
    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    // (this: string) => {}
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert!(signature.this_parameter.is_some());
        assert_eq!(signature.this_form, Some(ThisForm::Explicit));
        assert!(signature.parameters.is_empty());
        assert!(body.is_some());
    });
}

/// Parse receiver shorthand in arrow functions.
#[test]
fn test_parse_arrow_function_with_unqualified_this_parameter() {
    let test = TestParser::new("(this) => this");
    let mut parser = test.prepare();
    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    // (this) => this
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.this_form, Some(ThisForm::Implicit));
        assert!(signature.this_parameter.is_some());
        assert!(signature.parameters.is_empty());
        assert!(body.is_some());
    });
}

/// Parse owned receiver shorthand in arrow functions.
#[test]
fn test_parse_arrow_function_with_owned_this_parameter() {
    let test = TestParser::new("(^this) => this");
    let mut parser = test.prepare();
    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    // (^this) => this
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.this_form, Some(ThisForm::Implicit));
        assert!(signature.this_parameter.is_some());
        assert!(signature.parameters.is_empty());
        assert!(body.is_some());
    });
}

#[test]
fn test_parse_function_lambda_with_explicit_return_type() {
    let test = TestParser::new("(x): int32 => x");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    // (x): int32 => x
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body: Some(body), .. }) => {
        assert!(name.is_none());
        assert_eq!(signature.form, FunctionForm::Lambda);
        // x
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
            assert_string!(parser, *name, "x");
        });
        // int32
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
        });
        // x
        assert_node!(parser.tree, *body, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
    });
}

/// Recover an unclosed lambda return type before the following declaration.
#[test]
fn test_recover_unclosed_lambda_return_type() {
    let test = TestParser::new("const broken = (value): { item: ;\nconst recovered = value;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.errors.len(), 1);
}

/// Parse parenthesized void return types in arrow functions.
#[test]
fn test_parse_function_parenthesized_void_return_type() {
    let test = TestParser::new("(): (void) => {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(body.is_some());
            crate::assert_parenthesized!(parser.tree, signature.return_type.unwrap(), expression => {
                assert_node!(parser.tree, *expression, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
    });
}

/// Parse default parameters followed by required parameters.
#[test]
fn test_parse_function_default_parameter_followed_by_required() {
    let test = TestParser::new(r#"function func(greeting: string = "Hello", target: string) {}"#);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function func(greeting: string = "Hello", target: string) {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            // greeting: string = "Hello"
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "greeting");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, default.unwrap(), Expression::Literal(Literal::String(value)) => {
                    assert_string!(parser, *value, "Hello");
                });
            });
            // target: string
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "target");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert!(default.is_none());
            });
        });
    });
}

/// Recover reserved formal parameter bindings while preserving their tree shape.
#[test]
fn test_recover_reserved_formal_parameter_bindings() {
    let cases = [
        (
            "function* broken(yield) {}\nconst stable = 1",
            "yield",
            false,
        ),
        (
            "function* broken({ yield }) {}\nconst stable = 1",
            "yield",
            true,
        ),
        (
            "async function broken(await) {}\nconst stable = 1",
            "await",
            false,
        ),
        (
            "async function broken({ await }) {}\nconst stable = 1",
            "await",
            true,
        ),
    ];

    for (source, binding, is_pattern) in cases {
        let test = TestParser::new(source);
        let mut parser = test.prepare();
        let expressions = parser
            .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
            .unwrap();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.parameters.len(), 1);
                if is_pattern {
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Pattern { pattern, .. } => {
                        assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                            assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, .. } => {
                                assert_name!(parser, *name, binding);
                            });
                        });
                    });
                } else {
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, binding);
                    });
                }
            });
        });
        assert_node!(parser.tree, expressions[1], Expression::Let { .. });
        test.assert_errors(
            &parser,
            &[(None, Some(TokenType::Identifier), None, binding)],
        );
    }
}

/// Recover forbidden `yield` and `await` expressions in formal parameter defaults.
#[test]
fn test_recover_forbidden_formal_parameter_default_expressions() {
    let cases = [
        (
            "function* broken(value = yield 1) {}\nconst stable = 1",
            "yield",
        ),
        (
            "async function broken(value = await load()) {}\nconst stable = 1",
            "await",
        ),
    ];

    for (source, error_text) in cases {
        let test = TestParser::new(source);
        let mut parser = test.prepare();
        let expressions = parser
            .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
            .unwrap();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Error);
            });
        });
        assert_node!(parser.tree, expressions[1], Expression::Let { .. });
        test.assert_errors(
            &parser,
            &[(
                Some(NodeType::Parameter),
                Some(TokenType::Identifier),
                None,
                error_text,
            )],
        );
    }
}

#[test]
fn test_parse_function_new_type() {
    let test = TestParser::new("new(): $");
    let mut parser = test.prepare();

    let type_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();
    assert_node!(parser.tree, type_id, TypeExpression::Constructor(constructor) => {
        assert!(!constructor.is_abstract);
        assert!(constructor.parameters.is_empty());
        assert_expression_path!(parser, parser.tree.get(constructor.return_type.unwrap()), "$");
    });
}

#[test]
fn test_parse_function_new_type_with_generic_arguments() {
    let test = TestParser::new("new <T>(x: int32) => T");
    let mut parser = test.prepare();

    let type_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();
    assert_node!(parser.tree, type_id, TypeExpression::Constructor(constructor) => {
        let generic_parameters = &constructor.generic_parameters;
        // <T>
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "T");
            assert!(constraint.is_none());
        });
        // x: int32
        assert_eq!(constructor.parameters.len(), 1);
        assert_node!(parser.tree, constructor.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
            });
        });
        // T
        assert_expression_path!(parser, parser.tree.get(constructor.return_type.unwrap()), "T");
    });
}

#[test]
fn test_parse_function_with_where_clause() {
    let test = TestParser::new(
        r###"
function foo() => int32 where Guard: Limit {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // function name
        assert_string!(parser, name.expect("expected name").string(), "foo");
        // where Guard: Limit
        let where_clauses = &signature.where_clauses;
        assert_eq!(where_clauses.len(), 1);
        assert_node!(parser.tree, where_clauses[0], WhereClause { relation: _, left, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Guard");
            assert_expression_path!(parser, parser.tree.get(*right), "Limit");
        });
        // return type
        let ret = signature.return_type.expect("expected return type");
        assert_node!(parser.tree, ret, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
        });
    });
}

#[test]
fn test_parse_function_with_generic_and_dynamic_parameters() {
    let test = TestParser::new(
        r"
function compute<Validate: boolean, Precision: uint8>(data: uint8[]) {
    body
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // compute
        assert_string!(parser, name.expect("expected name").string(), "compute");
        let generic_parameters = &signature.generic_parameters;
        // <Validate: bool, Precision: uint8>
        assert_eq!(generic_parameters.len(), 2);

        // Validate: bool
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "Validate");
            assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Boolean);
            });
        });

        // Precision: uint8
        assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "Precision");
            assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 8, is_signed: false }));
            });
        });
        // data: uint8[]
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
            assert_string!(parser, *name, "data");
        });
    });
}

#[test]
fn test_parse_function_abstract_new_type_with_newline() {
    let test = TestParser::new("abstract\nnew (): T");
    let mut parser = test.prepare();

    let type_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();
    assert_node!(parser.tree, type_id, TypeExpression::Constructor(constructor) => {
        assert!(constructor.is_abstract);
        assert!(constructor.parameters.is_empty());
        assert_expression_path!(parser, parser.tree.get(constructor.return_type.unwrap()), "T");
    });
}

#[test]
fn test_parse_function_with_newline_between_generics_and_parameters() {
    let test = TestParser::new(
        r"
function h<T>
    (tag: T): T;
            ",
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body, .. }) => {
        // function name
        assert_string!(parser, name.expect("expected name").string(), "h");

        // generic parameter: T
        let generic_parameters = &signature.generic_parameters;
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "T");
            assert!(constraint.is_none());
        });

        // dynamic parameter tag: T
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "tag");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "T");
                assert!(generic_arguments.is_empty());
            });
        });

        // return type T
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "T");
            assert!(generic_arguments.is_empty());
        });

        // declaration signature has no body
        assert!(body.is_none());
    });

    let generic_parameter_container_range = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .unwrap();
    assert_eq!(parser.span_str(generic_parameter_container_range), "<T>");

    let parameter_container_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Parameters),
        )
        .unwrap();
    assert_eq!(parser.span_str(parameter_container_span), "(tag: T)");
}

#[test]
fn test_parse_function_with_newline_before_return_type_colon() {
    let test = TestParser::new(
        r#"function f<T>(value: T)
  : T {
  return value as never
}"#,
    );
    let mut parser = test.prepare();
    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        let return_type = signature.return_type.expect("expected return type");
        assert_expression_path!(parser, parser.tree.get(return_type), "T");

        let body_id = body.expect("expected function body");
        assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            assert_eq!(block.leading_expressions.len(), 1);
            assert!(block.tail_expression.is_none());
            assert_node!(parser.tree, block.leading_expressions[0], Expression::Return { value } => {
                    let value = value.expect("expected return value");
                    assert_node!(parser.tree, value, Expression::As { .. });
            });
        });
    });
}

#[test]
fn test_parse_function_with_variance_parameters() {
    let test = TestParser::new(
        r"
function transform<in T, out U>(value: T): U {
    value as U
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "transform");
        let generic_parameters = &signature.generic_parameters;
        assert_eq!(generic_parameters.len(), 2);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, variance, .. } => {
            assert_string!(parser, *name, "T");
            assert_eq!(*variance, Some(VarianceModifier::In));
        });
        assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, variance, .. } => {
            assert_string!(parser, *name, "U");
            assert_eq!(*variance, Some(VarianceModifier::Out));
        });
    });
}

#[test]
fn test_parse_function_with_invariant_parameter() {
    let test = TestParser::new(
        r"
function invariant<in out T>(value: T): T {
    value
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "invariant");
        let generic_parameters = &signature.generic_parameters;
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, variance, .. } => {
            assert_string!(parser, *name, "T");
            assert_eq!(*variance, Some(VarianceModifier::InOut));
        });
    });
}

#[test]
fn test_parse_function_with_function_return_type() {
    let test = TestParser::new("function foo() => (str: string) => boolean {}");
    let mut parser = test.prepare();

    // function foo() => (str: string) => boolean
    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // foo
        assert_string!(parser, name.expect("expected name").string(), "foo");

        // (str: string) => boolean
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Function(function) => {
            assert_eq!(function.parameters.len(), 1);
            // str: string
            assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                assert_string!(parser, *name, "str");
                assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });

            // boolean
            assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Boolean);
            });
        });
    });
}

#[test]
fn test_parse_function_return_type_with_generic_arguments() {
    let test = TestParser::new("function read<T, E>() => AliasBranch<T, E> {}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
        let return_type = signature.return_type.expect("expected return type");
        assert_node!(parser.tree, return_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "AliasBranch");
            assert_eq!(generic_arguments.len(), 2);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "E");
            });
        });
    });
}

#[test]
fn test_parse_function_with_async_generator() {
    let test = TestParser::new(
        r#"
async function* foo() => int32 {
    yield 1
    yield 2
    yield 3
}"#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    // async function* foo() => int32 { body }
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // foo
        assert_string!(parser, name.unwrap().string(), "foo");
        assert_eq!(signature.asynchrony, Asynchrony::Async);
        assert!(signature.is_generator);
    });
}

/// Parse a generator function with a bare yield call argument.
#[test]
fn test_parse_function_generator_call_argument_with_bare_yield() {
    // source: function* a() { b.c(yield); }
    let test = TestParser::new("function* a() { b.c(yield); }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function* a() { b.c(yield); }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            // { b.c(yield); }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                // b.c(yield);
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Call { left, arguments, .. } => {
                        assert_eq!(arguments.len(), 1);
                        // b.c
                        assert_expression_path!(parser, parser.tree.get(*left), "b.c");
                        // yield
                        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::Yield { cardinality, value } => {
                                assert_eq!(*cardinality, YieldCardinality::Scalar);
                                assert!(value.is_none());
                            });
                        });
                });
            });
        });
    });
}

/// Parse named function expression container spans.
#[test]
fn test_parse_function_expression_container_spans() {
    let test = TestParser::new("bar(...items, function callback() { return 1; });");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // bar(...items, function callback() { return 1; })
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);

        // function callback() { return 1; }
        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                let parameter_span = parser
                    .tree
                    .get_side_span(
                        *declaration_id,
                        NodeSpanType::Region(NodeSpanRegion::Parameters),
                    )
                    .unwrap();
                assert_eq!(parser.span_str(parameter_span), "()");

                let body_span = parser
                    .tree
                    .get_side_span(*declaration_id, NodeSpanType::Region(NodeSpanRegion::Body))
                    .unwrap();
                assert_eq!(parser.span_str(body_span), "{ return 1; }");
            });
        });
    });
}

/// Preserve the enclosing function when a call argument is missing before the block close.
#[test]
fn test_parse_function_body_preserves_declaration_for_missing_call_argument_before_block_close() {
    // source
    let test = TestParser::new(
        r#"
function greet(name: string, suffix: string) {}

function main() {
    const userName = "Alice";
    greet(userName,
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[
            (None, Some(TokenType::CloseBrace), None, "}"),
            (
                Some(NodeType::Expression),
                Some(TokenType::CloseBrace),
                Some(TokenType::CloseParenthesis),
                "}",
            ),
        ],
    );

    // top level expressions
    assert_eq!(expressions.len(), 2);

    // function main() { ... }
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, body: Some(body), .. }) => {
            assert_string!(parser, name.unwrap().string(), "main");

            // { const userName = "Alice"; greet(userName, }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                let tail_expression = block.tail_expression.expect("expected trailing malformed call");

                // const userName = "Alice";
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Let { declarators, .. } => {
                    assert_eq!(declarators.len(), 1);
                });

                // greet(userName,
                assert_node!(parser.tree, tail_expression, Expression::Call { left, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "greet");
                    assert_eq!(arguments.len(), 2);

                    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "userName");
                    });

                    assert_node!(parser.tree, arguments[1], Argument::Error);
                });
            });
        });
    });
}

/// Parse nested generator yield expressions.
#[test]
fn test_parse_function_generator_nested_yield() {
    // source: function *a() { yield yield }
    let test = TestParser::new("function *a() { yield yield }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function *a() { yield yield }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            // { yield yield }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                // yield yield
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Scalar);
                        assert!(value.is_some());
                        // yield
                        assert_node!(parser.tree, value.unwrap(), Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Scalar);
                            assert!(value.is_none());
                        });
                });
            });
        });
    });
}

/// Parse delegated generator yield with a direct identifier operand.
#[test]
fn test_parse_function_generator_delegate_yield() {
    // source: function *a() { yield *a }
    let test = TestParser::new("function *a() { yield *a }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function *a() { yield *a }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            // { yield *a }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                // yield *a
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Generator);
                        assert!(value.is_some());
                        assert_expression_path!(parser, parser.tree.get(value.unwrap()), "a");
                });
            });
        });
    });
}

/// Parse delegated generator yield with a nested bare yield operand.
#[test]
fn test_parse_function_generator_delegate_nested_yield() {
    // source: function *a() { yield *yield }
    let test = TestParser::new("function *a() { yield *yield }");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function *a() { yield *yield }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            // { yield *yield }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                // yield *yield
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Generator);
                        assert!(value.is_some());
                        // yield
                        assert_node!(parser.tree, value.unwrap(), Expression::Yield { cardinality, value } => {
                            assert_eq!(*cardinality, YieldCardinality::Scalar);
                            assert!(value.is_none());
                        });
                });
            });
        });
    });
}

/// Parse scalar yield followed by a dereference on the next line.
#[test]
fn test_parse_function_generator_yield_before_dereference() {
    // source: function *a(){yield
    // *a}
    let test = TestParser::new("function *a(){yield\n*a}");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // function *a(){yield
    // *a}
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);

            // { yield \n *a }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);

                // yield
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Scalar);
                        assert!(value.is_none());
                });

                // *a
                let tail_expression = block.tail_expression.expect("expected dereference tail");
                assert_node!(parser.tree, tail_expression, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::Dereference);
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                });
            });
        });
    });
}

/// Recover delegated generator yield without an operand before a following const statement.
#[test]
fn test_recover_function_generator_delegate_before_following_const() {
    // source: function *a(){yield*
    // const value = 1}
    let test = TestParser::new("function *a(){yield*\nconst value = 1}");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(Some(NodeType::Expression), None, None, "const")],
    );

    // function *a(){yield*
    // const value = 1}
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);

            // { yield* \n const value = 1 }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 2);
                assert!(block.tail_expression.is_none());

                // yield*
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Generator);
                        assert_node!(parser.tree, value.expect("expected missing generator operand"), Expression::Missing);
                });

                // const value = 1
                assert_node!(parser.tree, block.leading_expressions[1], Expression::Let { declarators, .. } => {
                    assert_eq!(declarators.len(), 1);
                });
            });
        });
    });
}

/// Parse generator yield in computed assignment targets.
#[test]
fn test_parse_function_generator_yield_in_computed_assignment() {
    // source: function* a(){({[yield]:a}=1)}
    let test = TestParser::new("function* a(){({[yield]:a}=1)}");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert!(block.leading_expressions.is_empty());
                let tail_expression = block.tail_expression.expect("expected assignment tail");
                crate::assert_parenthesized!(parser.tree, tail_expression);
            });
        });
    });
}

#[test]
fn test_parse_function_with_nested_lambda_type() {
    let test = TestParser::new(
        r#"
function onResolve(
    callback: (args) => {
        path: string;
        namespace?: string;
    } | void,
) => void;
        "#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_string!(parser, name.unwrap().string(), "onResolve");
        // callback: (args) => { .. } | void
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "callback");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Function(function) => {
                assert_eq!(function.parameters.len(), 1);
                // args
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: None, .. } => {
                    assert_string!(parser, *name, "args");
                });
                // { .. } | void
                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);

                    // { .. }
                    assert_node!(parser.tree, elements[0], TypeExpression::Object { members: properties } => {
                        assert_eq!(properties.len(), 2);
                    });

                    // void
                    assert_node!(parser.tree, elements[1], TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
        // void
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Void);
        });
    });
}

#[test]
fn test_parse_lambda_return_type_with_optional_parameter_function_type() {
    let test = TestParser::new(
        "(runtime, effect, options: Runtime.RunCallbackOptions<unknown, unknown> = {}): (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<unknown, unknown> | undefined) => void => 0",
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 3);

            // (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<unknown, unknown> | undefined) => void
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Function(function) => {
                assert_eq!(function.parameters.len(), 2);

                // fiberId?: FiberId.FiberId
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), is_optional, .. } => {
                    assert_string!(parser, *name, "fiberId");
                    assert!(*is_optional);
                    assert_expression_path!(parser, parser.tree.get(*ty), "FiberId.FiberId");
                });

                // options?: Runtime.RunCallbackOptions<unknown, unknown> | undefined
                assert_node!(parser.tree, function.parameters[1], Parameter::Named { name, declared_type: Some(ty), is_optional, .. } => {
                    assert_string!(parser, *name, "options");
                    assert!(*is_optional);
                    assert_node!(parser.tree, *ty, TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });

                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });

            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(0)));
        });
    });
}

#[test]
fn test_parse_lambda_head_boundary_comment_on_function_owner() {
    let test = TestParser::new("(x) /* lambda-head */ => x");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    parser.finalize_comments();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { .. }) => {
        let annotations = parser.tree.get_decorators(function_id.id);
        assert!(annotations.is_empty());
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " lambda-head");
}

#[test]
fn test_parse_lambda_body_boundary_comment_on_body_owner() {
    let test = TestParser::new("(x) =>\n// lambda-body\nx");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_expression_path!(parser, parser.tree.get(*body_id), "x");

            let annotations = parser.tree.get_decorators(body_id.id);
            assert!(annotations.is_empty());
        });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "lambda-body");
}

/// Parse empty parenthesized lambda heads that only contain comments.
#[test]
fn test_parse_empty_parenthesized_lambda_with_comment() {
    let test = TestParser::new("(/* empty */) => {}");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.parameters.is_empty());
        });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " empty");
}

#[test]
fn test_parse_lambda_comment_only_block_body_attaches_inside_block() {
    let test = TestParser::new("() => {\n  // code\n}");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            let body_span = parser.tree.get_span(*body_id);
            let comment = parser.comments()[0];

            assert!(comment.is_leading());
            assert!(comment.span.start >= body_span.start);
            assert!(comment.span.end <= body_span.end);
        });
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "code");
}

#[test]
fn test_parse_function_body_boundary_line_comment_stays_trailing() {
    let test = TestParser::new("function f(): void // body\n{}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let function_id = parser
        .parse_function(&start, DeclarationHeader::default(), Default::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
        let body_span = parser.tree.get_span(*body_id);
        let comment = parser.comments()[0];

        assert!(comment.is_trailing());
        assert!(comment.span.end <= body_span.start);
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "body");
}

/// Report direct calls on unparenthesized arrow functions.
#[test]
fn test_report_unparenthesized_arrow_call() {
    // source: () => {}()
    let test = TestParser::new("() => {}()");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    // (
    assert_eq!(parser.range_str(error.range()), "(");

    // source: a => {}()
    let test = TestParser::new("a => {}()");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    // (
    assert_eq!(parser.range_str(error.range()), "(");
}

/// Parse direct calls on parenthesized arrow functions.
#[test]
fn test_parse_parenthesized_arrow_call() {
    // source: (() => {})()
    let test = TestParser::new("(() => {})()");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // (() => {})()
    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        crate::assert_parenthesized!(parser.tree, *left, expression => {
            assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                });
            });
        });
    });
}

/// Parse direct calls on parenthesized arrow functions without preserved wrappers.
#[test]
fn test_parse_parenthesized_arrow_call_without_preserved_wrappers() {
    // source: (() => {})()
    let test = TestParser::new("(() => {})()");
    let mut parser = test.prepare_with_comment_retention(CommentRetention::All);
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // (() => {})()
    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
            });
        });
    });
}
