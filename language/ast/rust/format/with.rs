use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Keyword, NodeId, WithClause};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) fn format_with_clause<'ast>(
    f: &mut DystFormatter<'ast, '_>,
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
            f.join_with(&format_args![
                if_group_fits_on_line(&token(",")),
                soft_line_break_or_space()
            ])
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
