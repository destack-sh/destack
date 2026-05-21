use destack_dir::{
    Argument, Asynchrony, BlockContext, BlockForm, ClassDeclaration, CommentKind, CommentPosition,
    Declaration, Declarator, Expression, FunctionDeclaration, FunctionForm, FunctionPhase,
    FunctionRole, GenericArgument, GenericParameter, IntegerType, NodeType, Parameter, Pattern,
    ScalarLiteral, TypeDeclaration, TypeExpression, TypeLiteral, VarianceModifier, WhereClause,
    YieldCardinality,
};

use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{
    ParserOptions, ParserTriviaMode, TestParser, assert_comment, assert_expression_path,
    assert_name, assert_node, assert_path, assert_string,
};

#[test]
fn test_parse_function_lambda_with_newlines() {
    let mut test = TestParser::new(
        r#"(x: number):
    number =>
    x"#,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    // (x: number): number => x
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, body: Some(body), .. }) => {
        assert_eq!(*name, None);
        assert_eq!(signature.form, FunctionForm::Lambda);
        // x: number
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });
        // number
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
        // x
        assert_node!(parser.tree, *body, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
    });
}

#[test]
fn test_parse_function_missing_close_paren_keeps_following_declaration() {
    let mut test = TestParser::new_with_language(
        r#"
export function broken( {}
export function stableLater(): void {}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .eat_block_body_in_context(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // export function broken( {}
    let first_declaration_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // export function stableLater(): void {}
    let second_declaration_id = parser.unwrap_label_expression(expressions[1]);
    assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "stableLater");
        });
    });
}

#[test]
fn test_parse_comptime_function() {
    let mut test = TestParser::new("comptime function layout<T>(comptime value: T): usize {}");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_name!(parser, name.unwrap(), "layout");
        assert_eq!(signature.phase, FunctionPhase::Comptime);
        assert_eq!(signature.form, FunctionForm::Function);
        assert_eq!(signature.parameters.len(), 1);

        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, is_comptime, .. } => {
            assert_string!(parser, *name, "value");
            assert!(*is_comptime);
        });
    });
}

#[test]
fn test_parse_comptime_arrow_parameter() {
    let mut test = TestParser::new("(comptime value: int32) => value");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert_eq!(signature.parameters.len(), 1);

        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, is_comptime, .. } => {
            assert_string!(parser, *name, "value");
            assert!(*is_comptime);
        });

        assert_node!(parser.tree, *body, Expression::Identifier { name } => {
            assert_string!(parser, *name, "value");
        });
    });
}

#[test]
fn test_parse_function_missing_close_paren_before_following_function_keeps_declaration() {
    let mut test = TestParser::new_with_language(
        r#"
function broken(
function stableLater(): void {}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .eat_block_body_in_context(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // function broken(
    let first_declaration_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // function stableLater(): void {}
    let second_declaration_id = parser.unwrap_label_expression(expressions[1]);
    assert_node!(parser.tree, second_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "stableLater");
        });
    });
}

#[test]
fn test_parse_function_missing_close_paren_before_following_const_keeps_statement() {
    let mut test = TestParser::new_with_language(
        r#"
function broken(
const value = 1
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let expressions = parser
        .eat_block_body_in_context(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 2);

    // function broken(
    let first_declaration_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, first_declaration_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, .. }) => {
            assert_name!(parser, name.unwrap(), "broken");
        });
    });

    // const value = 1
    let second_expression_id = parser.unwrap_label_expression(expressions[1]);
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
    let mut test = TestParser::new_with_language(
        r#"
function configure(
    type: string,
): void {}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    // function configure(type: string): void {}
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_name!(parser, name.unwrap(), "configure");
        assert_eq!(signature.parameters.len(), 1);

        // type: string
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "type");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    });
}

#[test]
fn test_parse_function_parameter_named_namespace_after_newline() {
    let mut test = TestParser::new(
        r#"
function setns(
    namespace: ProcessNamespaceKind,
): void {}
"#,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test = TestParser::new("(\nvalue\n) => value");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    assert_eq!(parser.get_span_str(parameter_container_span), "(\nvalue\n)");

    let body_container_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.get_span_str(body_container_span), "value");
}

#[test]
fn test_parse_plain_identifier_lambda_parameters_span() {
    let mut test = TestParser::new("value => value");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    let parameters_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Parameters),
        )
        .unwrap();
    assert_eq!(parser.get_span_str(parameters_span), "value");

    let body_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.get_span_str(body_span), "value");
}

#[test]
fn test_parse_plain_lambda_parenthesized_body_span() {
    let mut test = TestParser::new("value => ({ key: value })");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    let body_span = parser
        .tree
        .get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Body))
        .unwrap();
    assert_eq!(parser.get_span_str(body_span), "({ key: value })");
}

#[test]
fn test_parse_plain_parenthesized_typed_lambda() {
    let mut test = TestParser::new("(value: number) => value");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
            assert_string!(parser, *name, "value");
            assert!(default.is_none());
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
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
    let mut test = TestParser::new("type T = (this: Foo, value: Bar) => Baz");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = (this: Foo, value: Bar) => Baz
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                assert!(function.this_parameter.is_some());
                assert_eq!(function.parameters.len(), 1);
                // value: Bar
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "Bar");
                });
            });
        });
    });
}

/// Parse an arrow function with an explicit this parameter.
#[test]
fn test_parse_arrow_function_with_this_parameter() {
    let mut test = TestParser::new_with_language("(this: string) => {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    // (this: string) => {}
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert!(signature.this_parameter.is_some());
        assert!(signature.parameters.is_empty());
        assert!(body.is_some());
    });
}

#[test]
fn test_parse_function_lambda_with_explicit_return_type() {
    let mut test = TestParser::new("(x): int32 => x");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
        });
        // x
        assert_node!(parser.tree, *body, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
    });
}

/// Parse parenthesized void return types in arrow functions.
#[test]
fn test_parse_function_parenthesized_void_return_type() {
    let mut test = TestParser::new_with_language("(): (void) => {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(body.is_some());
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });
        });
    });
}

/// Parse default parameters followed by required parameters.
#[test]
fn test_parse_function_default_parameter_followed_by_required() {
    let mut test = TestParser::new_with_language(
        r#"function func(greeting: string = "Hello", target: string) {}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // function func(greeting: string = "Hello", target: string) {}
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 2);
            // greeting: string = "Hello"
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "greeting");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, default.unwrap(), Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                    assert_string!(parser, *value, "Hello");
                });
            });
            // target: string
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default, .. } => {
                assert_string!(parser, *name, "target");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert!(default.is_none());
            });
        });
    });
}

#[test]
fn test_parse_function_new_type() {
    let mut test = TestParser::new("new(): $");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    // new (x) => int32
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert!(name.is_none());
        assert_eq!(signature.role, Some(FunctionRole::New));
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert!(signature.parameters.is_empty());
        assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "$");
    });
}

#[test]
fn test_parse_function_new_type_with_generic_arguments() {
    let mut test = TestParser::new("new <T>(x: int32) => T");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    // new <T>(x: int32) => T
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert!(name.is_none());
        // new
        assert_eq!(signature.role, Some(FunctionRole::New));
        assert_eq!(signature.form, FunctionForm::Lambda);
        let generic_parameters = &signature.generic_parameters;
        // <T>
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "T");
            assert!(constraint.is_none());
        });
        // x: int32
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
            });
        });
        // T
        assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
    });
}

#[test]
fn test_parse_function_with_where_clause() {
    let mut test = TestParser::new(
        r###"
function foo() => int32 where Guard: Limit {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // function name
        assert_string!(parser, name.expect("expected name").string(), "foo");
        // where Guard: Limit
        let where_clauses = &signature.where_clauses;
        assert_eq!(where_clauses.len(), 1);
        assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Guard");
            assert_expression_path!(parser, parser.tree.get(*right), "Limit");
        });
        // return type
        let ret = signature.return_type.expect("expected return type");
        assert_node!(parser.tree, ret, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
        });
    });
}

#[test]
fn test_parse_function_with_generic_and_dynamic_parameters() {
    let mut test = TestParser::new(
        r"
function compute<Validate: boolean, Precision: uint8>(data: uint8[]) {
    body
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
            assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Boolean);
            });
        });

        // Precision: uint8
        assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, constraint, .. } => {
            assert_string!(parser, *name, "Precision");
            assert_node!(parser.tree, constraint.unwrap(), TypeExpression::Literal { value } => {
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
    let mut test = TestParser::new("abstract\nnew (): T");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();

    // abstract\nnew (): T
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
        assert!(signature.is_abstract);
        assert_eq!(signature.role, Some(FunctionRole::New));
        assert_eq!(signature.form, FunctionForm::Lambda);
        assert!(signature.parameters.is_empty());
        assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
    });
}

#[test]
fn test_parse_function_with_newline_between_generics_and_parameters() {
    let mut test = TestParser::new(
        r"
function h<T>
    (tag: T): T;
            ",
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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

    let generic_parameter_container_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .unwrap();
    assert_eq!(parser.get_span_str(generic_parameter_container_span), "<T>");

    let parameter_container_span = parser
        .tree
        .get_side_span(
            function_id,
            NodeSpanType::Region(NodeSpanRegion::Parameters),
        )
        .unwrap();
    assert_eq!(parser.get_span_str(parameter_container_span), "(tag: T)");
}

#[test]
fn test_parse_function_with_newline_before_return_type_colon() {
    let mut test = TestParser::new_with_language(
        r#"function f<T>(value: T)
  : T {
  return value as never
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test = TestParser::new(
        r"
function transform<in T, out U>(value: T): U {
    value as U
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test = TestParser::new(
        r"
function invariant<in out T>(value: T): T {
    value
}
        ",
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test = TestParser::new("function foo() => (str: string) => boolean {}");
    let mut parser = test.prepare();

    // function foo() => (str: string) => boolean
    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        // foo
        assert_string!(parser, name.expect("expected name").string(), "foo");

        // (str: string) => boolean
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
            assert_eq!(function.parameters.len(), 1);
            // str: string
            assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "str");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });

            // boolean
            assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Boolean);
            });
        });
    });
}

#[test]
fn test_parse_function_return_type_with_generic_arguments() {
    let mut test = TestParser::new("function read<T, E>() => AliasBranch<T, E> {}");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test = TestParser::new(
        r#"
async function* foo() => int32 {
    yield 1
    yield 2
    yield 3
}"#,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
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
    let mut test =
        TestParser::new_with_language("function* a() { b.c(yield); }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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

/// Parse anonymous function expression container spans.
#[test]
fn test_parse_function_expression_container_spans() {
    let mut test = TestParser::new_with_language(
        "bar(...items, function() { return 1; });",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // bar(...items, function() { return 1; })
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);

        // function() { return 1; }
        assert_node!(parser.tree, arguments[1], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                let parameter_span = parser
                    .tree
                    .get_side_span(
                        *declaration_id,
                        NodeSpanType::Region(NodeSpanRegion::Parameters),
                    )
                    .unwrap();
                assert_eq!(parser.get_span_str(parameter_span), "()");

                let body_span = parser
                    .tree
                    .get_side_span(*declaration_id, NodeSpanType::Region(NodeSpanRegion::Body))
                    .unwrap();
                assert_eq!(parser.get_span_str(body_span), "{ return 1; }");
            });
        });
    });
}

/// Preserve the enclosing function when a call argument is missing before the block close.
#[test]
fn test_parse_function_body_preserves_declaration_for_missing_call_argument_before_block_close() {
    // source
    let mut test = TestParser::new(
        r#"
function greet(name: string, suffix: string) {}

function main() {
    const userName = "Alice";
    greet(userName,
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(
        &parser,
        &[(None, None, "}"), (Some(NodeType::Expression), None, "}")],
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
    let mut test =
        TestParser::new_with_language("function *a() { yield yield }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
    let mut test =
        TestParser::new_with_language("function *a() { yield *a }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
    let mut test =
        TestParser::new_with_language("function *a() { yield *yield }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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

/// Recover delegated generator yield when a line terminator appears before `*`.
#[test]
fn test_recover_function_generator_delegate_after_newline() {
    // source: function *a(){yield
    // *a}
    let mut test =
        TestParser::new_with_language("function *a(){yield\n*a}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // function *a(){yield
    // *a}
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);

            // { yield \n *a }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 2);
                assert!(block.tail_expression.is_none());

                // yield
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Yield { cardinality, value } => {
                        assert_eq!(*cardinality, YieldCardinality::Scalar);
                        assert!(value.is_none());
                });

                // *a
                assert_node!(parser.tree, block.leading_expressions[1], Expression::Error);
            });
        });
    });
}

/// Recover delegated generator yield without an operand before a following const statement.
#[test]
fn test_recover_function_generator_delegate_before_following_const() {
    // source: function *a(){yield*
    // const value = 1}
    let mut test = TestParser::new_with_language(
        "function *a(){yield*\nconst value = 1}",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

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

/// Parse generator yield in class heritage expression.
#[test]
fn test_parse_function_generator_yield_in_class_heritage() {
    // source: function* a(){(class extends (yield) {});}
    let mut test = TestParser::new_with_language(
        "function* a(){(class extends (yield) {});}",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // function* a(){(class extends (yield) {});}
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            // { (class extends (yield) {}); }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Declaration(class_id) => {
                        assert_node!(parser.tree, *class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
                            assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                                assert_node!(parser.tree, *expression, Expression::Yield { cardinality, value } => {
                                    assert_eq!(*cardinality, YieldCardinality::Scalar);
                                    assert!(value.is_none());
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse generator yield in computed property keys and assignment targets.
#[test]
fn test_parse_function_generator_yield_in_computed_keys() {
    // source: function* a(){(class {[yield](){}})};
    let mut test = TestParser::new_with_language(
        "function* a(){(class {[yield](){}})};",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { .. });
            });
        });
    });

    // source: function* a(){({[yield]:a}=1)}
    let mut test =
        TestParser::new_with_language("function* a(){({[yield]:a}=1)}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert!(signature.is_generator);
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Parenthesized { .. });
            });
        });
    });
}

#[test]
fn test_parse_function_with_nested_lambda_type() {
    let mut test = TestParser::new(
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

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
        assert_string!(parser, name.unwrap().string(), "onResolve");
        // callback: (args) => { .. } | void
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "callback");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
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
                    assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Void);
                    });
                });
            });
        });
        // void
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Void);
        });
    });
}

#[test]
fn test_parse_lambda_return_type_with_optional_parameter_function_type() {
    let mut test = TestParser::new_with_language(
        "(runtime, effect, options: Runtime.RunCallbackOptions<any, any> = {}): (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<any, any> | undefined) => void => 0",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 3);

            // (fiberId?: FiberId.FiberId, options?: Runtime.RunCallbackOptions<any, any> | undefined) => void
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::FunctionTypeDeclaration(function) => {
                assert_eq!(function.parameters.len(), 2);

                // fiberId?: FiberId.FiberId
                assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "fiberId");
                    assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "FiberId.FiberId");
                });

                // options?: Runtime.RunCallbackOptions<any, any> | undefined
                assert_node!(parser.tree, function.parameters[1], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "options");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });

                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });
            });

            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}

#[test]
fn test_parse_lambda_head_boundary_comment_on_function_owner() {
    let mut test =
        TestParser::new_with_language("(x) /* lambda-head */ => x", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    parser.attach_comments();
    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { .. }) => {
        let annotations = parser.tree.get_decorators(function_id.id);
        assert!(annotations.is_empty());
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " lambda-head");
}

#[test]
fn test_parse_lambda_body_boundary_comment_on_body_owner() {
    let mut test =
        TestParser::new_with_language("(x) =>\n// lambda-body\nx", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_expression_path!(parser, parser.tree.get(*body_id), "x");

            let annotations = parser.tree.get_decorators(body_id.id);
            assert!(annotations.is_empty());
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "lambda-body");
}

/// Parse empty parenthesized lambda heads that only contain comments.
#[test]
fn test_parse_empty_parenthesized_lambda_with_comment() {
    let mut test = TestParser::new_with_language("(/* empty */) => {}", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.parameters.is_empty());
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " empty");
}

#[test]
fn test_parse_lambda_comment_only_block_body_attaches_inside_block() {
    let mut test = TestParser::new_with_language("() => {\n  // code\n}", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            let body_span = parser.tree.get_span(*body_id);
            let comment = parser.tree.comments()[0];

            assert_eq!(comment.position, CommentPosition::Leading);
            assert!(comment.span.start >= body_span.start);
            assert!(comment.span.end <= body_span.end);
        });
    });

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "code");
}

#[test]
fn test_parse_function_body_boundary_line_comment_stays_trailing() {
    let mut test =
        TestParser::new_with_language("function f(): void // body\n{}", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let start = parser.span_start();
    let function_id = parser
        .eat_function(&start, DeclarationHeader::default())
        .unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
        let body_span = parser.tree.get_span(*body_id);
        let comment = parser.tree.comments()[0];

        assert_eq!(comment.position, CommentPosition::Trailing);
        assert_eq!(comment.attached_to, 0);
        assert!(comment.span.end <= body_span.start);
    });

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "body");
}

/// Reject direct calls on unparenthesized arrow functions.
#[test]
fn test_reject_unparenthesized_arrow_call() {
    // source: () => {}()
    let mut test = TestParser::new_with_language("() => {}()", LanguageType::Destack);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.flags).unwrap_err();

    // (
    assert_eq!(parser.get_span_str(error.leaf_span()), "(");

    // source: a => {}()
    let mut test = TestParser::new_with_language("a => {}()", LanguageType::Destack);
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.flags).unwrap_err();

    // (
    assert_eq!(parser.get_span_str(error.leaf_span()), "(");
}

/// Parse direct calls on parenthesized arrow functions.
#[test]
fn test_parse_parenthesized_arrow_call() {
    // source: (() => {})()
    let mut test = TestParser::new_with_language("(() => {})()", LanguageType::Destack);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // (() => {})()
    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
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
    let mut test = TestParser::new_with_language("(() => {})()", LanguageType::Destack);
    let mut parser = test.prepare();
    parser.apply_options(ParserOptions {
        trivia_mode: ParserTriviaMode::Full,
        preserve_parenthesized_wrappers: false,
        ..ParserOptions::default()
    });
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // (() => {})()
    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
            });
        });
    });
}
