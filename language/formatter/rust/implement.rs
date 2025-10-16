#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_implement_empty() {
        assert_format!(
            "implement Foo {}",
            "implement Foo { }",
            |p| p.eat_implement(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_implement_with_for() {
        assert_format!(
            "implement Foo: Bar { let X = 1 }",
            "implement Foo: Bar {\n\tlet X = 1\n}",
            |p| p.eat_implement(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_implement_with_static_arguments() {
        assert_format!(
            "implement<T> Foo<T> { }",
            "implement<T> Foo<T> { }",
            |p| p.eat_implement(None, None),
            DystFormatOptions::default()
        );
    }
}
