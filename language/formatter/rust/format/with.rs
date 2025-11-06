use dyst_fir::format::FormatResult;

use crate::{FormatNode, Keyword, LanguageFormatter, NodeId, WithClause};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) fn format_with_clause<'ast>(
    f: &mut LanguageFormatter<'ast, '_>,
    with: &[NodeId<WithClause>],
) -> FormatResult<()> {
    // keyword
    write!(f, [Keyword::With])?;
    if with.is_empty() {
        return Ok(());
    }
    write!(f, [space()])?;

    // clauses
    write!(
        f,
        [best_fit_parenthesize(&format_with(|f| {
            f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                .entries(with)
                .finish()
        }))]
    )?;

    Ok(())
}

impl<'ast> FormatNode<'ast, WithClause> for WithClause {
    fn format_node(
        &self,
        node_id: NodeId<WithClause>,
        f: &mut LanguageFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        if let Some(alias) = self.alias {
            write!(f, [alias, token(":"), space()])?;
        }
        write!(f, [self.right])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
