use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, ImportClause, ImportItem, NodeId};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, ImportClause> for ImportClause {
    fn format_node(
        &self,
        node_id: NodeId<ImportClause>,
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

impl<'ast> FormatNode<'ast, ImportItem> for ImportItem {
    fn format_node(
        &self,
        node_id: NodeId<ImportItem>,
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
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_import_simple() {
        assert_format!(
            "import foo",
            "import foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_multiple_clauses() {
        assert_format!(
            "import foo, bar, baz",
            "import foo, bar, baz",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_alias() {
        assert_format!(
            "import foo as bar",
            "import foo as bar",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_items() {
        assert_format!(
            "import foo.{bar, baz}",
            "import foo.{bar, baz}",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }
    #[test]
    fn test_format_import_with_items_from() {
        assert_format!(
            "import {bar, baz} from foo",
            "import foo.{bar, baz}",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_import_with_annotations() {
        assert_format!(
            "import #foo foo",
            "import #foo foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }
}
