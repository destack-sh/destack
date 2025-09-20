use crate::{DystFormatter, FormatNode, NodeId, RangeLiteral, ScalarLiteral, TupleLiteral};
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
                    .join_with(token(", "))
                    .entries(&self.elements)
                    .finish())),
                token(")"),
            ])]
        )
    }
}

#[cfg(test)]
mod tests {
    use dyst_language_fir::format::IndentStyle;

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
            "(\n\t1,\n\t2,\n\t3,\n\t4,\n\t5\n)",
            |p| p.eat_tuple_literal(),
            DystFormatOptions::default()
                .with_line_width(10)
                .with_indent_style(IndentStyle::Tab)
        );
    }
}
