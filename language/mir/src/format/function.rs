//! Function formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    FormatMirNode, Function, Linkage, LocalNodeId, MirFormatContext, MirFormatter, Mutability,
    Ownership,
};

impl<'a> FormatMirNode<'a, Function> for Function {
    fn format_node(
        &self,
        _id: LocalNodeId<Function>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let name = f.context().strings.get(self.name);

        // imported function: extern function @name(i32, i32) -> void
        if self.linkage.is_import() {
            write!(
                f,
                [
                    token("extern"),
                    space(),
                    token("function"),
                    space(),
                    token("@"),
                    text(name)
                ]
            )?;

            // parameters (just types for extern)
            write!(f, [token("(")])?;
            for (i, param) in self.parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [param.ty])?;
            }
            write!(f, [token(")")])?;

            return write!(f, [space(), token("->"), space(), self.return_type]);
        }

        // linkage prefix for exported functions
        if self.linkage == Linkage::Export {
            write!(f, [token("export"), space()])?;
        }

        // build block and local index maps for this function
        {
            let context = f.context_mut();
            context.block_indices.clear();
            context.local_indices.clear();
            for (i, block_id) in self.blocks.iter().enumerate() {
                context.block_indices.insert(*block_id, i);
            }
            for (i, local_id) in self.locals.iter().enumerate() {
                context.local_indices.insert(*local_id, i);
            }
        }

        // function signature: function @name(v0: i32, v1: i32) -> void {
        write!(f, [token("function"), space(), token("@"), text(name)])?;

        // parameters
        write!(f, [token("(")])?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [&param.value, token(":"), space(), param.ty])?;
        }
        write!(f, [token(")")])?;

        write!(
            f,
            [
                space(),
                token("->"),
                space(),
                self.return_type,
                space(),
                token("{"),
                hard_line_break()
            ]
        )?;

        // locals (if any)
        let locals = self.locals.clone();
        let blocks = self.blocks.clone();

        if !locals.is_empty() {
            write!(
                f,
                [block_indent(&format_with(
                    |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                        for (local_index, local_id) in locals.iter().enumerate() {
                            let local = f.context().tree.get(*local_id);
                            write!(
                                f,
                                [
                                    text(&format!("local{local_index}")),
                                    token(":"),
                                    space(),
                                    local.ty
                                ]
                            )?;

                            // ownership annotation
                            write!(f, [space(), token(";"), space()])?;
                            match local.ownership {
                                Ownership::Owned => write!(f, [token("owned")])?,
                                Ownership::Borrowed => write!(f, [token("borrowed")])?,
                                Ownership::Copy => write!(f, [token("copy")])?,
                            }

                            // mutability annotation
                            if local.mutability == Mutability::Mutable {
                                write!(f, [token(","), space(), token("var")])?;
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
        for block_id in &blocks {
            write!(f, [block_id, hard_line_break()])?;
        }

        write!(f, [token("}")])
    }
}
