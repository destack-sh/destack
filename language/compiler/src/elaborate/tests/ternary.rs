use crate::tests::TestProgram;

#[test]
fn test_transform_if_else_with_blocks() {
    // source if-else: blocks are unwrapped for ternary, then return added
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function abs(x: number): number {
    if (x < 0) -x else x
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function abs(x): number {
    return x < (0 as number) ? -x : x;
}
"#,
    );
}
