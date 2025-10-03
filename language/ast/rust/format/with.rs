use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, WithClause};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, WithClause> for WithClause {
    fn format_node(
        &self,
        node_id: NodeId<WithClause>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            WithClause::Declaration { target } => {
                // target
                write!(f, [*target])?;
            }
            WithClause::Assertion { target, assertion } => {
                // target and assertion
                write!(f, [*target, token(": "), *assertion])?;
            }
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
    fn test_format_with_single_clause() {
        assert_format!(
            "with Foo",
            "with Foo",
            |p| p.eat_with(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_with_alias() {
        assert_format!(
            "with Foo as Bar",
            "with Foo as Bar",
            |p| p.eat_with(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_with_assertion() {
        assert_format!(
            "with T: Numeric",
            "with T: Numeric",
            |p| p.eat_with(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_with_breaks() {
        assert_format!(
            "with Foo, Bar, Baz, Qux, Quux",
            "with (\n\tFoo\n\tBar\n\tBaz\n\tQux\n\tQuux\n)",
            |p| p.eat_with(),
            DystFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_with_annotations() {
        assert_format!(
            "with #foo Foo",
            "with #foo Foo",
            |p| p.eat_with(),
            DystFormatOptions::default()
        );
    }
}
