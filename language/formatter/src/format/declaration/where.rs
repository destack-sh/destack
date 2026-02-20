use destack_fir::format::FormatResult;

use crate::collection::list_like;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Keyword, LocalNodeId, WhereClause};
use destack_fir::prelude::*;
use destack_fir::write;

/// Format a where clause list.
pub(crate) fn format_where_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    // keyword
    write!(f, [Keyword::Where])?;
    if clauses.is_empty() {
        return Ok(());
    }
    write!(f, [space()])?;

    // clauses
    if clauses.len() == 1 {
        write!(f, [&clauses[0]])?;
    } else {
        let clauses_vec = clauses.to_vec();
        write!(f, [list_like("(", ")", ",", &clauses_vec)])?;
    }

    Ok(())
}

/// Format a where clause list in a soft break group.
pub(crate) fn format_where_clause_with_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    write!(
        f,
        [group(&destack_fir::format_args![
            soft_line_break_or_space(),
            format_with(|f| format_where_clause(f, clauses)),
        ])]
    )?;

    Ok(())
}

impl<'ast> FormatNode<'ast, WhereClause> for WhereClause {
    fn format_node(
        &self,
        node_id: LocalNodeId<WhereClause>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        write!(f, [self.left, token(":"), space(), self.right])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
