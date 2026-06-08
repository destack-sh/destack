use serde::{Deserialize, Serialize};

use super::{
    AnnotationIndex, CallIndex, DefinitionIndex, ImportIndex, MemberIndex, ReferenceIndex,
    SpecifierIndex, SymbolIndex,
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
    /// Definition relations and extension declarations.
    pub definitions: DefinitionIndex,
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
        self.definitions.finish();
        self.specifiers.finish();
        self.annotations.finish();
    }
}
