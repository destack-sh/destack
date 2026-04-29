use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_function_type_return_conditional() {
    let mut test = TestParser::new(
        "type Getter<T, P> = (target: T, propertyKey: P) => P extends keyof T ? T[P] : any",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Getter<T, P> = (target: T, propertyKey: P) => P extends keyof T ? T[P] : any
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                    assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "P");
                    });
                    assert_node!(parser.tree, *extends_type, TypeExpression::KeyOf { target_type } => {
                        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "T");
                        });
                    });
                    assert_node!(parser.tree, *then_type, TypeExpression::Index { left, index } => {
                        assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "T");
                        });
                        assert_node!(parser.tree, *index, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "P");
                        });
                    });
                    assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
                });
            });
        });
    });
}

/// Parse function type return predicates.
#[test]
fn test_parse_function_type_return_predicate() {
    let mut test = TestParser::new("type Is<T> = (value: any) => value is T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // type Is<T> = (value: any) => value is T
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                assert_eq!(function.parameters.len(), 1);
                assert!(function.this_parameter.is_none());
                assert_node!(parser.tree, function.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                    assert!(!*asserts);
                    assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                    assert_expression_path!(parser, parser.tree.get(target.expect("expected predicate target")), "T");
                });
            });
        });
    });
}

/// Parse function type predicates inside conditional type tests.
#[test]
fn test_parse_function_type_predicate_in_conditional_type() {
    let mut test = TestParser::new(
        "type Guarded<Actual> = Actual extends (value: any, ...args: any[]) => value is infer T ? T : never",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // type Guarded<Actual> = Actual extends (...) => value is infer T ? T : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "Actual");
                assert_node!(parser.tree, *extends_type, TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 2);
                    assert_node!(parser.tree, function.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                        assert!(!*asserts);
                        assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                        assert_node!(parser.tree, target.expect("expected predicate target"), TypeExpression::Infer { name, constraint } => {
                            assert_string!(parser, *name, "T");
                            assert!(constraint.is_none());
                        });
                    });
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "T");
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Never);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_arguments_with_conditional() {
    let mut test = TestParser::new(
        "type Descriptor<P, T> = TypedPropertyDescriptor<P extends keyof T ? T[P] : any>",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Descriptor<P, T> = TypedPropertyDescriptor<P extends keyof T ? T[P] : any>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { generic_arguments, .. } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                            assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                                assert!(generic_arguments.is_empty());
                                assert_path!(parser, *path, "P");
                            });
                            assert_node!(parser.tree, *extends_type, TypeExpression::KeyOf { target_type } => {
                                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                                    assert!(generic_arguments.is_empty());
                                    assert_path!(parser, *path, "T");
                                });
                            });
                            assert_node!(parser.tree, *then_type, TypeExpression::Index { left, index } => {
                                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                                    assert!(generic_arguments.is_empty());
                                    assert_path!(parser, *path, "T");
                                });
                                assert_node!(parser.tree, *index, TypeExpression::Reference { path, generic_arguments } => {
                                    assert!(generic_arguments.is_empty());
                                    assert_path!(parser, *path, "P");
                                });
                            });
                            assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Any);
                            });
                        });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_index_with_conditional() {
    let mut test =
        TestParser::new("type Lookup<Depth> = Foo[Depth extends -1 ? \"done\" : \"recur\"]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Lookup<Depth> = Foo[Depth extends -1 ? "done" : "recur"]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Foo");
                });
                assert_node!(parser.tree, *index, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                    assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "Depth");
                    });
                    assert_node!(parser.tree, *extends_type, TypeExpression::ScalarLiteral { value } => {
                        assert_eq!(*value, ScalarLiteral::Integer(-1));
                    });
                    assert_node!(parser.tree, *then_type, TypeExpression::ScalarLiteral { value } => {
                        let ScalarLiteral::String(then_id) = value else {
                            panic!("expected string literal, got {value:?}");
                        };
                        assert_string!(parser, *then_id, "done");
                    });
                    assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                        let ScalarLiteral::String(else_id) = value else {
                            panic!("expected string literal, got {value:?}");
                        };
                        assert_string!(parser, *else_id, "recur");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_index_access_chain() {
    let mut test = TestParser::new("type T = A[B][C]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = A[B][C]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                assert_expression_path!(parser, parser.tree.get(*index), "C");
                assert_node!(parser.tree, *left, TypeExpression::Index { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert_expression_path!(parser, parser.tree.get(*index), "B");
                });
            });
        });
    });
}

/// Indexed access type inside generic arguments.
#[test]
fn test_parse_generic_with_indexed_access_type() {
    // Foo<T[number]> - indexed access inside generic
    let mut test = TestParser::new("type A = Foo<T[number]>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_eq!(path.segments.len(), 1);
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                            assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                        });
                });
            });
        });
    });
}

/// Indexed access type followed by array suffix.
#[test]
fn test_parse_indexed_access_with_array_suffix() {
    // T[number][]
    let mut test = TestParser::new("type A = T[number][]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // T[number][]
            assert_node!(parser.tree, *value, TypeExpression::Array { element } => {
                // T[number]
                assert_node!(parser.tree, *element, TypeExpression::Index { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "T");
                    assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

/// Generic type with indexed access, followed by array suffix.
#[test]
fn test_parse_generic_indexed_access_with_array_suffix() {
    // Foo<T[number]>[]
    let mut test = TestParser::new("type A = Foo<T[number]>[]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // Foo<T[number]>[]
            assert_node!(parser.tree, *value, TypeExpression::Array { element } => {
                // Foo<T[number]>
                assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                    assert_eq!(path.segments.len(), 1);
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::Number);
                                });
                            });
                    });
                });
            });
        });
    });
}

/// Parenthesized leading-pipe unions should remain grouped before array suffixes.
#[test]
fn test_parse_parenthesized_leading_pipe_union_with_array_suffix() {
    let mut test = TestParser::new_with_options(
        r#"type Result = (
  | "a"
  | "b"
)[]"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Array { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_associated_projection_with_generic_arguments() {
    let mut test = TestParser::new("type A = Pair<int32, string>.Swap<boolean>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Member { left, name, generic_arguments } => {
                assert_string!(parser, *name, "Swap");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Boolean);
                        });
                });

                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Pair");
                    assert_eq!(generic_arguments.len(), 2);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                                assert!(matches!(value, TypeLiteral::Int(_)));
                            });
                    });
                    assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::String);
                            });
                    });
                });
            });
        });
    });
}

/// Parse a generic type with indexed access in a declaration file.
#[test]
fn test_parse_generic_indexed_access_in_declaration_file() {
    // Foo<T[number]>[]
    let mut test = TestParser::new_with_options(
        "type A = Foo<T[number]>[]",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // Foo<T[number]>[]
            assert_node!(parser.tree, *value, TypeExpression::Array { element } => {
                // Foo<T[number]>
                assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                    assert_eq!(path.segments.len(), 1);
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::Number);
                                });
                            });
                    });
                });
            });
        });
    });
}

/// Nested generic closings should not parse as shift-right operators in type context.
#[test]
fn test_parse_nested_generic_closings_in_type() {
    let mut test = TestParser::new("type A = Foo<Bar<Baz<Qux>>>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_eq!(path.segments.len(), 1);
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert_eq!(path.segments.len(), 1);
                        assert_eq!(generic_arguments.len(), 1);
                        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                assert_eq!(path.segments.len(), 1);
                                assert_path!(parser, *path, "Baz");
                                assert_eq!(generic_arguments.len(), 1);
                                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_expression_path!(parser, parser.tree.get(*value), "Qux");
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Tuple expressions inside generic arguments should parse as a single argument.
#[test]
fn test_parse_tuple_generic_argument() {
    let mut test = TestParser::new_with_options(
        "type A = And<[Left, Right]>",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "And");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                        assert_eq!(elements.len(), 2);
                        assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "Left");
                        });
                        assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "Right");
                        });
                    });
                });
            });
        });
    });
}

/// Parenthesized union expressions inside generic arguments should stay grouped.
#[test]
fn test_parse_parenthesized_union_generic_argument() {
    let mut test = TestParser::new_with_options(
        "type Alias = Wrap<(number | string)>;",
        LanguageType::Destack,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Wrap");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, TypeExpression::Union { elements } => {
                            assert_eq!(elements.len(), 2);
                                assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::Number);
                                });
                                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::String);
                                });
                            });
                        });
                });
            });
        });
    });
}

/// Comptime value expressions inside generic arguments stay in value space.
#[test]
fn test_parse_value_expression_generic_argument() {
    let mut test =
        TestParser::new_with_options("type Alias = Buffer<1 + 2>", LanguageType::Destack);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Alias = Buffer<1 + 2>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Buffer");
                assert_eq!(generic_arguments.len(), 1);

                // 1 + 2
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Value { value } => {
                    assert_node!(parser.tree, *value, Expression::Binary { left, operator, right } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
                            assert_eq!(*value, 1);
                        });
                        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
                            assert_eq!(*value, 2);
                        });
                    });
                });
            });
        });
    });
}

/// Nested generic arguments inside tuple generic arguments should stay grouped.
#[test]
fn test_parse_tuple_generic_argument_with_nested_generics() {
    let mut test = TestParser::new_with_options(
        r#"type A<Actual> = And<[Extends<PrintType<Actual>, "...">, Not<IsAny<Actual>>]>;"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { generic_arguments, .. } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

/// Tuple generic arguments inside a conditional type should stay grouped.
#[test]
fn test_parse_tuple_generic_argument_in_type_conditional() {
    let mut test = TestParser::new_with_options(
        r#"type A<Actual, Expected> = And<[Extends<PrintType<Actual>, "...">, Not<IsAny<Actual>>]> extends true ? Actual : Expected;"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, .. } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { generic_arguments, .. } => {
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                            assert_eq!(elements.len(), 2);
                        });
                    });
                });
            });
        });
    });
}

/// Readonly prefix with generic containing indexed access type.
#[test]
fn test_parse_readonly_generic_indexed_access() {
    // readonly Foo<T[number]>
    let mut test = TestParser::new_with_options(
        "type A = readonly Foo<T[number]>",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // readonly Foo<T[number]>
            assert_node!(parser.tree, *value, TypeExpression::Readonly { target_type } => {
                // Foo<T[number]>
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert_eq!(path.segments.len(), 1);
                    assert_eq!(generic_arguments.len(), 1);
                    // T[number]
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "T");
                                assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                                    assert_eq!(*value, TypeLiteral::Number);
                                });
                            });
                    });
                });
            });
        });
    });
}

/// Readonly prefix with generic containing indexed access and array suffix.
#[test]
fn test_parse_readonly_generic_indexed_access_array() {
    // readonly Foo<T[number]>[]
    let mut test = TestParser::new_with_options(
        "type A = readonly Foo<T[number]>[]",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // readonly Foo<T[number]>[]
            assert_node!(parser.tree, *value, TypeExpression::Readonly { target_type } => {
                // Foo<T[number]>[]
                assert_node!(parser.tree, *target_type, TypeExpression::Array { element } => {
                    // Foo<T[number]>
                    assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                        assert_eq!(path.segments.len(), 1);
                        assert_eq!(generic_arguments.len(), 1);
                        // T[number]
                        assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "T");
                                    assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                                        assert_eq!(*value, TypeLiteral::Number);
                                    });
                                });
                        });
                    });
                });
            });
        });
    });
}

/// Complex conditional type from deno builtins with nested indexed access.
#[test]
fn test_parse_deno_conditional_indexed_access() {
    let mut test = TestParser::new_with_options(
        r#"type ToNativeParameterTypes<T extends readonly NativeType[]> =
[T[number][]] extends [T] ? ToNativeType<T[number]>[]
  : [readonly T[number][]] extends [T] ? readonly ToNativeType<T[number]>[]
  : never"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { generic_parameters, value, .. }) => {
            let params = generic_parameters;
            assert_eq!(params.len(), 1);
            assert_node!(parser.tree, *value, TypeExpression::Conditional { .. });
        });
    });
}

#[test]
fn test_reject_type_predicate_alias_expression() {
    let mut test = TestParser::new("type T = value is string");
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();
    let (span, node_type, expected) = error.leaf_content();

    assert_eq!(
        (node_type, expected, parser.get_span_str(span)),
        (None, None, "is")
    );
}

#[test]
fn test_parse_this_type_predicate_alias_expression() {
    let mut test = TestParser::new("type T = this is Foo");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Predicate { asserts, subject, target } => {
                assert!(!asserts);
                assert_eq!(*subject, TypePredicateSubject::This);
                assert_expression_path!(parser, parser.tree.get(target.unwrap()), "Foo");
            });
        });
    });
}

#[test]
fn test_reject_parenthesized_type_predicate_subject() {
    let mut test = TestParser::new("function isString(value: unknown): (value) is string {}");
    let mut parser = test.prepare();
    let error = parser.eat_expression(parser.options).unwrap_err();
    let (span, node_type, expected) = error.leaf_content();

    assert_eq!(
        (node_type, expected, parser.get_span_str(span)),
        (None, None, "(value)")
    );
}
