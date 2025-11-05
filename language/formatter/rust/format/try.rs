#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_try_expression() {
        assert_format!(
            "try operation()",
            "try operation()",
            |p| p.eat_try(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_block() {
        assert_format!(
            "try { const X = 1 }",
            "try {\n\tconst X = 1\n}",
            |p| p.eat_try(),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_try_expression_with_catch_match() {
        let source = r"try {
    foo()
} catch match e {
    Error(err) => err
}";
        assert_format!(
            source,
            source,
            |p| p.eat_try(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_expression_with_catch_pattern_and_finally() {
        let source = r"try {
    foo()
} catch e {
    bar()
} finally {
    baz()
}";
        assert_format!(
            source,
            source,
            |p| p.eat_try(),
            DystFormatOptions::default()
        );
    }
}
