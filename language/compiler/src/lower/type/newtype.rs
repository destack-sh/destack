use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{NominalField, TypeLowerer};

impl TypeLowerer<'_, '_> {
    /// Lower one newtype declaration to its MIR type.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
        ty: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<NominalField>> {
        // tagged newtypes lower their checked variants directly
        if definition.is_tagged() {
            return self.lower_tagged_newtype(definition, ty);
        }

        // compiler-known newtypes take their intrinsic representations
        if matches!(self.lowerer.ty(definition.backing)?, dir::Type::Intrinsic) {
            return self.lower_intrinsic(symbol, ty, arguments);
        }

        // wrap the backing type transparently
        let inner = self.lower(definition.backing)?;
        let copy = self.tree.get(inner).copy(self.tree);
        self.tree
            .define_type(ty, mir::Type::Newtype { inner, copy });

        Ok(Vec::new())
    }

    /// Lower one tagged newtype declaration to its variant carrier.
    fn lower_tagged_newtype(
        &mut self,
        definition: dir::NewtypeDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // lower each checked case and its exact backing leaf together
        let mut cases = Vec::new();
        let mut payloads = Vec::new();
        for variant in definition.tagged_variants() {
            cases.push(NominalField {
                key: variant.key,
                symbol: variant.symbol.local_id,
            });
            payloads.push(self.lower(variant.backing)?);
        }

        // concrete payload representations decide whether values copy or move
        let is_copy = payloads
            .iter()
            .all(|payload| self.tree.get(*payload).copy(self.tree) == mir::Copy::Yes);
        let copy = if is_copy {
            mir::Copy::Yes
        } else {
            mir::Copy::No
        };
        let variant = self.variant_type(payloads, copy);
        self.tree.define_type(ty, variant);

        Ok(cases)
    }
}
