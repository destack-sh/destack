use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Select the non-nullish operand inspected by one chain segment.
    pub(in crate::sema) fn select_chain_operand(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read past the nullish arm the chain drops
        let Some(split) = self.split_nullish_type(origin, ty)? else {
            return Ok(ty);
        };
        if !is_optional {
            self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
        }

        Ok(split.value)
    }
}
