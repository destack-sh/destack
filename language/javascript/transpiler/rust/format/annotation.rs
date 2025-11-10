use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Annotation, NodeId};

use dyst_fir::prelude::*;
use dyst_fir::write;

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: NodeId<Annotation>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        assert!(
            f.context().include_annotations(),
            "annotation in non-annotation context: {node_id:?}"
        );
        match self {
            Annotation::Doc { string } => {
                write!(f, [token("/**"), space(), string, token("*/")])?;
            }
            Annotation::Comment { string } => {
                write!(f, [token("//"), space(), string])?;
            }
        }
        Ok(())
    }
}

// nocheckin: support annotations for FormatResult
