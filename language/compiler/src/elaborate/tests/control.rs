use crate::tests::TestProgram;

#[test]
fn test_reify_must_nullable_to_explicit_downcast() {
    // must on nullable values becomes an explicit downcast cast expression
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function requireName(name: string | null): string {
    name!
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function requireName(name): string {
    return (name as string);
}
"#,
    );
}

#[test]
fn test_reify_must_non_nullable_simplifies_to_operand() {
    // must on non nullable values is eliminated
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identityName(name: string): string {
    name!
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function identityName(name): string {
    return name;
}
"#,
    );
}
