use serde::{Deserialize, Serialize};

use super::{
    CallIndex, ExtensionIndex, ImportIndex, NominalIndex, ReferenceIndex, SpecifierIndex,
    SymbolIndex,
};

/// Durable query index payload for one artifact scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryIndex {
    /// Searchable symbol declarations.
    pub symbols: SymbolIndex,
    /// Importable module exports.
    pub imports: ImportIndex,
    /// Reference target membership by module.
    pub references: ReferenceIndex,
    /// Call graph edges.
    pub calls: CallIndex,
    /// Nominal type relations.
    pub nominal: NominalIndex,
    /// Extension declarations by target symbol.
    pub extensions: ExtensionIndex,
    /// Import specifier rewrite candidates.
    pub specifiers: SpecifierIndex,
}

impl QueryIndex {
    /// Merge many query indexes into one deterministic index.
    pub fn merge(indexes: impl IntoIterator<Item = Self>) -> Self {
        let mut merged = Self::default();

        for index in indexes {
            merged.symbols.extend(index.symbols);
            merged.imports.extend(index.imports);
            merged.references.extend(index.references);
            merged.calls.extend(index.calls);
            merged.nominal.extend(index.nominal);
            merged.extensions.extend(index.extensions);
            merged.specifiers.extend(index.specifiers);
        }

        merged.finish();

        merged
    }

    /// Sort and deduplicate all index sections.
    pub fn finish(&mut self) {
        self.symbols.finish();
        self.imports.finish();
        self.references.finish();
        self.calls.finish();
        self.nominal.finish();
        self.extensions.finish();
        self.specifiers.finish();
    }
}
