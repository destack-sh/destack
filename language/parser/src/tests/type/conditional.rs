use crate::tests::*;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse conditional types with infer constraints.
#[test]
fn test_parse_type_conditional_with_infer_constraint() {
    let mut test = TestParser::new(
        "type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Infer { constraint, .. } => {
                    assert!(constraint.is_some());
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "A");
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Never);
                });
            });
        });
    });
}

/// Parse conditional types where infer-extends is a constraint inside parentheses.
#[test]
fn test_parse_type_conditional_infer_extends_parenthesized_constraint() {
    let mut test = TestParser::new_with_language(
        "type X = T extends (infer U extends number) ? 1 : 0",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type X = T extends (infer U extends number) ? 1 : 0
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_node!(parser.tree, *extends_type, TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint } => {
                        assert_string!(parser, *name, "U");
                        assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                });
                assert_node!(parser.tree, *then_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Integer(1));
                });
                assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Integer(0));
                });
            });
        });
    });
}

/// Parse conditional types where infer-extends starts a nested conditional.
#[test]
fn test_parse_type_conditional_infer_extends_parenthesized_conditional() {
    let mut test = TestParser::new_with_language(
        "type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_node!(parser.tree, *extends_type, TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Conditional { left, extends_type: right, then_type, else_type } => {
                        assert_node!(parser.tree, *left, TypeExpression::Infer { name, constraint } => {
                            assert_string!(parser, *name, "U");
                            assert!(constraint.is_none());
                        });
                        assert_node!(parser.tree, *right, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                        assert_node!(parser.tree, *then_type, TypeExpression::ScalarLiteral { value } => {
                            assert_eq!(*value, ScalarLiteral::Integer(1));
                        });
                        assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                            assert_eq!(*value, ScalarLiteral::Integer(0));
                        });
                    });
                });
                assert_node!(parser.tree, *then_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Integer(1));
                });
                assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Integer(0));
                });
            });
        });
    });
}

/// Parse parenthesized nested conditional types in conditional branches.
#[test]
fn test_parse_type_conditional_with_parenthesized_nested_branch() {
    let mut test = TestParser::new_with_language(
        "type Nested<T> = T extends string ? (T extends \"a\" ? 1 : 2) : 3",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_node!(parser.tree, *extends_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                        assert_node!(parser.tree, *extends_type, TypeExpression::ScalarLiteral { value } => {
                            let ScalarLiteral::String(string_id) = value else {
                                panic!("expected string literal, got {value:?}");
                            };
                            assert_string!(parser, *string_id, "a");
                        });
                        assert_node!(parser.tree, *then_type, TypeExpression::ScalarLiteral { value } => {
                            assert_eq!(*value, ScalarLiteral::Integer(1));
                        });
                        assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                            assert_eq!(*value, ScalarLiteral::Integer(2));
                        });
                    });
                });
                assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Integer(3));
                });
            });
        });
    });
}

#[test]
fn test_parse_type_expression_with_generic_arguments() {
    let mut test = TestParser::new("type T<A, B>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type T<A, B>
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "T");
            assert_eq!(generic_arguments.len(), 2);
        });
    });
}

#[test]
fn test_parse_type_expression() {
    let mut test = TestParser::new("type 1 | 2 | 3");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type 1 | 2 |3
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 3);
            assert_node!(parser.tree, elements[0], TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(1));
            });
            assert_node!(parser.tree, elements[1], TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(2));
            });
            assert_node!(parser.tree, elements[2], TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(3));
            });
        });
    });
}

#[test]
fn test_parse_optional_type_rejected() {
    let mut test = TestParser::new("type T = Foo?");
    let mut parser = test.prepare();
    assert!(parser.eat_expression(parser.flags).is_err());
}

#[test]
fn test_parse_readonly_type_expression() {
    let mut test = TestParser::new("readonly T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // readonly T
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Readonly { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
    });
}

#[test]
fn test_parse_newtype_type_expression() {
    let mut test = TestParser::new("newtype T = int32");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // newtype T = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, is_nominal, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert!(*is_nominal);
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_inline_object() {
    let mut test = TestParser::new("type T = A extends B ? {} : { a: string | undefined }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends B ? {} : { a: string | undefined }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, else_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Object { members: properties } => {
                    assert!(properties.is_empty());
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_union_right() {
    let mut test = TestParser::new("type T = A extends B | C ? D : E");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends B | C ? D : E
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *extends_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "D");
                assert_expression_path!(parser, parser.tree.get(*else_type), "E");
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_intersection_right() {
    let mut test = TestParser::new("type T = A extends B & C ? D : E");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends B & C ? D : E
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *extends_type, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "D");
                assert_expression_path!(parser, parser.tree.get(*else_type), "E");
            });
        });
    });
}

#[test]
fn test_parse_type_extends_with_union_right() {
    let mut test = TestParser::new("type T = A extends B | C");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends B | C
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *extends_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Missing);
                assert_node!(parser.tree, *else_type, TypeExpression::Missing);
            });
        });
    });
}

/// Parse conditional types with function right-hand sides.
#[test]
fn test_parse_conditional_type_with_function_right() {
    let mut test = TestParser::new("type T = A extends (x: number) => any ? C : D");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends (x: number) => any ? C : D
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 1);
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "C");
                assert_expression_path!(parser, parser.tree.get(*else_type), "D");
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_abstract_construct_signature() {
    let mut test =
        TestParser::new("type T = A extends abstract new (x: number) => infer U ? U : unknown");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends abstract new (x: number) => infer U ? U : unknown
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *extends_type, TypeExpression::ConstructorTypeDeclaration(function) => {
                    assert!(function.is_abstract);
                    assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Infer { name, constraint } => {
                        assert_string!(parser, *name, "U");
                        assert!(constraint.is_none());
                    });
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "U");
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_multiline_abstract_construct_signature() {
    let mut test =
        TestParser::new("type T = A extends abstract\n  new (x: number) => infer U ? U : unknown");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends abstract\nnew (x: number) => infer U ? U : unknown
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::ConstructorTypeDeclaration(function) => {
                    assert!(function.is_abstract);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_union_with_construct_signature() {
    let mut test = TestParser::new("type T = RegExp | (new() => object)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = RegExp | (new() => object)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "RegExp");
                assert_node!(parser.tree, elements[1], TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::ConstructorTypeDeclaration(function) => {
                        assert_eq!(function.parameters.len(), 0);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_semicolon_terminated_properties() {
    let mut test =
        TestParser::new("type T = X extends Y ? {} : { a: string | undefined; b: number; }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = X extends Y ? {} : { a: string | undefined; b: number; }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { else_type, .. } => {
                assert_node!(parser.tree, *else_type, TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 2, "expected 2 properties but got {}", properties.len());
                });
            });
        });
    });
}

#[test]
fn test_parse_type_conditional_multiline_nested() {
    let mut test = TestParser::new(
        r#"type T = A extends B ?
C extends D ? E : F
: G"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends B ? C extends D ? E : F : G
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Conditional { .. });
            });
        });
    });
}

/// Parse extends with readonly array types.
#[test]
fn test_parse_type_extends_readonly_array() {
    let mut test = TestParser::new("type T = A extends readonly unknown[]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = A extends readonly unknown[]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Readonly { .. });
                assert_node!(parser.tree, *then_type, TypeExpression::Missing);
                assert_node!(parser.tree, *else_type, TypeExpression::Missing);
            });
        });
    });
}

/// Parse multiline conditional types with readonly array constraints.
#[test]
fn test_parse_type_conditional_multiline_readonly_array() {
    let mut test = TestParser::new(
        r#"type IsTuple<T> = T extends readonly unknown[]
  ? number extends T["length"]
? false
: true
  : false"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type IsTuple<T> = T extends readonly unknown[] ? number extends T["length"] ? false : true : false
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Conditional { .. });
            });
        });
    });
}

#[test]
fn test_parse_type_intersection_with_inline_object() {
    let mut test = TestParser::new("type T = Z & { a: string | undefined }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = Z & { a: string | undefined }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_array_tuple_type() {
    let mut test = TestParser::new("type T = [string, number]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [string, number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_infer_with_constraint() {
    let mut test = TestParser::new("type T = infer U extends V");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = infer U extends V
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint } => {
                assert_string!(parser, *name, "U");
                assert_expression_path!(parser, parser.tree.get(constraint.unwrap()), "V");
            });
        });
    });
}

#[test]
fn test_parse_type_infer_constraint_with_boundary_comment() {
    let source = "type T = infer U extends // infer-bound\nV";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { constraint, .. } => {
                let constraint = constraint.expect("expected infer constraint");
                assert_expression_path!(parser, parser.tree.get(constraint), "V");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "infer-bound");
}

#[test]
fn test_parse_type_infer_with_wildcard() {
    let mut test = TestParser::new("type T = infer _");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = infer _
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint } => {
                assert_string!(parser, *name, "_");
                assert!(constraint.is_none());
            });
        });
    });
}

/// Parse constrained infer operands in unions and intersections without absorbing the operator into the constraint.
#[test]
fn test_parse_type_conditional_with_parenthesized_constrained_infer_binary_operand() {
    let mut test = TestParser::new_with_language(
        r#"type X<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type X<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], TypeExpression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint } => {
                            assert_string!(parser, *name, "U");
                            assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                        });
                    });
                    assert_node!(parser.tree, elements[1], TypeExpression::Object { .. });
                });
            });
        });
    });
}

/// Parse constrained infer operands in intersections without absorbing the operator into the constraint.
#[test]
fn test_parse_type_conditional_with_parenthesized_constrained_infer_intersection_operand() {
    let mut test = TestParser::new_with_language(
        r#"type Y<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type Y<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], TypeExpression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint } => {
                            assert_string!(parser, *name, "U");
                            assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                        });
                    });
                });
            });
        });
    });
}
