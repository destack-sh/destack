#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::{DeclarationDescriptor, TypeKind};

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface { }",
            |p| p.eat_interface(DeclarationDescriptor::default(), TypeKind::Structural),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo extends Bar, Baz {}",
            "interface Foo extends Bar, Baz { }",
            |p| p.eat_interface(DeclarationDescriptor::default(), TypeKind::Structural),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { static X = 1 }",
            "interface Foo {\n\tstatic X = 1,\n}",
            |p| p.eat_interface(DeclarationDescriptor::default(), TypeKind::Structural),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_newtype_interface_empty() {
        assert_format!(
            "interface {}",
            "newtype interface { }",
            |p| p.eat_interface(DeclarationDescriptor::default(), TypeKind::Nominal),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_newtype_interface_with_method() {
        assert_format!(
            "interface Add<T> { add(other: T): Self }",
            "newtype interface Add<T> {\n\tadd(other: T): Self\n}",
            |p| p.eat_interface(DeclarationDescriptor::default(), TypeKind::Nominal),
            DestackFormatOptions::default_tab()
        );
    }
}
