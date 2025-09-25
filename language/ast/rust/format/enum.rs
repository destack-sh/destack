use dyst_fir::format::FormatResult;
use dyst_fir::format_args;

use crate::{
    DystFormatter, Enum, EnumField, FormatNode, Keyword, NodeId, empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, Enum> for Enum {
    fn format_node(
        &self,
        node_id: NodeId<Enum>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // header
        write!(f, [Keyword::Enum])?;
        if let Some(r#type) = self.r#type {
            write!(f, [token("("), r#type, token(")"), space()])?;
        } else {
            write!(f, [space()])?;
        }
        if let Some(name) = self.name {
            write!(f, [name, space()])?;
        }
        if self.fields.is_empty() && self.statements.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id)])?;
            return Ok(());
        }

        // body
        write!(f, [token("{"), hard_line_break()])?;

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
            if !f.context().has_blank_prefix_annotation(self.statements[0]) {
                write!(f, [empty_line()])?;
            }
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

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        _node_id: NodeId<EnumField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [self.name])?;
        if let Some(value) = self.value {
            write!(f, [token(" = "), value])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_enum_empty() {
        assert_format!(
            "enum { }",
            "enum { }",
            |p| p.eat_enum(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_enum_with_simple_fields() {
        assert_format!(
            "enum { A, B }",
            "enum {\n\tA\n\tB\n}",
            |p| p.eat_enum(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_type_fields_and_values() {
        assert_format!(
            "enum(int4) { A = 1, B = 2, C, D = 4 }",
            "enum(int4) {\n\tA = 1\n\tB = 2\n\tC\n\tD = 4\n}",
            |p| p.eat_enum(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_statements() {
        assert_format!(
            r"enum { 
				let X = 1
			}",
            "enum {\n\tlet X = 1\n}",
            |p| p.eat_enum(None),
            DystFormatOptions::default_tab()
        );
    }
}
