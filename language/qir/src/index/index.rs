use serde::{Deserialize, Serialize};

use super::{
    AnnotationIndex, CallIndex, ExtensionIndex, ImportIndex, MemberIndex, NominalIndex,
    ReferenceIndex, SpecifierIndex, SymbolIndex,
};

/// Durable query index payload for one artifact scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryIndex {
    /// Searchable symbol declarations.
    pub symbols: SymbolIndex,
    /// Searchable source members.
    pub members: MemberIndex,
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
    /// Annotation and decorator entries.
    pub annotations: AnnotationIndex,
}

impl QueryIndex {
    /// Sort and deduplicate all index sections.
    pub fn finish(&mut self) {
        self.symbols.finish();
        self.members.finish();
        self.imports.finish();
        self.references.finish();
        self.calls.finish();
        self.nominal.finish();
        self.extensions.finish();
        self.specifiers.finish();
        self.annotations.finish();
    }
}
