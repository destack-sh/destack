use crate::{
    Error, FramePointer, FunctionPointer, HeapReference, RawPointer, ReferenceMeta,
    SharedHeapReference, SharedRawPointer, StackPointer, StaticPointer, Word,
};
use destack_mir as mir;

use super::repr_type;

/// Scalar value layout for typed vector and tensor operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarLayout {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point values with a bit width.
    Float {
        /// The bit width.
        width: u16,
    },
    /// Boolean values.
    Bool,
}

/// Runtime class for pointer-like values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerClass {
    /// Local heap reference.
    Heap,
    /// Shared heap reference.
    SharedHeap,
    /// Interior local heap address.
    HeapAddress,
    /// Interior shared heap address.
    SharedHeapAddress,
    /// Raw heap pointer.
    Raw,
    /// Shared raw-space pointer.
    SharedRaw,
    /// Stack pointer.
    Stack,
    /// Frame pointer.
    Frame,
    /// Static pointer.
    Static,
    /// Unknown pointer class.
    Unknown,
}

/// Runtime value layout used for op selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueLayout {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed or unsigned integer with width.
    Int { width: u16, signed: bool },
    /// Floating point value with width.
    Float { width: u16 },
    /// Unicode character value.
    Char,
    /// Pointer-like value with pointee type.
    Pointer {
        pointee: mir::LocalNodeId<mir::Type>,
        pointer_class: PointerClass,
        reference: ReferenceMeta,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: mir::LocalNodeId<mir::Type> },
    /// Opaque callable value.
    Callable { ty: mir::LocalNodeId<mir::Type> },
    /// Frame byte value with concrete type.
    FrameBytes { ty: mir::LocalNodeId<mir::Type> },
    /// Fixed-size array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
    },
    /// Unknown or unsupported type.
    Unknown,
}

/// Native word layout for one load or store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordLayout {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed integer value.
    Int { width: u8 },
    /// Unsigned integer value.
    Uint { width: u8 },
    /// Float32 value.
    Float32,
    /// Float64 value.
    Float64,
    /// Local heap reference.
    HeapReference,
    /// Shared heap reference.
    SharedHeapReference,
    /// Raw heap pointer.
    RawPointer,
    /// Shared raw-space pointer.
    SharedRawPointer,
    /// Stack pointer.
    StackPointer,
    /// Frame pointer.
    FramePointer,
    /// Static pointer.
    StaticPointer,
    /// Function pointer.
    FunctionPointer,
}

impl WordLayout {
    /// Return the memory byte width for this word layout.
    #[inline(always)]
    pub(crate) fn byte_len(self, pointer_bytes: usize) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool => 1,
            Self::Int { width } | Self::Uint { width } => (width as usize).div_ceil(8),
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::HeapReference
            | Self::SharedHeapReference
            | Self::RawPointer
            | Self::SharedRawPointer
            | Self::StackPointer
            | Self::FramePointer
            | Self::StaticPointer
            | Self::FunctionPointer => pointer_bytes,
        }
    }

    /// Decode raw memory bits into one VM word.
    #[inline(always)]
    pub(crate) fn decode(self, raw: u64) -> Word {
        match self {
            Self::Void => Word::VOID,
            Self::Bool => Word::bool(raw != 0),
            Self::Int { width } => Word::int(raw as i64, width),
            Self::Uint { width } => Word::uint(raw, width),
            Self::Float32 => Word::float32(f32::from_bits(raw as u32)),
            Self::Float64 => Word::float64(f64::from_bits(raw)),
            Self::HeapReference => Word::heap_reference(HeapReference::from_bits(raw as usize)),
            Self::SharedHeapReference => {
                Word::shared_heap_reference(SharedHeapReference::from_bits(raw as usize))
            }
            Self::RawPointer => Word::raw_pointer(RawPointer::from_bits(raw as usize)),
            Self::SharedRawPointer => {
                Word::shared_raw_pointer(SharedRawPointer::from_bits(raw as usize))
            }
            Self::StackPointer => Word::stack_pointer(StackPointer::from_address(raw as usize)),
            Self::FramePointer => Word::frame_pointer(FramePointer::from_address(raw as usize)),
            Self::StaticPointer => Word::static_pointer(StaticPointer::from_address(raw as usize)),
            Self::FunctionPointer => {
                Word::function_pointer(FunctionPointer::from_bits(raw as usize))
            }
        }
    }

    /// Encode one VM word into raw memory bits.
    #[inline(always)]
    pub(crate) fn encode(self, value: Word) -> u64 {
        match self {
            Self::Void => 0,
            Self::Bool => u64::from(value.as_bool()),
            Self::Int { .. } => value.bits(),
            Self::Uint { .. } => value.as_uint(),
            Self::Float32 => value.as_float32().to_bits() as u64,
            Self::Float64 => value.as_float64().to_bits(),
            Self::HeapReference => value.as_heap_reference().bits() as u64,
            Self::SharedHeapReference => value.as_shared_heap_reference().bits() as u64,
            Self::RawPointer => value.as_raw_pointer().bits() as u64,
            Self::SharedRawPointer => value.as_shared_raw_pointer().bits() as u64,
            Self::StackPointer => value.as_stack_pointer().bits() as u64,
            Self::FramePointer => value.as_frame_pointer().bits() as u64,
            Self::StaticPointer => value.as_static_pointer().bits() as u64,
            Self::FunctionPointer => value.as_function_pointer().bits() as u64,
        }
    }
}

/// One word encoded as raw bytes.
pub(crate) struct WordEncoding {
    /// The byte buffer.
    bytes: [u8; Word::BYTE_LEN],
    /// The number of initialized bytes.
    len: usize,
}

impl WordEncoding {
    /// Return the initialized bytes.
    #[inline(always)]
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return the initialized byte count.
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

/// Encode one VM word into raw bits for the given type.
pub(crate) fn encode_word_bits(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<(u64, usize), Error> {
    // resolve the scalar layout once
    let Some(layout) = word_layout_from_type(tree, ty) else {
        return Err(Error::TypeMismatch {
            expected: "scalar or reference raw store".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    // encode into memory bits
    let raw = layout.encode(value);
    let byte_len = layout.byte_len(tree.pointer_bytes() as usize);

    Ok((raw, byte_len))
}

/// Encode one VM word into scalar bytes.
pub(crate) fn encode_word_bytes(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<WordEncoding, Error> {
    let (raw, byte_len) = encode_word_bits(tree, ty, value)?;

    Ok(WordEncoding {
        bytes: raw.to_le_bytes(),
        len: byte_len,
    })
}

/// Return the runtime value layout for a MIR type.
pub(crate) fn value_layout_from_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> ValueLayout {
    // map mir type to value layout
    match tree.get(ty) {
        mir::Type::Void => ValueLayout::Void,
        mir::Type::Boolean => ValueLayout::Bool,
        mir::Type::Int { width, is_signed } => ValueLayout::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Type::Isize => ValueLayout::Int {
            width: usize::BITS as u16,
            signed: true,
        },
        mir::Type::Usize => ValueLayout::Int {
            width: usize::BITS as u16,
            signed: false,
        },
        mir::Type::Float(float_type) => ValueLayout::Float {
            width: float_type.width(),
        },
        mir::Type::TypeDescriptor | mir::Type::TypeId => ValueLayout::Int {
            width: usize::BITS as u16,
            signed: false,
        },
        mir::Type::Reference {
            kind,
            space,
            access,
            pointee,
            nullability,
            ..
        } => match pointee.ty() {
            Some(pointee) => ValueLayout::Pointer {
                pointee,
                pointer_class: pointer_class_from_reference(space.clone(), *kind),
                reference: ReferenceMeta::new(*kind, space.clone(), *access, *nullability),
            },
            None => ValueLayout::Unknown,
        },
        mir::Type::FunctionSignature { result, .. } => match result.ty() {
            Some(result) => ValueLayout::FunctionPointer { result },
            None => ValueLayout::Unknown,
        },
        mir::Type::FunctionPointer { signature } => match signature.ty() {
            Some(signature) => match tree.get(signature) {
                mir::Type::FunctionSignature { result, .. } => match result.ty() {
                    Some(result) => ValueLayout::FunctionPointer { result },
                    None => ValueLayout::Unknown,
                },
                _ => ValueLayout::Unknown,
            },
            None => ValueLayout::Unknown,
        },
        mir::Type::Array {
            element,
            length,
            copy: _,
        } => match element.ty() {
            Some(element) => ValueLayout::Array {
                element,
                length: *length,
            },
            None => ValueLayout::Unknown,
        },
        mir::Type::Slice { .. } => ValueLayout::FrameBytes { ty },
        mir::Type::Any { .. } => ValueLayout::FrameBytes { ty },
        mir::Type::Atomic { value } => match value.ty() {
            Some(value) => value_layout_from_type(tree, value),
            None => ValueLayout::Unknown,
        },
        mir::Type::Newtype { inner, .. } => match inner.ty() {
            Some(inner) => value_layout_from_type(tree, inner),
            None => ValueLayout::Unknown,
        },
        mir::Type::Closure { .. } => ValueLayout::Callable { ty },
        mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Variant { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => ValueLayout::FrameBytes { ty },
        mir::Type::TensorView { .. } => ValueLayout::FrameBytes { ty },
    }
}

/// Return the native word layout for a MIR type.
pub(crate) fn word_layout_from_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<WordLayout> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Void => Some(WordLayout::Void),
        mir::Type::Boolean => Some(WordLayout::Bool),
        mir::Type::Int { width, is_signed } => {
            let width = u8::try_from(*width).ok()?;

            if *is_signed {
                Some(WordLayout::Int { width })
            } else {
                Some(WordLayout::Uint { width })
            }
        }
        mir::Type::Isize => Some(WordLayout::Int {
            width: usize::BITS as u8,
        }),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            Some(WordLayout::Uint {
                width: usize::BITS as u8,
            })
        }
        mir::Type::Float(mir::FloatType::Float32) => Some(WordLayout::Float32),
        mir::Type::Float(mir::FloatType::Float64) => Some(WordLayout::Float64),
        mir::Type::Reference { kind, space, .. } => {
            word_layout_from_pointer_class(pointer_class_from_reference(space.clone(), *kind))
        }
        mir::Type::Closure { .. } => Some(WordLayout::HeapReference),
        mir::Type::FunctionSignature { .. } | mir::Type::FunctionPointer { .. } => {
            Some(WordLayout::FunctionPointer)
        }
        mir::Type::Atomic { value } => word_layout_from_type(tree, value.ty()?),
        _ => None,
    }
}

/// Return the scalar layout for one MIR type.
pub(crate) fn scalar_layout_from_type(
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
            width: float_type.width(),
        }),
        mir::Type::Boolean => Some(ScalarLayout::Bool),
        _ => None,
    }
}

/// Map a reference kind to one runtime pointer class.
pub(crate) fn pointer_class_from_reference(
    space: mir::Space,
    kind: mir::ReferenceKind,
) -> PointerClass {
    match space {
        mir::Space::Local => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => PointerClass::Heap,
            mir::ReferenceKind::Borrowed => PointerClass::HeapAddress,
            mir::ReferenceKind::Raw => PointerClass::Raw,
        },
        mir::Space::Shared => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Unique => PointerClass::SharedHeap,
            mir::ReferenceKind::Borrowed => PointerClass::SharedHeapAddress,
            mir::ReferenceKind::Raw => PointerClass::SharedRaw,
        },
        mir::Space::Frame => PointerClass::Frame,
        mir::Space::Static => PointerClass::Static,
    }
}

/// Map one pointer class to the word layout carried by memory.
pub(crate) fn word_layout_from_pointer_class(pointer_class: PointerClass) -> Option<WordLayout> {
    Some(match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => WordLayout::HeapReference,
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            WordLayout::SharedHeapReference
        }
        PointerClass::Raw => WordLayout::RawPointer,
        PointerClass::SharedRaw => WordLayout::SharedRawPointer,
        PointerClass::Stack => WordLayout::StackPointer,
        PointerClass::Frame => WordLayout::FramePointer,
        PointerClass::Static => WordLayout::StaticPointer,
        PointerClass::Unknown => return None,
    })
}
