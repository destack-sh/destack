use destack_dir as dir;

use crate::TestProgram;

#[test]
fn test_execute_comptime_literal_patch() {
    let test = TestProgram::memory_sequential().with_profile_libs(&[]);
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

#[test]
fn test_execute_comptime_results_recorded() {
    let test = TestProgram::memory_sequential().with_profile_libs(&[]);
    test.add_package("test", None);
    let module_id = test.add_module(
        "test.ds",
        r#"
const VALUE = comptime 2 + 4;
"#,
    );

    test.execute_module(module_id);
    test.compile_check_clean();

    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let comptime = module.comptime(profile);
    let (_, result) = comptime
        .results
        .iter()
        .next()
        .expect("missing comptime result");

    let Some(output) = result else {
        panic!("missing comptime output");
    };

    assert_eq!(
        output.dir.as_ref(),
        Some(&dir::StaticExpression::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(6),
        })
    );
}
