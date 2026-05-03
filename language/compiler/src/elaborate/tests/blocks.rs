use crate::tests::TestProgram;

#[test]
fn test_transform_unwrap_single_expression_blocks_in_if_else() {
    // single expression branch blocks unwrap before later transforms
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a } else { b }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, a, b): int32 {
    if (flag) a else b
}
"#,
    );
}

#[test]
fn test_transform_keep_multi_expression_blocks_in_if_else() {
    // multi expression branch blocks stay blocks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { let t = a; t } else { let t = b; t }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, a, b): int32 {
    if (flag) {
        let t = a;
        t;
    } else {
        let t = b;
        t;
    }
}
"#,
    );
}

#[test]
fn test_transform_drop_parenthesized_expression() {
    // parenthesized wrappers are dropped in canonical dir
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a: int32, b: int32): int32 {
    return (a + b);
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function add(a, b): int32 {
    return a + b;
}
"#,
    );
}
