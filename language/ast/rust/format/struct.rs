use dyst_language_fir::format::FormatResult;
use dyst_language_fir::format_args;

use crate::{DystFormatter, FormatNode, Keyword, NodeId, Struct, StructField};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

impl<'ast> FormatNode<'ast, Struct> for Struct {
    fn format_node(
        &self,
        _node_id: NodeId<Struct>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // header
        write!(f, [Keyword::Struct])?;
        if let Some(representation_type) = self.representation_type {
            write!(f, [token("("), representation_type, token(")"), space()])?;
        } else {
            write!(f, [space()])?;
        }
        if let Some(name) = self.name {
            write!(f, [name, space()])?;
        }
        if self.fields.is_empty() && self.statements.is_empty() {
            write!(f, [token("{ }")])?;
            return Ok(());
        }
        write!(f, [token("{"), hard_line_break()])?;
        // body
        // fields
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.fields)
                .finish())),])]
        )?;
        // blank line
        if !self.fields.is_empty() && !self.statements.is_empty() {
            write!(f, [hard_line_break()])?;
        }
        // statements
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.statements)
                .finish())),])]
        )?;
        write!(f, [hard_line_break(), token("}")])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, StructField> for StructField {
    fn format_node(
        &self,
        _node_id: NodeId<StructField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Some(name) = self.name {
            write!(f, [name, token(": ")])?;
        }
        write!(f, [self.r#type])?;
        if let Some(default) = self.default {
            write!(f, [token(" = "), default])?;
        }
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
}
