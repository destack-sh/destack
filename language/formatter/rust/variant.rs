use dyst_ast::{Keyword, Mutability, ScopedMutability};
use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, VariantField};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, VariantField> for VariantField {
    fn format_node(
        &self,
        node_id: NodeId<VariantField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // visibility
        if let Some(visibility) = self.visibility {
            write!(f, [visibility, space()])?;
        }

        // mutability
        if matches!(
            self.mutability,
            Some(ScopedMutability::Unscoped {
                mutability: Mutability::Immutable
            })
        ) {
            write!(f, [Keyword::Readonly, space()])?;
        }

        // name
        if let Some(name) = self.name {
            write!(f, [name, token(": ")])?;
        }

        // type
        write!(f, [self.ty])?;

        // default
        if let Some(default) = self.default {
            write!(f, [token(" = "), default])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct { }",
            "struct { }",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct { a: int32, b: boolean }",
            "struct {\n\ta: int32\n\tb: boolean\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct { readonly a: int32, private b: boolean }",
            "struct {\n\treadonly a: int32\n\tprivate b: boolean\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_representation_type() {
        assert_format!(
            "struct(uint64) Foo { a: int32 }",
            "struct(uint64) Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_fields() {
        assert_format!(
            "struct Foo(int32, boolean) { }",
            "struct Foo(int32, boolean) { }",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_body_statements() {
        assert_format!(
            "struct Foo(int32) { const X = 2 }",
            "struct Foo(int32) {\n\tconst X = 2\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a: int32 = 42, b: boolean }",
            "struct {\n\ta: int32 = 42\n\tb: boolean\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_expressions() {
        assert_format!(
            r"struct { const X = 1 }",
            r"struct {
	const X = 1
}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_statements() {
        assert_format!(
            "struct { a: int32, const X = 1 }",
            "struct {\n\ta: int32\n\n\tconst X = 1\n}",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_super_types() {
        assert_format!(
            "struct Foo<T: Numeric>: Bar, Baz { }",
            "struct Foo<T: Numeric>: Bar, Baz { }",
            |p| p.eat_struct(None, None),
            DystFormatOptions::default()
        );
    }
}
