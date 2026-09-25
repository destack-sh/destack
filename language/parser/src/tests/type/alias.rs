use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{Parser, assert_expression_path, assert_node, assert_path, assert_string};
use tspp_dir::{
    Access, Declaration, Expression, GenericParameter, IntegerType, Literal, LocalNodeId,
    Mutability, NodeType, TokenType, TypeDeclaration, TypeExpression, TypeLiteral,
};

#[test]
fn test_parse_type_alias() {
    let test = TestParser::new("type T = int32");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    assert_eq!(expressions.len(), 1);
    let expr_id = expressions[0];

    // type T = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
        });
    });
}

#[test]
fn test_parse_declare_type_alias_kind() {
    let test = TestParser::declaration("declare type T = string");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    let expression_id = expressions[0];

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, is_ambient, value, .. }) => {
            assert!(*is_ambient);
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    });
}

/// Parse a type alias followed by a tree literal.
#[test]
fn test_parse_type_alias_before_tree_literal() {
    let test = TestParser::new("type X = typeof Array\n<div>a</div>;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
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
    let test = TestParser::new("let x: 0n;");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let declarator = parser.tree.get(declarators[0]);
        let ty = declarator.ty.expect("expected type");
        assert_node!(parser.tree, ty, TypeExpression::Literal { value } => {
            assert_eq!(*value, Literal::Bigint(0));
        });
    });
}

/// Parse `this` in a type alias.
#[test]
fn test_parse_this_type_alias() {
    let test = TestParser::new("type Builder = this");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    // type Builder = this
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Builder");
            assert_node!(parser.tree, *value, TypeExpression::This);
            let main_span = parser
                .tree
                .get_main_span(*value)
                .expect("expected this type main span");
            assert_eq!(parser.span_str(main_span), "this");
        });
    });
}

#[test]
fn test_parse_type_alias_with_generic_parameters() {
    let test = TestParser::new("type T<A, B> = isize");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    // type T<A, B> = int32
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, generic_parameters, .. }) => {
            assert_string!(parser, name.string(), "T");
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Pointer { is_signed: true }));
            });
            assert_eq!(generic_parameters.len(), 2);
        });
    });
}

/// Parse a type alias when `>` and `=` are adjacent.
#[test]
fn test_parse_type_alias_with_generic_parameters_without_spacing() {
    let test = TestParser::new("type T<U>=U;");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type Box<> = string;");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type Box<> = string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, generic_parameters, .. }) => {
            assert_string!(parser, name.string(), "Box");
            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
            assert!(generic_parameters.is_empty());
        });
    });
}

/// Recover later declarations after damaged type alias values.
#[test]
fn test_recover_type_alias_value_declaration_boundaries() {
    let cases = [
        "type Broken = string | ;\ntype Recovered = string;",
        "type Broken = { readonly key: ;\ntype Recovered = string;",
        "type Broken = [head: string, ... ;\ntype Recovered = string;",
        "type Broken<T> = T extends ;\ntype Recovered = string;",
        "type Broken<T> = { [Key in keyof ;\ntype Recovered = string;",
        "type Broken = (value: ;\ntype Recovered = string;",
        "type Broken<T> = T[ ;\ntype Recovered = string;",
    ];

    for source in cases {
        let test = TestParser::new(source);
        let mut parser = test.prepare();
        let roots = parser.parse_in_place();

        assert!(
            !parser.errors.is_empty(),
            "expected recovery diagnostics for {source:?}",
        );
        let names = declaration_names(&parser, &roots);
        assert!(
            names.iter().any(|name| name == "Recovered"),
            "expected recovered declaration root for {source:?}, got {names:?} with errors {:#?}",
            parser.errors,
        );
    }
}

/// Parse parenthesized multiline unions with a leading separator and comments.
#[test]
fn test_parse_type_alias_parenthesized_multiline_union_with_comment() {
    let test = TestParser::new(
        r#"type Reflect = (
  | // leading separator comment
  {
  anyOf: readonly string[]
}
  | {
  oneOf: readonly string[]
}
)"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Reflect");
            crate::assert_parenthesized!(parser.tree, *value, expression => {
                assert_node!(parser.tree, *expression, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_alias_parenthesized_missing_close_parenthesis() {
    let test = TestParser::new("type T = (string");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::TypeExpression),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );

    // type T = (string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            crate::assert_parenthesized!(parser.tree, *value, expression => {
                assert_node!(parser.tree, *expression, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
}

/// Parse pointer types in a type alias.
#[test]
fn test_parse_pointer_type_alias() {
    let test = TestParser::new("type Ptr = *int32");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Ptr");
            assert_node!(parser.tree, *value, TypeExpression::PointerOf { mutability, target_type } => {
                assert_eq!(*mutability, Some(Mutability::Mutable));
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
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
    let test = TestParser::new("type Borrowed = &Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Borrowed");
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime: None, access, variance, target_type } => {
                assert_eq!(*access, Some(Access::Mutable));
                assert!(variance.is_none());
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Buffer");
                });
            });
        });
    });
}

/// Parse compact nested borrowed reference types in a type alias.
#[test]
fn test_parse_borrowed_reference_type_chain_compact() {
    let test = TestParser::new("type Borrowed = &&Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Borrowed");
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime: None, access, variance, target_type } => {
                assert_eq!(*access, Some(Access::Mutable));
                assert!(variance.is_none());
                assert_node!(parser.tree, *target_type, TypeExpression::BorrowedOf { lifetime: None, access, variance, target_type } => {
                    assert_eq!(*access, Some(Access::Mutable));
                    assert!(variance.is_none());
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "Buffer");
                    });
                });
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse readonly borrowed reference types in a type alias.
#[test]
fn test_parse_readonly_borrowed_reference_type_alias() {
    let test = TestParser::new("type Borrowed = &readonly Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "Borrowed");
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime: None, access, variance, target_type } => {
                assert_eq!(*access, Some(Access::Readonly));
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
    let test = TestParser::new("type MaybeBorrowed = &int32 | undefined");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], TypeExpression::BorrowedOf { target_type, .. } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
                    });
                });

                assert_node!(parser.tree, elements[1], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_pointer_type_alias_before_union() {
    let test = TestParser::new("type MaybePointer = *int32 | undefined");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                assert_node!(parser.tree, elements[0], TypeExpression::PointerOf { target_type, .. } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
                    });
                });

                assert_node!(parser.tree, elements[1], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_parameter_function_constraint() {
    let test = TestParser::new("type Parameters<T: (a: unknown) => unknown> = T");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();
    // type Parameters<T: (a: unknown) => unknown> = T
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { generic_parameters, .. }) => {
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { constraint: Some(constraint), .. } => {
                assert_node!(parser.tree, *constraint, TypeExpression::Function(function) => {
                    assert_eq!(function.parameters.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_parameter_default_conditional() {
    let test = TestParser::new(
        "type Wrapper<F: Function, ReturnType = F extends (...args: unknown) => infer T ? T : unknown> = ReturnType",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type Wrapper<F: Function, ReturnType = F extends (...args: unknown) => infer T ? T : unknown> = ReturnType
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { generic_parameters, .. }) => {
            assert_eq!(generic_parameters.len(), 2);
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, default: Some(default), .. } => {
                assert_string!(parser, *name, "ReturnType");
                assert_node!(parser.tree, *default, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "F");
                    assert_node!(parser.tree, *extends_type, TypeExpression::Function(function) => {
                        assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Infer { name, constraint, .. } => {
                            assert_string!(parser, name.expect("expected infer name"), "T");
                            assert!(constraint.is_none());
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(*then_type), "T");
                    assert_node!(parser.tree, *else_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Unknown);
                    });
                });
            });
        });
    });
}

/// Return root type declaration names in source order.
fn declaration_names(parser: &Parser, roots: &[LocalNodeId<Expression>]) -> Vec<String> {
    roots
        .iter()
        .filter_map(|root| match parser.tree.get(*root) {
            Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
                Declaration::Type(declaration) => {
                    Some(parser.strings.get(declaration.name.string()).to_string())
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}
