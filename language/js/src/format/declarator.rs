use crate::{Declarator, FormatNode, Formatter};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast> for Declarator {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let Declarator { pattern, value } = self;

        // pattern
        write!(f, [pattern])?;

        // value
        if let Some(value) = value {
            write!(f, [space()])?;
            write!(f, [token("=")])?;
            write!(f, [space()])?;
            write!(f, [value])?;
        }

        Ok(())
    }
}
