use crate::{
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, MemberKind, MemberOrigin, MemberSpace, Postings,
    StaticKey,
};
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

/// Indexed members.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberIndex {
    /// The members in stable source order.
    entries: Vec<MemberEntry>,
    /// Member indexes ordered by source node.
    by_source: Vec<usize>,
    /// Member indexes ordered by owner symbol.
    by_owner: Vec<usize>,
    /// Member indexes ordered by declaring symbol.
    by_declaring: Vec<usize>,
    /// Member indexes ordered by member symbol.
    by_symbol: Vec<usize>,
}

/// Member postings by lookup key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberPostings {
    /// Member key postings.
    pub keys: Postings<StaticKey>,
    /// Member owner postings.
    pub owners: Postings<GlobalSymbolId>,
    /// Member declaring symbol postings.
    pub declaring: Postings<GlobalSymbolId>,
}

impl MemberIndex {
    /// Create a member index from entries.
    pub fn new(entries: Vec<MemberEntry>) -> Self {
        let mut index = Self {
            entries,
            by_source: Vec::new(),
            by_owner: Vec::new(),
            by_declaring: Vec::new(),
            by_symbol: Vec::new(),
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by(MemberEntry::compare_by_source);
        self.entries.dedup();
        self.rebuild_views();
    }

    /// Iterate members declared at one source node.
    pub fn source_entries(&self, source: GlobalNodeIdAny) -> impl Iterator<Item = &MemberEntry> {
        let range = self.source_range(source);

        self.by_source[range]
            .iter()
            .map(|index| &self.entries[*index])
    }

    /// Iterate members declared on one owner symbol.
    pub fn owner_entries(&self, owner: GlobalSymbolId) -> impl Iterator<Item = &MemberEntry> {
        let range = self.owner_range(owner);

        self.by_owner[range]
            .iter()
            .map(|index| &self.entries[*index])
    }

    /// Iterate members contained in one declaring symbol.
    pub fn declaring_entries(
        &self,
        declaring: GlobalSymbolId,
    ) -> impl Iterator<Item = &MemberEntry> {
        let range = self.declaring_range(declaring);

        self.by_declaring[range]
            .iter()
            .map(|index| &self.entries[*index])
    }

    /// Return the indexed member with one member symbol.
    pub fn symbol_entry(&self, symbol: GlobalSymbolId) -> Option<&MemberEntry> {
        let start = self.by_symbol.partition_point(|index| {
            self.entries[*index]
                .symbol
                .is_some_and(|entry_symbol| entry_symbol < symbol)
        });
        let entry_index = *self.by_symbol.get(start)?;
        let entry = &self.entries[entry_index];

        (entry.symbol == Some(symbol)).then_some(entry)
    }

    /// Return all indexed members.
    pub fn entries(&self) -> &[MemberEntry] {
        &self.entries
    }

    /// Rebuild secondary sorted views.
    fn rebuild_views(&mut self) {
        self.by_source = (0..self.entries.len()).collect();
        self.by_source
            .sort_by_key(|index| self.entries[*index].source);

        self.by_owner = (0..self.entries.len()).collect();
        self.by_owner
            .retain(|index| self.entries[*index].owner.is_some());
        self.by_owner
            .sort_by(|left, right| self.entries[*left].compare_by_owner(&self.entries[*right]));

        self.by_declaring = (0..self.entries.len()).collect();
        self.by_declaring
            .sort_by(|left, right| self.entries[*left].compare_by_declaring(&self.entries[*right]));

        self.by_symbol = (0..self.entries.len()).collect();
        self.by_symbol
            .retain(|index| self.entries[*index].symbol.is_some());
        self.by_symbol
            .sort_by(|left, right| self.entries[*left].compare_by_symbol(&self.entries[*right]));
    }

    /// Return the stored range for one source node.
    fn source_range(&self, source: GlobalNodeIdAny) -> std::ops::Range<usize> {
        let start = self
            .by_source
            .partition_point(|index| self.entries[*index].source < source);
        let end = self.by_source[start..]
            .partition_point(|index| self.entries[*index].source == source)
            + start;

        start..end
    }

    /// Return the stored range for one owner symbol.
    fn owner_range(&self, owner: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_owner
            .partition_point(|index| self.entries[*index].owner < Some(owner));
        let end = self.by_owner[start..]
            .partition_point(|index| self.entries[*index].owner == Some(owner))
            + start;

        start..end
    }

    /// Return the stored range for one declaring symbol.
    fn declaring_range(&self, declaring: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_declaring
            .partition_point(|index| self.entries[*index].declaring < declaring);
        let end = self.by_declaring[start..]
            .partition_point(|index| self.entries[*index].declaring == declaring)
            + start;

        start..end
    }
}

impl MemberPostings {
    /// Build member postings from module index sections.
    pub fn build(indexes: &[&MemberIndex]) -> Self {
        let keys = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .filter_map(move |entry| entry.key.map(|key| (key, module)))
        }));
        let owners = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .filter_map(move |entry| entry.owner.map(|owner| (owner, module)))
        }));
        let declaring = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.declaring, module))
        }));

        Self {
            keys,
            owners,
            declaring,
        }
    }

    /// Replace postings for one module member index.
    pub fn update(&mut self, module: u32, index: &MemberIndex) {
        self.keys
            .replace(module, index.entries().iter().filter_map(|entry| entry.key));
        self.owners.replace(
            module,
            index.entries().iter().filter_map(|entry| entry.owner),
        );
        self.declaring
            .replace(module, index.entries().iter().map(|entry| entry.declaring));
    }
}

/// One indexed member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberEntry {
    /// The member key when the member is keyed.
    pub key: Option<StaticKey>,
    /// The member kind.
    pub kind: MemberKind,
    /// The symbol this member belongs to or extends.
    pub owner: Option<GlobalSymbolId>,
    /// The symbol whose definition declares this member.
    pub declaring: GlobalSymbolId,
    /// The member symbol when this member declares one.
    pub symbol: Option<GlobalSymbolId>,
    /// The source node that defines the member.
    pub source: GlobalNodeIdAny,
    /// The source file.
    pub file: FileId,
    /// The source range.
    pub span: Span,
    /// The member name range when authored.
    pub selection: Option<Span>,
    /// The containing symbol display name.
    pub container: Option<String>,
    /// The type of the member when known.
    pub ty: Option<GlobalTypeId>,
    /// The member origin.
    pub origin: MemberOrigin,
    /// The member space.
    pub space: MemberSpace,
}

impl MemberEntry {
    /// Return the sortable member name range.
    fn selection_key(&self) -> Option<(FileId, u32, u32)> {
        self.selection.map(|span| (span.file, span.start, span.end))
    }

    /// Compare two members in stable source order.
    pub fn compare_by_source(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.selection_key(),
            self.owner,
            self.declaring,
            self.symbol,
        );
        let right = (
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.selection_key(),
            other.owner,
            other.declaring,
            other.symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable owner order.
    fn compare_by_owner(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.owner,
            self.key,
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.selection_key(),
            self.declaring,
            self.symbol,
        );
        let right = (
            other.owner,
            other.key,
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.selection_key(),
            other.declaring,
            other.symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable declaring-symbol order.
    fn compare_by_declaring(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.declaring,
            self.key,
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.selection_key(),
            self.owner,
            self.symbol,
        );
        let right = (
            other.declaring,
            other.key,
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.selection_key(),
            other.owner,
            other.symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable member-symbol order.
    fn compare_by_symbol(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.symbol,
            self.declaring,
            self.owner,
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.selection_key(),
        );
        let right = (
            other.symbol,
            other.declaring,
            other.owner,
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.selection_key(),
        );

        left.cmp(&right)
    }
}
