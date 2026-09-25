use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};
use tspp_dir::{
    CommentKind, Declaration, Declarator, Expression, GenericArgument, InferForm, IntegerType,
    Literal, TupleElement, TypeDeclaration, TypeExpression, TypeLiteral,
};

/// Parse conditional types with infer constraints.
#[test]
fn test_parse_type_conditional_with_infer_constraint() {
    let test = TestParser::new(
        "type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type Wrapper<T> = T extends infer A extends readonly unknown[] ? A : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Infer { constraint, .. } => {
                    assert!(constraint.is_some());
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "A");
                assert_node!(parser.tree, *else_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Never);
                });
            });
        });
    });
}

/// Parse an infer constraint that ends at the template span boundary.
#[test]
fn test_parse_type_conditional_infer_constraint_inside_template_span() {
    let test = TestParser::new("type Parse<T> = T extends `${infer N extends number}` ? N : never");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::TemplateLiteral { spans, .. } => {
                    assert_eq!(spans.len(), 1);
                    assert_node!(parser.tree, spans[0], TypeExpression::Infer { name, constraint, .. } => {
                        assert_string!(parser, name.expect("expected infer name"), "N");
                        assert_node!(parser.tree, constraint.expect("expected infer constraint"), TypeExpression::Keyword { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "N");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse tuple types in conditional type branches.
#[test]
fn test_parse_type_conditional_tuple_then_branch() {
    let test = TestParser::new("type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, else_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Tuple { elements, .. } => {
                    assert_eq!(elements.len(), 2);
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Never);
                });
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse conditional types where infer-extends is a constraint inside parentheses.
#[test]
fn test_parse_type_conditional_infer_extends_parenthesized_constraint() {
    let test = TestParser::new("type X = T extends (infer U extends number) ? 1 : 0");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type X = T extends (infer U extends number) ? 1 : 0
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                crate::assert_parenthesized!(parser.tree, *extends_type, expression => {
                    assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint, .. } => {
                        assert_string!(parser, name.expect("expected infer name"), "U");
                        assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Keyword { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, Literal::Integer(1));
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, Literal::Integer(0));
                });
            });
        });
    });
}

/// Parse conditional types where infer-extends starts a nested conditional.
#[test]
fn test_parse_type_conditional_infer_extends_parenthesized_conditional() {
    let test = TestParser::new("type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type X = T extends (infer U extends number ? 1 : 0) ? 1 : 0
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                crate::assert_parenthesized!(parser.tree, *extends_type, expression => {
                    assert_node!(parser.tree, *expression, TypeExpression::Conditional { left, extends_type: right, then_type, else_type } => {
                        assert_node!(parser.tree, *left, TypeExpression::Infer { name, constraint, .. } => {
                            assert_string!(parser, name.expect("expected infer name"), "U");
                            assert!(constraint.is_none());
                        });
                        assert_node!(parser.tree, *right, TypeExpression::Keyword { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                        assert_node!(parser.tree, *then_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, Literal::Integer(1));
                        });
                        assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, Literal::Integer(0));
                        });
                    });
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, Literal::Integer(1));
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, Literal::Integer(0));
                });
            });
        });
    });
}

/// Parse parenthesized nested conditional types in conditional branches.
#[test]
fn test_parse_type_conditional_with_parenthesized_nested_branch() {
    let test = TestParser::new("type Nested<T> = T extends string ? (T extends \"a\" ? 1 : 2) : 3");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_node!(parser.tree, *extends_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                crate::assert_parenthesized!(parser.tree, *then_type, expression => {
                    assert_node!(parser.tree, *expression, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                        assert_node!(parser.tree, *extends_type, TypeExpression::Literal { value } => {
                            let Literal::String(string_id) = value else {
                                panic!("expected string literal, got {value:?}");
                            };
                            assert_string!(parser, *string_id, "a");
                        });
                        assert_node!(parser.tree, *then_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, Literal::Integer(1));
                        });
                        assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, Literal::Integer(2));
                        });
                    });
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, Literal::Integer(3));
                });
            });
        });
    });
}

#[test]
fn test_parse_type_expression_with_generic_arguments() {
    let test = TestParser::new("type T<A, B>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("type 1 | 2 | 3");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    // type 1 | 2 |3
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 3);
            assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Integer(1));
            });
            assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Integer(2));
            });
            assert_node!(parser.tree, elements[2], TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Integer(3));
            });
        });
    });
}

#[test]
fn test_report_optional_type() {
    let test = TestParser::new("type T = Foo?");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "?");
}

#[test]
fn test_parse_readonly_type_expression() {
    let test = TestParser::new("readonly T");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("newtype T = int32");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    // newtype T = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, is_nominal, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert!(*is_nominal);
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_inline_object() {
    let test = TestParser::new("type T = A extends B ? {} : { a: string | undefined }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = A extends B | C ? D : E");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = A extends B & C ? D : E");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = A extends B | C");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends B | C
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Extends { left, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *right, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
            });
        });
    });
}

/// Parse conditional types with function right-hand sides.
#[test]
fn test_parse_conditional_type_with_function_right() {
    let test = TestParser::new("type T = A extends (x: number) => unknown ? C : D");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends (x: number) => unknown ? C : D
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Function(function) => {
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
    let test =
        TestParser::new("type T = A extends abstract new (x: number) => infer U ? U : unknown");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends abstract new (x: number) => infer U ? U : unknown
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *extends_type, TypeExpression::Constructor(function) => {
                    assert!(function.is_abstract);
                    assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Infer { name, constraint, .. } => {
                        assert_string!(parser, name.expect("expected infer name"), "U");
                        assert!(constraint.is_none());
                    });
                });
                assert_expression_path!(parser, parser.tree.get(*then_type), "U");
                assert_node!(parser.tree, *else_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_multiline_abstract_construct_signature() {
    let test =
        TestParser::new("type T = A extends abstract\n  new (x: number) => infer U ? U : unknown");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends abstract\nnew (x: number) => infer U ? U : unknown
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Constructor(function) => {
                    assert!(function.is_abstract);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_union_with_construct_signature() {
    let test = TestParser::new("type T = RegExp | (new() => object)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = RegExp | (new() => object)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "RegExp");
                crate::assert_parenthesized!(parser.tree, elements[1], expression => {
                    assert_node!(parser.tree, *expression, TypeExpression::Constructor(function) => {
                        assert_eq!(function.parameters.len(), 0);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_conditional_type_with_semicolon_terminated_properties() {
    let test = TestParser::new("type T = X extends Y ? {} : { a: string | undefined; b: number; }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new(
        r#"type T = A extends B ?
C extends D ? E : F
: G"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends B ? C extends D ? E : F : G
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Conditional { .. });
            });
        });
    });
}

/// Parse a long right-associative conditional type ladder.
#[test]
fn test_parse_long_type_conditional_ladder() {
    let mut source = String::from("type T = ");

    for index in 0..2_100 {
        source.push_str(&format!("T extends Case{index} ? Result{index} : "));
    }

    source.push_str("never");

    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { .. });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse extends with readonly array types.
#[test]
fn test_parse_type_extends_readonly_array() {
    let test = TestParser::new("type T = A extends readonly unknown[]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = A extends readonly unknown[]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Extends { left, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert_node!(parser.tree, *right, TypeExpression::Readonly { .. });
            });
        });
    });
}

/// Parse multiline conditional types with readonly array constraints.
#[test]
fn test_parse_type_conditional_multiline_readonly_array() {
    let test = TestParser::new(
        r#"type IsTuple<T> = T extends readonly unknown[]
  ? number extends T["length"]
? false
: true
  : false"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = Z & { a: string | undefined }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
fn test_parse_tuple_type() {
    let test = TestParser::new("type T = (string, number)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (string, number)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_infer_with_constraint() {
    let test = TestParser::new("type T = infer U extends V");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = infer U extends V
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint, .. } => {
                assert_string!(parser, name.expect("expected infer name"), "U");
                assert_expression_path!(parser, parser.tree.get(constraint.unwrap()), "V");
            });
        });
    });
}

#[test]
fn test_parse_type_infer_constraint_with_boundary_comment() {
    let source = "type T = infer U extends // infer-bound\nV";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { constraint, .. } => {
                let constraint = constraint.expect("expected infer constraint");
                assert_expression_path!(parser, parser.tree.get(constraint), "V");
            });
        });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "infer-bound");
}

#[test]
fn test_parse_type_infer_with_wildcard() {
    let test = TestParser::new("type T = infer _");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = infer _
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Infer);
                assert!(name.is_none());
                assert!(constraint.is_none());
            });
        });
    });
}

#[test]
fn test_parse_type_infer_anonymous_constraint() {
    let test = TestParser::new("type T = infer _ extends V");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = infer _ extends V
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Infer);
                assert!(name.is_none());
                assert_expression_path!(parser, parser.tree.get(constraint.unwrap()), "V");
            });
        });
    });
}

#[test]
fn test_parse_type_infer_hole() {
    let test = TestParser::new("type T = _");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = _
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Hole);
                assert!(name.is_none());
                assert!(constraint.is_none());
            });
        });
    });
}

#[test]
fn test_parse_type_infer_hole_as_generic_argument() {
    let test = TestParser::new("type T = Box<_>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = Box<_>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { generic_arguments, .. } => {
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                        assert_eq!(*form, InferForm::Hole);
                        assert!(name.is_none());
                        assert!(constraint.is_none());
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_infer_hole_as_declarator_type() {
    let test = TestParser::new("let value: _ = load()");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // let value: _ = load()
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { ty: Some(ty), .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Hole);
                assert!(name.is_none());
                assert!(constraint.is_none());
            });
        });
    });
}

/// Parse constrained infer operands in unions and intersections without absorbing the operator into the constraint.
#[test]
fn test_parse_type_conditional_with_parenthesized_constrained_infer_binary_operand() {
    let test = TestParser::new(
        r#"type X<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type X<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    crate::assert_parenthesized!(parser.tree, elements[0], expression => {
                        assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint, .. } => {
                            assert_string!(parser, name.expect("expected infer name"), "U");
                            assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Keyword { value } => {
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
    let test = TestParser::new(
        r#"type Y<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type Y<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    crate::assert_parenthesized!(parser.tree, elements[0], expression => {
                        assert_node!(parser.tree, *expression, TypeExpression::Infer { name, constraint, .. } => {
                            assert_string!(parser, name.expect("expected infer name"), "U");
                            assert_node!(parser.tree, constraint.expect("expected constraint"), TypeExpression::Keyword { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                        });
                    });
                });
            });
        });
    });
}
