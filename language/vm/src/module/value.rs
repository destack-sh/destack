use crate::ReferenceMeta;
use destack_mir as mir;

/// Runtime class for pointer-like values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerClass {
    /// Local heap reference.
    Heap,
    /// Shared heap reference.
    SharedHeap,
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

/// Scalar and aggregate kinds used for typed dispatch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueKind {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed or unsigned integer with width.
    Int { width: u8, signed: bool },
    /// Floating point value with width.
    Float { width: u8 },
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
    /// Aggregate value with concrete type.
    Aggregate { ty: mir::LocalNodeId<mir::Type> },
    /// Fixed-size array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
    },
    /// Unknown or unsupported type.
    Unknown,
}

/// Get the kind for a MIR type.
pub(crate) fn kind_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> ValueKind {
    // map mir type to value kind
    match tree.get(ty) {
        mir::Type::Void => ValueKind::Void,
        mir::Type::Boolean => ValueKind::Bool,
        mir::Type::Int { width, is_signed } => ValueKind::Int {
            width: *width as u8,
            signed: *is_signed,
        },
        mir::Type::Isize => ValueKind::Int {
            width: usize::BITS as u8,
            signed: true,
        },
        mir::Type::Usize => ValueKind::Int {
            width: usize::BITS as u8,
            signed: false,
        },
        mir::Type::Float { width } => ValueKind::Float {
            width: *width as u8,
        },
        mir::Type::TypeDescriptor | mir::Type::TypeId => ValueKind::Int {
            width: usize::BITS as u8,
            signed: false,
        },
        mir::Type::Reference {
            kind,
            address_space,
            mutability,
            pointee,
            is_nullable,
        } => match pointee.ty() {
            Some(pointee) => ValueKind::Pointer {
                pointee,
                pointer_class: pointer_class_from_reference(address_space.clone(), *kind),
                reference: ReferenceMeta::new(
                    *kind,
                    address_space.clone(),
                    *mutability,
                    *is_nullable,
                ),
            },
            None => ValueKind::Unknown,
        },
        mir::Type::FunctionSignature { result, .. } => match result.ty() {
            Some(result) => ValueKind::FunctionPointer { result },
            None => ValueKind::Unknown,
        },
        mir::Type::FunctionPointer { signature } => match signature.ty() {
            Some(signature) => match tree.get(signature) {
                mir::Type::FunctionSignature { result, .. } => match result.ty() {
                    Some(result) => ValueKind::FunctionPointer { result },
                    None => ValueKind::Unknown,
                },
                _ => ValueKind::Unknown,
            },
            None => ValueKind::Unknown,
        },
        mir::Type::Array {
            element,
            length,
            copy: _,
        } => match element.ty() {
            Some(element) => ValueKind::Array {
                element,
                length: *length,
            },
            None => ValueKind::Unknown,
        },
        mir::Type::Slice { .. } => ValueKind::Aggregate { ty },
        mir::Type::Newtype { inner, .. } => match inner.ty() {
            Some(inner) => kind_from_type(tree, inner),
            None => ValueKind::Unknown,
        },
        mir::Type::Callable { .. } => ValueKind::Callable { ty },
        mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => ValueKind::Aggregate { ty },
        mir::Type::TensorView {
            kind,
            address_space,
            mutability,
            element,
            is_nullable,
            ..
        } => match element.ty() {
            Some(element) => ValueKind::Pointer {
                pointee: element,
                pointer_class: pointer_class_from_reference(address_space.clone(), *kind),
                reference: ReferenceMeta::new(
                    *kind,
                    address_space.clone(),
                    *mutability,
                    *is_nullable,
                ),
            },
            None => ValueKind::Unknown,
        },
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
            mir::ReferenceKind::Borrowed => PointerClass::Unknown,
            mir::ReferenceKind::Raw => PointerClass::Raw,
        },
        mir::AddressSpace::Shared => match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Owned => PointerClass::SharedHeap,
            mir::ReferenceKind::Borrowed => PointerClass::Unknown,
            mir::ReferenceKind::Raw => PointerClass::SharedRaw,
        },
        mir::AddressSpace::Stack => PointerClass::Stack,
        mir::AddressSpace::Frame => PointerClass::Frame,
        mir::AddressSpace::Static => PointerClass::Static,
        mir::AddressSpace::Named(_) => PointerClass::Unknown,
    }
}
