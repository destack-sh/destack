#[cfg(test)]
mod tests {
    use crate::{DystFormatOptions, TestFormatter, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface { }",
            |p| p.eat_interface(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo extends Bar, Baz {}",
            "interface Foo extends Bar, Baz { }",
            |p| p.eat_interface(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_with() {
        assert_format!(
            "interface Foo with Bar { }",
            "interface Foo with Bar { }",
            |p| p.eat_interface(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { static X = 1 }",
            "interface Foo {\n\tstatic X = 1\n}",
            |p| p.eat_interface(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }
}
