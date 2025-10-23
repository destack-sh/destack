#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DeclarationKind;

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface { }",
            |p| p.eat_interface(DeclarationKind::Definition, None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo: Bar, Baz {}",
            "interface Foo: Bar, Baz { }",
            |p| p.eat_interface(DeclarationKind::Definition, None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_with() {
        assert_format!(
            "interface Foo with Bar { }",
            "interface Foo with Bar { }",
            |p| p.eat_interface(DeclarationKind::Definition, None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { const X = 1 }",
            "interface Foo {\n\tconst X = 1\n}",
            |p| p.eat_interface(DeclarationKind::Definition, None, None),
            DystFormatOptions::default_tab()
        );
    }
}
