#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_normalize_if_in_let_with_blocks() {
        // if expression with block branches in let binding becomes uninitialized let plus if
        // (uses multi-statement branches to prevent ternary optimization)
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    let x = if (flag) { let t = a; t } else { let t = b; t };
    return x;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function choose(flag, a, b): int32 {
    let x;
    if (flag) {
        let t = a;
        x = t;
    } else {
        let t = b;
        x = t;
    }
    return x;
}
"#,
        );
    }

    #[test]
    fn test_normalize_if_in_return_with_blocks() {
        // if expression with block branches in return becomes if with returns
        // (uses multi-statement branches to prevent ternary optimization)
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    return if (flag) { let t = a; t } else { let t = b; t };
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
        return t;
    } else {
        let t = b;
        return t;
    }
}
"#,
        );
    }
}
