use dyst_fir::format::FormatResult;
use dyst_fir::format_args;

use crate::argument::list_like;
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
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // header
        write!(f, [Keyword::Enum])?;
        // type
        if let Some(r#type) = self.tag_type {
            write!(f, [token("("), r#type, token(")"), space()])?;
        } else {
            write!(f, [space()])?;
        }
        // name
        if let Some(name) = self.name {
            write!(f, [name])?;
        }

        // static parameters
        if let Some(static_parameters) = &self.static_parameters
            && !static_parameters.is_empty()
        {
            write!(f, [list_like("<", ">", ",", static_parameters)])?;
            write!(f, [space()])?;
        } else if self.name.is_some() {
            write!(f, [space()])?;
        }

        // empty block
        if self.fields.is_empty() && self.expressions.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id)])?;
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
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
        if !self.fields.is_empty() && !self.expressions.is_empty() {
            write!(f, [hard_line_break()])?;
            if !f.context().has_blank_prefix_annotation(self.expressions[0]) {
                write!(f, [empty_line()])?;
            }
        }

        // statements
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.expressions)
                .finish())),])]
        )?;
        write!(f, [f.context().block_infix_annotations(node_id)])?;
        write!(f, [hard_line_break(), token("}")])?;

        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: NodeId<EnumField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // name
        write!(f, [self.name])?;

        // value
        if let Some(value) = self.value {
            write!(f, [token(" = "), value])?;
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
    fn test_format_enum_with_annotations() {
        assert_format!(
            "enum { A }",
            "enum {\n\tA\n}",
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

    #[test]
    fn test_format_enum_with_static_parameters() {
        let source = r"enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1
    B = T
    @if(IsSomething)
    C = 3
}";
        assert_format!(
            source,
            source,
            |p| p.eat_enum(None),
            DystFormatOptions::default()
        );
    }
}
