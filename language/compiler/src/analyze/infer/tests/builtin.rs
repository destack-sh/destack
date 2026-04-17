use super::*;
/// Builtin Pick from lib surface preserves optional fields.
#[test]
fn test_analyze_es5_pick_optional_shape() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;
const ok: AgeOnly = {};
const ok2: AgeOnly = { age: 42 };
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}

/// Builtin Pick from lib surface rejects excess object literal properties.
#[test]
fn test_analyze_es5_pick_reports_excess_property_diagnostics() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;
const bad: NameOnly = { name: "Ada", age: 42 };
"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA208"]);
}

/// Destack overloads merge into a callable value type.
#[test]
fn test_analyze_destack_function_overload_merge() {
    // overload implementations share a merged value shape
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function parse(value: string): string {
    return value;
}

function parse(value: int32): int32 {
    return value + 1;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load module data for inspection
    let view = test.view(module_id);
    let parse_symbol = test.resolve_to_symbol("test.ds", "parse").unwrap();
    let value_ty_id = view
        .types()
        .get_value_type_id(parse_symbol)
        .expect("expected parse value type");
    let value_ty_id = match view.types().get_type(value_ty_id) {
        Type::Value { value } => *value,
        _ => value_ty_id,
    };

    match view.types().get_type(value_ty_id) {
        Type::Object {
            call_signatures, ..
        } => {
            assert_eq!(call_signatures.len(), 2);

            let mut has_string = false;
            let mut has_int32 = false;
            for signature_id in call_signatures {
                let Type::Function { parameters, .. } = view.types().get_type(*signature_id) else {
                    panic!("expected function signature");
                };
                let param_ty_id = *parameters.first().expect("expected parameter type");
                match view.types().get_type(param_ty_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    } => has_string = true,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
                    } => has_int32 = true,
                    _ => {}
                }
            }

            assert!(has_string);
            assert!(has_int32);
        }
        Type::Function { .. } => {
            panic!("expected overload set, found single signature");
        }
        _ => {
            panic!("expected callable value type");
        }
    }
}

/// Analyze builtin Omit mapped types.
#[test]
fn test_analyze_builtin_omit_shape() {
    // load libs so Omit comes from the builtin surface instead of a test-local shadow copy
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;
const ok: WithoutAge = { name: "Ada" };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
}
