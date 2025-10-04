use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, StructField};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, StructField> for StructField {
    fn format_node(
        &self,
        node_id: NodeId<StructField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

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
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct { }",
            "struct { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct { a: int32, b: boolean }",
            "struct {\n\ta: int32\n\tb: boolean\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_representation_type() {
        assert_format!(
            "struct(uint64) Foo { a: int32 }",
            "struct(uint64) Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_fields() {
        assert_format!(
            "struct Foo(int32, boolean) { }",
            "struct Foo(int32, boolean) { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_body_statements() {
        assert_format!(
            "struct Foo(int32) { let X = 2 }",
            "struct Foo(int32) {\n\tlet X = 2\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a: int32 = 42, b: boolean }",
            "struct {\n\ta: int32 = 42\n\tb: boolean\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_statements() {
        assert_format!(
            r"struct { let X = 1 }",
            r"struct {
	let X = 1
}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_statements() {
        assert_format!(
            "struct { a: int32, let X = 1 }",
            "struct {\n\ta: int32\n\n\tlet X = 1\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_super_types() {
        assert_format!(
            "struct Foo<T: Numeric>: Bar, Baz { }",
            "struct Foo<T: Numeric>: Bar, Baz { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }
}
