use std::fmt;

use destack_core::{
    EntryRange, EntryStore, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_heap::TraceView;
use destack_mir::{TraceId, TraceMap, TraceVariant};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Program trace table stored in sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TraceTable {
    /// Top-level trace roots indexed by TraceId.
    roots: SectionSlice<u32>,
    /// Flat trace nodes.
    nodes: SectionSlice<TraceNode>,
    /// Flattened trace byte offsets.
    offsets: SectionSlice<u32>,
    /// Flattened child node ids.
    children: SectionSlice<u32>,
    /// Flattened tagged variant entries.
    variants: SectionSlice<TraceVariantNode>,
}

impl TraceTable {
    /// Pack one program trace table from compiler trace maps.
    pub fn pack(sections: &mut SectionPacker, source: &destack_mir::TraceTable) -> Self {
        let mut builder = TraceTableBuilder::new();

        // preserve TraceId order for all top-level roots
        for trace in source.traces() {
            let root = builder.push_map(trace);
            builder.roots.push(root);
        }

        builder.pack(sections)
    }

    /// Decode all program trace maps in TraceId order.
    fn decode(&self, sections: SectionImage<'_>) -> Result<Vec<TraceMap>, TraceTableError> {
        let roots = sections.entries(self.roots);
        let mut traces = Vec::with_capacity(roots.len());

        // rebuild the hot runtime view once during program load or construction
        for root in roots {
            let trace = self.decode_map(sections, *root)?;
            traces.push(trace);
        }

        Ok(traces)
    }

    /// Decode one program trace node into a semantic trace map.
    fn decode_map(
        &self,
        sections: SectionImage<'_>,
        node_id: u32,
    ) -> Result<TraceMap, TraceTableError> {
        let Some(node) = sections.entries(self.nodes).get(node_id as usize).copied() else {
            return Err(TraceTableError::MissingNode { node: node_id });
        };

        // decode the compact trace node by kind
        match node.kind {
            TraceNodeKind::EMPTY => Ok(TraceMap::Empty),
            TraceNodeKind::FIXED => Ok(TraceMap::Fixed {
                local_offsets: node
                    .local_offsets
                    .slice(sections.entries(self.offsets))
                    .into(),
                shared_offsets: node
                    .shared_offsets
                    .slice(sections.entries(self.offsets))
                    .into(),
            }),
            TraceNodeKind::NESTED => Ok(TraceMap::Nested {
                byte_offset: node.byte_offset,
                map: Box::new(self.decode_map(sections, node.first_node)?),
            }),
            TraceNodeKind::COMPOSITE => {
                let maps = node
                    .children
                    .slice(sections.entries(self.children))
                    .iter()
                    .map(|child| self.decode_map(sections, *child))
                    .collect::<Result<Box<[_]>, _>>()?;

                Ok(TraceMap::Composite { maps })
            }
            TraceNodeKind::REPEATED => Ok(TraceMap::Repeated {
                count: node.count,
                stride: node.stride,
                element: Box::new(self.decode_map(sections, node.first_node)?),
            }),
            TraceNodeKind::TAGGED => {
                let variants = node
                    .variants
                    .slice(sections.entries(self.variants))
                    .iter()
                    .map(|variant| {
                        Ok(TraceVariant {
                            tag: variant.tag,
                            payload_offset: variant.payload_offset,
                            map: self.decode_map(sections, variant.node)?,
                        })
                    })
                    .collect::<Result<Box<[_]>, TraceTableError>>()?;

                Ok(TraceMap::Tagged {
                    tag_bytes: node.tag_bytes as u8,
                    variants,
                })
            }
            kind => Err(TraceTableError::UnknownKind { kind: kind.raw() }),
        }
    }
}

/// Decoded trace maps used by heap and GC paths.
#[derive(Debug, Clone)]
pub struct TraceCache {
    /// Decoded trace maps indexed by TraceId.
    maps: Vec<TraceMap>,
}

impl TraceCache {
    /// Decode one trace cache from a section-backed trace table.
    pub fn decode(table: &TraceTable, sections: SectionImage<'_>) -> Result<Self, TraceTableError> {
        let maps = table.decode(sections)?;

        Ok(Self { maps })
    }

    /// Return decoded trace maps as a heap trace view.
    pub fn maps(&self) -> TraceView<'_> {
        TraceView::new(&self.maps)
    }

    /// Return one decoded trace map by id.
    pub fn trace(&self, id: TraceId) -> Option<&TraceMap> {
        self.maps().trace(id)
    }
}

/// Invalid program trace table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceTableError {
    /// A trace node id does not name a stored trace node.
    MissingNode {
        /// Missing node id.
        node: u32,
    },
    /// A trace node carries an unknown trace kind.
    UnknownKind {
        /// Invalid trace kind.
        kind: u32,
    },
}

impl fmt::Display for TraceTableError {
    /// Format one trace table error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node } => write!(formatter, "missing trace node {node}"),
            Self::UnknownKind { kind } => write!(formatter, "unknown trace kind {kind}"),
        }
    }
}

impl std::error::Error for TraceTableError {}

/// Flat program trace node.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceNode {
    /// Trace node kind.
    kind: TraceNodeKind,
    /// Byte offset used by nested maps.
    byte_offset: u32,
    /// Repeated element count.
    count: u32,
    /// Repeated element byte stride.
    stride: u32,
    /// First child node id for single-child maps.
    first_node: u32,
    /// Tagged variant tag byte width.
    tag_bytes: u32,
    /// Fixed local reference byte offsets.
    local_offsets: EntryRange<u32>,
    /// Fixed shared reference byte offsets.
    shared_offsets: EntryRange<u32>,
    /// Composite child node ids.
    children: EntryRange<u32>,
    /// Tagged variant entries.
    variants: EntryRange<TraceVariantNode>,
}

impl TraceNode {
    /// Create one trace node with empty payload ranges.
    fn new(kind: TraceNodeKind) -> Self {
        Self {
            kind,
            byte_offset: 0,
            count: 0,
            stride: 0,
            first_node: 0,
            tag_bytes: 0,
            local_offsets: EntryRange::empty(),
            shared_offsets: EntryRange::empty(),
            children: EntryRange::empty(),
            variants: EntryRange::empty(),
        }
    }
}

/// Fixed-width trace node kind stored in program sections.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceNodeKind(u32);

impl TraceNodeKind {
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

/// Flat program trace variant node.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct TraceVariantNode {
    /// Normalized variant tag value.
    tag: u64,
    /// Variant payload byte offset.
    payload_offset: u32,
    /// Variant payload trace node id.
    node: u32,
}

/// Build-time program trace table builder.
struct TraceTableBuilder {
    /// Top-level trace root node ids.
    roots: Vec<u32>,
    /// Flat trace nodes.
    nodes: Vec<TraceNode>,
    /// Flattened trace byte offsets.
    offsets: EntryStore<u32>,
    /// Flattened child node ids.
    children: EntryStore<u32>,
    /// Flattened tagged variant entries.
    variants: EntryStore<TraceVariantNode>,
}

impl TraceTableBuilder {
    /// Create one empty trace table builder.
    fn new() -> Self {
        Self {
            roots: Vec::new(),
            nodes: Vec::new(),
            offsets: EntryStore::new(),
            children: EntryStore::new(),
            variants: EntryStore::new(),
        }
    }

    /// Push one trace map and return its node id.
    fn push_map(&mut self, map: &TraceMap) -> u32 {
        let node_id = self.nodes.len() as u32;
        let node = self.node(map);
        self.nodes.push(node);

        node_id
    }

    /// Build one flat node for one trace map.
    fn node(&mut self, map: &TraceMap) -> TraceNode {
        match map {
            TraceMap::Empty => TraceNode::new(TraceNodeKind::EMPTY),
            TraceMap::Fixed {
                local_offsets,
                shared_offsets,
            } => {
                let mut node = TraceNode::new(TraceNodeKind::FIXED);
                node.local_offsets = self.offsets.append(local_offsets.iter().copied());
                node.shared_offsets = self.offsets.append(shared_offsets.iter().copied());

                node
            }
            TraceMap::Nested { byte_offset, map } => {
                let mut node = TraceNode::new(TraceNodeKind::NESTED);
                node.byte_offset = *byte_offset;
                node.first_node = self.push_map(map);

                node
            }
            TraceMap::Composite { maps } => {
                let children = maps
                    .iter()
                    .map(|map| self.push_map(map))
                    .collect::<Vec<_>>();

                let mut node = TraceNode::new(TraceNodeKind::COMPOSITE);
                node.children = self.children.append(children);

                node
            }
            TraceMap::Repeated {
                count,
                stride,
                element,
            } => {
                let mut node = TraceNode::new(TraceNodeKind::REPEATED);
                node.count = *count;
                node.stride = *stride;
                node.first_node = self.push_map(element);

                node
            }
            TraceMap::Tagged {
                tag_bytes,
                variants,
            } => {
                let variants = variants
                    .iter()
                    .map(|variant| TraceVariantNode {
                        tag: variant.tag,
                        payload_offset: variant.payload_offset,
                        node: self.push_map(&variant.map),
                    })
                    .collect::<Vec<_>>();

                let mut node = TraceNode::new(TraceNodeKind::TAGGED);
                node.tag_bytes = *tag_bytes as u32;
                node.variants = self.variants.append(variants);

                node
            }
        }
    }

    /// Pack one section-backed program trace table.
    fn pack(self, sections: &mut SectionPacker) -> TraceTable {
        let roots = sections.insert(self.roots);
        let nodes = sections.insert(self.nodes);
        let offsets = sections.insert(self.offsets.into_entries());
        let children = sections.insert(self.children.into_entries());
        let variants = sections.insert(self.variants.into_entries());

        TraceTable {
            roots,
            nodes,
            offsets,
            children,
            variants,
        }
    }
}

// SAFETY: trace nodes are fixed-width program entries containing only integers and ranges.
unsafe impl SectionEntry for TraceNodeKind {}
unsafe impl SectionEntry for TraceNode {}
unsafe impl SectionEntry for TraceVariantNode {}
