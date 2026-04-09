use std::mem::size_of;
use std::num::NonZeroU32;

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
pub struct LayoutId(NonZeroU32);

impl LayoutId {
    /// Create a layout identifier from one raw value.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        match NonZeroU32::new(raw) {
            Some(raw) => Self(raw),
            None => panic!("layout identifiers must be non-zero"),
        }
    }

    /// Return the raw layout identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Return the zero based layout-table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

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
        let next_index = self.layouts.len() + 1;
        let id = LayoutId::new(next_index as u32);
        self.layouts.push(layout);
        id
    }

    /// Return a layout entry for an id.
    pub fn layout(&self, id: LayoutId) -> &Layout {
        let index = id.index();
        self.layouts
            .get(index)
            .unwrap_or_else(|| panic!("missing layout entry {index}"))
    }

    /// Return the owned bytes for this layout table.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.layouts.capacity() * size_of::<Layout>();

        for layout in &self.layouts {
            owned_bytes += layout.fields.capacity() * size_of::<LayoutField>();
        }

        owned_bytes
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
    /// Function environment layout.
    FunctionEnvironment,
    /// Function value layout.
    Closure,
}
