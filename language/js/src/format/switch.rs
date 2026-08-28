use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::block::format_block_of_statements;
use crate::{Keyword, SwitchCase};

use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast> for SwitchCase {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        if let Some(value) = self.value {
            write!(f, [Keyword::Case, space(), value, token(":")])?;
        } else {
            write!(f, [Keyword::Default, token(":")])?;
        }

        if !self.body.is_empty() {
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [block_indent(&format_with(|f| format_block_of_statements(
                    f, &self.body
                )))]
            )?;
        }

        Ok(())
    }
}
