use crate::{Declarator, FormatNode, Formatter, LocalNodeId};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Declarator>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        let Declarator { pattern, value } = self;

        // pattern
        write!(f, [pattern])?;

        // value
        if let Some(value) = value {
            write!(f, [space()])?;
            write!(f, [token("=")])?;
            write!(f, [space()])?;
            write!(f, [value])?;
        }

        Ok(())
    }
}
