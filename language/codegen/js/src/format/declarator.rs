use crate::{CodegenJsFormatter, Declarator, FormatNode, LocalNodeId};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Declarator>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Declarator::Binding { pattern, ty, value } => {
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
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
        }
        Ok(())
    }
}
