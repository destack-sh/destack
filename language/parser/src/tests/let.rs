use destack_dir::{
    Asynchrony, Declaration, Declarator, Expression, FloatType, FunctionDeclaration, FunctionForm,
    GenericArgument, GenericParameter, IntegerType, Key, LetKind, Name, Parameter, Pattern,
    PatternField, ScalarLiteral, TypeExpression, TypeLiteral, TypeMember,
};
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{
    TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
};

#[test]
fn test_parse_let_scalar() {
    let mut test = TestParser::new(
        r###"
const x: int32 = 1
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            let operator_span = parser
                .tree
                .get_main_span(declarators[0])
                .expect("expected declarator operator span");
            assert_eq!(parser.get_span_str(operator_span), "=");

            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });

            // int32
            let ty_id = ty.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, TypeExpression::Literal { value: TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }) });

            // 1
            let value_id = value.expect("expected value");
            assert_node!(parser.tree, value_id, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_shared_const_binding() {
    let mut test = TestParser::new(
        r###"
class Registry {}

shared const registry: Registry = new Registry();
"###,
    );
    let mut parser = test.prepare();

    let roots = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(roots.len(), 2);

    assert_node!(parser.tree, roots[1], Expression::Let { kind, is_shared, declarators, .. } => {
        assert_eq!(*kind, LetKind::Const);
        assert!(*is_shared);
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "registry");
            });

            let ty = ty.expect("expected type annotation");
            assert_expression_path!(parser, parser.tree.get(ty), "Registry");

            let value = value.expect("expected initializer");
            assert_node!(parser.tree, value, Expression::New { .. });
        });
    });
}

#[test]
fn test_parse_typescript_shared_const_keeps_shared_identifier() {
    let mut test =
        TestParser::new_with_language("shared const value = 1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_expression_path!(parser, parser.tree.get(expression_id), "shared");

    let next_span = parser.peek().unwrap().span;
    assert_eq!(parser.get_span_str(next_span), "const");
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_let_type_annotation_newline() {
    let mut test = TestParser::new_with_language(
        r###"
const constants:
    & typeof Foo
    & typeof Bar
"###,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { ty, value, .. } => {
            assert!(value.is_none());
            let ty_id = ty.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
            });
        });
    });
}

#[test]
fn test_parse_let_recovers_missing_type_annotation_value() {
    let mut test = TestParser::new("const value: ");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // const value:
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "value");
            });

            let ty = ty.expect("expected recovered type");
            assert_node!(parser.tree, ty, TypeExpression::Missing);
            assert!(value.is_none());
        });
    });
}

#[test]
fn test_parse_let_recovers_missing_type_before_initializer() {
    let mut test = TestParser::new("const value: = 1");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // const value: = 1
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "value");
            });

            let ty = ty.expect("expected recovered type");
            assert_node!(parser.tree, ty, TypeExpression::Missing);

            let value = value.expect("expected initializer");
            assert_node!(parser.tree, value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_let_recovers_missing_initializer_value() {
    let mut test = TestParser::new("const value = ");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // const value =
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "value");
            });

            assert!(ty.is_none());
            let value = value.expect("expected recovered value");
            assert_node!(parser.tree, value, Expression::Missing);
        });
    });
}

#[test]
fn test_parse_using_scalar() {
    let mut test = TestParser::new(
        r###"
using x = open()
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let using_id = parser
        .eat_using(&start, DeclarationHeader::default(), Asynchrony::Sync)
        .unwrap();

    assert_node!(parser.tree, using_id, Expression::Using { asynchrony, declarators, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Sync);
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert!(value.is_some());
        });
    });
}

#[test]
fn test_parse_let_generic_arrow_initializer() {
    let mut test = TestParser::new_with_language(
        "const foo: Tmp = <T,>(str: T): T => { return str; }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // const foo: Tmp = <T,>(str: T): T => { return str; }
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "foo");
            });
            assert_node!(parser.tree, ty.unwrap(), TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Tmp");
            });
            let value_id = value.expect("expected value");
            assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.generic_parameters.len(), 1);
                    assert_node!(parser.tree, signature.generic_parameters[0], GenericParameter::Type { name, .. } => {
                        assert_string!(parser, *name, "T");
                    });
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "T");
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_let_readonly_identifier_with_type_annotation() {
    let mut test = TestParser::new_with_language(
        "const readonly: <A>(value: A) => Readonly<A> = identity",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "readonly");
            });
            assert!(ty.is_some());
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::Identifier { name } => {
                assert_string!(parser, *name, "identity");
            });
        });
    });
}

#[test]
fn test_parse_let_array_pattern_readonly_identifier() {
    let mut test = TestParser::new_with_language(
        "const [readonly, setReadonly] = useState(false)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                assert_eq!(fields.len(), 2);
                assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                    assert_name!(parser, *name, "readonly");
                });
                assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
                    assert_name!(parser, *name, "setReadonly");
                });
            });
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "useState");
            });
        });
    });
}

#[test]
fn test_parse_await_using_scalar() {
    let mut test = TestParser::new(
        r###"
await using conn = openConnection()
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let using_id = parser
        .eat_using(&start, DeclarationHeader::default(), Asynchrony::Async)
        .unwrap();

    assert_node!(parser.tree, using_id, Expression::Using { asynchrony, declarators, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Async);
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_parse_using_multiple_declarators() {
    let mut test = TestParser::new(
        r###"
using a = openA(), b = openB()
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let using_id = parser
        .eat_using(&start, DeclarationHeader::default(), Asynchrony::Sync)
        .unwrap();

    assert_node!(parser.tree, using_id, Expression::Using { declarators, .. } => {
        assert_eq!(declarators.len(), 2);
    });
}

#[test]
fn test_parse_let_array_undefined() {
    let mut test = TestParser::new(
        r###"
let x: float64[3] = undefined
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let x: float64[3] = undefined
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });

            // float64[3]
            let ty_id = ty.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, TypeExpression::Index { left, index } => {
                assert_node!(parser.tree, *left, TypeExpression::Literal { value: TypeLiteral::Float(float_ty) } => {
                    assert_eq!(*float_ty, FloatType::Float64);
                });
                assert_node!(parser.tree, *index, TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(3) });
            });
        });
    });
}

#[test]
fn test_parse_let_tuple_pattern() {
    let mut test = TestParser::new(
        r###"
const (x, y) = foo()
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // const (x, y) = foo()
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            // (x, y)
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields, .. } => {
                assert_eq!(fields.len(), 2);
                // x
                assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                    assert_name!(parser, *name, "x");
                });
                // y
                assert_node!(parser.tree, fields[1], PatternField::Named { name, .. } => {
                    assert_name!(parser, *name, "y");
                });
            });

            assert!(ty.is_none());

            assert!(value.is_some());
        });
    });
}

#[test]
fn test_parse_let_definite_assignment_pattern() {
    let mut test = TestParser::new_with_language("let {}! = {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, let_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Must(inner) => {
                assert_node!(parser.tree, *inner, Pattern::Object { .. } => {});
            });
        });
    });
}

#[test]
fn test_parse_let_implicit_undefined() {
    let mut test = TestParser::new("const x: int32");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let x: int32
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert!(ty.is_some());
            assert!(value.is_none());
        });
    });
}

#[test]
fn test_parse_let_multiline_value() {
    let mut test = TestParser::new(
        r###"
const x =
    foo.parse()
"###,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // const x = foo.parse()
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            // foo.parse()
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, generic_arguments: _, arguments: _ } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo.parse");
            });
        });
    });
}

#[test]
fn test_parse_let_multiline_with_generic_arguments() {
    let mut test = TestParser::new(
        r###"
const registry: Map<
  string,
  Set<type {count: number}>
> = new Map()
"###,
    );
    let mut parser = test.prepare();

    let let_id = parser.eat_expression(parser.flags).unwrap();

    // const registry: Map<..., ...> = new Map()
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 1);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert!(ty.is_some());
            assert!(value.is_some());

            // registry
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "registry");
            });

            // Map<string, Set<type {count: number}>>
            assert_node!(parser.tree, ty.unwrap(), TypeExpression::Reference { path, generic_arguments } => {
                // Map
                assert_path!(parser, *path, "Map");

                // <string, Set<type {count: number}>>
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    // string
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::String });
                });

                assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                    // Set<type {count: number}>
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                            // Set
                            assert_path!(parser, *path, "Set");

                            // <type {count: number}>
                            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                                    assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                                        assert_eq!(properties.len(), 1);
                                        assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), .. } => {
                                            assert_string!(parser, *name, "count");
                                        });
                                    });
                            });
                        });
                });
            });
        });
    });
}

#[test]
fn test_parse_let_multiple_declarators() {
    let mut test = TestParser::new("let a: int32 = 1, b: string = \"hello\"");
    let mut parser = test.prepare();
    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();
    // let a: int32 = 1, b: string = "hello"
    assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability: _, .. } => {
        assert_eq!(declarators.len(), 2);

        // a: int32 = 1
        assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "a");
            });
            assert!(ty.is_some());
            assert!(value.is_some());
        });
        // b: string = "hello"
        assert_node!(parser.tree, declarators[1], Declarator { pattern, ty, value } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "b");
            });
            assert!(ty.is_some());
            assert!(value.is_some());
        });
    });
}

#[test]
fn test_parse_let_declarators_with_leading_comma_newline() {
    let mut test = TestParser::new_with_language(
        r#"let args = new Array(arguments.length - 1)
  , callbacks = this._callbacks['$' + event]"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let args = new Array(arguments.length - 1)
    //   , callbacks = this._callbacks['$' + event]
    assert_node!(parser.tree, let_id, Expression::Let { kind, mutability: _, declarators, .. } => {
        assert_eq!(*kind, LetKind::Let);
        assert_eq!(declarators.len(), 2);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "args");
            });
        });

        assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "callbacks");
            });
        });
    });
}

#[test]
fn test_parse_const_declarators_with_newline_after_keyword() {
    let mut test = TestParser::new_with_language(
        r#"const
  first = 1,
  second = 2"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // const
    //   first = 1,
    //   second = 2
    assert_node!(parser.tree, let_id, Expression::Let { kind, mutability: _, declarators, .. } => {
        assert_eq!(*kind, LetKind::Const);
        assert_eq!(declarators.len(), 2);

        assert_node!(parser.tree, declarators[0], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "first");
            });
        });

        assert_node!(parser.tree, declarators[1], Declarator { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "second");
            });
        });
    });
}

#[test]
fn test_parse_const_declarator_boundary_with_line_terminator_trivia() {
    let mut test = TestParser::new_with_language(
        r#"const result = CreateRecord(IntegerKey, value) /*
*/ return result as never"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let start = parser.span_start();
    let let_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // const result = CreateRecord(IntegerKey, value)
    assert_node!(parser.tree, let_id, Expression::Let { kind, declarators, .. } => {
        assert_eq!(*kind, LetKind::Const);
        assert_eq!(declarators.len(), 1);
    });

    // return result as never
    let return_id = parser.eat_return().unwrap();
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        let value = value.expect("expected return value");
        assert_node!(parser.tree, value, Expression::As { .. } => {
        });
    });
}

#[test]
fn test_parse_let_else_with_block_branch() {
    let mut test = TestParser::new("let { x } = value else { return }");
    let mut parser = test.prepare();
    let start = parser.span_start();
    let expression_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let { x } = value else { return }
    assert_node!(parser.tree, expression_id, Expression::LetElse { kind, mutability: _, declarator, else_branch } => {
        assert_eq!(*kind, LetKind::Let);

        let else_span = parser
            .tree
            .get_side_span(expression_id, NodeSpanType::Region(NodeSpanRegion::Clause))
            .expect("expected else clause span");
        assert_eq!(parser.get_span_str(else_span), "else");

        // let { x } = value
        assert_node!(parser.tree, *declarator, Declarator { pattern, value, .. } => {
            assert_expression_path!(parser, parser.tree.get(value.expect("expected initializer")), "value");
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 1);
            });
        });

        // else { return }
        assert_node!(parser.tree, *else_branch, Expression::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            let branch_expression = block
                .leading_expressions
                .first()
                .copied()
                .or(block.tail_expression)
                .expect("expected else branch expression");

            assert_node!(parser.tree, branch_expression, Expression::Return { .. } => {
            });
        });
    });
}

#[test]
fn test_parse_let_else_literal_pattern() {
    let mut test = TestParser::new(r#"let "ok" = value else { return }"#);
    let mut parser = test.prepare();
    let start = parser.span_start();
    let expression_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let "ok" = value else { return }
    assert_node!(parser.tree, expression_id, Expression::LetElse { declarator, else_branch, .. } => {
        assert_node!(parser.tree, *declarator, Declarator { pattern, value, .. } => {
            assert_expression_path!(parser, parser.tree.get(value.expect("expected initializer")), "value");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                    assert_eq!(parser.strings.get(*value), "ok");
                });
            });
        });

        assert_node!(parser.tree, *else_branch, Expression::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            let branch_expression = block
                .leading_expressions
                .first()
                .copied()
                .or(block.tail_expression)
                .expect("expected else branch expression");

            assert_node!(parser.tree, branch_expression, Expression::Return { .. } => {
            });
        });
    });
}

#[test]
fn test_parse_literal_pattern_without_let_else() {
    let mut test = TestParser::new(r#"let "ok" = value"#);
    let mut parser = test.prepare();
    let start = parser.span_start();
    let expression_id = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap();

    // let "ok" = value
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value: pattern_value } => {
                assert_node!(parser.tree, *pattern_value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                    assert_eq!(parser.strings.get(*value), "ok");
                });
            });
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Identifier { name } => {
                assert_eq!(parser.strings.get(*name), "value");
            });
        });
    });
}

#[test]
fn test_reject_let_else_without_initializer() {
    let mut test = TestParser::new("let x else { return }");
    let mut parser = test.prepare();
    let start = parser.span_start();
    let error = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap_err();

    // let x else { return }
    assert_eq!(parser.get_span_str(error.leaf_span()), "else");
}

#[test]
fn test_reject_let_else_without_block_branch() {
    let mut test = TestParser::new("let x = value else return");
    let mut parser = test.prepare();
    let start = parser.span_start();
    let error = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap_err();

    // let x = value else return
    assert_eq!(parser.get_span_str(error.leaf_span()), "return");
}

#[test]
fn test_reject_indexed_declarator_target_in_untyped_source() {
    // let a[0] = 0
    let mut test = TestParser::new_with_language("let a[0]=0;", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let start = parser.span_start();
    let error = parser
        .eat_let(&start, DeclarationHeader::default())
        .unwrap_err();

    assert_eq!(parser.get_span_str(error.leaf_span()), "[");
}

#[test]
fn test_parse_let_lambda_initializer_before_next_line_expression() {
    let mut test = TestParser::new_with_language(
        "let f1 = (/* ... */) => {}\n(function (/* ... */) {})(/* ... */)\n",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "f1");
            });
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Call { left, arguments, .. } => {
            assert!(arguments.is_empty());
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.form, FunctionForm::Function);
                    });
                });
            });
    });
}
