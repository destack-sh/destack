use crate::TestProgram;
use destack_source::ModuleId;

fn builtin_module_id_with_uri_suffix(test: &TestProgram, suffix: &str) -> ModuleId {
    test.program
        .visible_modules()
        .into_iter()
        .find_map(|module| {
            let module = module.as_ref();
            module.uri.as_ref().ends_with(suffix).then_some(module.id)
        })
        .unwrap_or_else(|| panic!("missing library module ending with {suffix}"))
}

/// Execute comptime addition and patch it into DIR.
#[test]
fn test_execute_comptime_literal_add() {
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let module_id = test.add_module(
        "test.ds",
        r#"
const VALUE = comptime 2 + 4;
"#,
    );

    test.execute_module(module_id);
    test.compile_check_clean();
    test.assert_executed(
        module_id,
        r#"
const VALUE = 6;
"#,
    );
}

/// Execute comptime string literals and patch them into DIR.
#[test]
fn test_execute_comptime_string_literal() {
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let module_id = test.add_module(
        "test.ds",
        r#"
const VALUE = comptime "hello";
"#,
    );

    test.execute_module(module_id);
    test.compile_check_clean();
    test.assert_executed(
        module_id,
        r#"
const VALUE = "hello";
"#,
    );
}

/// Execute the builtin ipc unix module cleanly through the patched DIR boundary.
#[test]
fn test_execute_builtin_platform_ipc_unix_module() {
    let test = TestProgram::memory_sequential_with_prelude()
        ;
    let module_id = builtin_module_id_with_uri_suffix(&test, "platform/ipc/unix.ds");

    test.execute_module(module_id);
    test.compile_check_clean();
}
