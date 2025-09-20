use crate::{
    ArrayLiteral, DystFormatter, FieldLiteral, FormatNode, NodeId, RangeLiteral, ScalarLiteral,
    StructLiteral, TupleLiteral,
};
use dyst_language_fir::format::{Format, FormatResult, group, text, token};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, ScalarLiteral> for ScalarLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<ScalarLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match *self {
            ScalarLiteral::Void => token("void").format(f),
            ScalarLiteral::Null => token("null").format(f),
            ScalarLiteral::Boolean(value) => token(if value { "true" } else { "false" }).format(f),
            ScalarLiteral::Byte(value) => text(&value.to_string()).format(f),
            ScalarLiteral::Integer(value, _) => text(&value.to_string()).format(f),
            ScalarLiteral::Float(value, _) => text(&value.to_string()).format(f),
            ScalarLiteral::Character(value) => text(&value.to_string()).format(f),
            _ => todo!("strings!"),
        }
    }
}

impl<'ast> FormatNode<'ast, RangeLiteral> for RangeLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<RangeLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [self.start, token(".."), self.end,])
    }
}

impl<'ast> FormatNode<'ast, TupleLiteral> for TupleLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<TupleLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.elements)
                    .finish())),
                token(")"),
            ])]
        )
    }
}

impl<'ast> FormatNode<'ast, ArrayLiteral> for ArrayLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<ArrayLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            ArrayLiteral::Fixed { elements } => {
                write!(
                    f,
                    [group(&format_args![
                        token("["),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(elements)
                            .finish())),
                        token("]"),
                    ])]
                )
            }
        }
    }
}

impl<'ast> FormatNode<'ast, StructLiteral> for StructLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<StructLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [group(&format_args![
                self.r#type,
                space(),
                token("{"),
                if_group_fits_on_line(&space()),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.fields)
                    .finish())),
                if_group_fits_on_line(&space()),
                token("}"),
            ])]
        )
    }
}

impl<'ast> FormatNode<'ast, FieldLiteral> for FieldLiteral {
    fn format_node(
        &self,
        _node_id: NodeId<FieldLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            FieldLiteral::Named { name, value } => {
                write!(f, [name, token(": "), value])
            }
            FieldLiteral::NamedShorthand { name } => {
                write!(f, [name])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_void() {
        assert_format!("void", "void", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_null() {
        assert_format!("null", "null", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_boolean() {
        assert_format!("true", "true", |p| p.eat_scalar_literal());
        assert_format!("false", "false", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_integer() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    // todo!: test more literals

    /// If the tuple fits on a single line, it should print on a single line.
    #[test]
    fn test_format_tuple_short() {
        assert_format!("(1, 2, 3)", "(1, 2, 3)", |p| p.eat_tuple_literal());
    }

    /// If the tuple doesn't fit on a single line, it should print one element per line (indented).
    #[test]
    fn test_format_tuple_long() {
        assert_format!(
            "(1, 2, 3, 4, 5)",
            "(\n\t1\n\t2\n\t3\n\t4\n\t5\n)",
            |p| p.eat_tuple_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }

    /// If the array fits on a single line, it should print on a single line.
    #[test]
    fn test_format_array_short() {
        assert_format!("[1, 2, 3]", "[1, 2, 3]", |p| p.eat_array_literal());
    }

    /// If the array doesn't fit on a single line, it should print one element per line (indented).
    #[test]
    fn test_format_array_long() {
        assert_format!(
            "[1, 2, 3, 4, 5]",
            "[\n\t1\n\t2\n\t3\n\t4\n\t5\n]",
            |p| p.eat_array_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }

    /// If the struct fits on a single line, it should print on a single line.
    #[test]
    fn test_format_struct_short() {
        assert_format!("Vector2 { x: 1, y: 2 }", "Vector2 { x: 1, y: 2 }", |p| p
            .eat_struct_literal());
    }

    /// If the struct doesn't fit on a single line, it should print one field per line (indented).
    #[test]
    fn test_format_struct_long() {
        assert_format!(
            "Vector4 { x: 1, y: 2, z: 3, w: 4 }",
            "Vector4 {\n\tx: 1\n\ty: 2\n\tz: 3\n\tw: 4\n}",
            |p| p.eat_struct_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }
}
