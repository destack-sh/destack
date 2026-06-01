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
    /// Member entry indexes ordered by declaring symbol.
    by_declaring: Vec<usize>,
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
            by_declaring: Vec::new(),
            by_symbol: Vec::new(),
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by(MemberEntry::cmp_source);
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

    /// Return members contained in one declaring symbol.
    pub fn for_declaring(&self, declaring_symbol: dir::GlobalSymbolId) -> Vec<MemberEntry> {
        let range = self.declaring_range(declaring_symbol);

        self.by_declaring[range]
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
        self.by_name
            .sort_by(|left, right| self.entries[*left].cmp_name(&self.entries[*right]));

        self.by_owner = (0..self.entries.len()).collect();
        self.by_owner
            .sort_by(|left, right| self.entries[*left].cmp_owner(&self.entries[*right]));

        self.by_declaring = (0..self.entries.len()).collect();
        self.by_declaring
            .sort_by(|left, right| self.entries[*left].cmp_declaring(&self.entries[*right]));

        self.by_symbol = (0..self.entries.len()).collect();
        self.by_symbol
            .sort_by(|left, right| self.entries[*left].cmp_symbol(&self.entries[*right]));
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

    /// Return the stored range for one declaring symbol.
    fn declaring_range(&self, declaring_symbol: dir::GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_declaring
            .partition_point(|index| self.entries[*index].declaring_symbol < declaring_symbol);
        let end = self.by_declaring[start..]
            .partition_point(|index| self.entries[*index].declaring_symbol == declaring_symbol)
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
    /// The symbol whose declaration contains this member.
    pub declaring_symbol: dir::GlobalSymbolId,
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
    /// The member source family.
    pub source: MemberSource,
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

impl MemberEntry {
    /// Compare two members in stable source order.
    pub fn cmp_source(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
            self.owner_symbol,
            self.declaring_symbol,
            self.member_symbol,
        );
        let right = (
            other.module_id,
            other.file_id,
            other.range.start,
            other.range.end,
            other.owner_symbol,
            other.declaring_symbol,
            other.member_symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable name order.
    fn cmp_name(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.name.clone(),
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
            self.owner_symbol,
            self.declaring_symbol,
            self.member_symbol,
        );
        let right = (
            other.name.clone(),
            other.module_id,
            other.file_id,
            other.range.start,
            other.range.end,
            other.owner_symbol,
            other.declaring_symbol,
            other.member_symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable owner order.
    fn cmp_owner(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.owner_symbol,
            self.name.clone(),
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
            self.declaring_symbol,
            self.member_symbol,
        );
        let right = (
            other.owner_symbol,
            other.name.clone(),
            other.module_id,
            other.file_id,
            other.range.start,
            other.range.end,
            other.declaring_symbol,
            other.member_symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable declaring-symbol order.
    fn cmp_declaring(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.declaring_symbol,
            self.name.clone(),
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
            self.owner_symbol,
            self.member_symbol,
        );
        let right = (
            other.declaring_symbol,
            other.name.clone(),
            other.module_id,
            other.file_id,
            other.range.start,
            other.range.end,
            other.owner_symbol,
            other.member_symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable member-symbol order.
    fn cmp_symbol(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.member_symbol,
            self.declaring_symbol,
            self.owner_symbol,
            self.module_id,
            self.file_id,
            self.range.start,
            self.range.end,
        );
        let right = (
            other.member_symbol,
            other.declaring_symbol,
            other.owner_symbol,
            other.module_id,
            other.file_id,
            other.range.start,
            other.range.end,
        );

        left.cmp(&right)
    }
}

/// Searchable source member source family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberSource {
    /// Nominal declaration member.
    Declaration,
    /// Type declaration member.
    TypeMember,
    /// Enum variant declaration.
    EnumVariant,
    /// Extension declaration member.
    Extension,
}
