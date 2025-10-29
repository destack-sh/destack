#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DefinitionMeta;

    #[test]
    fn test_format_extension_empty() {
        assert_format!(
            "extension Foo {}",
            "extension Foo { }",
            |p| p.eat_extension(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_extension_with_for() {
        assert_format!(
            "extension Foo: Bar { const X = 1 }",
            "extension Foo: Bar {\n\tconst X = 1\n}",
            |p| p.eat_extension(DefinitionMeta::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_extension_with_static_arguments() {
        assert_format!(
            "extension<T> Foo<T> { }",
            "extension<T> Foo<T> { }",
            |p| p.eat_extension(DefinitionMeta::default()),
            DystFormatOptions::default()
        );
    }
}
