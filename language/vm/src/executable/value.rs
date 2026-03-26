use destack_heap::ReferenceMeta;
use {destack_engine as engine, destack_mir as mir};

/// Storage class for pointer-like values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PointerStorage {
    /// Managed heap reference.
    Managed,
    /// Raw heap pointer.
    Raw,
    /// Stack pointer.
    Stack,
    /// Local pointer.
    Local,
    /// Global pointer.
    Global,
    /// Unknown pointer storage.
    Unknown,
}

/// Scalar and aggregate kinds used for typed dispatch selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueKind {
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
        storage: PointerStorage,
        reference: ReferenceMeta,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: mir::LocalNodeId<mir::Type> },
    /// Heap aggregate value with concrete type.
    Aggregate { ty: mir::LocalNodeId<mir::Type> },
    /// Managed array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
    },
    /// Unknown or unsupported type.
    Unknown,
}

/// Get the kind for a MIR type.
pub(super) fn kind_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> ValueKind {
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
        } => ValueKind::Pointer {
            pointee: *pointee,
            storage: pointer_storage_from_reference(*address_space, *kind),
            reference: ReferenceMeta::new(*kind, *address_space, *mutability, *is_nullable),
        },
        mir::Type::FunctionPointer { result, .. } => ValueKind::FunctionPointer { result: *result },
        mir::Type::Array {
            element,
            length,
            copyability: _,
        } => ValueKind::Array {
            element: *element,
            length: *length,
        },
        mir::Type::Newtype { inner, .. } => kind_from_type(tree, *inner),
        mir::Type::FunctionValue { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => ValueKind::Aggregate { ty },
        mir::Type::TensorReference {
            kind,
            address_space,
            mutability,
            element,
            is_nullable,
            ..
        } => ValueKind::Pointer {
            pointee: *element,
            storage: pointer_storage_from_reference(*address_space, *kind),
            reference: ReferenceMeta::new(*kind, *address_space, *mutability, *is_nullable),
        },
    }
}

/// Classify the runtime payload stored in one frame slot of this type.
pub(super) fn frame_slot_value_class_from_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> engine::FrameSlotValueClass {
    match kind_from_type(tree, ty) {
        ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        } => engine::FrameSlotValueClass::ManagedReference,
        ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        } => engine::FrameSlotValueClass::RawPointer,
        ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        } => engine::FrameSlotValueClass::StackPointer,
        ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        } => engine::FrameSlotValueClass::LocalPointer,
        ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        } => engine::FrameSlotValueClass::GlobalPointer,
        ValueKind::Pointer {
            storage: PointerStorage::Unknown,
            ..
        } => engine::FrameSlotValueClass::UnknownPointer,
        ValueKind::FunctionPointer { .. } => engine::FrameSlotValueClass::Function,
        ValueKind::Aggregate { .. } | ValueKind::Array { .. } => {
            engine::FrameSlotValueClass::ManagedObject
        }
        _ => engine::FrameSlotValueClass::Plain,
    }
}

/// Map a reference kind to a pointer storage class.
pub(super) fn pointer_storage_from_reference(
    address_space: mir::AddressSpace,
    kind: mir::ReferenceKind,
) -> PointerStorage {
    match address_space {
        mir::AddressSpace::Stack => PointerStorage::Stack,
        mir::AddressSpace::Global | mir::AddressSpace::Constant => PointerStorage::Global,
        mir::AddressSpace::Shared | mir::AddressSpace::Local | mir::AddressSpace::Target(_) => {
            PointerStorage::Unknown
        }
        mir::AddressSpace::Generic => match kind {
            mir::ReferenceKind::Managed => PointerStorage::Managed,
            mir::ReferenceKind::Owned | mir::ReferenceKind::Raw => PointerStorage::Raw,
            mir::ReferenceKind::Borrowed => PointerStorage::Unknown,
        },
    }
}
