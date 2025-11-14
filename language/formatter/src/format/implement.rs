#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_implement_empty() {
        assert_format!(
            "implement Foo {}",
            "implement Foo { }",
            |p| p.eat_implement(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_implement_with_implements() {
        assert_format!(
            "implement Foo implements Bar { static X = 1 }",
            "implement Foo implements Bar {\n\tstatic X = 1\n}",
            |p| p.eat_implement(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_implement_with_static_arguments() {
        assert_format!(
            "implement<T> Foo<T> { }",
            "implement<T> Foo<T> { }",
            |p| p.eat_implement(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }
}
