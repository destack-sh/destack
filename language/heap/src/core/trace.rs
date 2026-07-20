use std::fmt;
use std::marker::PhantomData;

use destack_core::{
    EntryRange, EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_mir as mir;
use destack_mir::{
    Discriminant, DiscriminantField, TraceId, TraceMap, VariantEncoding, VariantTrace,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{HeapResult, ReferenceClass, ReferenceRange};

const TRACE_EMPTY: u32 = 0;
const TRACE_FIXED: u32 = 1;
const TRACE_NESTED: u32 = 2;
const TRACE_COMPOSITE: u32 = 3;
const TRACE_REPEATED: u32 = 4;
const TRACE_VARIANT: u32 = 5;
const VARIANT_DIRECT: u32 = 0;
const VARIANT_NICHE: u32 = 1;

/// Compact heap trace table stored in program sections.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct TraceTable {
    /// Top-level trace roots indexed by TraceId.
    roots: SectionSlice<u32>,
    /// Flat trace entries.
    entries: SectionSlice<TraceEntry>,
    /// Fixed trace payloads.
    fixed: SectionSlice<FixedTrace>,
    /// Nested trace payloads.
    nested: SectionSlice<NestedTrace>,
    /// Composite trace payloads.
    composite: SectionSlice<CompositeTrace>,
    /// Repeated trace payloads.
    repeated: SectionSlice<RepeatedTrace>,
    /// Variant trace payloads.
    variants: SectionSlice<VariantTraceEntry>,
    /// Flattened trace byte offsets.
    offsets: SectionSlice<u32>,
    /// Flattened child entry ids.
    children: SectionSlice<u32>,
    /// Flattened variant cases.
    cases: SectionSlice<VariantTraceCase>,
}

impl TraceTable {
    /// Pack one heap trace table from compiler trace maps.
    pub fn pack(sections: &mut SectionBuilder, source: &mir::TraceTable) -> Self {
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
            fixed: sections.entries(self.fixed),
            nested: sections.entries(self.nested),
            composite: sections.entries(self.composite),
            repeated: sections.entries(self.repeated),
            variants: sections.entries(self.variants),
            offsets: sections.entries(self.offsets),
            children: sections.entries(self.children),
            cases: sections.entries(self.cases),
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
    /// Fixed trace payloads.
    fixed: &'a [FixedTrace],
    /// Nested trace payloads.
    nested: &'a [NestedTrace],
    /// Composite trace payloads.
    composite: &'a [CompositeTrace],
    /// Repeated trace payloads.
    repeated: &'a [RepeatedTrace],
    /// Variant trace payloads.
    variants: &'a [VariantTraceEntry],
    /// Flattened trace byte offsets.
    offsets: &'a [u32],
    /// Flattened child entry ids.
    children: &'a [u32],
    /// Flattened variant cases.
    cases: &'a [VariantTraceCase],
}

impl TraceView<'_> {
    /// Return whether one trace may contain one reference class.
    pub(crate) fn has_reference<R: ReferenceClass>(self, id: TraceId) -> HeapResult<bool> {
        let mut visitor = ReferencePresence::<R> {
            is_present: false,
            reference: PhantomData,
        };
        self.walk(id, 0, ReferenceRange::All, &mut visitor)?;

        Ok(visitor.is_present)
    }

    /// Return whether one trace reference class overlaps a byte range.
    pub(crate) fn overlaps_reference<R: ReferenceClass>(
        self,
        id: TraceId,
        range: ReferenceRange,
    ) -> HeapResult<bool> {
        let mut visitor = ReferencePresence::<R> {
            is_present: false,
            reference: PhantomData,
        };
        self.walk(id, 0, range, &mut visitor)?;

        Ok(visitor.is_present)
    }

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

        // decode the compact trace entry by tag
        match entry.tag {
            TRACE_EMPTY => Ok(TraceMap::Empty),
            TRACE_FIXED => {
                let fixed = self.fixed(entry)?;

                Ok(TraceMap::Fixed {
                    local_offsets: fixed.local_offsets.slice(self.offsets).into(),
                    shared_offsets: fixed.shared_offsets.slice(self.offsets).into(),
                    frame_offsets: fixed.frame_offsets.slice(self.offsets).into(),
                })
            }
            TRACE_NESTED => {
                let nested = self.nested(entry)?;

                Ok(TraceMap::Nested {
                    byte_offset: nested.byte_offset,
                    map: Box::new(self.decode_map(nested.child)?),
                })
            }
            TRACE_COMPOSITE => {
                let composite = self.composite(entry)?;
                let maps = composite
                    .children
                    .slice(self.children)
                    .iter()
                    .map(|child| self.decode_map(*child))
                    .collect::<Result<Box<[_]>, _>>()?;

                Ok(TraceMap::Composite { maps })
            }
            TRACE_REPEATED => {
                let repeated = self.repeated(entry)?;

                Ok(TraceMap::Repeated {
                    count: repeated.count,
                    stride: repeated.stride,
                    element: Box::new(self.decode_map(repeated.element)?),
                })
            }
            TRACE_VARIANT => {
                let variant = self.variant(entry)?;
                let cases = variant
                    .cases
                    .slice(self.cases)
                    .iter()
                    .map(|case| {
                        Ok(VariantTrace {
                            discriminant: case.discriminant,
                            payload_offset: case.payload_offset,
                            map: self.decode_map(case.child)?,
                        })
                    })
                    .collect::<Result<Box<[_]>, TraceTableError>>()?;

                Ok(TraceMap::Variant {
                    encoding: variant.encoding()?,
                    cases,
                })
            }
            tag => Err(TraceTableError::InvalidEntryTag { tag }),
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

        // dispatch by compact trace entry tag
        match entry.tag {
            TRACE_EMPTY => {}
            TRACE_FIXED => {
                let fixed = self.fixed(entry)?;

                walker.fixed(
                    fixed.local_offsets.slice(self.offsets),
                    fixed.shared_offsets.slice(self.offsets),
                    fixed.frame_offsets.slice(self.offsets),
                    base_offset,
                    range,
                )?;
            }
            TRACE_NESTED => {
                let nested = self.nested(entry)?;
                let base_offset = base_offset + nested.byte_offset as usize;

                self.walk_entry(nested.child, base_offset, range, walker)?;
            }
            TRACE_COMPOSITE => {
                let composite = self.composite(entry)?;

                for child in composite.children.slice(self.children) {
                    self.walk_entry(*child, base_offset, range, walker)?;
                }
            }
            TRACE_REPEATED => {
                let repeated = self.repeated(entry)?;
                let window =
                    range.element_window(base_offset, repeated.stride as usize, repeated.count);
                for index in window {
                    let element_offset = base_offset + index as usize * repeated.stride as usize;

                    self.walk_entry(repeated.element, element_offset, range, walker)?;
                }
            }
            TRACE_VARIANT => {
                let variant = self.variant(entry)?;
                let encoding = variant.encoding()?;
                let field = encoding.field();
                let offset = base_offset + field.offset as usize;
                let Some(scalar) = walker.scalar(offset, field.byte_len)? else {
                    for case in variant.cases.slice(self.cases) {
                        let variant_offset = base_offset + case.payload_offset as usize;

                        self.walk_entry(case.child, variant_offset, range, walker)?;
                    }

                    return Ok(());
                };
                let cases = variant.cases.slice(self.cases);
                let case = match encoding {
                    VariantEncoding::Direct { field } => {
                        let discriminant = field.extract(scalar);

                        cases
                            .iter()
                            .find(|case| case.discriminant.bits() == discriminant)
                    }
                    encoding @ VariantEncoding::Niche { .. } => encoding
                        .decode_niche(scalar)
                        .and_then(|index| cases.get(index as usize)),
                };
                let Some(case) = case else {
                    return Ok(());
                };
                let variant_offset = base_offset + case.payload_offset as usize;

                self.walk_entry(case.child, variant_offset, range, walker)?;
            }
            tag => return Err(TraceTableError::InvalidEntryTag { tag }.into()),
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

    /// Return one fixed trace payload.
    fn fixed(self, entry: TraceEntry) -> Result<FixedTrace, TraceTableError> {
        self.fixed
            .get(entry.index as usize)
            .copied()
            .ok_or(TraceTableError::MissingPayload {
                tag: entry.tag,
                index: entry.index,
            })
    }

    /// Return one nested trace payload.
    fn nested(self, entry: TraceEntry) -> Result<NestedTrace, TraceTableError> {
        self.nested
            .get(entry.index as usize)
            .copied()
            .ok_or(TraceTableError::MissingPayload {
                tag: entry.tag,
                index: entry.index,
            })
    }

    /// Return one composite trace payload.
    fn composite(self, entry: TraceEntry) -> Result<CompositeTrace, TraceTableError> {
        self.composite
            .get(entry.index as usize)
            .copied()
            .ok_or(TraceTableError::MissingPayload {
                tag: entry.tag,
                index: entry.index,
            })
    }

    /// Return one repeated trace payload.
    fn repeated(self, entry: TraceEntry) -> Result<RepeatedTrace, TraceTableError> {
        self.repeated
            .get(entry.index as usize)
            .copied()
            .ok_or(TraceTableError::MissingPayload {
                tag: entry.tag,
                index: entry.index,
            })
    }

    /// Return one variant trace payload.
    fn variant(self, entry: TraceEntry) -> Result<VariantTraceEntry, TraceTableError> {
        self.variants
            .get(entry.index as usize)
            .copied()
            .ok_or(TraceTableError::MissingPayload {
                tag: entry.tag,
                index: entry.index,
            })
    }
}

/// Presence probe for one trace reference class.
struct ReferencePresence<R> {
    /// Whether the trace contains this reference class.
    is_present: bool,
    /// The reference class selected by this probe.
    reference: PhantomData<R>,
}

impl<R: ReferenceClass> TraceVisitor for ReferencePresence<R> {
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        frame_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()> {
        self.is_present |= R::offsets(local_offsets, shared_offsets, frame_offsets)
            .iter()
            .any(|offset| range.overlaps(base_offset + *offset as usize, R::BYTE_LEN));

        Ok(())
    }

    fn scalar(&mut self, _offset: usize, _byte_len: u8) -> HeapResult<Option<u128>> {
        Ok(None)
    }
}

/// Consumer for compact trace walking.
pub(crate) trait TraceVisitor {
    /// Walk one fixed trace map at the given byte offset.
    fn fixed(
        &mut self,
        local_offsets: &[u32],
        shared_offsets: &[u32],
        frame_offsets: &[u32],
        base_offset: usize,
        range: ReferenceRange,
    ) -> HeapResult<()>;

    /// Return the active variant discriminant scalar at the given offset.
    fn scalar(&mut self, offset: usize, byte_len: u8) -> HeapResult<Option<u128>>;
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
    /// A trace entry tag is not defined.
    InvalidEntryTag {
        /// Invalid trace entry tag.
        tag: u32,
    },
    /// A trace entry payload index is missing.
    MissingPayload {
        /// Trace entry tag.
        tag: u32,
        /// Missing payload index.
        index: u32,
    },
    /// A variant encoding tag is not defined.
    InvalidVariantTag {
        /// Invalid variant encoding tag.
        tag: u32,
    },
}

impl fmt::Display for TraceTableError {
    /// Format one trace table error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTrace { trace } => write!(formatter, "missing trace {trace:?}"),
            Self::MissingEntry { entry } => write!(formatter, "missing trace entry {entry}"),
            Self::InvalidEntryTag { tag } => write!(formatter, "invalid trace entry tag {tag}"),
            Self::MissingPayload { tag, index } => {
                write!(formatter, "missing trace payload {index} for tag {tag}")
            }
            Self::InvalidVariantTag { tag } => {
                write!(formatter, "invalid trace variant tag {tag}")
            }
        }
    }
}

impl std::error::Error for TraceTableError {}

/// Flat compact trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct TraceEntry {
    /// Trace payload tag.
    tag: u32,
    /// Index inside the tagged payload table.
    index: u32,
}

const _: () = assert!(std::mem::size_of::<TraceEntry>() == 8);

/// Fixed reference offsets for one trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct FixedTrace {
    /// Local reference byte offsets.
    local_offsets: EntryRange<u32>,
    /// Shared reference byte offsets.
    shared_offsets: EntryRange<u32>,
    /// Frame reference byte offsets.
    frame_offsets: EntryRange<u32>,
}

/// One nested trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct NestedTrace {
    /// Nested payload byte offset.
    byte_offset: u32,
    /// Nested trace entry id.
    child: u32,
}

/// Multiple nested trace entries.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct CompositeTrace {
    /// Nested trace entry ids.
    children: EntryRange<u32>,
}

/// Repeated elements sharing one trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct RepeatedTrace {
    /// Element count.
    count: u32,
    /// Element byte stride.
    stride: u32,
    /// Element trace entry id.
    element: u32,
}

/// Discriminant-selected trace entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct VariantTraceEntry {
    /// Variant encoding tag.
    tag: u32,
    /// Physical discriminant field.
    field: DiscriminantField,
    /// Case represented outside one niche range.
    untagged_case: u32,
    /// First case represented by the niche range.
    niche_case_start: u32,
    /// Last case represented by the niche range.
    niche_case_end: u32,
    /// First physical niche value.
    niche_start: Discriminant,
    /// Variant trace cases.
    cases: EntryRange<VariantTraceCase>,
}

const _: () = assert!(std::mem::size_of::<VariantTraceEntry>() <= 64);

impl VariantTraceEntry {
    /// Build one stable variant trace payload.
    fn new(encoding: VariantEncoding, cases: EntryRange<VariantTraceCase>) -> Self {
        match encoding {
            VariantEncoding::Direct { field } => Self {
                tag: VARIANT_DIRECT,
                field,
                untagged_case: 0,
                niche_case_start: 0,
                niche_case_end: 0,
                niche_start: Discriminant::from_bits(0),
                cases,
            },
            VariantEncoding::Niche {
                field,
                untagged_case,
                niche_case_start,
                niche_case_end,
                niche_start,
            } => Self {
                tag: VARIANT_NICHE,
                field,
                untagged_case,
                niche_case_start,
                niche_case_end,
                niche_start,
                cases,
            },
        }
    }

    /// Return the MIR variant encoding represented by this payload.
    fn encoding(self) -> Result<VariantEncoding, TraceTableError> {
        match self.tag {
            VARIANT_DIRECT => Ok(VariantEncoding::Direct { field: self.field }),
            VARIANT_NICHE => Ok(VariantEncoding::Niche {
                field: self.field,
                untagged_case: self.untagged_case,
                niche_case_start: self.niche_case_start,
                niche_case_end: self.niche_case_end,
                niche_start: self.niche_start,
            }),
            tag => Err(TraceTableError::InvalidVariantTag { tag }),
        }
    }
}

/// Flat compact trace variant entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
struct VariantTraceCase {
    /// Logical variant discriminant value.
    discriminant: Discriminant,
    /// Variant payload byte offset.
    payload_offset: u32,
    /// Variant payload child entry id.
    child: u32,
}

const _: () = assert!(std::mem::size_of::<VariantTraceCase>() <= 24);

/// Build-time compact trace table builder.
struct TraceTableBuilder {
    /// Top-level trace root entry ids.
    roots: Vec<u32>,
    /// Flat trace entries.
    entries: Vec<TraceEntry>,
    /// Fixed trace payloads.
    fixed: Vec<FixedTrace>,
    /// Nested trace payloads.
    nested: Vec<NestedTrace>,
    /// Composite trace payloads.
    composite: Vec<CompositeTrace>,
    /// Repeated trace payloads.
    repeated: Vec<RepeatedTrace>,
    /// Variant trace payloads.
    variants: Vec<VariantTraceEntry>,
    /// Flattened trace byte offsets.
    offsets: EntryStore<u32>,
    /// Flattened child entry ids.
    children: EntryStore<u32>,
    /// Flattened variant cases.
    cases: EntryStore<VariantTraceCase>,
}

impl TraceTableBuilder {
    /// Create one empty trace table builder.
    fn new() -> Self {
        Self {
            roots: Vec::new(),
            entries: Vec::new(),
            fixed: Vec::new(),
            nested: Vec::new(),
            composite: Vec::new(),
            repeated: Vec::new(),
            variants: Vec::new(),
            offsets: EntryStore::new(),
            children: EntryStore::new(),
            cases: EntryStore::new(),
        }
    }

    /// Push one trace map and return its entry id.
    fn push_map(&mut self, map: &TraceMap) -> u32 {
        let entry = self.entry(map);
        let entry_id = self.entries.len() as u32;
        self.entries.push(entry);

        entry_id
    }

    /// Build one flat entry for one trace map.
    fn entry(&mut self, map: &TraceMap) -> TraceEntry {
        match map {
            TraceMap::Empty => TraceEntry {
                tag: TRACE_EMPTY,
                index: 0,
            },
            TraceMap::Fixed {
                local_offsets,
                shared_offsets,
                frame_offsets,
            } => {
                let fixed = FixedTrace {
                    local_offsets: self.offsets.append(local_offsets.iter().copied()),
                    shared_offsets: self.offsets.append(shared_offsets.iter().copied()),
                    frame_offsets: self.offsets.append(frame_offsets.iter().copied()),
                };
                let index = self.fixed.len() as u32;
                self.fixed.push(fixed);

                TraceEntry {
                    tag: TRACE_FIXED,
                    index,
                }
            }
            TraceMap::Nested { byte_offset, map } => {
                let nested = NestedTrace {
                    byte_offset: *byte_offset,
                    child: self.push_map(map),
                };
                let index = self.nested.len() as u32;
                self.nested.push(nested);

                TraceEntry {
                    tag: TRACE_NESTED,
                    index,
                }
            }
            TraceMap::Composite { maps } => {
                let children = maps
                    .iter()
                    .map(|map| self.push_map(map))
                    .collect::<Vec<_>>();
                let composite = CompositeTrace {
                    children: self.children.append(children),
                };
                let index = self.composite.len() as u32;
                self.composite.push(composite);

                TraceEntry {
                    tag: TRACE_COMPOSITE,
                    index,
                }
            }
            TraceMap::Repeated {
                count,
                stride,
                element,
            } => {
                let repeated = RepeatedTrace {
                    count: *count,
                    stride: *stride,
                    element: self.push_map(element),
                };
                let index = self.repeated.len() as u32;
                self.repeated.push(repeated);

                TraceEntry {
                    tag: TRACE_REPEATED,
                    index,
                }
            }
            TraceMap::Variant { encoding, cases } => {
                let cases = cases
                    .iter()
                    .map(|case| VariantTraceCase {
                        discriminant: case.discriminant,
                        payload_offset: case.payload_offset,
                        child: self.push_map(&case.map),
                    })
                    .collect::<Vec<_>>();
                let cases = self.cases.append(cases);
                let variant = VariantTraceEntry::new(*encoding, cases);
                let index = self.variants.len() as u32;
                self.variants.push(variant);

                TraceEntry {
                    tag: TRACE_VARIANT,
                    index,
                }
            }
        }
    }

    /// Pack one section-backed compact trace table.
    fn pack(self, sections: &mut SectionBuilder) -> TraceTable {
        let roots = sections.insert(self.roots);
        let entries = sections.insert(self.entries);
        let fixed = sections.insert(self.fixed);
        let nested = sections.insert(self.nested);
        let composite = sections.insert(self.composite);
        let repeated = sections.insert(self.repeated);
        let variants = sections.insert(self.variants);
        let offsets = sections.insert(self.offsets.into_entries());
        let children = sections.insert(self.children.into_entries());
        let cases = sections.insert(self.cases.into_entries());

        TraceTable {
            roots,
            entries,
            fixed,
            nested,
            composite,
            repeated,
            variants,
            offsets,
            children,
            cases,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_core::{SectionBuilder, SectionImage};
    use destack_mir as mir;
    use destack_mir::{DiscriminantField, VariantEncoding};

    use super::*;

    /// Roundtrip direct and niche variant maps through compact trace sections.
    #[test]
    fn test_roundtrip_variant_trace_maps() {
        let cases = vec![
            VariantTrace {
                discriminant: 3u128.into(),
                payload_offset: 8,
                map: TraceMap::Fixed {
                    local_offsets: vec![0].into_boxed_slice(),
                    shared_offsets: Box::default(),
                    frame_offsets: Box::default(),
                },
            },
            VariantTrace {
                discriminant: 7u128.into(),
                payload_offset: 16,
                map: TraceMap::Empty,
            },
        ]
        .into_boxed_slice();
        let direct = TraceMap::Variant {
            encoding: VariantEncoding::Direct {
                field: DiscriminantField::scalar(4, 1),
            },
            cases: cases.clone(),
        };
        let niche = TraceMap::Variant {
            encoding: VariantEncoding::Niche {
                field: DiscriminantField::scalar(8, 8),
                untagged_case: 0,
                niche_case_start: 1,
                niche_case_end: 1,
                niche_start: 0u128.into(),
            },
            cases,
        };
        let mut source = mir::TraceTable::new();
        let direct_id = source.insert(direct.clone());
        let niche_id = source.insert(niche.clone());
        let mut sections = SectionBuilder::new();
        let table = TraceTable::pack(&mut sections, &source);
        let storage = sections.build();
        let sections = SectionImage::new(&storage);
        let view = table.view(sections);

        assert_eq!(view.trace_map(direct_id), Ok(direct));
        assert_eq!(view.trace_map(niche_id), Ok(niche));
    }
}
