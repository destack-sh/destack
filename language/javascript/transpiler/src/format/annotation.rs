use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Annotation, LocalNodeId};

use dyst_fir::prelude::*;
use dyst_fir::write;

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: LocalNodeId<Annotation>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        assert!(
            f.context().include_annotations(),
            "annotation in non-annotation context: {node_id:?}"
        );
        match self {
            Annotation::Doc {
                position: _,
                string,
            } => {
                write!(f, [token("/**"), space(), string, token("*/")])?;
            }
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
