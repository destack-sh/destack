use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Keyword, LocalNodeId, SwitchCase};

use crate::{CodegenJsFormatter, FormatNode};

impl<'ast> FormatNode<'ast, SwitchCase> for SwitchCase {
    fn format_node(
        &self,
        _node_id: LocalNodeId<SwitchCase>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Case, space(), self.value, token(":")])?;
        write!(f, [block_indent(&format_with(|f| self.body.format(f)))])?;
        Ok(())
    }
}
