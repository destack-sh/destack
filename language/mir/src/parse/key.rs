use destack_core::StringId;

use crate::{
    AddressSpace, Attribute, Copy, Field, FloatType, LocalNodeId, Mutability, ReferenceKind,
    TensorDimension, TensorLayout, Type, TypeReference,
};

/// Interning key for struct fields.
/// Captures the structural identity of a field for deduplication during parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FieldKey {
    /// Optional field name.
    name: Option<StringId>,
    /// Field type.
    ty: TypeReference,
    /// Attributes attached to the field.
    attributes: Vec<Attribute>,
}

impl FieldKey {
    /// Create a key from a field definition.
    pub(super) fn from_field(field: &Field, attributes: &[Attribute]) -> Self {
        Self {
            name: field.name,
            ty: field.ty,
            attributes: attributes.to_vec(),
        }
    }
}

/// Interning key for MIR types.
/// Captures the structural identity of a type for deduplication during parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TypeKey {
    /// Unit type.
    Void,
    /// Boolean type.
    Boolean,
    /// Fixed-width integer.
    Int { width: u16, signed: bool },
    /// Pointer-sized signed integer.
    Isize,
    /// Pointer-sized unsigned integer.
    Usize,
    /// Floating point.
    Float(FloatType),
    /// Runtime type descriptor.
    TypeDescriptor,
    /// Runtime type id.
    TypeId,
    /// Atomic storage cell type.
    Atomic { value: TypeReference },
    /// Reference/pointer type.
    Reference {
        kind: ReferenceKind,
        address_space: AddressSpace,
        mutability: Mutability,
        pointee: TypeReference,
        is_nullable: bool,
    },
    /// Fixed-length array.
    Array {
        element: TypeReference,
        length: u64,
        copy: Copy,
    },
    /// Slice view.
    Slice {
        kind: ReferenceKind,
        element: TypeReference,
        address_space: AddressSpace,
        mutability: Mutability,
    },
    /// Tuple of heterogeneous elements.
    Tuple {
        elements: Vec<TypeReference>,
        copy: Copy,
    },
    /// Struct with named or positional fields.
    Struct {
        fields: Vec<LocalNodeId<Field>>,
        copy: Copy,
    },
    /// Nominal newtype wrapper.
    Newtype { inner: TypeReference, copy: Copy },
    /// Fixed-width SIMD vector.
    Vector {
        element: TypeReference,
        lanes: u32,
        copy: Copy,
    },
    /// Tensor value type.
    Tensor {
        element: TypeReference,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        copy: Copy,
    },
    /// Tensor view type.
    TensorView {
        kind: ReferenceKind,
        address_space: AddressSpace,
        mutability: Mutability,
        element: TypeReference,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        is_nullable: bool,
    },
    /// Bare function signature.
    FunctionSignature {
        parameters: Vec<TypeReference>,
        result: TypeReference,
    },
    /// Function pointer type.
    FunctionPointer { signature: TypeReference },
    /// Opaque callable value.
    Callable { signature: TypeReference },
}

impl TypeKey {
    /// Create a key from a type definition.
    pub(super) fn from_type(ty: &Type) -> Self {
        match ty {
            Type::Void => TypeKey::Void,
            Type::Boolean => TypeKey::Boolean,
            Type::Int { width, is_signed } => TypeKey::Int {
                width: *width,
                signed: *is_signed,
            },
            Type::Isize => TypeKey::Isize,
            Type::Usize => TypeKey::Usize,
            Type::Float(float_type) => TypeKey::Float(*float_type),
            Type::TypeDescriptor => TypeKey::TypeDescriptor,
            Type::TypeId => TypeKey::TypeId,
            Type::Atomic { value } => TypeKey::Atomic { value: *value },

            Type::Reference {
                kind,
                address_space,
                mutability,
                pointee,
                is_nullable,
            } => TypeKey::Reference {
                kind: *kind,
                address_space: address_space.clone(),
                mutability: *mutability,
                pointee: *pointee,
                is_nullable: *is_nullable,
            },

            Type::Array {
                element,
                length,
                copy,
            } => TypeKey::Array {
                element: *element,
                length: *length,
                copy: *copy,
            },
            Type::Slice {
                kind,
                element,
                address_space,
                mutability,
            } => TypeKey::Slice {
                kind: *kind,
                element: *element,
                address_space: address_space.clone(),
                mutability: *mutability,
            },

            Type::Tuple { elements, copy } => TypeKey::Tuple {
                elements: elements.clone(),
                copy: *copy,
            },

            Type::Struct { fields, copy } => TypeKey::Struct {
                fields: fields.clone(),
                copy: *copy,
            },
            Type::Newtype { inner, copy } => TypeKey::Newtype {
                inner: *inner,
                copy: *copy,
            },
            Type::Vector {
                element,
                lanes,
                copy,
            } => TypeKey::Vector {
                element: *element,
                lanes: *lanes,
                copy: *copy,
            },
            Type::Tensor {
                element,
                shape,
                layout,
                copy,
            } => TypeKey::Tensor {
                element: *element,
                shape: shape.clone(),
                layout: layout.clone(),
                copy: *copy,
            },
            Type::TensorView {
                kind,
                address_space,
                mutability,
                element,
                shape,
                layout,
                is_nullable,
            } => TypeKey::TensorView {
                kind: *kind,
                address_space: address_space.clone(),
                mutability: *mutability,
                element: *element,
                shape: shape.clone(),
                layout: layout.clone(),
                is_nullable: *is_nullable,
            },

            Type::FunctionSignature { parameters, result } => TypeKey::FunctionSignature {
                parameters: parameters.clone(),
                result: *result,
            },
            Type::FunctionPointer { signature } => TypeKey::FunctionPointer {
                signature: *signature,
            },
            Type::Callable { signature } => TypeKey::Callable {
                signature: *signature,
            },
        }
    }
}
