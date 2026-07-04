use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, MemberSpace, Postings};
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

/// Indexed checked members.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberIndex {
    /// The members in stable source order.
    entries: Vec<MemberEntry>,
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
    /// Member name postings.
    pub names: Postings<String>,
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

    /// Iterate members declared on one owner symbol.
    pub fn for_owner(&self, owner: GlobalSymbolId) -> impl Iterator<Item = &MemberEntry> {
        let range = self.owner_range(owner);

        self.by_owner[range]
            .iter()
            .map(|index| &self.entries[*index])
    }

    /// Iterate members contained in one declaring symbol.
    pub fn for_declaring(&self, declaring: GlobalSymbolId) -> impl Iterator<Item = &MemberEntry> {
        let range = self.declaring_range(declaring);

        self.by_declaring[range]
            .iter()
            .map(|index| &self.entries[*index])
    }

    /// Return the indexed member for one member symbol.
    pub fn for_symbol(&self, symbol: GlobalSymbolId) -> Option<&MemberEntry> {
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
        self.by_owner = (0..self.entries.len()).collect();
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

    /// Return the stored range for one owner symbol.
    fn owner_range(&self, owner: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_owner
            .partition_point(|index| self.entries[*index].owner < owner);
        let end = self.by_owner[start..]
            .partition_point(|index| self.entries[*index].owner == owner)
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
    pub fn new(indexes: &[&MemberIndex]) -> Self {
        let names = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.name.clone(), module))
        }));
        let owners = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.owner, module))
        }));
        let declaring = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.declaring, module))
        }));

        Self {
            names,
            owners,
            declaring,
        }
    }
}

/// One indexed member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberEntry {
    /// The member name.
    pub name: String,
    /// The member kind.
    pub kind: MemberKind,
    /// The symbol whose member surface receives this member.
    pub owner: GlobalSymbolId,
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
    /// The containing symbol display name.
    pub container: Option<String>,
    /// The checked type of the member when known.
    pub ty: Option<GlobalTypeId>,
    /// The member origin.
    pub origin: MemberOrigin,
    /// The checked member space.
    pub space: MemberSpace,
    /// Whether implementers must supply this member.
    pub is_abstract: bool,
    /// Whether this member overrides an inherited member.
    pub is_override: bool,
    /// Whether this member supplies a default implementation.
    pub is_default: bool,
    /// Whether this member belongs to the static surface.
    pub is_static: bool,
}

impl MemberEntry {
    /// Compare two members in stable source order.
    pub fn compare_by_source(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.owner,
            self.declaring,
            self.symbol,
        );
        let right = (
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
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
            self.name.as_str(),
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.declaring,
            self.symbol,
        );
        let right = (
            other.owner,
            other.name.as_str(),
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.declaring,
            other.symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable declaring-symbol order.
    fn compare_by_declaring(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.declaring,
            self.name.as_str(),
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
            self.owner,
            self.symbol,
        );
        let right = (
            other.declaring,
            other.name.as_str(),
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
            other.owner,
            other.symbol,
        );

        left.cmp(&right)
    }

    /// Compare two members in stable member-symbol order.
    fn compare_by_symbol(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.symbol.unwrap_or(self.declaring),
            self.declaring,
            self.owner,
            self.source.module_id,
            self.file,
            self.span.start,
            self.span.end,
        );
        let right = (
            other.symbol.unwrap_or(other.declaring),
            other.declaring,
            other.owner,
            other.source.module_id,
            other.file,
            other.span.start,
            other.span.end,
        );

        left.cmp(&right)
    }
}

/// Indexed member kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum MemberKind {
    /// Field member.
    Field,
    /// Method member.
    Method,
    /// Constructor member.
    Constructor,
    /// Call signature member.
    CallSignature,
    /// Construct signature member.
    ConstructSignature,
    /// Index signature member.
    IndexSignature,
    /// Associated type member.
    AssociatedType,
    /// Associated constant member.
    AssociatedConst,
    /// Enum variant member.
    Variant,
}

/// Indexed member origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberOrigin {
    /// Definition table member.
    Definition,
    /// Extension definition member.
    Extension,
}
