use crate::{Declarator, FormatNode, JsFormatter, LocalNodeId};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Declarator>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let Declarator { pattern, ty, value } = self;

        // pattern
        write!(f, [pattern])?;

        // type
        if f.context().include_types()
            && let Some(ty) = ty
        {
            write!(f, [token(":"), space(), ty])?;
        }

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
