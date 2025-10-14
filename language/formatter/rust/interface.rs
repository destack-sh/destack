#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface { }",
            |p| p.eat_interface(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo: Bar, Baz {}",
            "interface Foo: Bar, Baz { }",
            |p| p.eat_interface(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_with() {
        assert_format!(
            "interface Foo with Bar { }",
            "interface Foo with Bar { }",
            |p| p.eat_interface(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { let X = 1 }",
            "interface Foo {\n\tlet X = 1\n}",
            |p| p.eat_interface(None, None),
            DystFormatOptions::default_tab()
        );
    }
}
