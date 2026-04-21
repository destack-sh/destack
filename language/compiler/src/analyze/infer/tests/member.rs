use super::*;

/// Resolve member access on object literal to field type.
#[test]
fn test_analyze_member_access_object_field() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
    );

    // run resolve and analyze pipeline
    test.resolve_language_environment();
    test.resolve_libs();
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Resolve member access across multiple fields.
#[test]
fn test_analyze_member_access_multiple_fields() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello", z: true };
let a = obj.x;
let b = obj.y;
let c = obj.z;
"#,
    );

    // run resolve and analyze pipeline
    test.resolve_language_environment();
    test.resolve_libs();
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Preserve annotated interface receiver types for index signature member lookups.
#[test]
fn test_analyze_index_signature_member_preserves_annotated_receiver_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 };
const value = bag.missing;
"#,
    );

    // run analyze pipeline without requiring clean diagnostics
    test.analyze_module(module_id);
    test.compile();

    // load typed module data
    let view = test.view(module_id);

    // resolve the binding and expression ids
    let bag_name = test.program.strings.intern("bag");
    let value_name = test.program.strings.intern("value");
    let bag_symbol = view.expect_binding_symbol(bag_name);
    let value_initializer_id = view.expect_initializer(value_name);
    let (receiver_id, _) = view.expect_member_expression(value_initializer_id);
    let receiver_symbol = view.expect_reference_symbol(receiver_id);
    assert_eq!(receiver_symbol, bag_symbol);

    // preserve the Bag annotation on the direct binding
    let bag_value_type_id = view.expect_value_type_id(bag_symbol);
    let bag_annotation_type_id = match view.types().get_type(bag_value_type_id) {
        Type::Value { value } => *value,
        _ => bag_value_type_id,
    };
    assert_type!(view.types(), bag_annotation_type_id, Type::Reference { .. });

    // preserve the receiver type used for member lookup
    let receiver_type_id = view.expect_inferred_type_id(receiver_id);
    let receiver_value_type_id = match view.types().get_type(receiver_type_id) {
        Type::Value { value } => *value,
        _ => receiver_type_id,
    };
    assert_type!(view.types(), receiver_value_type_id, Type::Reference { .. });
}

/// Resolve chained member access on nested objects.
#[test]
fn test_analyze_member_access_chained() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Defer missing-member checks in callback bodies without emitting callback-order diagnostics.
#[test]
fn test_analyze_deferred_member_lookup_preserves_callback_inference() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box {
    count: int32
}

declare function project<T, U>(callback: (value: T) => U, value: T): U;

let out = project(value => value.count, Box { count: 1 });
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
}

/// Substitute `this` types for member calls.
#[test]
fn test_analyze_this_type_member_call() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Builder { value: int32 }

extension of Builder {
    combine(other: this): this { return other; }
}

let builder = Builder { value: 0 };
let other = Builder { value: 1 };
let result = builder.combine(other);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve symbols
    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();
    let result_ty_id = view
        .types()
        .get_value_type_id(result_symbol)
        .expect("expected result type");

    assert_type!(view.types(), result_ty_id, Type::Reference { symbol, .. } => {
        assert_eq!(*symbol, builder_symbol);
    });
}

/// Substitute `this` types inside static type arguments.
#[test]
fn test_analyze_this_type_static_argument() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Box<T> { value: T }

struct Builder { value: int32 }

extension of Builder {
    box(): Box<this> { return { value: this }; }
}

let builder = Builder { value: 0 };
let boxed = builder.box();
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve symbols
    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();
    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let boxed_symbol = test.resolve_to_symbol("test.ds", "boxed").unwrap();
    let boxed_ty_id = view
        .types()
        .get_value_type_id(boxed_symbol)
        .expect("expected boxed type");

    let boxed_ty = view.types().get_type(boxed_ty_id).clone();
    let (boxed_symbol, boxed_arguments) = match boxed_ty {
        Type::Value { value } => match view.types().get_type(value).clone() {
            Type::Reference {
                symbol,
                generic_arguments,
            } => (symbol, generic_arguments),
            other => panic!("expected boxed reference type, got {other:?}"),
        },
        Type::Reference {
            symbol,
            generic_arguments,
        } => (symbol, generic_arguments),
        other => panic!("expected boxed value type, got {other:?}"),
    };

    assert_eq!(boxed_symbol, box_symbol);
    let boxed_arguments = boxed_arguments.expect("expected static arguments");
    assert_eq!(boxed_arguments.len(), 1);
    let StaticArgument::Evaluated { value, .. } = &boxed_arguments[0] else {
        panic!("expected evaluated static argument");
    };
    let StaticExpression::Type { ty } = value else {
        panic!("expected static type argument");
    };
    let inner_ty = view.types().get_type(*ty).clone();
    let builder_reference = match inner_ty {
        Type::Reference { symbol, .. } => symbol,
        Type::Value { value } => match view.types().get_type(value).clone() {
            Type::Reference { symbol, .. } => symbol,
            other => panic!("expected builder reference type, got {other:?}"),
        },
        other => panic!("expected builder value type, got {other:?}"),
    };
    assert_eq!(builder_reference, builder_symbol);
}

/// Analyze member instance inherited static arguments.
#[test]
fn test_analyze_member_instance_inherited_static_arguments() {
    // registers inherited static arguments for member instances
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let result = getContainer().map<string>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    // resolve the member instance
    let instance_id = view
        .types()
        .get_instance_for_node(value_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    // resolve the member symbol
    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module_id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.generic_arguments.len(), 2);

    // first static argument is inherited Container T = number
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

    // second static argument is explicit U = string
    match &instance.generic_arguments[1] {
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
}

/// Analyze member instance from member expression.
#[test]
fn test_analyze_member_instance_from_member_expression() {
    // registers static arguments on member expressions
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let mapper = getContainer().map<string>;
mapper(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the member expression initializer
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    // resolve the member instance
    let instance_id = view
        .types()
        .get_instance_for_node(value_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    // resolve the member symbol
    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module_id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.generic_arguments.len(), 2);

    // first static argument is inherited Container T = number
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

    // second static argument is explicit U = string
    match &instance.generic_arguments[1] {
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
}

/// Preserve outer generic inference for member signatures that reference associated aliases.
#[test]
fn test_analyze_associated_alias_member_keeps_outer_generic_inference() {
    // associated aliases in member signatures should not erase outer class inference
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box<T> {
    type Item = T;
    value: Item;

    constructor(value: Item) {
        this.value = value;
    }
}

let box = new Box("ok");
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let box_symbol = test
        .resolve_to_symbol("test.ds", "box")
        .expect("expected box symbol");
    let box_type_id = view
        .types()
        .get_value_type_id(box_symbol)
        .expect("expected box value type");

    match view.types().get_type(box_type_id) {
        Type::Reference {
            generic_arguments: Some(arguments),
            ..
        } => {
            let first_argument = arguments
                .first()
                .expect("expected one static type argument");
            match first_argument {
                StaticArgument::Evaluated { value, .. } => match value {
                    StaticExpression::Type { ty } => match view.types().get_type(*ty) {
                        Type::TypeLiteral {
                            value:
                                TypeLiteral::Primitive(PrimitiveType::String)
                                | TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
                        } => {}
                        other => panic!("expected string-like static argument, found {other:?}"),
                    },
                    _ => panic!("expected type static argument"),
                },
                _ => panic!("expected evaluated static argument"),
            }
        }
        other => panic!("expected generic class reference, found {other:?}"),
    }
}

/// Analyze substitute type reference arguments.
#[test]
fn test_analyze_substitute_type_reference_arguments() {
    // substitutes type arguments inside type references during inference
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> { value: T }

declare function wrap<T>(value: T): Box<T>;

let result = wrap(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // resolve the result symbol
    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();

    // load typed module data
    let view = test.view(module_id);

    // resolve the result type
    let result_ty_id = view
        .types()
        .get_value_type_id(result_symbol)
        .expect("expected result type");
    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // result type is Box with a literal type argument
    assert_type!(
        view.types(),
        result_ty_id,
        Type::Reference {
            symbol,
            generic_arguments,
        } => {
            // type reference targets Box
            assert_eq!(*symbol, box_symbol);
            let generic_arguments = generic_arguments.as_ref().expect("expected arguments");

            // type reference carries one static argument
            assert_eq!(generic_arguments.len(), 1);

            // static argument is the inferred literal type
            match &generic_arguments[0] {
                StaticArgument::Evaluated { value, .. } => match value {
                    StaticExpression::Type { ty } => {
                        assert_type!(
                            view.types(),
                            *ty,
                            Type::TypeLiteral {
                                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
                            }
                        );
                    }
                    _ => panic!("expected type argument"),
                },
                _ => panic!("expected evaluated argument"),
            }
        }
    );
}
