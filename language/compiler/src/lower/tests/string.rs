use crate::TestProgram;

/// Lower string literals into MIR and preserve UTF8 contents.
#[test]
fn test_lower_string_literal() {
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["native"]);
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

    let string_alias = test.string_type_alias_definition();
    let string_name = test.string_literal_global_name("Hello, VM");
    let expected = r#"
${string_alias}
global ${string_name}: ref<String, managed, readonly>, readonly = "Hello, VM"

function greet(): ref<String, managed, readonly> {
entry0:
    value0: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${string_name}
    value1: ref<String, managed, readonly> = load value0
    return value1
}"#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${string_name}", &string_name);
    test.assert_target_mir(module_id, "native", &expected);
}
