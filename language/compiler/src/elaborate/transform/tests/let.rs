#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_if_let_literal() {
        // if let literal becomes equality check
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): number {
    if let 1 = x {
        1
    } else {
        0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): number {
    if (x == 1) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_if_let_union() {
        // if let union becomes || checks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): number {
    if let 1 | 2 = x {
        1
    } else {
        0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): number {
    if (x == 1 || x == 2) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_if_let_else_if_chain() {
        // if let else-if chains become nested if expressions
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function classify(x: number): string {
    if let 1 = x {
        "one"
    } else if let 2 = x {
        "two"
    } else {
        "other"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function classify(x): string {
    if (x == 1) {
        return "one";
    } else {
        if (x == 2) {
            return "two";
        } else {
            return "other";
        }
    }
}
"#,
        );
    }
}
