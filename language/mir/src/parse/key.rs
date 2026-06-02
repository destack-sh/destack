use destack_core::StringId;

use crate::{
    Access, Attribute, BorrowObligation, Copy, Field, FloatType, Lifetime, LocalNodeId,
    Nullability, ReferenceKind, Space, TensorDimension, TensorLayout, TensorViewLayout, Type,
    TypeReference, VariantCase,
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
            ty: field.ty.clone(),
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
    /// Runtime-erased dynamic value.
    Dynamic { constraint: TypeReference },
    /// Linear uninitialized allocation token.
    Uninit { value: TypeReference },
    /// Reference/pointer type.
    Reference {
        kind: ReferenceKind,
        lifetime: Lifetime,
        space: Space,
        access: Access,
        pointee: TypeReference,
        nullability: Nullability,
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
        lifetime: Lifetime,
        element: TypeReference,
        space: Space,
        access: Access,
        nullability: Nullability,
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
    /// Physical tagged sum.
    Variant {
        tag: TypeReference,
        storage: TypeReference,
        cases: Vec<VariantCase>,
        copy: Copy,
    },
    /// Fixed-width vector value.
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
        lifetime: Lifetime,
        space: Space,
        access: Access,
        element: TypeReference,
        shape: Vec<TensorDimension>,
        layout: TensorViewLayout,
        nullability: Nullability,
    },
    /// Bare function signature.
    FunctionSignature {
        parameters: Vec<TypeReference>,
        result: TypeReference,
        borrow_obligations: Vec<BorrowObligation>,
    },
    /// Function pointer type.
    FunctionPointer { signature: TypeReference },
    /// Closure value.
    Closure {
        signature: TypeReference,
        environment: TypeReference,
    },
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
            Type::Atomic { value } => TypeKey::Atomic {
                value: value.clone(),
            },
            Type::Dynamic { constraint } => TypeKey::Dynamic {
                constraint: constraint.clone(),
            },
            Type::Uninit { value } => TypeKey::Uninit {
                value: value.clone(),
            },

            Type::Reference {
                kind,
                lifetime,
                space,
                access,
                pointee,
                nullability,
            } => TypeKey::Reference {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                pointee: pointee.clone(),
                nullability: *nullability,
            },

            Type::Array {
                element,
                length,
                copy,
            } => TypeKey::Array {
                element: element.clone(),
                length: *length,
                copy: *copy,
            },
            Type::Slice {
                kind,
                lifetime,
                element,
                space,
                access,
                nullability,
            } => TypeKey::Slice {
                kind: *kind,
                lifetime: lifetime.clone(),
                element: element.clone(),
                space: space.clone(),
                access: *access,
                nullability: *nullability,
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
                inner: inner.clone(),
                copy: *copy,
            },
            Type::Variant {
                tag,
                storage,
                cases,
                copy,
            } => TypeKey::Variant {
                tag: tag.clone(),
                storage: storage.clone(),
                cases: cases.clone(),
                copy: *copy,
            },
            Type::Vector {
                element,
                lanes,
                copy,
            } => TypeKey::Vector {
                element: element.clone(),
                lanes: *lanes,
                copy: *copy,
            },
            Type::Tensor {
                element,
                shape,
                layout,
                copy,
            } => TypeKey::Tensor {
                element: element.clone(),
                shape: shape.clone(),
                layout: layout.clone(),
                copy: *copy,
            },
            Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                layout,
                nullability,
            } => TypeKey::TensorView {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                element: element.clone(),
                shape: shape.clone(),
                layout: layout.clone(),
                nullability: *nullability,
            },

            Type::FunctionSignature {
                parameters,
                result,
                borrow_obligations,
            } => TypeKey::FunctionSignature {
                parameters: parameters.clone(),
                result: result.clone(),
                borrow_obligations: borrow_obligations.clone(),
            },
            Type::FunctionPointer { signature } => TypeKey::FunctionPointer {
                signature: signature.clone(),
            },
            Type::Closure {
                signature,
                environment,
            } => TypeKey::Closure {
                signature: signature.clone(),
                environment: environment.clone(),
            },
        }
    }
}
