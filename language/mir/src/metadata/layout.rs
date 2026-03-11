use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LocalNodeId, Type};

/// Canonical module data layout metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataLayout {
    /// Native pointer size in bytes for this module.
    pub native_pointer_bytes: u8,
    /// Managed reference representation for this module.
    pub managed_reference_layout: ManagedReferenceLayout,
}

/// Managed reference representation metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedReferenceLayout {
    /// Managed reference size in bytes.
    pub bytes: u8,
    /// Managed reference alignment in bytes.
    pub alignment: u8,
    /// Managed reference encoding.
    pub representation: ManagedReferenceRepresentation,
}

/// Managed reference encoding strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedReferenceRepresentation {
    /// Native machine pointer.
    NativePointer,
    /// Offset from a managed heap base.
    CompressedOffset32,
    /// Indirect 32 bit handle.
    Handle32,
    /// Indirect 64 bit handle.
    Handle64,
}

impl Default for DataLayout {
    fn default() -> Self {
        Self {
            native_pointer_bytes: 8,
            managed_reference_layout: ManagedReferenceLayout::default(),
        }
    }
}

impl Default for ManagedReferenceLayout {
    fn default() -> Self {
        Self {
            bytes: 8,
            alignment: 8,
            representation: ManagedReferenceRepresentation::NativePointer,
        }
    }
}

impl DataLayout {
    /// Create a data layout with a specific pointer size.
    pub fn with_pointer_bytes(pointer_bytes: u8) -> Self {
        Self {
            native_pointer_bytes: pointer_bytes,
            managed_reference_layout: ManagedReferenceLayout {
                bytes: pointer_bytes,
                alignment: pointer_bytes,
                representation: ManagedReferenceRepresentation::NativePointer,
            },
        }
    }

    /// Return pointer width in bits.
    pub fn pointer_bits(self) -> u16 {
        u16::from(self.native_pointer_bytes) * 8
    }

    /// Return managed reference width in bits.
    pub fn managed_reference_bits(self) -> u16 {
        u16::from(self.managed_reference_layout.bytes) * 8
    }
}

/// Opaque identifier for a concrete memory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayoutId(pub u32);

/// Shared layout table for all aggregate types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    pub layouts: Vec<Layout>,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a layout entry and return its id.
    pub fn insert(&mut self, layout: Layout) -> LayoutId {
        let id = LayoutId(self.layouts.len() as u32);
        self.layouts.push(layout);
        id
    }

    /// Return a layout entry for an id.
    pub fn layout(&self, id: LayoutId) -> &Layout {
        let index = id.0 as usize;
        self.layouts
            .get(index)
            .unwrap_or_else(|| panic!("missing layout entry {index}"))
    }
}

/// Concrete memory layout for an aggregate type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// The layout kind and kind specific data.
    pub layout_type: LayoutType,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Field layouts in concrete memory order.
    pub fields: Vec<LayoutField>,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: StringId,
    /// MIR type of the field.
    pub ty: LocalNodeId<Type>,
    /// Byte offset from the start of the aggregate.
    pub offset: u32,
    /// Size of the field in bytes.
    pub size: u32,
    /// Alignment requirement of the field in bytes.
    pub alignment: u32,
    /// Original source index for stable mapping.
    pub source_index: Option<u32>,
}

/// Aggregate layout kinds with kind specific data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutType {
    /// Plain struct layout.
    Struct,
    /// Tuple layout with ordered elements.
    Tuple,
    /// Array layout with stride and optional fixed count.
    Array {
        /// The array element type.
        element_type: LocalNodeId<Type>,
        /// The stride between array elements in bytes.
        element_stride: u32,
        /// The fixed element count when known.
        element_count: Option<u32>,
    },
    /// Union layout with tag and payload offsets.
    Union {
        /// The tag type used for discriminants.
        tag_type: LocalNodeId<Type>,
        /// The byte offset of the tag field.
        tag_offset: u32,
        /// The byte offset of the payload field.
        payload_offset: u32,
    },
    /// Interface layout with object and table offsets.
    Interface {
        /// The byte offset of the object pointer.
        object_offset: u32,
        /// The byte offset of the table pointer.
        table_offset: u32,
    },
    /// Closure environment layout.
    ClosureEnv,
    /// Function value layout.
    FunctionValue,
}
