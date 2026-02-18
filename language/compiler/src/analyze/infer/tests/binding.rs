use super::*;

/// Analyze let expression infer type.
#[test]
fn test_analyze_let_expression_infer_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x = 42");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the let expression
    let let_expr_id = view.root_expression_id(0);
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // no declared type
    assert!(
        view.types()
            .get_declared_type(let_expr_id.into_global_any(module_id))
            .is_none()
    );

    // value_type[x] = int32
    let x_ty = view.types().get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );

    // instance_type[x] = undefined
    assert!(view.types().get_instance_type(x_symbol).is_none());
}

/// Analyze let expression declare type.
#[test]
fn test_analyze_let_expression_declare_type() {
    // use compatible types: string annotation with string value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", r#"let x: string = "hello""#);

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // declared_type[declarator] = string
    let declared = view
        .types()
        .get_declared_type(declarator_id.into_global(module_id).into())
        .unwrap();
    assert_eq!(
        *declared,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // value_type[x] = string
    let x_ty = view.types().get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // instance_type[x] = undefined
    assert!(view.types().get_instance_type(x_symbol).is_none());
}

/// Analyze let expression infer tuple type with pattern.
#[test]
fn test_analyze_let_expression_infer_tuple_type_with_pattern() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let (x, y, ...rest, z) = (123, 'abc', true, 456);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve binding symbols
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
    let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
    let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
    let z_symbol = test.resolve_to_symbol("test.ds", "z").unwrap();

    // value_type[x] = int32
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );

    // value_type[y] = string
    let y_ty_id = view.types().get_value_type_id(y_symbol).unwrap();
    assert_type!(
        view.types(),
        y_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // value_type[rest] = (boolean,)
    let rest_ty_id = view.types().get_value_type_id(rest_symbol).unwrap();
    assert_type!(view.types(), rest_ty_id, Type::Tuple { elements, is_readonly: _ } => {
        assert_eq!(elements.len(), 1);
        assert_type!(view.types(), elements[0].ty, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean)
        });
    });

    // value_type[z] = int32
    let z_ty_id = view.types().get_value_type_id(z_symbol).unwrap();
    assert_type!(
        view.types(),
        z_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Bind array pattern elements from the array element type.
#[test]
fn test_bind_array_pattern_elements() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let [first, second] = [1, 2];
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let first_symbol = test.resolve_to_symbol("test.ds", "first").unwrap();
    let second_symbol = test.resolve_to_symbol("test.ds", "second").unwrap();

    // accept int32 or literal unions
    let assert_element_type = |ty_id: LocalTypeId| match view.types().get_type(ty_id) {
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        } => {}
        Type::Union { elements } => {
            // each element should be int32 or a literal integer
            for element_ty_id in elements {
                match view.types().get_type(*element_ty_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
                    } => {}
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
                    } => {}
                    other => panic!("unexpected array element type: {other:?}"),
                }
            }
        }
        other => panic!("unexpected array element type: {other:?}"),
    };

    // check both bindings
    let first_ty_id = view.types().get_value_type_id(first_symbol).unwrap();
    assert_element_type(first_ty_id);
    let second_ty_id = view.types().get_value_type_id(second_symbol).unwrap();
    assert_element_type(second_ty_id);
}

/// Bind newtype pattern inner values to the underlying scalar type.
#[test]
fn test_bind_newtype_pattern_inner_value() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype UserId = int64;

function next_id(id: UserId): int64 {
    match (id) {
        UserId(value) => value
    }
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // walk the function body to the bound pattern symbol
    let next_id_symbol = test.resolve_to_symbol("test.ds", "next_id").unwrap();
    let next_id_entry = view.symbols().get_symbol(next_id_symbol.local_id);
    let declaration_id = next_id_entry
        .primary_declaration
        .and_then(|decl| decl.try_into_typed::<Declaration>().ok())
        .map(LocalNodeId::from)
        .expect("next_id should resolve to a function declaration");
    let Declaration::Function {
        body: Some(body), ..
    } = view.tree().get(declaration_id)
    else {
        panic!("next_id should have a body");
    };

    // normalize the function body to the last expression
    let match_expression_id = match view.tree().get(*body) {
        Expression::Match { .. } => *body,
        Expression::Block { block } => {
            let block = view.tree().get(*block);
            *block
                .expressions
                .last()
                .expect("function body block should have an expression")
        }
        other => panic!("unexpected function body kind: {}", other.kind_name()),
    };
    let Expression::Match { cases, .. } = view.tree().get(match_expression_id) else {
        panic!("next_id body should end with a match expression");
    };
    let MatchCase::Expression { selector, .. } = view.tree().get(cases[0]) else {
        panic!("match case should be an expression case");
    };
    let MatchSelector::Pattern { pattern, .. } = selector else {
        panic!("match selector should be a pattern");
    };
    let Pattern::TaggedTuple { fields, .. } = view.tree().get(*pattern) else {
        panic!("pattern should be a tagged tuple");
    };

    // accept either positional or named binding fields
    let value_symbol = match view.tree().get(fields[0]) {
        PatternField::Positional { pattern, .. } => {
            let Pattern::Binding { symbol, .. } = view.tree().get(*pattern) else {
                panic!("positional field should bind a symbol");
            };
            symbol.into_global(module_id)
        }
        PatternField::Named { pattern, .. } => {
            let Some(pattern_id) = pattern else {
                panic!("named field should contain a nested binding pattern");
            };
            let Pattern::Binding { symbol, .. } = view.tree().get(*pattern_id) else {
                panic!("named field pattern should be a binding");
            };
            symbol.into_global(module_id)
        }
        PatternField::Alias { symbol, .. } => symbol.into_global(module_id),
        other => panic!("unexpected tagged tuple field: {other:?}"),
    };

    // the bound value should be the underlying int64
    let value_ty_id = view.types().get_value_type_id(value_symbol).unwrap();
    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int64))
        }
    );
}

/// Infer negative bigint literals through template literal spans.
#[test]
fn test_infer_template_negative_bigint_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function parseBig<T extends bigint>(value: `${T}`): T;

let ok = parseBig("-1");
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // resolve the module level binding
    let view = test.view(module_id);
    let ok_symbol = test.resolve_to_symbol("test.ds", "ok").unwrap();

    // the inferred type should preserve the sign
    let ok_ty_id = view.types().get_value_type_id(ok_symbol).unwrap();
    assert_type!(
        view.types(),
        ok_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(-1))
        }
    );
}
