use crate::StaticAddress;
use destack_heap::{HeapReference, SharedHeapReference};
use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::vm::error::Error;
use crate::vm::{Cell, FramePointer, FunctionPointer, ReferenceMeta, StackPointer};

use super::repr_type;

/// Scalar value shape for typed vector and tensor operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalarLayout {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point values with a concrete format.
    Float {
        /// The concrete float format.
        format: mir::FloatType,
    },
    /// Boolean values.
    Bool,
}

/// Runtime address space for addressable values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressSpace {
    /// Local heap storage.
    Local,
    /// Shared heap storage.
    Shared,
    /// Raw native storage.
    Raw,
    /// Stack pointer.
    Stack,
    /// Frame pointer.
    Frame,
    /// Static address.
    Static,
}

/// Runtime value shape used for op selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueShape {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed or unsigned integer with width.
    Int { width: u16, signed: bool },
    /// Floating point value with format.
    Float { format: mir::FloatType },
    /// Unicode character value.
    Char,
    /// Addressable value with pointee type.
    Pointer {
        pointee: mir::LocalNodeId<mir::Type>,
        address_space: AddressSpace,
        reference: ReferenceMeta,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: mir::LocalNodeId<mir::Type> },
    /// Opaque closure value.
    Closure { ty: mir::LocalNodeId<mir::Type> },
    /// Frame byte value with concrete type.
    FrameBytes { ty: mir::LocalNodeId<mir::Type> },
    /// Fixed-size array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
    },
}

impl ValueShape {
    /// Return whether this value lives inline in the current frame.
    #[inline(always)]
    pub const fn is_frame_storage(self) -> bool {
        matches!(self, Self::FrameBytes { .. } | Self::Array { .. })
    }
}

/// Native cell layout for one load or store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellLayout {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed integer value.
    Int { width: u8 },
    /// Unsigned integer value.
    Uint { width: u8 },
    /// Float16 value.
    Float16,
    /// BF16 value.
    Bfloat16,
    /// Float32 value.
    Float32,
    /// Float64 value.
    Float64,
    /// Local heap reference.
    HeapReference,
    /// Shared heap reference.
    SharedHeapReference,
    /// Native address.
    Address,
    /// Stack pointer.
    StackPointer,
    /// Frame pointer.
    FramePointer,
    /// Static address.
    StaticAddress,
    /// Function pointer.
    FunctionPointer,
}

impl CellLayout {
    /// Return the memory byte width for this cell layout.
    #[inline(always)]
    pub fn byte_len(self, pointer_bytes: usize) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool => 1,
            Self::Int { width } | Self::Uint { width } => (width as usize).div_ceil(8),
            Self::Float16 | Self::Bfloat16 => 2,
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::HeapReference
            | Self::SharedHeapReference
            | Self::Address
            | Self::StackPointer
            | Self::FramePointer
            | Self::FunctionPointer => pointer_bytes,
            Self::StaticAddress => Cell::BYTE_LEN,
        }
    }

    /// Decode raw memory bits into one VM cell.
    #[inline(always)]
    pub fn decode(self, raw: u64) -> Cell {
        match self {
            Self::Void => Cell::ZERO,
            Self::Bool => Cell::bool(raw != 0),
            Self::Int { width } => Cell::int(raw as i64, width),
            Self::Uint { width } => Cell::uint(raw, width),
            Self::Float16 | Self::Bfloat16 => Cell::from_bits(raw),
            Self::Float32 => Cell::float32(f32::from_bits(raw as u32)),
            Self::Float64 => Cell::float64(f64::from_bits(raw)),
            Self::HeapReference => Cell::heap_reference(HeapReference::from_bits(raw as usize)),
            Self::SharedHeapReference => {
                Cell::shared_heap_reference(SharedHeapReference::from_bits(raw as usize))
            }
            Self::Address => Cell::address(raw as usize),
            Self::StackPointer => Cell::stack_pointer(StackPointer::from_address(raw as usize)),
            Self::FramePointer => Cell::frame_pointer(FramePointer::from_address(raw as usize)),
            Self::StaticAddress => Cell::static_address(StaticAddress::from_bits(raw)),
            Self::FunctionPointer => {
                Cell::function_pointer(FunctionPointer::from_bits(raw as usize))
            }
        }
    }

    /// Encode one VM cell into raw memory bits.
    #[inline(always)]
    pub fn encode(self, value: Cell) -> u64 {
        match self {
            Self::Void => 0,
            Self::Bool => u64::from(value.as_bool()),
            Self::Int { .. } => value.bits(),
            Self::Uint { .. } => value.as_u64(),
            Self::Float16 | Self::Bfloat16 => value.bits(),
            Self::Float32 => value.as_f32().to_bits() as u64,
            Self::Float64 => value.as_f64().to_bits(),
            Self::HeapReference => value.as_heap_reference().bits() as u64,
            Self::SharedHeapReference => value.as_shared_heap_reference().bits() as u64,
            Self::Address => value.as_address() as u64,
            Self::StackPointer => value.as_stack_pointer().bits() as u64,
            Self::FramePointer => value.as_frame_pointer().bits() as u64,
            Self::StaticAddress => value.as_static_address().bits(),
            Self::FunctionPointer => value.as_function_pointer().bits() as u64,
        }
    }
}

/// One cell encoded as raw bytes.
#[derive(Debug)]
pub struct CellEncoding {
    /// The byte buffer.
    bytes: [u8; Cell::BYTE_LEN],
    /// The number of initialized bytes.
    len: usize,
}

impl CellEncoding {
    /// Return the initialized bytes.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return the initialized byte count.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return whether the encoding is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Encode one VM cell into raw bits for the given type.
pub fn encode_cell_bits(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
) -> Result<(u64, usize), Error> {
    // resolve the scalar layout once
    let Some(layout) = cell_layout_from_type(tree, ty) else {
        return Err(Error::type_mismatch(
            "scalar or reference raw store",
            format!("{ty:?}"),
        ));
    };

    // encode into memory bits
    let raw = layout.encode(value);
    let byte_len = layout.byte_len(tree.pointer_bytes() as usize);

    Ok((raw, byte_len))
}

/// Encode one VM cell into scalar bytes.
pub fn encode_cell_bytes(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
) -> Result<CellEncoding, Error> {
    let (raw, byte_len) = encode_cell_bits(tree, ty, value)?;

    Ok(CellEncoding {
        bytes: raw.to_le_bytes(),
        len: byte_len,
    })
}

/// Return the runtime value shape for a MIR type.
pub fn value_shape_from_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<ValueShape> {
    // map mir type to value shape
    match tree.get(ty) {
        mir::Type::Error => None,
        mir::Type::WithLifetimes { base, .. } => value_shape_from_type(tree, *base),
        mir::Type::Void => Some(ValueShape::Void),
        mir::Type::Boolean => Some(ValueShape::Bool),
        mir::Type::Int { width, is_signed } => Some(ValueShape::Int {
            width: *width,
            signed: *is_signed,
        }),
        mir::Type::Isize => Some(ValueShape::Int {
            width: usize::BITS as u16,
            signed: true,
        }),
        mir::Type::Usize => Some(ValueShape::Int {
            width: usize::BITS as u16,
            signed: false,
        }),
        mir::Type::Float(float_type) => Some(ValueShape::Float {
            format: *float_type,
        }),
        mir::Type::TypeDescriptor | mir::Type::TypeId => Some(ValueShape::Int {
            width: usize::BITS as u16,
            signed: false,
        }),
        mir::Type::Reference {
            kind,
            space,
            access,
            pointee,
            nullability,
            ..
        } => {
            let address_space = address_space_from_reference(space.clone(), *kind);

            Some(ValueShape::Pointer {
                pointee: *pointee,
                address_space,
                reference: ReferenceMeta::new(*kind, space.clone(), *access, *nullability),
            })
        }
        mir::Type::FunctionSignature { result, .. } => {
            Some(ValueShape::FunctionPointer { result: *result })
        }
        mir::Type::FunctionPointer { signature } => match tree.get(*signature) {
            mir::Type::FunctionSignature { result, .. } => {
                Some(ValueShape::FunctionPointer { result: *result })
            }
            _ => None,
        },
        mir::Type::FixedArray {
            element,
            length,
            copy: _,
        } => Some(ValueShape::Array {
            element: *element,
            length: *length,
        }),
        mir::Type::Slice { .. } => Some(ValueShape::FrameBytes { ty }),
        mir::Type::Uninit { value } => value_shape_from_type(tree, *value),
        mir::Type::Dynamic { .. } => Some(ValueShape::FrameBytes { ty }),
        mir::Type::Atomic { value } => value_shape_from_type(tree, *value),
        mir::Type::Newtype { inner, .. } => value_shape_from_type(tree, *inner),
        mir::Type::Closure { .. } => Some(ValueShape::Closure { ty }),
        mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Variant { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => Some(ValueShape::FrameBytes { ty }),
        mir::Type::TensorView { .. } => Some(ValueShape::FrameBytes { ty }),
    }
}

/// Return the native cell layout for a MIR type.
pub fn cell_layout_from_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<CellLayout> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Void => Some(CellLayout::Void),
        mir::Type::Boolean => Some(CellLayout::Bool),
        mir::Type::Int { width, is_signed } => {
            let width = u8::try_from(*width).ok()?;

            if *is_signed {
                Some(CellLayout::Int { width })
            } else {
                Some(CellLayout::Uint { width })
            }
        }
        mir::Type::Isize => Some(CellLayout::Int {
            width: usize::BITS as u8,
        }),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            Some(CellLayout::Uint {
                width: usize::BITS as u8,
            })
        }
        mir::Type::Float(mir::FloatType::Float16) => Some(CellLayout::Float16),
        mir::Type::Float(mir::FloatType::Bfloat16) => Some(CellLayout::Bfloat16),
        mir::Type::Float(mir::FloatType::Float32) => Some(CellLayout::Float32),
        mir::Type::Float(mir::FloatType::Float64) => Some(CellLayout::Float64),
        mir::Type::Reference { kind, space, .. } => {
            let address_space = address_space_from_reference(space.clone(), *kind);

            cell_layout_from_address_space(address_space)
        }
        mir::Type::Uninit { value } => cell_layout_from_type(tree, *value),
        mir::Type::Closure { .. } => Some(CellLayout::HeapReference),
        mir::Type::FunctionSignature { .. } | mir::Type::FunctionPointer { .. } => {
            Some(CellLayout::FunctionPointer)
        }
        mir::Type::Atomic { value } => cell_layout_from_type(tree, *value),
        _ => None,
    }
}

/// Return the scalar layout for one MIR type.
pub fn scalar_layout_from_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<ScalarLayout> {
    match tree.get(ty) {
        mir::Type::Int { width, is_signed } => Some(ScalarLayout::Int {
            width: *width,
            is_signed: *is_signed,
        }),
        mir::Type::Isize => Some(ScalarLayout::Int {
            width: usize::BITS as u16,
            is_signed: true,
        }),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            Some(ScalarLayout::Int {
                width: usize::BITS as u16,
                is_signed: false,
            })
        }
        mir::Type::Float(float_type) => Some(ScalarLayout::Float {
            format: *float_type,
        }),
        mir::Type::Boolean => Some(ScalarLayout::Bool),
        _ => None,
    }
}

/// Map a reference kind to one runtime address space.
pub fn address_space_from_reference(space: mir::Space, kind: mir::ReferenceKind) -> AddressSpace {
    match space {
        mir::Space::Local => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => AddressSpace::Local,
            mir::ReferenceKind::Borrowed => AddressSpace::Local,
            mir::ReferenceKind::Raw => AddressSpace::Raw,
        },
        mir::Space::Shared => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => AddressSpace::Shared,
            mir::ReferenceKind::Borrowed => AddressSpace::Shared,
            mir::ReferenceKind::Raw => AddressSpace::Raw,
        },
        mir::Space::Frame => AddressSpace::Frame,
        mir::Space::Static => AddressSpace::Static,
    }
}

/// Map one address space to the cell layout carried by memory.
pub fn cell_layout_from_address_space(address_space: AddressSpace) -> Option<CellLayout> {
    Some(match address_space {
        AddressSpace::Local => CellLayout::HeapReference,
        AddressSpace::Shared => CellLayout::SharedHeapReference,
        AddressSpace::Raw => CellLayout::Address,
        AddressSpace::Stack => CellLayout::StackPointer,
        AddressSpace::Frame => CellLayout::FramePointer,
        AddressSpace::Static => CellLayout::StaticAddress,
    })
}
