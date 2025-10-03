#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_trait_empty() {
        assert_format!(
            "trait {}",
            "trait { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_supers() {
        assert_format!(
            "trait Foo: Bar, Baz {}",
            "trait Foo: Bar, Baz { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_with_clause() {
        assert_format!(
            "trait Foo with Bar { }",
            "trait Foo with Bar { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_body() {
        assert_format!(
            "trait Foo { let X = 1 }",
            "trait Foo {\n\tlet X = 1\n}",
            |p| p.eat_trait(None),
            DystFormatOptions::default_tab()
        );
    }
}
