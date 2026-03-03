#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_match_literal_patterns() {
        // match on literal values transforms to nested if else with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): string {
    match (x) {
        1 => "one"
        2 => "two"
        _ => "other"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x): string {
    if (x == 1) {
        return "one";
    } else if (x == 2) {
        return "two";
    } else {
        return "other";
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_boolean() {
        // match on boolean: exhaustive optimization skips last check
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(b: boolean): number {
    match (b) {
        true => 1
        false => 0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(b): number {
    if (b == true) {
        return 1;
    } else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_with_guard() {
        // match with guard clause transforms to nested if else
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function classify(x: number): string {
    match (x) {
        n if n > 0 => "positive"
        n if n < 0 => "negative"
        _ => "zero"
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
    if ({
        const n = x;
        n > (0 as number)
    }) {
        const n = x;
        return "positive";
    } else if ({
        const n = x;
        n < (0 as number)
    }) {
        const n = x;
        return "negative";
    } else {
        return "zero";
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_wildcard_only() {
        // match with only wildcard becomes the body in a block with explicit return
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function always(x: number): number {
    match (x) {
        _ => 42
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function always(x): number {
    return 42;
}
"#,
        );
    }

    #[test]
    fn test_transform_match_union_pattern() {
        // union patterns transform to OR checks with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function isWeekend(day: number): boolean {
    match (day) {
        0 | 6 => true
        _ => false
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function isWeekend(day): boolean {
    if (day == 0 || day == 6) {
        return true;
    } else {
        return false;
    }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_to_if_else() {
        // match generates if else with proper blocks
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function sign(x: number): number {
    match (x) {
        0 => 0
        _ => 1
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function sign(x): number {
    if (x == 0) {
        return 0;
    } else {
        return 1;
    }
}
"#,
        );
    }
}
