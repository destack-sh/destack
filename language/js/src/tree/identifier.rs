use destack_core::StringId;
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use crate::{SymbolId, SymbolTable};

/// One lexical identifier occurrence.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Identifier {
    /// The identifier name before JavaScript renaming.
    pub original_name: StringId,
    /// The referenced symbol.
    pub symbol: SymbolId,
    /// The provenance of this occurrence.
    pub provenance: ProvenanceId,
}

impl Identifier {
    /// Return the emitted name of this identifier.
    pub fn emitted_name(self, symbols: &SymbolTable) -> StringId {
        let symbol = symbols.canonical(self.symbol);

        symbols.get(symbol).name
    }
}

/// One fixed ECMAScript identifier name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct IdentifierName {
    /// The identifier text.
    pub text: StringId,
    /// The provenance of this occurrence.
    pub provenance: ProvenanceId,
}
