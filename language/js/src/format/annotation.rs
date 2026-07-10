use crate::{Annotation, LocalNodeId};
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatNode, JsFormatter};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: LocalNodeId<Annotation>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        assert!(
            f.context().include_types(),
            "annotation in non-annotation context: {node_id:?}"
        );
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
