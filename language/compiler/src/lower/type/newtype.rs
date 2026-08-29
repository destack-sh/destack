use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::TypeLowerer;

impl TypeLowerer<'_, '_> {
    /// Lower one newtype declaration to its representation.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
        ty: mir::TypeId,
        arguments: &[dir::GlobalTypeId],
        copy: mir::Copy,
    ) -> CompilerResult<()> {
        // take the intrinsic representation for compiler-known newtypes
        if matches!(self.lowerer.ty(definition.backing)?, dir::Type::Intrinsic) {
            self.lower_intrinsic(symbol, ty, arguments)?;

            return Ok(());
        }

        // wrap the backing type transparently under the checked copy conformance
        let inner = self.lower(definition.backing)?;
        self.tree
            .define_type(ty, mir::Type::Newtype { inner, copy });

        Ok(())
    }
}
