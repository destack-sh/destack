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
        copy: mir::Copy,
    ) -> CompilerResult<Vec<NominalField>> {
        // take the intrinsic representation for compiler-known newtypes
        if matches!(self.lower.ty(definition.backing)?, dir::Type::Intrinsic) {
            let representation = self.lower_intrinsic(symbol, arguments)?;
            let representation = self.tree.get(representation).clone();
            self.tree.define_type(ty, representation);

            return Ok(Vec::new());
        }

        // wrap the backing type transparently under the checked copy conformance
        let inner = self.lower(definition.backing)?;
        self.tree
            .define_type(ty, mir::Type::Newtype { inner, copy });

        Ok(Vec::new())
    }
}
