use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::abi;

/// Native imports required by one native code payload.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ImportTable {
    /// Native imports in linker order.
    import: SectionSlice<Import>,
}

/// Build-time native import table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportTableBuilder {
    /// Native imports in linker order.
    imports: Vec<Import>,
}

impl ImportTableBuilder {
    /// Create an empty native import table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set native imports in linker order.
    pub fn imports(mut self, imports: impl IntoIterator<Item = Import>) -> Self {
        self.imports = imports.into_iter().collect();

        self
    }

    /// Build this import table into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> ImportTable {
        ImportTable {
            import: sections.insert(self.imports),
        }
    }
}

impl ImportTable {
    /// Create one empty native import table.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return native imports in linker order.
    pub fn imports<'a>(&self, sections: SectionImage<'a>) -> &'a [Import] {
        sections.entries(self.import)
    }
}

/// One native import required by generated native code.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum Import {
    /// Fixed runtime operation import.
    Runtime(abi::Operation),
    /// External linker-visible symbol import.
    Symbol(SymbolImport),
}

impl Import {
    /// Return this import as a fixed runtime operation.
    pub fn operation(self) -> Option<abi::Operation> {
        match self {
            Self::Runtime(operation) => Some(operation),
            Self::Symbol(_) => None,
        }
    }

    /// Return this import as an external symbol.
    pub fn symbol(self) -> Option<SymbolImport> {
        match self {
            Self::Runtime(_) => None,
            Self::Symbol(symbol) => Some(symbol),
        }
    }
}

impl From<abi::Operation> for Import {
    /// Convert one fixed runtime operation import.
    fn from(operation: abi::Operation) -> Self {
        Self::Runtime(operation)
    }
}

impl From<SymbolImport> for Import {
    /// Convert one external linker-visible symbol import.
    fn from(symbol: SymbolImport) -> Self {
        Self::Symbol(symbol)
    }
}

/// External linker-visible symbol import.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SymbolImport {
    /// The imported native symbol.
    pub symbol: StringId,
}

impl SymbolImport {
    /// Create one native symbol import.
    pub const fn new(symbol: StringId) -> Self {
        Self { symbol }
    }
}
