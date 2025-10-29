#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DefinitionMeta;

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface { }",
            |p| p.eat_interface(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo extends Bar, Baz {}",
            "interface Foo extends Bar, Baz { }",
            |p| p.eat_interface(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_with() {
        assert_format!(
            "interface Foo with Bar { }",
            "interface Foo with Bar { }",
            |p| p.eat_interface(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { const X = 1 }",
            "interface Foo {\n\tconst X = 1\n}",
            |p| p.eat_interface(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }
}
