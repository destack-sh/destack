#[cfg(test)]
mod tests {
    use crate::{DystFormatOptions, TestFormatter, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_extension_empty() {
        assert_format!(
            "extension Foo {}",
            "extension Foo { }",
            |p| p.eat_extension(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_extension_with_implements() {
        assert_format!(
            "extension Foo implements Bar { static X = 1 }",
            "extension Foo implements Bar {\n\tstatic X = 1\n}",
            |p| p.eat_extension(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_extension_with_static_arguments() {
        assert_format!(
            "extension<T> Foo<T> { }",
            "extension<T> Foo<T> { }",
            |p| p.eat_extension(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }
}
