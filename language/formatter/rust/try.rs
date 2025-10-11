#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_try_expression() {
        assert_format!(
            "try operation()",
            "try operation()",
            |p| p.eat_try(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_block() {
        assert_format!(
            "try { let X = 1 }",
            "try {\n\tlet X = 1\n}",
            |p| p.eat_try(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_try_block_with_catch() {
        assert_format!(
            "try { let X = risky() } catch err { Error(err) => err }",
            "try {\n\tlet X = risky()\n} catch err {\n\tError(err) => err\n}",
            |p| p.eat_try(None),
            DystFormatOptions::default_tab()
        );
    }
}
