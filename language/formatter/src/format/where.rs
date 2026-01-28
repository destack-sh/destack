use destack_fir::format::{BestFittingMode, FormatResult};

use crate::argument::list_like;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Keyword, LocalNodeId, WhereClause};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, write};

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

pub(crate) fn format_where_clause_with_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    let inline = format_with(|f| {
        write!(f, [space()])?;
        format_where_clause(f, clauses)
    });
    let break_line = format_with(|f| {
        write!(f, [hard_line_break()])?;
        format_where_clause(f, clauses)
    });

    best_fitting![inline, break_line]
        .with_mode(BestFittingMode::AllLines)
        .format(f)?;

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
