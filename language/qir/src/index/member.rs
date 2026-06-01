use destack_dir as dir;
use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

use super::Name;

/// Searchable source member index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberIndex {
    /// The member entries in stable source order.
    entries: Vec<MemberEntry>,
    /// Member entry indexes ordered by name.
    by_name: Vec<usize>,
    /// Member entry indexes ordered by owner symbol.
    by_owner: Vec<usize>,
    /// Member entry indexes ordered by member symbol.
    by_symbol: Vec<usize>,
}

impl MemberIndex {
    /// Create a member index from entries.
    pub fn new(entries: Vec<MemberEntry>) -> Self {
        let mut index = Self {
            entries,
            by_name: Vec::new(),
            by_owner: Vec::new(),
            by_symbol: Vec::new(),
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by_key(member_entry_order);
        self.entries.dedup();
        self.rebuild_views();
    }

    /// Return entries whose name contains the case-insensitive query text.
    pub fn search(&self, query: &str) -> Vec<MemberEntry> {
        let query = query.to_lowercase();

        self.by_name
            .iter()
            .map(|index| &self.entries[*index])
            .filter(|entry| entry.name.matches_lowercase_query(&query))
            .cloned()
            .collect()
    }

    /// Return members declared on one owner symbol.
    pub fn for_owner(&self, owner_symbol: dir::GlobalSymbolId) -> Vec<MemberEntry> {
        let range = self.owner_range(owner_symbol);

        self.by_owner[range]
            .iter()
            .map(|index| self.entries[*index].clone())
            .collect()
    }

    /// Return the indexed member for one member symbol.
    pub fn for_symbol(&self, member_symbol: dir::GlobalSymbolId) -> Option<MemberEntry> {
        let start = self
            .by_symbol
            .partition_point(|index| self.entries[*index].member_symbol < member_symbol);
        let entry_index = *self.by_symbol.get(start)?;
        let entry = &self.entries[entry_index];

        (entry.member_symbol == member_symbol).then(|| entry.clone())
    }

    /// Return all indexed member entries.
    pub fn entries(&self) -> &[MemberEntry] {
        &self.entries
    }

    /// Rebuild secondary sorted views.
    fn rebuild_views(&mut self) {
        self.by_name = (0..self.entries.len()).collect();
        self.by_name.sort_by_key(|index| member_name_order(&self.entries[*index]));

        self.by_owner = (0..self.entries.len()).collect();
        self.by_owner
            .sort_by_key(|index| member_owner_order(&self.entries[*index]));

        self.by_symbol = (0..self.entries.len()).collect();
        self.by_symbol
            .sort_by_key(|index| member_symbol_order(&self.entries[*index]));
    }

    /// Return the stored range for one owner symbol.
    fn owner_range(&self, owner_symbol: dir::GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_owner
            .partition_point(|index| self.entries[*index].owner_symbol < owner_symbol);
        let end = self.by_owner[start..]
            .partition_point(|index| self.entries[*index].owner_symbol == owner_symbol)
            + start;

        start..end
    }
}

/// Searchable source member entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberEntry {
    /// The member name.
    pub name: Name,
    /// The member kind.
    pub kind: MemberKind,
    /// The owner symbol.
    pub owner_symbol: dir::GlobalSymbolId,
    /// The member symbol.
    pub member_symbol: dir::GlobalSymbolId,
    /// The source node that declares the member.
    pub source_node: dir::GlobalNodeIdAny,
    /// The module containing the member declaration.
    pub module_id: ModuleId,
    /// The source file.
    pub file_id: FileId,
    /// The source range.
    pub range: Span,
    /// The containing symbol display name.
    pub container_name: Option<String>,
    /// The checked type of the member when known.
    pub declared_type: Option<dir::GlobalTypeId>,
    /// The member origin.
    pub origin: MemberOrigin,
    /// Whether this member belongs to the static surface.
    pub is_static: bool,
}

/// Searchable source member kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemberKind {
    /// Field member.
    Field,
    /// Method member.
    Method,
    /// Associated type member.
    AssociatedType,
    /// Associated constant member.
    AssociatedConst,
    /// Enum variant member.
    Variant,
}

/// Searchable source member origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemberOrigin {
    /// Nominal declaration member.
    Declaration,
    /// Type declaration member.
    TypeMember,
    /// Enum variant declaration.
    EnumVariant,
    /// Extension declaration member.
    Extension,
}

/// Return the stable entry order for one member.
fn member_entry_order(
    entry: &MemberEntry,
) -> (
    ModuleId,
    FileId,
    u32,
    u32,
    dir::GlobalSymbolId,
    dir::GlobalSymbolId,
) {
    (
        entry.module_id,
        entry.file_id,
        entry.range.start,
        entry.range.end,
        entry.owner_symbol,
        entry.member_symbol,
    )
}

/// Return the stable name order for one member.
fn member_name_order(
    entry: &MemberEntry,
) -> (
    Name,
    ModuleId,
    FileId,
    u32,
    u32,
    dir::GlobalSymbolId,
    dir::GlobalSymbolId,
) {
    (
        entry.name.clone(),
        entry.module_id,
        entry.file_id,
        entry.range.start,
        entry.range.end,
        entry.owner_symbol,
        entry.member_symbol,
    )
}

/// Return the stable owner order for one member.
fn member_owner_order(
    entry: &MemberEntry,
) -> (
    dir::GlobalSymbolId,
    Name,
    ModuleId,
    FileId,
    u32,
    u32,
    dir::GlobalSymbolId,
) {
    (
        entry.owner_symbol,
        entry.name.clone(),
        entry.module_id,
        entry.file_id,
        entry.range.start,
        entry.range.end,
        entry.member_symbol,
    )
}

/// Return the stable member-symbol order for one member.
fn member_symbol_order(
    entry: &MemberEntry,
) -> (
    dir::GlobalSymbolId,
    dir::GlobalSymbolId,
    ModuleId,
    FileId,
    u32,
    u32,
) {
    (
        entry.member_symbol,
        entry.owner_symbol,
        entry.module_id,
        entry.file_id,
        entry.range.start,
        entry.range.end,
    )
}
