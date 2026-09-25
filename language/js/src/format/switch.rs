use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::format::block::format_block_of_statements;
use crate::{Keyword, LocalNodeId, SwitchCase};

use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast, SwitchCase> for SwitchCase {
    fn format_node(
        &self,
        _node_id: LocalNodeId<SwitchCase>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Some(value) = self.value {
            write!(f, [Keyword::Case, space(), value, token(":")])?;
        } else {
            write!(f, [Keyword::Default, token(":")])?;
        }

        let body = f.context().tree.get(self.body);
        if !body.statements.is_empty() {
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [block_indent(&format_with(|f| {
                    format_block_of_statements(f, &body.statements)
                }))]
            )?;
        }

        Ok(())
    }
}
