use crate::{
    FramePointer, FunctionPointer, HeapReference, RawPointer, ReferenceMeta, SharedHeapReference,
    SharedRawPointer, StackPointer, StaticPointer, Word,
};
use destack_mir as mir;

use super::repr_type;

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

/// Runtime value representation used for dispatch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueRepr {
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

/// Native representation for one word load or store.
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
}

/// Get the runtime representation for a MIR type.
pub(crate) fn value_repr_from_type(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> ValueRepr {
    // map mir type to value representation
    match tree.get(ty) {
        mir::Type::Void => ValueRepr::Void,
        mir::Type::Boolean => ValueRepr::Bool,
        mir::Type::Int { width, is_signed } => ValueRepr::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Type::Isize => ValueRepr::Int {
            width: usize::BITS as u16,
            signed: true,
        },
        mir::Type::Usize => ValueRepr::Int {
            width: usize::BITS as u16,
            signed: false,
        },
        mir::Type::Float { width } => ValueRepr::Float { width: *width },
        mir::Type::TypeDescriptor | mir::Type::TypeId => ValueRepr::Int {
            width: usize::BITS as u16,
            signed: false,
        },
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            pointee,
            is_nullable,
        } => match pointee.ty() {
            Some(pointee) => ValueRepr::Pointer {
                pointee,
                pointer_class: pointer_class_from_reference(address_space.clone(), *kind),
                reference: ReferenceMeta::new(
                    *kind,
                    address_space.clone(),
                    *mutability,
                    *is_nullable,
                ),
            },
            None => ValueRepr::Unknown,
        },
        mir::Type::FunctionSignature { result, .. } => match result.ty() {
            Some(result) => ValueRepr::FunctionPointer { result },
            None => ValueRepr::Unknown,
        },
        mir::Type::FunctionPointer { signature } => match signature.ty() {
            Some(signature) => match tree.get(signature) {
                mir::Type::FunctionSignature { result, .. } => match result.ty() {
                    Some(result) => ValueRepr::FunctionPointer { result },
                    None => ValueRepr::Unknown,
                },
                _ => ValueRepr::Unknown,
            },
            None => ValueRepr::Unknown,
        },
        mir::Type::Array {
            element,
            length,
            copy: _,
        } => match element.ty() {
            Some(element) => ValueRepr::Array {
                element,
                length: *length,
            },
            None => ValueRepr::Unknown,
        },
        mir::Type::Slice { .. } => ValueRepr::FrameBytes { ty },
        mir::Type::Newtype { inner, .. } => match inner.ty() {
            Some(inner) => value_repr_from_type(tree, inner),
            None => ValueRepr::Unknown,
        },
        mir::Type::Callable { .. } => ValueRepr::Callable { ty },
        mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => ValueRepr::FrameBytes { ty },
        mir::Type::TensorView {
            kind,
            address_space,
            mutability,
            element,
            is_nullable,
            ..
        } => match element.ty() {
            Some(element) => ValueRepr::Pointer {
                pointee: element,
                pointer_class: pointer_class_from_reference(address_space.clone(), *kind),
                reference: ReferenceMeta::new(
                    *kind,
                    address_space.clone(),
                    *mutability,
                    *is_nullable,
                ),
            },
            None => ValueRepr::Unknown,
        },
    }
}

/// Get the native word representation for a MIR type.
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
        mir::Type::Float { width: 32 } => Some(WordLayout::Float32),
        mir::Type::Float { width: 64 } => Some(WordLayout::Float64),
        mir::Type::Reference {
            kind,
            address_space,
            ..
        } => word_layout_from_pointer_class(pointer_class_from_reference(
            address_space.clone(),
            *kind,
        )),
        mir::Type::Callable { .. } => Some(WordLayout::HeapReference),
        mir::Type::FunctionSignature { .. } | mir::Type::FunctionPointer { .. } => {
            Some(WordLayout::FunctionPointer)
        }
        _ => None,
    }
}

/// Map a reference kind to one runtime pointer class.
pub(crate) fn pointer_class_from_reference(
    address_space: mir::AddressSpace,
    kind: mir::ReferenceKind,
) -> PointerClass {
    match address_space {
        mir::AddressSpace::Local => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Owned => PointerClass::Heap,
            mir::ReferenceKind::Borrowed => PointerClass::HeapAddress,
            mir::ReferenceKind::Raw => PointerClass::Raw,
        },
        mir::AddressSpace::Shared => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Owned => PointerClass::SharedHeap,
            mir::ReferenceKind::Borrowed => PointerClass::SharedHeapAddress,
            mir::ReferenceKind::Raw => PointerClass::SharedRaw,
        },
        mir::AddressSpace::Stack => PointerClass::Stack,
        mir::AddressSpace::Frame => PointerClass::Frame,
        mir::AddressSpace::Static => PointerClass::Static,
        mir::AddressSpace::Named(_) => PointerClass::Unknown,
    }
}

/// Map one pointer class to the word representation carried by memory.
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
