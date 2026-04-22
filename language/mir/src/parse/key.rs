use destack_core::StringId;

use crate::{
    AddressSpace, Attribute, Copyability, Field, LocalNodeId, Mutability, ReferenceKind,
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
    Float { width: u16 },
    /// Runtime type descriptor.
    TypeDescriptor,
    /// Runtime type id.
    TypeId,
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
        copyability: Copyability,
    },
    /// Dynamic array.
    DynamicArray {
        element: TypeReference,
        copyability: Copyability,
    },
    /// Tuple of heterogeneous elements.
    Tuple {
        elements: Vec<TypeReference>,
        copyability: Copyability,
    },
    /// Struct with named or positional fields.
    Struct {
        fields: Vec<LocalNodeId<Field>>,
        copyability: Copyability,
    },
    /// Nominal newtype wrapper.
    Newtype {
        inner: TypeReference,
        copyability: Copyability,
    },
    /// Fixed-width SIMD vector.
    Vector {
        element: TypeReference,
        lanes: u32,
        copyability: Copyability,
    },
    /// Tensor value type.
    Tensor {
        element: TypeReference,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        copyability: Copyability,
    },
    /// Tensor view type.
    TensorReference {
        kind: ReferenceKind,
        address_space: AddressSpace,
        mutability: Mutability,
        element: TypeReference,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        is_nullable: bool,
    },
    /// Function pointer signature.
    FunctionPointer {
        parameters: Vec<TypeReference>,
        result: TypeReference,
    },
    /// Callable closure value.
    Closure { signature: TypeReference },
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
            Type::Float { width } => TypeKey::Float { width: *width },
            Type::TypeDescriptor => TypeKey::TypeDescriptor,
            Type::TypeId => TypeKey::TypeId,

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
                copyability,
            } => TypeKey::Array {
                element: *element,
                length: *length,
                copyability: *copyability,
            },
            Type::DynamicArray {
                element,
                copyability,
            } => TypeKey::DynamicArray {
                element: *element,
                copyability: *copyability,
            },

            Type::Tuple {
                elements,
                copyability,
            } => TypeKey::Tuple {
                elements: elements.clone(),
                copyability: *copyability,
            },

            Type::Struct {
                fields,
                copyability,
            } => TypeKey::Struct {
                fields: fields.clone(),
                copyability: *copyability,
            },
            Type::Newtype { inner, copyability } => TypeKey::Newtype {
                inner: *inner,
                copyability: *copyability,
            },
            Type::Vector {
                element,
                lanes,
                copyability,
            } => TypeKey::Vector {
                element: *element,
                lanes: *lanes,
                copyability: *copyability,
            },
            Type::Tensor {
                element,
                shape,
                layout,
                copyability,
            } => TypeKey::Tensor {
                element: *element,
                shape: shape.clone(),
                layout: layout.clone(),
                copyability: *copyability,
            },
            Type::TensorReference {
                kind,
                address_space,
                mutability,
                element,
                shape,
                layout,
                is_nullable,
            } => TypeKey::TensorReference {
                kind: *kind,
                address_space: address_space.clone(),
                mutability: *mutability,
                element: *element,
                shape: shape.clone(),
                layout: layout.clone(),
                is_nullable: *is_nullable,
            },

            Type::FunctionPointer { parameters, result } => TypeKey::FunctionPointer {
                parameters: parameters.clone(),
                result: *result,
            },
            Type::Closure { signature } => TypeKey::Closure {
                signature: *signature,
            },
        }
    }
}
