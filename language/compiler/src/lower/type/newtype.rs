use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::{ModuleLowerer, Nominal, NominalField};

impl ModuleLowerer<'_> {
    /// Lower one newtype declaration to its MIR type.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
    ) -> CompilerResult<Nominal> {
        // tagged newtypes lower their checked variants directly
        if definition.is_tagged() {
            return self.lower_tagged_newtype(tree, symbol, definition);
        }

        // wrap the backing type transparently
        let inner = self.lower_type_id(tree, definition.backing)?;
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = tree.insert(mir::Type::Newtype { inner, copy });

        Ok(Nominal {
            ty,
            value: ty,
            fields: Vec::new(),
        })
    }

    /// Lower one tagged newtype declaration to its variant carrier.
    fn lower_tagged_newtype(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
    ) -> CompilerResult<Nominal> {
        // lower each checked case and its exact backing leaf together
        let mut cases = Vec::new();
        let mut payloads = Vec::new();
        for variant in definition.tagged_variants() {
            cases.push(NominalField {
                key: variant.key,
                symbol: variant.symbol.local_id,
            });
            payloads.push(self.lower_type_id(tree, variant.backing)?);
        }

        // conformance decides whether values copy or move
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = self.variant_type(tree, payloads, copy);

        Ok(Nominal {
            ty,
            value: ty,
            fields: cases,
        })
    }
}
