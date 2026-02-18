use super::*;

/// Merge global interface members across imported modules.
#[test]
fn test_merge_global_declarations_across_imports() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
declare global {
    interface GlobalThing {
        value: number
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    interface GlobalThing {
        label: string
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

const thing: GlobalThing = { value: 1, label: "ok" };
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);

    // load module data for inspection
    let view = test.view(main_id);
    let module = test.program.modules.get(main_id);
    let module = module.read();
    let profile = view.profile_id();
    let dir = module.dir(profile);
    let thing_name = test.program.strings.intern("thing");
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = test
        .compiler
        .get_global_symbol_group(main_id, profile, global_key, SymbolSpace::Type)
        .expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 2);

    for global_symbol in &global_group {
        let remote_profile = test.default_profile_id(global_symbol.module_id);
        test.compiler.with_module_types(
            &module,
            remote_profile,
            global_symbol.module_id,
            |_, remote_types| {
                assert!(
                    remote_types.get_instance_type_id(*global_symbol).is_some(),
                    "expected instance type for GlobalThing in module {:?}",
                    global_symbol.module_id,
                );
            },
        );
    }

    // locate the bound symbol for thing in the module scope
    let namespace_scope = view.symbols().get_scope_by_id(dir.namespace_scope);
    let thing_symbol = view
        .symbols()
        .active_named_symbols(namespace_scope)
        .find_map(|(key, symbol_id)| match key {
            StaticKey::Name(name_id) if name_id == thing_name => Some(symbol_id),
            _ => None,
        })
        .expect("missing symbol for thing")
        .into_global(main_id);
    let thing_entry = view.symbols().get_symbol(thing_symbol.local_id);
    assert!(
        thing_entry.target_symbol.is_none(),
        "unexpected target_symbol on thing binding",
    );
    assert!(
        thing_entry.canonical_symbol.is_none(),
        "unexpected canonical_symbol on thing binding",
    );
    let canonical_symbol = test.compiler.canonical_symbol_id(
        &module,
        view.symbols(),
        profile,
        thing_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    assert_eq!(canonical_symbol, thing_symbol);

    // assert the binding uses a nominal reference type
    let value_ty_id = view
        .types()
        .get_value_type_id(thing_symbol)
        .expect("missing value type for thing");
    let value_ty = view.types().get_type(value_ty_id);
    let Type::Reference {
        symbol: global_thing,
        ..
    } = value_ty
    else {
        panic!("expected reference type for thing");
    };
    assert_eq!(global_thing.ty(), SymbolType::Interface);

    // assert the merged instance type includes both fields
    let instance_ty_id = view
        .types()
        .get_instance_type_id(*global_thing)
        .expect("missing instance type for GlobalThing");
    let instance_ty = view.types().get_type(instance_ty_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected object instance type for GlobalThing");
    };
    let value_key = StaticKey::Name(test.program.strings.intern("value"));
    let label_key = StaticKey::Name(test.program.strings.intern("label"));
    assert!(fields.iter().any(|field| field.key.matches(&value_key)));
    assert!(fields.iter().any(|field| field.key.matches(&label_key)));
}

/// Analyze cross module type import.
#[test]
fn test_analyze_cross_module_type_import() {
    // import a value from another module and verify its type is correctly imported
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let value = 42;
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { value } from "./lib.ds";
let x = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have int32 type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Analyze cross module array type import.
#[test]
fn test_analyze_cross_module_array_type_import() {
    // import an array value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let items = [1, 2, 3];
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { items } from "./lib.ds";
let x = items;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have an int32 array element type
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(view.types(), x_ty_id, Type::Array { element: Some(element_id), .. } => {
        match view.types().get_type(*element_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
            } => {}
            Type::Union { elements } => {
                // each element should be int32 or a literal integer
                for element_id in elements {
                    match view.types().get_type(*element_id) {
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
        }
    });
}

/// Analyze cross module string type import.
#[test]
fn test_analyze_cross_module_string_type_import() {
    // import a string value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let greeting = "hello";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { greeting } from "./lib.ds";
let x = greeting;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have string type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}

/// Analyze cross module member access using declared shapes.
#[test]
fn test_analyze_cross_module_member_access_declared_shape() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export struct Box {
    value: number
}

export let boxed: Box = { value: 1 };
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { boxed } from "./lib.ds";

let value = boxed.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // value should be number from the declared field type
    let value_symbol = test.resolve_to_symbol("main.ds", "value").unwrap();
    let value_ty_id = view.types().get_value_type_id(value_symbol).unwrap();

    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze circular imports with a declared export anchor.
#[test]
fn test_analyze_cross_module_circular_imports_with_declared_anchor() {
    // arrange circular modules with a declared anchor
    let test = TestProgram::memory_sequential();
    let b_module_id = test.add_module(
        "b.ds",
        r#"
import { a } from "./a.ds";

export let b = a;
"#,
    );
    let a_module_id = test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a: number = b;
"#,
    );

    // run analyze pipeline for the anchor module
    test.analyze_module_and_check_clean(a_module_id);

    // load typed module data
    let view = test.view(b_module_id);

    // b should pick up the declared number type from a
    let b_symbol = test.resolve_to_symbol("b.ds", "b").unwrap();
    let b_ty_id = view.types().get_value_type_id(b_symbol).unwrap();

    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze export chain surface inference.
#[test]
fn test_analyze_export_chain_surface_inference() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
export let a = 1;
export let b = a;
"#,
    );

    // load typed module data
    let view = test.view(module_id);

    // locate exported declarators
    let a_name = test.program.strings.intern("a");
    let b_name = test.program.strings.intern("b");
    let a_declarator_id = view.expect_let_declarator(a_name);
    let b_declarator_id = view.expect_let_declarator(b_name);

    // resolve binding symbols
    let a_pattern = view.tree().get(view.tree().get(a_declarator_id).pattern);
    let b_pattern = view.tree().get(view.tree().get(b_declarator_id).pattern);
    let Pattern::Binding {
        symbol: a_symbol, ..
    } = a_pattern
    else {
        panic!("expected binding");
    };
    let Pattern::Binding {
        symbol: b_symbol, ..
    } = b_pattern
    else {
        panic!("expected binding");
    };

    // read value types
    let a_ty_id = view
        .types()
        .get_value_type_id(a_symbol.into_global(module_id))
        .expect("expected a type");
    let b_ty_id = view
        .types()
        .get_value_type_id(b_symbol.into_global(module_id))
        .expect("expected b type");

    // both exports resolve to the int32 type
    assert_type!(
        view.types(),
        a_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

// re-exported data loader overrides keep text semantics at the consumer import site
#[test]
fn test_analyze_reexport_text_loader_attribute_infers_string() {
    let test = TestProgram::memory_parallel();
    test.add_file("data.json", r#"{ "key": "value" }"#);
    test.add_module(
        "bridge.ds",
        r#"
export { default as text } from "./data.json" with { type: "text" };
"#,
    );
    let main_id = test.analyze_module_with_source(
        "main.ds",
        r#"
import { text } from "./bridge.ds";
let value = text;
"#,
    );

    let view = test.view(main_id);
    let text_symbol = test
        .resolve_to_symbol("main.ds", "text")
        .expect("expected imported text symbol");
    let text_ty_id = view
        .types()
        .get_value_type_id(text_symbol)
        .expect("missing value type for imported text symbol");

    assert_type!(
        view.types(),
        text_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}
