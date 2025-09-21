use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Keyword, NodeId, Use, UseClause, UseItem, Visibility};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Use> for Use {
    fn format_node(
        &self,
        _node_id: NodeId<Use>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // visibility
        if let Some(visibility) = self.visibility {
            let keyword = match visibility {
                Visibility::Public => Keyword::Public,
                Visibility::Private => Keyword::Private,
            };
            write!(f, [keyword, space()])?;
        }

        // keyword
        write!(f, [Keyword::Use, space()])?;
        {
            let mut first = true;
            for clause in &self.clauses {
                if !first {
                    write!(f, [token(", ")])?;
                }
                first = false;
                write!(f, [*clause])?;
            }
        }

        // scoped body
        if let Some(body) = self.body {
            write!(f, [space(), body])?;
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, UseClause> for UseClause {
    fn format_node(
        &self,
        _node_id: NodeId<UseClause>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
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

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, UseItem> for UseItem {
    fn format_node(
        &self,
        _node_id: NodeId<UseItem>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // name and alias
        write!(f, [self.name])?;
        if let Some(alias) = self.alias {
            write!(f, [token(" as "), alias])?;
        }
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
            |p| p.eat_use(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_multiple_clauses() {
        assert_format!(
            "use foo, bar, baz",
            "use foo, bar, baz",
            |p| p.eat_use(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_with_alias() {
        assert_format!(
            "use foo as bar",
            "use foo as bar",
            |p| p.eat_use(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_use_with_items() {
        assert_format!(
            "use foo.{bar, baz}",
            "use foo.{bar, baz}",
            |p| p.eat_use(None),
            DystFormatOptions::default_with_line_width(60)
        );
    }
}
