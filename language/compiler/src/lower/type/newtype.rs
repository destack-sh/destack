use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{NominalField, TypeLowerer};

impl TypeLowerer<'_, '_> {
    /// Lower one newtype declaration to its representation.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
        ty: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<NominalField>> {
        // take the intrinsic representation for compiler-known newtypes
        if matches!(self.lowerer.ty(definition.backing)?, dir::Type::Intrinsic) {
            self.lower_intrinsic(symbol, ty, arguments)?;

            return Ok(Vec::new());
        }

        // wrap the backing type transparently
        let inner = self.lower(definition.backing)?;
        let copy = self.tree.get(inner).copy(self.tree);
        self.tree
            .define_type(ty, mir::Type::Newtype { inner, copy });

        Ok(Vec::new())
    }
}
