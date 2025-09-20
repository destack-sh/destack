use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Keyword, Let, Mutability, NodeId};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

impl<'ast> FormatNode<'ast, Let> for Let {
    fn format_node(
        &self,
        _node_id: NodeId<Let>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [if self.mutability == Mutability::Mutable {
                Keyword::Var
            } else {
                Keyword::Let
            }]
        )?;
		write!(f, [self.pattern])?;
		if let Some(r#type) = self.r#type {
			write!(f, [token(": "), r#type])?;
		}
		if let Some(value) = self.value {
			write!(f, [token(" = "), value])?;
		}
		Ok(())
    }
}
