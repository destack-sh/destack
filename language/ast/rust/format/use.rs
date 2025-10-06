use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, UseClause, UseItem};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, UseClause> for UseClause {
    fn format_node(
        &self,
        node_id: NodeId<UseClause>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // target expression
        write!(f, [self.target])?;

        // grouped items
        if let Some(items) = &self.items
            && !items.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token(".{"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![token(","), soft_line_break_or_space()])
                            .entries(items)
                            .finish()
                    })),
                    token("}")
                ])]
            )?;
        }
        // alias
        else if let Some(alias) = self.alias {
            write!(f, [token(" as "), alias])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, UseItem> for UseItem {
    fn format_node(
        &self,
        node_id: NodeId<UseItem>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // name and alias
        write!(f, [self.name])?;
        if let Some(alias) = self.alias {
            write!(f, [token(" as "), alias])?;
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
    fn test_format_use_simple() {
        assert_format!(
            "use foo",
            "use foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_multiple_clauses() {
        assert_format!(
            "use foo, bar, baz",
            "use foo, bar, baz",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_with_alias() {
        assert_format!(
            "use foo as bar",
            "use foo as bar",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_with_items() {
        assert_format!(
            "use foo.{bar, baz}",
            "use foo.{bar, baz}",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_use_with_annotations() {
        assert_format!(
            "use #foo foo",
            "use #foo foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }
}
