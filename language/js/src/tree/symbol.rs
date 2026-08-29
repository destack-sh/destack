use destack_core::{Arena, StringId};
use destack_serde::Reflect;
use destack_source::{ProvenanceId, ProvenanceJournal};
use serde::{Deserialize, Serialize};

use crate::ScopeId;

/// One stable JavaScript symbol identifier.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct SymbolId(pub u32);

/// One JavaScript symbol namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum SymbolNamespace {
    /// Runtime values and lexical bindings.
    Value,
    /// Statement labels.
    Label,
    /// Private class names.
    Private,
}

/// One JavaScript symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Symbol {
    /// The emitted name.
    pub name: StringId,
    /// The canonical linked symbol.
    pub link: SymbolId,
    /// The symbol namespace.
    pub namespace: SymbolNamespace,
    /// The declaring scope.
    pub scope: ScopeId,
    /// The provenance of the symbol.
    pub provenance: ProvenanceId,
}

/// The symbols declared by one JavaScript module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SymbolTable {
    /// The symbols in allocation order.
    symbols: Arena<Symbol>,
}

impl SymbolTable {
    /// Create an empty symbol table for one module.
    pub fn new() -> Self {
        Self {
            symbols: Arena::new(),
        }
    }

    /// Allocate one symbol.
    pub(crate) fn insert(
        &mut self,
        name: StringId,
        namespace: SymbolNamespace,
        scope: ScopeId,
        provenance: ProvenanceId,
    ) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        let symbol = Symbol {
            name,
            link: id,
            namespace,
            scope,
            provenance,
        };
        self.symbols.allocate(symbol);

        id
    }

    /// Return one symbol.
    #[inline]
    pub fn get(&self, id: SymbolId) -> &Symbol {
        self.symbols.get(id.0)
    }

    /// Iterate over symbols in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, &Symbol)> {
        self.symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (SymbolId(index as u32), symbol))
    }

    /// Return one mutable symbol.
    #[inline]
    fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        self.symbols.get_mut(id.0)
    }

    /// Return the canonical symbol reached through symbol links.
    pub fn canonical(&self, mut id: SymbolId) -> SymbolId {
        loop {
            let link = self.get(id).link;
            if link == id {
                return id;
            }

            id = link;
        }
    }

    /// Rename one canonical symbol and record the transformation.
    pub fn rename(&mut self, id: SymbolId, name: StringId, provenance: &mut ProvenanceJournal<'_>) {
        let id = self.canonical(id);
        let source = self.get(id).provenance;
        let symbol = self.get_mut(id);
        symbol.name = name;
        symbol.provenance = provenance.derive(source);
    }

    /// Link one canonical symbol to another and record the transformation.
    pub fn link(
        &mut self,
        source: SymbolId,
        target: SymbolId,
        provenance: &mut ProvenanceJournal<'_>,
    ) {
        let source = self.canonical(source);
        let target = self.canonical(target);
        if source == target {
            return;
        }

        let source_symbol = self.get(source);
        let target_symbol = self.get(target);
        assert_eq!(
            source_symbol.namespace, target_symbol.namespace,
            "linked JS symbols belong to different namespaces"
        );
        let linked_provenance =
            provenance.fuse(&[source_symbol.provenance, target_symbol.provenance]);

        let source_symbol = self.get_mut(source);
        source_symbol.link = target;
        source_symbol.provenance = linked_provenance;
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
