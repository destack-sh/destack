use std::fmt;

use destack_core::{
    EntryRange, EntryStore, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_mir::{TraceId, TraceMap, TraceVariant};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{HeapResult, ReferenceRange};

/// Compact heap trace table stored in program sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TraceTable {
    /// Top-level trace roots indexed by TraceId.
    roots: SectionSlice<u32>,
    /// Flat trace entries.
    entries: SectionSlice<TraceEntry>,
    /// Flattened trace byte offsets.
    offsets: SectionSlice<u32>,
    /// Flattened child entry ids.
    children: SectionSlice<u32>,
    /// Flattened tagged variant entries.
    variants: SectionSlice<TraceVariantEntry>,
}

impl TraceTable {
    /// Pack one heap trace table from compiler trace maps.
    pub fn pack(sections: &mut SectionPacker, source: &destack_mir::TraceTable) -> Self {
        let mut builder = TraceTableBuilder::new();

        // preserve TraceId order for all top-level roots
        for trace in source.traces() {
            let root = builder.push_map(trace);
            builder.roots.push(root);
        }

        builder.pack(sections)
    }

    /// Return one borrowed compact trace view.
    pub fn view<'a>(&self, sections: SectionImage<'a>) -> TraceView<'a> {
        TraceView {
            roots: sections.entries(self.roots),
            entries: sections.entries(self.entries),
            offsets: sections.entries(self.offsets),
            children: sections.entries(self.children),
            variants: sections.entries(self.variants),
        }
    }
}

/// Borrowed compact heap trace rows.
#[derive(Debug, Clone, Copy)]
pub struct TraceView<'a> {
    /// Top-level trace roots indexed by TraceId.
    roots: &'a [u32],
    /// Flat trace entries.
    entries: &'a [TraceEntry],
    /// Flattened trace byte offsets.
    offsets: &'a [u32],
    /// Flattened child entry ids.
    children: &'a [u32],
    /// Flattened tagged variant entries.
    variants: &'a [TraceVariantEntry],
}

impl TraceView<'_> {
    /// Decode one trace map by id.
    pub fn trace_map(self, id: TraceId) -> Result<TraceMap, TraceTableError> {
        let root = self.root(id)?;

        self.decode_map(root)
    }

    /// Decode one compact trace entry into a semantic trace map.
    fn decode_map(self, entry_id: u32) -> Result<TraceMap, TraceTableError> {
        let Some(entry) = self.entries.get(entry_id as usize).copied() else {
            return Err(TraceTableError::MissingEntry { entry: entry_id });
        };

        // decode the compact trace entry by kind
        match entry.kind {
            TraceEntryKind::EMPTY => Ok(TraceMap::Empty),
            TraceEntryKind::FIXED => Ok(TraceMap::Fixed {
                local_offsets: entry.local_offsets.slice(self.offsets).into(),
                shared_offsets: entry.shared_offsets.slice(self.offsets).into(),
            }),
            TraceEntryKind::NESTED => Ok(TraceMap::Nested {
                byte_offset: entry.byte_offset,
                map: Box::new(self.decode_map(entry.first_child)?),
            }),
            TraceEntryKind::COMPOSITE => {
                let maps = entry
                    .children
                    .slice(self.children)
                    .iter()
                    .map(|child| self.decode_map(*child))
                    .collect::<Result<Box<[_]>, _>>()?;

                Ok(TraceMap::Composite { maps })
            }
            TraceEntryKind::REPEATED => Ok(TraceMap::Repeated {
                count: entry.count,
                stride: entry.stride,
                element: Box::new(self.decode_map(entry.first_child)?),
            }),
            TraceEntryKind::TAGGED => {
                let variants = entry
                    .variants
                    .slice(self.variants)
                    .iter()
                    .map(|variant| {
                        Ok(TraceVariant {
                            tag: variant.tag,
                            payload_offset: variant.payload_offset,
                            map: self.decode_map(variant.child)?,
                        })
                    })
                    .collect::<Result<Box<[_]>, TraceTableError>>()?;

                Ok(TraceMap::Tagged {
                    tag_bytes: entry.tag_bytes as u8,
                    variants,
                })
            }
            kind => Err(TraceTableError::UnknownKind { kind: kind.raw() }),
        }
    }

    /// Walk one trace map by id.
    pub(crate) fn walk(
        self,
        id: TraceId,
        base_offset: usize,
        range: ReferenceRange,
        walker: &mut impl TraceVisitor,
    ) -> HeapResult<()> {
        let root = self.root(id)?;

        self.walk_entry(root, base_offset, range, walker)
    }

    /// Return one trace root entry id.
    fn root(self, id: TraceId) -> Result<u32, TraceTableError> {
        self.roots
            .get(id.index())
            .copied()
            .ok_or(TraceTableError::MissingTrace { trace: id })
    }

    /// Walk one compact trace entry.
    fn walk_entry(
        self,
        entry_id: u32,
        base_offset: usize,
        range: ReferenceRange,
        walker: &mut impl TraceVisitor,
    ) -> HeapResult<()> {
        let entry = self.entry(entry_id)?;

        // dispatch by compact trace entry kind
        match entry.kind {
            TraceEntryKind::EMPTY => {}
            TraceEntryKind::FIXED => {
                walker.fixed(
                    entry.local_offsets.slice(self.offsets),
                    entry.shared_offsets.slice(self.offsets),
                    base_offset,
                    range,
                )?;
            }
            TraceEntryKind::NESTED => {
                let base_offset = base_offset + entry.byte_offset as usize;

                self.walk_entry(entry.first_child, base_offset, range, walker)?;
            }
            TraceEntryKind::COMPOSITE => {
                for child in entry.children.slice(self.children) {
                    self.walk_entry(*child, base_offset, range, walker)?;
                }
            }
            TraceEntryKind::REPEATED => {
                let window = range.element_window(base_offset, entry.stride as usize, entry.count);
                for index in window {
                    let element_offset = base_offset + index as usize * entry.stride as usize;

                    self.walk_entry(entry.first_child, element_offset, range, walker)?;
                }
            }
            TraceEntryKind::TAGGED => {
                let Some(tag) = walker.tag(base_offset, entry.tag_bytes as u8)? else {
                    for variant in entry.variants.slice(self.variants) {
                        let variant_offset = base_offset + variant.payload_offset as usize;

                        self.walk_entry(variant.child, variant_offset, range, walker)?;
                    }

                    return Ok(());
                };
                let Some(variant) = entry
                    .variants
                    .slice(self.variants)
                    .iter()
                    .find(|variant| variant.tag == tag)
                else {
                    return Ok(());
                };
                let variant_offset = base_offset + variant.payload_offset as usize;

                self.walk_entry(variant.child, variant_offset, range, walker)?;
            }
            kind => return Err(TraceTableError::UnknownKind { kind: kind.raw() }.into()),
        }

        Ok(())
    }

    /// Return one compact trace entry.
    fn entry(self, entry_id: u32) -> Result<TraceEntry, TraceTableError> {
        self.entries
            .get(entry_id as usize)
            .copied()
            .ok_or(TraceTableError::MissingEntry { entry: entry_id })
    }
}

/// Consumer for compact trace walking.
pub(crate) trait TraceVisitor {
    /// Walk one fixed trace map at the given byte offset.
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()>;

    /// Return the active variant tag at the given offset.
    fn tag(&mut self, offset: usize, width: u8) -> HeapResult<Option<u64>>;
}

/// Invalid compact heap trace table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceTableError {
    /// A trace id does not name a stored trace root.
    MissingTrace {
        /// Missing trace id.
        trace: TraceId,
    },
    /// A trace entry id does not name a stored trace entry.
    MissingEntry {
        /// Missing entry id.
        entry: u32,
    },
    /// A trace entry carries an unknown trace kind.
    UnknownKind {
        /// Invalid trace kind.
        kind: u32,
    },
}

impl fmt::Display for TraceTableError {
    /// Format one trace table error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTrace { trace } => write!(formatter, "missing trace {trace:?}"),
            Self::MissingEntry { entry } => write!(formatter, "missing trace entry {entry}"),
            Self::UnknownKind { kind } => write!(formatter, "unknown trace kind {kind}"),
        }
    }
}

impl std::error::Error for TraceTableError {}

/// Flat compact trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceEntry {
    /// Trace entry kind.
    kind: TraceEntryKind,
    /// Byte offset used by nested maps.
    byte_offset: u32,
    /// Repeated element count.
    count: u32,
    /// Repeated element byte stride.
    stride: u32,
    /// First child entry id for single-child maps.
    first_child: u32,
    /// Tagged variant tag byte width.
    tag_bytes: u32,
    /// Fixed local reference byte offsets.
    local_offsets: EntryRange<u32>,
    /// Fixed shared reference byte offsets.
    shared_offsets: EntryRange<u32>,
    /// Composite child entry ids.
    children: EntryRange<u32>,
    /// Tagged variant entries.
    variants: EntryRange<TraceVariantEntry>,
}

impl TraceEntry {
    /// Create one trace entry with empty payload ranges.
    fn new(kind: TraceEntryKind) -> Self {
        Self {
            kind,
            byte_offset: 0,
            count: 0,
            stride: 0,
            first_child: 0,
            tag_bytes: 0,
            local_offsets: EntryRange::empty(),
            shared_offsets: EntryRange::empty(),
            children: EntryRange::empty(),
            variants: EntryRange::empty(),
        }
    }
}

/// Fixed-width compact trace entry kind.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceEntryKind(u32);

impl TraceEntryKind {
    /// Empty trace map.
    const EMPTY: Self = Self(0);
    /// Fixed offset trace map.
    const FIXED: Self = Self(1);
    /// Nested offset trace map.
    const NESTED: Self = Self(2);
    /// Composite trace map.
    const COMPOSITE: Self = Self(3);
    /// Repeated element trace map.
    const REPEATED: Self = Self(4);
    /// Tagged variant trace map.
    const TAGGED: Self = Self(5);

    /// Return the raw trace kind.
    const fn raw(self) -> u32 {
        self.0
    }
}

/// Flat compact trace variant entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceVariantEntry {
    /// Normalized variant tag value.
    tag: u64,
    /// Variant payload byte offset.
    payload_offset: u32,
    /// Variant payload child entry id.
    child: u32,
}

/// Build-time compact trace table builder.
struct TraceTableBuilder {
    /// Top-level trace root entry ids.
    roots: Vec<u32>,
    /// Flat trace entries.
    entries: Vec<TraceEntry>,
    /// Flattened trace byte offsets.
    offsets: EntryStore<u32>,
    /// Flattened child entry ids.
    children: EntryStore<u32>,
    /// Flattened tagged variant entries.
    variants: EntryStore<TraceVariantEntry>,
}

impl TraceTableBuilder {
    /// Create one empty trace table builder.
    fn new() -> Self {
        Self {
            roots: Vec::new(),
            entries: Vec::new(),
            offsets: EntryStore::new(),
            children: EntryStore::new(),
            variants: EntryStore::new(),
        }
    }

    /// Push one trace map and return its entry id.
    fn push_map(&mut self, map: &TraceMap) -> u32 {
        let entry_id = self.entries.len() as u32;
        let entry = self.entry(map);
        self.entries.push(entry);

        entry_id
    }

    /// Build one flat entry for one trace map.
    fn entry(&mut self, map: &TraceMap) -> TraceEntry {
        match map {
            TraceMap::Empty => TraceEntry::new(TraceEntryKind::EMPTY),
            TraceMap::Fixed {
                local_offsets,
                shared_offsets,
            } => {
                let mut entry = TraceEntry::new(TraceEntryKind::FIXED);
                entry.local_offsets = self.offsets.append(local_offsets.iter().copied());
                entry.shared_offsets = self.offsets.append(shared_offsets.iter().copied());

                entry
            }
            TraceMap::Nested { byte_offset, map } => {
                let mut entry = TraceEntry::new(TraceEntryKind::NESTED);
                entry.byte_offset = *byte_offset;
                entry.first_child = self.push_map(map);

                entry
            }
            TraceMap::Composite { maps } => {
                let children = maps
                    .iter()
                    .map(|map| self.push_map(map))
                    .collect::<Vec<_>>();

                let mut entry = TraceEntry::new(TraceEntryKind::COMPOSITE);
                entry.children = self.children.append(children);

                entry
            }
            TraceMap::Repeated {
                count,
                stride,
                element,
            } => {
                let mut entry = TraceEntry::new(TraceEntryKind::REPEATED);
                entry.count = *count;
                entry.stride = *stride;
                entry.first_child = self.push_map(element);

                entry
            }
            TraceMap::Tagged {
                tag_bytes,
                variants,
            } => {
                let variants = variants
                    .iter()
                    .map(|variant| TraceVariantEntry {
                        tag: variant.tag,
                        payload_offset: variant.payload_offset,
                        child: self.push_map(&variant.map),
                    })
                    .collect::<Vec<_>>();

                let mut entry = TraceEntry::new(TraceEntryKind::TAGGED);
                entry.tag_bytes = *tag_bytes as u32;
                entry.variants = self.variants.append(variants);

                entry
            }
        }
    }

    /// Pack one section-backed compact trace table.
    fn pack(self, sections: &mut SectionPacker) -> TraceTable {
        let roots = sections.insert(self.roots);
        let entries = sections.insert(self.entries);
        let offsets = sections.insert(self.offsets.into_entries());
        let children = sections.insert(self.children.into_entries());
        let variants = sections.insert(self.variants.into_entries());

        TraceTable {
            roots,
            entries,
            offsets,
            children,
            variants,
        }
    }
}

// SAFETY: trace entries are fixed-width program entries containing only integers and ranges.
unsafe impl SectionEntry for TraceEntryKind {}
unsafe impl SectionEntry for TraceEntry {}
unsafe impl SectionEntry for TraceVariantEntry {}
