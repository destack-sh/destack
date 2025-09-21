use dyst_language_fir::format::FormatResult;

use crate::{Call, DystFormatter, FormatNode, Index, NodeId, Runtime};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Call> for Call {
    fn format_node(
        &self,
        _node_id: NodeId<Call>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if self.runtime == Runtime::Static {
            write!(f, [token("@")])?;
        }
        write!(f, [self.receiver])?;
        // todo!: static arguments
        // omit () for static calls without arguments
        if self.runtime == Runtime::Static && self.dynamic_arguments.is_empty() {
            return Ok(()); // 
        }
        // dynamic arguments
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.dynamic_arguments)
                    .finish())),
                token(")"),
            ])]
        )
    }
}

impl<'ast> FormatNode<'ast, Index> for Index {
    fn format_node(
        &self,
        _node_id: NodeId<Index>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Index::Explicit { receiver, index } => {
                write!(f, [receiver, token("["), index, token("]")])
            }
            Index::Implicit { receiver, index } => {
                write!(f, [receiver, token("."), text(&index.to_string())])
            }
        }
    }
}
