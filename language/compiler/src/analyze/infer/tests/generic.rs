use super::*;

/// Analyze infer parameter types from call arguments.
#[test]
fn test_analyze_infer_parameter_types_from_call_arguments() {
    // infers parameter types from call arguments
    let test = TestProgram::memory_sequential();
    test.add_package("test", Some(r#""noImplicitAny": false"#));
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a, b) {
    return a + b
}
add(1, 2)
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the function type
    let fn_symbol = test.expect_nth_function_symbol(module_id, 0);
    let fn_ty_id = view
        .types()
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses literal argument types and a number return type
    assert_type!(view.types(), fn_ty_id, Type::Function { parameters, return_type, .. } => {
        // two parameters inferred from call arguments
        assert_eq!(parameters.len(), 2);

        // first parameter matches literal 1
        assert_type!(view.types(), parameters[0], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        });

        // second parameter matches literal 2
        assert_type!(view.types(), parameters[1], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2))
        });

        // return type is number from arithmetic
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze infer parameter type from default.
#[test]
fn test_analyze_infer_parameter_type_from_default() {
    // infers parameter type from default value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function greet(name = "hi") {
    return name
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the function type
    let fn_symbol = test.expect_nth_function_symbol(module_id, 0);
    let fn_ty_id = view
        .types()
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses default value literal and returns a string
    assert_type!(view.types(), fn_ty_id, Type::Function { parameters, return_type, .. } => {
        // one parameter inferred from default value
        assert_eq!(parameters.len(), 1);

        // parameter widens the default literal
        assert_type!(view.types(), parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        });

        // return type follows the parameter type
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        });
    });
}

/// Analyze static type arguments on call.
#[test]
fn test_analyze_static_type_arguments_on_call() {
    // resolves explicit static type arguments at call sites
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
identity<number>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the call expression
    let call_expression_id = view.root_expression_id(1);
    let call_ty_id = view.expect_inferred_type_id(call_expression_id);

    // call expression uses explicit number type argument
    assert_type!(
        view.types(),
        call_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze static type arguments inferred.
#[test]
fn test_analyze_static_type_arguments_inferred() {
    // infers static type arguments from call arguments
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let one = identity(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // identity returns the literal type of the inferred argument
    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
}

/// Analyze static type arguments on reference.
#[test]
fn test_analyze_static_type_arguments_on_reference() {
    // resolves explicit static arguments on function references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let as_number = identity<number>;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // specialized function reference uses number parameter and return type
    assert_type!(view.types(), value_ty_id, Type::Function { parameters, return_type, .. } => {
        // single parameter is number
        assert_eq!(parameters.len(), 1);

        // parameter uses the explicit number type argument
        assert_type!(view.types(), parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type matches the parameter
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze type reference instance.
#[test]
fn test_analyze_type_reference_instance() {
    // registers instances for type reference annotations
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> {
    value: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the type annotation
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    // resolve the instance
    let instance_id = view
        .types()
        .get_instance_for_node(type_expression_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // instance targets Box with a single static type argument
    assert_eq!(instance.symbol_id, box_symbol);
    assert_eq!(instance.generic_arguments.len(), 1);

    // static argument is the explicit number type
    match &instance.generic_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze type reference default static value.
#[test]
fn test_analyze_type_reference_default_static_value() {
    // applies default static value arguments for type references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Buffer<T, comptime N: number = 4> {
    value: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
"#,
    );
    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the type annotation
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    // resolve the instance
    let instance_id = view
        .types()
        .get_instance_for_node(type_expression_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    let buffer_symbol = test.resolve_to_symbol("test.ds", "Buffer").unwrap();

    // instance targets Buffer with explicit T and default N value
    assert_eq!(instance.symbol_id, buffer_symbol);
    assert_eq!(instance.generic_arguments.len(), 2);

    // first static argument is the explicit string type
    match &instance.generic_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is the default value N = 4
    match &instance.generic_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(4));
            }
            _ => panic!("expected scalar literal argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze static type arguments that are type parameters.
#[test]
fn test_analyze_static_type_argument_parameter_constraint() {
    // type parameter arguments satisfy parameter bounds
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Node {}
interface Box<T extends Node> {
    value: T
}

type Use<T extends Node> = Box<T>;

declare let value: Use<Node>;
let boxed: Box<Node> = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let boxed_name = test.program.strings.intern("boxed");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let boxed_declarator_id = view.expect_let_declarator(boxed_name);

    // inspect instance arguments for Use<Node>
    let value_declarator = view.tree().get(value_declarator_id);
    let value_type_expression_id = value_declarator.ty.expect("expected value type annotation");
    let use_instance_id = view
        .types()
        .get_instance_for_node(value_type_expression_id.into_global_any(module_id))
        .expect("expected Use instance");
    let use_instance = view.types().get_instance(use_instance_id);
    let use_symbol = test.resolve_to_symbol("test.ds", "Use").unwrap();
    let node_symbol = test.resolve_to_symbol("test.ds", "Node").unwrap();

    // instance targets Use with Node as the static argument
    assert_eq!(use_instance.symbol_id, use_symbol);
    assert_eq!(use_instance.generic_arguments.len(), 1);
    match &use_instance.generic_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::Reference {
                        symbol,
                        generic_arguments
                    } => {
                        assert_eq!(*symbol, node_symbol);
                        assert!(generic_arguments.is_none());
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // verify Use<Node> is assignable to Box<Node>
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let boxed_type_id = view
        .types()
        .get_declared_type_id(boxed_declarator_id.into_global(module_id).into())
        .expect("expected boxed declared type");

    // compare assignability using the module profile
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let options = test.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = is_type_assignable(
        &test.compiler,
        module,
        profile,
        view.tree(),
        &symbols,
        boxed_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

/// Analyze index access on constrained type parameters.
#[test]
fn test_analyze_index_access_uses_parameter_constraint() {
    // index access respects the parameter constraint type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Map {
    foo: number
}

type Pick<K extends keyof Map> = Map[K];

declare let value: Pick<"foo">;
let number_value: number = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let number_value_name = test.program.strings.intern("number_value");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let number_declarator_id = view.expect_let_declarator(number_value_name);

    // inspect instance arguments for Pick<"foo">
    let value_declarator = view.tree().get(value_declarator_id);
    let value_type_expression_id = value_declarator.ty.expect("expected value type annotation");
    let pick_instance_id = view
        .types()
        .get_instance_for_node(value_type_expression_id.into_global_any(module_id))
        .expect("expected Pick instance");
    let pick_instance = view.types().get_instance(pick_instance_id);
    let pick_symbol = test.resolve_to_symbol("test.ds", "Pick").unwrap();
    let foo_name = test.program.strings.intern("foo");

    // instance targets Pick with a string literal argument
    assert_eq!(pick_instance.symbol_id, pick_symbol);
    assert_eq!(pick_instance.generic_arguments.len(), 1);
    match &pick_instance.generic_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::String(foo_name));
            }
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(name))
                    } => {
                        assert_eq!(*name, foo_name);
                    }
                );
            }
            _ => panic!("expected scalar literal argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // verify Pick<"foo"> is assignable to number
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let number_type_id = view
        .types()
        .get_declared_type_id(number_declarator_id.into_global(module_id).into())
        .expect("expected number_value declared type");

    // compare assignability using the module profile
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let options = test.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = is_type_assignable(
        &test.compiler,
        module,
        profile,
        view.tree(),
        &symbols,
        number_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

/// Analyze tuple index access on constrained type parameters.
#[test]
fn test_analyze_tuple_index_access_uses_parameter_constraint() {
    // tuple index access respects the parameter constraint type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Element<Types extends [boolean, boolean]> = Types[0];

declare let value: Element<[true, false]>;
let ok: boolean = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let ok_name = test.program.strings.intern("ok");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let ok_declarator_id = view.expect_let_declarator(ok_name);

    // compare assignability using the module profile
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let ok_type_id = view
        .types()
        .get_declared_type_id(ok_declarator_id.into_global(module_id).into())
        .expect("expected ok declared type");
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let options = test.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = is_type_assignable(
        &test.compiler,
        module,
        profile,
        view.tree(),
        &symbols,
        ok_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

// static value arguments accept imported constants
#[test]
fn test_analyze_static_value_argument_imported_constant() {
    let test = TestProgram::memory_sequential();
    let _utils_id = test.add_module("utils.ds", "export const SIZE = 4;");
    let main_id = test.add_module(
        "main.ds",
        r#"
import { SIZE } from "./utils.ds";

type Buffer<comptime N: number> = uint8[N];

declare let value: Buffer<SIZE>;
"#,
    );

    test.analyze_module(main_id);
    test.compile();

    test.with_dir_types_mut(main_id, |module, profile, dir, tree, symbols, types| {
        let key = StaticKey::Name(test.program.strings.intern("SIZE"));
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let symbol_id = symbols
            .find_active_symbol_up_to(namespace_scope, key, LocalScopeMark::end())
            .map(|symbol| symbol.into_global(module.id))
            .expect("expected imported SIZE symbol");

        let mut expression_id = None;
        for argument_id in tree.iter_node_ids_of_type::<Argument>() {
            let argument = tree.get(argument_id);
            let value_id = argument.value();
            let value_expression = tree.get(value_id);
            let is_reference = match value_expression {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => *target_symbol == symbol_id,
                _ => false,
            };
            if is_reference {
                expression_id = Some(value_id);
                break;
            }
        }
        let expression_id = expression_id.expect("expected SIZE reference");
        let options = test.analyze_context_options_for_module(module.id);
        let compiler_context = test.context();
        let mut ctx = TypeContext::new(
            &compiler_context,
            module,
            profile,
            &options,
            tree,
            symbols,
            types,
            AnalyzeIndex::default(),
        );
        let value = test
            .compiler
            .evaluate_static_expression_value(&mut ctx, expression_id, None)
            .expect("static evaluation failed")
            .expect("expected SIZE reference to evaluate");

        match value {
            StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => assert_eq!(value, 4),
            other => panic!("expected reference literal, got {other:?}"),
        }
    });
}

/// Instantiate constrained associated projections in generic call return types.
#[test]
fn test_analyze_call_return_type_projects_constrained_associated_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface LocalCursor<T> {
    type Item;
    read(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension for Counter implements LocalCursor<int32> {
    type Item = int32;

    read(): Item {
        this.value
    }
}

function project<I: LocalCursor<int32>>(owner: I): I.Item {
    owner.read()
}

declare const counter: Counter;
let result = project(counter);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let result_symbol = test
        .resolve_to_symbol("test.ds", "result")
        .expect("expected result symbol");
    let result_type = view
        .types()
        .get_value_type(result_symbol)
        .expect("expected result type");
    let result_declarator_id = view.expect_let_declarator(test.program.strings.intern("result"));
    let result_declarator = view.tree().get(result_declarator_id);
    let result_initializer_id = result_declarator
        .value
        .expect("expected result initializer");
    let initializer_type = view
        .types()
        .get_inferred_type(result_initializer_id.into_global_any(module_id))
        .expect("expected initializer inferred type");
    let result_instance_id = view
        .types()
        .get_instance_for_node(result_initializer_id.into_global_any(module_id))
        .expect("expected call instance");
    let result_instance = view.types().get_instance(result_instance_id);
    assert_eq!(result_instance.generic_arguments.len(), 1);
    assert_eq!(result_instance.generic_parameter_symbols.len(), 1);
    let counter_symbol = test
        .resolve_to_symbol("test.ds", "Counter")
        .expect("expected counter symbol");
    match &result_instance.generic_arguments[0] {
        StaticArgument::Evaluated {
            value: StaticExpression::Type { ty },
            ..
        } => match view.types().get_type(*ty) {
            Type::Reference { symbol, .. } => assert_eq!(*symbol, counter_symbol),
            other => panic!("expected Counter static argument, found {other:?}"),
        },
        other => panic!("expected evaluated type static argument, found {other:?}"),
    }
    assert_eq!(
        *initializer_type,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );

    // result type should be int32
    assert_eq!(
        *result_type,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}
