use dyst_language_fir::format::FormatResult;
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

use crate::{DystFormatter, FormatNode, NodeId, Tuple, TupleField};

impl<'ast> FormatNode<'ast, Tuple> for Tuple {
    fn format_node(
        &self,
        _node_id: NodeId<Tuple>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.elements)
                    .finish())),
                token(")"),
            ])]
        )
    }
}

impl<'ast> FormatNode<'ast, TupleField> for TupleField {
    fn format_node(
        &self,
        _node_id: NodeId<TupleField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            TupleField::Named { name, r#type } => {
                write!(f, [name, token(":"), space(), r#type])
            }
            TupleField::Positional { r#type } => {
                write!(f, [r#type])
            }
        }
    }
}
