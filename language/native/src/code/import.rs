use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
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
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Import {
    /// Import kind.
    kind: ImportKind,
    /// Fixed runtime operation payload.
    operation: Optional<abi::Operation>,
    /// External symbol payload.
    symbol: Optional<SymbolImport>,
}

impl Import {
    /// Create one fixed runtime operation import.
    pub fn from_operation(operation: abi::Operation) -> Self {
        Self {
            kind: ImportKind::Runtime,
            operation: Optional::some(operation),
            symbol: Optional::none(),
        }
    }

    /// Create one external symbol import.
    pub fn from_symbol(symbol: SymbolImport) -> Self {
        Self {
            kind: ImportKind::Symbol,
            operation: Optional::none(),
            symbol: Optional::some(symbol),
        }
    }

    /// Return this import as a fixed runtime operation.
    pub fn operation(self) -> Option<abi::Operation> {
        if self.kind == ImportKind::Runtime {
            self.operation.get()
        } else {
            None
        }
    }

    /// Return this import as an external symbol.
    pub fn symbol(self) -> Option<SymbolImport> {
        if self.kind == ImportKind::Symbol {
            self.symbol.get()
        } else {
            None
        }
    }
}

/// Native import kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum ImportKind {
    /// Fixed runtime operation import.
    Runtime = 0,
    /// External symbol import.
    Symbol = 1,
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
