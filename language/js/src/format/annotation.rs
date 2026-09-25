use crate::{Annotation, LocalNodeId};
use tspp_fir::format::FormatResult;

use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Annotation>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Comment {
                position: _,
                string,
            } => {
                write!(f, [token("//"), space(), string])?;
            }
        }

        Ok(())
    }
}
