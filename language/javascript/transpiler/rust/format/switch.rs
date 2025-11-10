use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;

use dyst_javascript_ast::{Keyword, NodeId, SwitchCase};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, SwitchCase> for SwitchCase {
    fn format_node(
        &self,
        node_id: NodeId<SwitchCase>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Case, space(), self.value, token(":")])?;
        write!(f, [block_indent(&format_with(|f| self.body.format(f)))])?;
        Ok(())
    }
}
