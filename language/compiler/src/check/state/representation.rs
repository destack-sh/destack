use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{CheckState, TypeOperand};
use crate::{CompilerError, CompilerResult};

/// Checked representation facts before DIR commit.
#[derive(Debug, Clone, Default)]
pub(in crate::check) struct RepresentationTable {
    /// Newtype representations keyed by declaring symbol.
    newtypes: IndexMap<dir::GlobalSymbolId, NewtypeRepresentation>,
}

impl RepresentationTable {
    /// Create an empty representation table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Insert one newtype representation.
    pub(in crate::check) fn insert_newtype(&mut self, representation: NewtypeRepresentation) {
        self.newtypes.insert(representation.symbol, representation);
    }

    /// Return one newtype representation.
    pub(in crate::check) fn newtype(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&NewtypeRepresentation> {
        self.newtypes.get(&symbol)
    }

    /// Iterate newtype representations in insertion order.
    pub(in crate::check) fn iter_newtypes(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, &NewtypeRepresentation)> + '_ {
        self.newtypes
            .iter()
            .map(|(symbol, representation)| (*symbol, representation))
    }
}

/// Runtime representation of one checked newtype before DIR commit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct NewtypeRepresentation {
    /// The newtype symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The backing type operand.
    pub(in crate::check) backing: TypeOperand,
}

impl CheckState<'_> {
    /// Return one newtype representation visible from a component module.
    pub(in crate::check) fn newtype_representation(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<NewtypeRepresentation>> {
        if self.modules.contains_key(&symbol.module_id) {
            return Ok(self.representations.newtype(symbol).copied());
        }

        if self.module(module).dependencies.contains(&symbol.module_id) {
            let dependency = self.load_dependency(symbol.module_id)?;
            let representation =
                dependency
                    .layouts
                    .newtype_representation(symbol)
                    .map(|representation| NewtypeRepresentation {
                        symbol: representation.symbol,
                        backing: representation.backing.into(),
                    });

            return Ok(representation);
        }

        Err(CompilerError::Internal {
            message: format!("symbol {symbol:?} was not loaded for module {module:?}"),
        })
    }
}
