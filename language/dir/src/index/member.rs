use crate::{GlobalNodeIdAny, GlobalSymbolId, ImplementationEdge, Postings};
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Indexed member declarations and implementations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberIndex {
    /// Symbol-backed members ordered by source node.
    entries: Vec<MemberEntry>,
    /// Exact member implementation edges ordered by declaration symbol.
    implementations: Vec<ImplementationEdge>,
    /// Entry ordinals in member-symbol order.
    by_symbol: Vec<u32>,
    /// Implementation ordinals in implementation-symbol order.
    by_implementation: Vec<u32>,
}

/// Member implementation postings by declaration and implementation symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberPostings {
    /// Declared member postings.
    pub declarations: Postings<GlobalSymbolId>,
    /// Implementing member postings.
    pub implementations: Postings<GlobalSymbolId>,
}

impl MemberIndex {
    /// Create a member index from declarations and implementation edges.
    pub fn new(entries: Vec<MemberEntry>, implementations: Vec<ImplementationEdge>) -> Self {
        let mut index = Self {
            entries,
            implementations,
            by_symbol: Vec::new(),
            by_implementation: Vec::new(),
        };
        index.finish();

        index
    }

    /// Sort, deduplicate, and index this member index.
    pub fn finish(&mut self) {
        // normalize member declarations
        self.entries
            .sort_by_key(|entry| (entry.source, entry.symbol, entry.declaring));
        self.entries.dedup();

        // normalize implementation edges
        self.implementations.sort_by_key(|implementation| {
            (implementation.declaration, implementation.implementation)
        });
        self.implementations.dedup();

        // index member symbols
        self.by_symbol = (0..self.entries.len() as u32).collect();
        self.by_symbol.sort_by_key(|ordinal| {
            let entry = self.entries[*ordinal as usize];

            (entry.symbol, entry.source, entry.declaring)
        });

        // index implementing symbols
        self.by_implementation = (0..self.implementations.len() as u32).collect();
        self.by_implementation.sort_by_key(|ordinal| {
            let implementation = self.implementations[*ordinal as usize];

            (implementation.implementation, implementation.declaration)
        });
    }

    /// Iterate members declared at one source node.
    pub fn source_entries(&self, source: GlobalNodeIdAny) -> impl Iterator<Item = &MemberEntry> {
        let start = self.entries.partition_point(|entry| entry.source < source);
        let end = self.entries[start..].partition_point(|entry| entry.source == source) + start;

        self.entries[start..end].iter()
    }

    /// Return the indexed member with one member symbol.
    pub fn symbol_entry(&self, symbol: GlobalSymbolId) -> Option<&MemberEntry> {
        let start = self
            .by_symbol
            .partition_point(|ordinal| self.entries[*ordinal as usize].symbol < symbol);
        let entry = &self.entries[*self.by_symbol.get(start)? as usize];

        (entry.symbol == symbol).then_some(entry)
    }

    /// Iterate members implementing one declaration.
    pub fn implementations(
        &self,
        declaration: GlobalSymbolId,
    ) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        let start = self
            .implementations
            .partition_point(|implementation| implementation.declaration < declaration);
        let end = self.implementations[start..]
            .partition_point(|implementation| implementation.declaration == declaration)
            + start;

        self.implementations[start..end]
            .iter()
            .map(|implementation| implementation.implementation)
    }

    /// Iterate declarations satisfied by one implementing member.
    pub fn declarations(
        &self,
        implementation: GlobalSymbolId,
    ) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        let start = self.by_implementation.partition_point(|ordinal| {
            self.implementations[*ordinal as usize].implementation < implementation
        });
        let end = self.by_implementation[start..].partition_point(|ordinal| {
            self.implementations[*ordinal as usize].implementation == implementation
        }) + start;

        self.by_implementation[start..end]
            .iter()
            .map(|ordinal| self.implementations[*ordinal as usize].declaration)
    }

    /// Return all indexed implementation edges.
    pub fn implementation_entries(&self) -> &[ImplementationEdge] {
        &self.implementations
    }
}

impl MemberPostings {
    /// Build member postings from module indexes.
    pub fn build(indexes: &[&MemberIndex]) -> Self {
        let declarations = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .implementation_entries()
                .iter()
                .map(move |implementation| (implementation.declaration, module))
        }));
        let implementations = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .implementation_entries()
                .iter()
                .map(move |implementation| (implementation.implementation, module))
        }));

        Self {
            declarations,
            implementations,
        }
    }

    /// Replace postings for one module member index.
    pub fn update(&mut self, module: u32, index: &MemberIndex) {
        self.declarations.replace(
            module,
            index
                .implementation_entries()
                .iter()
                .map(|implementation| implementation.declaration),
        );
        self.implementations.replace(
            module,
            index
                .implementation_entries()
                .iter()
                .map(|implementation| implementation.implementation),
        );
    }
}

/// One symbol-backed member declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberEntry {
    /// The source node that declares the member.
    pub source: GlobalNodeIdAny,
    /// The symbol whose definition contains the member.
    pub declaring: GlobalSymbolId,
    /// The member symbol.
    pub symbol: GlobalSymbolId,
}
