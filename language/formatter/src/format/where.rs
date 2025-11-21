use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode};
use dyst_ast::{Keyword, LocalNodeId, WhereClause};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) fn format_where_clause<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    // keyword
    write!(f, [Keyword::Where])?;
    if clauses.is_empty() {
        return Ok(());
    }
    write!(f, [space()])?;

    // clauses
    write!(
        f,
        [best_fit_parenthesize(&format_with(|f| {
            f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                .entries(clauses)
                .finish()
        }))]
    )?;

    Ok(())
}

impl<'ast> FormatNode<'ast, WhereClause> for WhereClause {
    fn format_node(
        &self,
        node_id: LocalNodeId<WhereClause>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            WhereClause::Assertion { left, right } => {
                write!(f, [*left, token(":"), space(), *right])?;
            }
            WhereClause::Guard { guard } => {
                write!(f, [*guard])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
