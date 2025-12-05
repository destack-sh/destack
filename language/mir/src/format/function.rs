//! Function formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatMirNode, Function, LocalNodeId, MirFormatContext, MirFormatter, Mutability, Ownership};

impl<'a> FormatMirNode<'a, Function> for Function {
    fn format_node(
        &self,
        id: LocalNodeId<Function>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // function signature
        write!(f, [token("func @"), text(&format!("func{}", id.id)), token("(")])?;

        // parameters
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, [token(", ")])?;
            }
            write!(f, [&param.value, token(": "), param.ty])?;
        }

        write!(f, [token(") -> "), self.return_type, token(" {"), hard_line_break()])?;

        // locals (if any)
        let locals = self.locals.clone();
        let blocks = self.blocks.clone();

        if !locals.is_empty() {
            write!(
                f,
                [block_indent(&format_with(
                    |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                        let tree = f.context().tree;
                        for local_id in &locals {
                            let local = tree.get(*local_id);
                            write!(
                                f,
                                [
                                    text(&format!("local{}", local_id.id)),
                                    token(": "),
                                    local.ty
                                ]
                            )?;

                            // ownership annotation
                            match local.ownership {
                                Ownership::Owned => write!(f, [token(" ; owned")])?,
                                Ownership::Borrowed => write!(f, [token(" ; borrowed")])?,
                                Ownership::Copy => write!(f, [token(" ; copy")])?,
                            }

                            // mutability annotation
                            if local.mutability == Mutability::Mutable {
                                write!(f, [token(", var")])?;
                            }

                            write!(f, [hard_line_break()])?;
                        }
                        Ok(())
                    }
                ))]
            )?;
            write!(f, [hard_line_break()])?;
        }

        // blocks
        for (i, block_id) in blocks.iter().enumerate() {
            if i > 0 {
                write!(f, [hard_line_break()])?;
            }
            write!(f, [block_indent(block_id)])?;
        }

        write!(f, [token("}")])
    }
}
