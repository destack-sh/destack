use crate::TestProgram;

#[test]
fn test_execute_comptime_literal_add() {
    let test = TestProgram::memory_sequential()
        .with_profile_libs(&[])
        .with_options_mut(|options| {
            options.retain_comptime_as_comment = true;
        });
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
const VALUE = 6; // comptime 2 + 4
"#,
    );
}
