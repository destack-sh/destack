use super::*;

/// Analyze contextual lambda from annotation.
#[test]
fn test_analyze_contextual_lambda_from_annotation() {
    // infers lambda signature from contextual function type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const add: (a: number, b: number) => number = (a, b) => a + b;
"#,
    );

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
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected function value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // contextual annotation yields number, number to number
    assert_type!(view.types(), value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // two parameters inherited from annotation
        assert_eq!(dynamic_parameters.len(), 2);

        // first parameter is number
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // second parameter is number
        assert_type!(view.types(), dynamic_parameters[1], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual lambda from argument.
#[test]
fn test_analyze_contextual_lambda_from_argument() {
    // infers lambda signature from argument type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function apply(fn: (a: number) => number): number {
    return fn(1);
}
apply((a) => a + 1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the call expression
    let call_expression_id = view.root_expression_id(1);
    let call_expression = view.tree().get(call_expression_id);
    let Expression::Call {
        dynamic_arguments, ..
    } = call_expression
    else {
        panic!("expected call expression");
    };

    // resolve the argument type
    let argument_id = dynamic_arguments.first().expect("expected argument");
    let argument = view.tree().get(*argument_id);
    let argument_value_id = argument.value();
    let argument_ty_id = view.expect_inferred_type_id(argument_value_id);

    // contextual argument yields number to number function type
    assert_type!(view.types(), argument_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // one parameter inherited from argument context
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter is number
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual `this` parameter from annotation.
#[test]
fn test_analyze_contextual_this_parameter_from_annotation() {
    // infers `this` parameter type from contextual function type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: number;
}

const add: (this: Counter, delta: number) => number = function(this, delta) {
    return this.value + delta;
};
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the function expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected function value");
    let Expression::Declaration(declaration) = view.tree().get(value_id) else {
        panic!("expected function declaration expression");
    };

    // check the inferred signature type
    let signature_ty_id = view
        .types()
        .get_signature_type_for_node((*declaration).into_global_any(module_id))
        .expect("expected signature type for function");
    let counter_symbol = view
        .test
        .resolve_to_symbol("test.ds", "Counter")
        .expect("expected Counter symbol");

    assert_type!(view.types(), signature_ty_id, Type::Function { this_parameter, dynamic_parameters, return_type, .. } => {
        // contextual this parameter uses the declared Counter type
        let this_parameter = this_parameter.expect("expected this parameter");
        assert_type!(view.types(), this_parameter, Type::Reference { symbol, .. } => {
            assert_eq!(*symbol, counter_symbol);
        });

        // delta parameter stays number
        assert_eq!(dynamic_parameters.len(), 1);
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type stays number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual `this` in a lambda.
#[test]
fn test_analyze_contextual_lambda_this_parameter() {
    // infers `this` parameter type from contextual lambda annotation
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: number;
}

const add: (this: Counter, delta: number) => number = (delta) => this.value + delta;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the lambda expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected lambda value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // check the inferred signature type
    let counter_symbol = view
        .test
        .resolve_to_symbol("test.ds", "Counter")
        .expect("expected Counter symbol");

    assert_type!(view.types(), value_ty_id, Type::Function { this_parameter, dynamic_parameters, return_type, .. } => {
        // contextual this parameter uses the declared Counter type
        let this_parameter = this_parameter.expect("expected this parameter");
        assert_type!(view.types(), this_parameter, Type::Reference { symbol, .. } => {
            assert_eq!(*symbol, counter_symbol);
        });

        // delta parameter stays number
        assert_eq!(dynamic_parameters.len(), 1);
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type stays number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual array literal.
#[test]
fn test_analyze_contextual_array_literal() {
    // infers array literal element types from contextual type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const numbers: number[] = [1, 2];
"#,
    );

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
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected array value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // array element type is number from number[] context
    assert_type!(view.types(), value_ty_id, Type::Array { element: Some(element_id), .. } => {
        assert_type!(view.types(), *element_id, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

// Binds mapped type parameters for use in value type.
#[test]
fn test_analyze_type_mapped_parameter_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Map<T> = { [K in keyof T]: T[K] };");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the mapped type symbol
    let map_symbol = test
        .resolve_to_symbol("test.ds", "Map")
        .expect("expected Map symbol");
    let map_instance_id = view
        .types()
        .get_instance_type_id(map_symbol)
        .expect("expected Map instance type");

    assert_type!(view.types(), map_instance_id, Type::Mapped { parameter, value, .. } => {
        assert_string!(test.program, parameter.name, "K");
        assert_type!(view.types(), parameter.constraint, Type::KeyOf { target_type: right } => {
            assert_type!(view.types(), *right, Type::Reference { symbol, static_arguments } => {
                assert!(static_arguments.is_none());
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
        });
        assert_type!(view.types(), *value, Type::Index { left, index } => {
            assert_type!(view.types(), *left, Type::Reference { symbol, .. } => {
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
            assert_type!(view.types(), *index, Type::Reference { symbol, .. } => {
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_eq!(symbol.kind, SymbolKind::Local);
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "K");
            });
        });
    });
}

/// Bind infer variables for use in conditional true branch.
#[ignore]
#[test]
fn test_analyze_type_infer_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Foo<T> = T extends infer U ? U : never;");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the conditional type symbol
    let foo_symbol = test
        .resolve_to_symbol("test.ds", "Foo")
        .expect("expected Foo symbol");
    let foo_instance_id = view
        .types()
        .get_instance_type_id(foo_symbol)
        .expect("expected Foo instance type");

    assert_type!(
        view.types(),
        foo_instance_id,
        Type::Conditional {
            distributive_symbol,
            left,
            right,
            then_type,
            else_type
        } => {
        assert!(distributive_symbol.is_some());
        assert_type!(view.types(), *left, Type::Reference { symbol, .. } => {
            let symbol = view.symbols().get_symbol(symbol.into_local());
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
        });
        assert_type!(view.types(), *right, Type::Infer { name, constraint } => {
            assert_string!(test.program, *name, "U");
            assert!(constraint.is_none());
        });
        assert_type!(view.types(), *then_type, Type::Reference { symbol, .. } => {
            let symbol = view.symbols().get_symbol(symbol.into_local());
            assert_eq!(symbol.kind, SymbolKind::Local);
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "U");
        });
        assert_type!(view.types(), *else_type, Type::TypeLiteral { value: TypeLiteral::Never });
    });
}
