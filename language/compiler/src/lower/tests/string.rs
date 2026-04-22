use crate::{TestProgram, materialized_plain_value};

/// Lower string literals into MIR and preserve UTF8 contents.
#[test]
fn test_lower_string_literal() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function greet(): string {
    return "Hello, VM";
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let mut interpreter = test.mir_isolate(module_id, "native");
    let output = interpreter
        .run_function_by_name_output("greet", &[])
        .expect("execution failed");
    let value = materialized_plain_value(&output.value);
    let actual = interpreter.string_value(value).expect("string value");
    assert_eq!(actual, "Hello, VM");
}
