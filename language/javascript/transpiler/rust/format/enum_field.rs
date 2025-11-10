use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{EnumField, NodeId};
use dyst_fir::write;
use dyst_fir::prelude::*;

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        _node_id: NodeId<EnumField>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [self.name])?;
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }
        Ok(())
    }
}
