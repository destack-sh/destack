#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_split_declarators() {
        // multi-declarator let statements are split into individual lets
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1, b = 2, c = 3;
    a + b + c
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    let c = 3;
    return a + b + c;
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_with_types() {
        // split declarators with type annotations (types are inferred after analysis)
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a: number = 1, b: number = 2;
    a + b
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    let b = 2;
    return a + b;
}
"#,
        );
    }

    #[test]
    fn test_transform_split_declarators_single_unchanged() {
        // single declarator let statements are unchanged
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): number {
    let a = 1;
    a
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): number {
    let a = 1;
    return a;
}
"#,
        );
    }
}
