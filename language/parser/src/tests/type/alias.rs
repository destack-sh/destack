use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_dir::*;
use destack_source::LanguageType;

#[test]
fn test_parse_type_alias() {
    let mut test = TestParser::new("type T = int32");
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);
    let expr_id = expressions[0];

    // type T = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
        });
    });
}

#[test]
fn test_parse_declare_type_alias_kind() {
    let mut test = TestParser::new_with_language(
        "declare type T = string",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    let expression_id = parser.unwrap_label_expression(expressions[0]);

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, is_ambient, value, .. }) => {
            assert_eq!(*is_ambient, true);
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    });
}

/// Parse a type alias followed by a tree literal.
#[test]
fn test_parse_type_alias_before_tree_literal() {
    let mut test = TestParser::new_with_language(
        "type X = typeof Array\n<div>a</div>;",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 2);

    // type X = typeof Array
    assert_node!(parser.tree, expressions[0], Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, .. }) => {
            assert_string!(parser, name.string(), "X");
        });
    });

    // <div>a</div>
    let tree_id = expressions[1];
    assert_node!(parser.tree, tree_id, Expression::TreeExpression { .. } => {});
}

#[test]
fn test_parse_bigint_literal_type() {
    let mut test = TestParser::new_with_language("let x: 0n;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let declarator = parser.tree.get(declarators[0]);
        let ty = declarator.ty.expect("expected type");
        assert_node!(parser.tree, ty, TypeExpression::ScalarLiteral { value } => {
            assert_eq!(*value, ScalarLiteral::Bigint(0));
        });
    });
}

/// Parse `this` in a type alias.
#[test]
fn test_parse_this_type_alias() {
    let mut test = TestParser::new("type Builder = this");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type Builder = this
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Builder");
            assert_node!(parser.tree, *value, TypeExpression::This);
        });
    });
}

#[test]
fn test_parse_type_alias_with_generic_parameters() {
    let mut test = TestParser::new("type T<A, B> = isize");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type T<A, B> = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, generic_parameters, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Pointer { is_signed: true }));
            });
            assert_eq!(generic_parameters.len(), 2);
        });
    });
}

/// Parse a type alias when `>` and `=` are adjacent.
#[test]
fn test_parse_type_alias_with_generic_parameters_without_spacing() {
    let mut test = TestParser::new_with_language("type T<U>=U;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T<U>=U
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, generic_parameters, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "U");
                assert!(generic_arguments.is_empty());
            });
            assert_eq!(generic_parameters.len(), 1);
        });
    });
}

/// Parse a type alias with an explicit empty generic list.
#[test]
fn test_parse_type_alias_with_empty_generic_parameters() {
    let mut test = TestParser::new_with_language("type Box<> = string;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type Box<> = string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, generic_parameters, .. }) => {
            assert_string!(parser, name.string(), "Box");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
            assert!(generic_parameters.is_empty());
        });
    });
}

/// Parse parenthesized multiline unions with a leading separator and comments.
#[test]
fn test_parse_type_alias_parenthesized_multiline_union_with_comment() {
    let mut test = TestParser::new_with_language(
        r#"type Schema = (
  | // leading separator comment
  {
  anyOf: readonly string[]
}
  | {
  oneOf: readonly string[]
}
)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Schema");
            assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_alias_parenthesized_missing_close_parenthesis() {
    let mut test = TestParser::new("type T = (string");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::TypeExpression), None, "")]);

    // type T = (string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
}

/// Parse pointer types in a type alias.
#[test]
fn test_parse_pointer_type_alias() {
    let mut test = TestParser::new("type Ptr = *int32");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Ptr");
            assert_node!(parser.tree, *value, TypeExpression::PointerOf { mutability, target_type } => {
                assert_eq!(*mutability, Some(Mutability::Mutable));
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                    }));
                });
            });
        });
    });
}

/// Parse borrowed reference types in a type alias.
#[test]
fn test_parse_borrowed_reference_type_alias() {
    let mut test = TestParser::new("type Borrowed = &Buffer");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Borrowed");
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { mutability, variance, target_type } => {
                assert_eq!(*mutability, Some(Mutability::Mutable));
                assert!(variance.is_none());
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Buffer");
                });
            });
        });
    });
}

/// Parse readonly borrowed reference types in a type alias.
#[test]
fn test_parse_readonly_borrowed_reference_type_alias() {
    let mut test = TestParser::new("type Borrowed = &readonly Buffer");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Borrowed");
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { mutability, variance, target_type } => {
                assert_eq!(*mutability, Some(Mutability::Immutable));
                assert!(variance.is_none());
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Buffer");
                });
            });
        });
    });
}

#[test]
fn test_parse_borrowed_reference_type_alias_before_union() {
    let mut test = TestParser::new("type MaybeBorrowed = &int32 | undefined");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], TypeExpression::BorrowedOf { target_type, .. } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
                    });
                });

                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_pointer_type_alias_before_union() {
    let mut test = TestParser::new("type MaybePointer = *int32 | undefined");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], TypeExpression::PointerOf { target_type, .. } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
                    });
                });

                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_parameter_function_constraint() {
    let mut test = TestParser::new("type Parameters<T extends (a: any) => any> = T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // type Parameters<T extends (a: any) => any> = T
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { generic_parameters, .. }) => {
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { constraint: Some(constraint), .. } => {
                assert_node!(parser.tree, *constraint, TypeExpression::FunctionTypeDeclaration(function) => {
                    assert_eq!(function.parameters.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_parameter_default_conditional() {
    let mut test = TestParser::new(
        "type Wrapper<F extends Function, ReturnType = F extends (...args: any) => infer T ? T : unknown> = ReturnType",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type Wrapper<F extends Function, ReturnType = F extends (...args: any) => infer T ? T : unknown> = ReturnType
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { generic_parameters, .. }) => {
            assert_eq!(generic_parameters.len(), 2);
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, default: Some(default), .. } => {
                assert_string!(parser, *name, "ReturnType");
                assert_node!(parser.tree, *default, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "F");
                    assert_node!(parser.tree, *extends_type, TypeExpression::FunctionTypeDeclaration(function) => {
                        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Infer { name, constraint, .. } => {
                            assert_string!(parser, name.expect("expected infer name"), "T");
                            assert!(constraint.is_none());
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "T");
                    assert_node!(parser.tree, *else_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Unknown);
                    });
                });
            });
        });
    });
}
