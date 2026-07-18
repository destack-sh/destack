use destack_core::StringId;

use crate::{
    Access, Attribute, Copy, Field, FloatType, Lifetime, LocalNodeId, Nullability, ReferenceKind,
    SignatureParameter, Space, TensorDimension, TensorFormat, TensorSharding, TensorViewFormat,
    Type, TypeId, VariantCase,
};

/// Interning key for struct fields.
/// Captures the structural identity of a field for deduplication during parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FieldKey {
    /// Optional field name.
    name: Option<StringId>,
    /// Field type.
    ty: TypeId,
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
    /// Invalid recovered type.
    Error,
    /// Unit type.
    Void,
    /// Boolean type.
    Boolean,
    /// Fixed-width integer.
    Int {
        width: u16,
        signed: bool,
    },
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
    Atomic {
        value: TypeId,
    },
    /// Runtime-erased dynamic value.
    Dynamic {
        constraint: TypeId,
    },
    /// Type use with applied lifetime arguments.
    WithLifetimes {
        base: TypeId,
        lifetimes: Vec<Lifetime>,
    },
    /// Linear uninitialized allocation token.
    Uninit {
        value: TypeId,
    },
    ManuallyDrop {
        value: TypeId,
    },
    /// Reference/pointer type.
    Reference {
        kind: ReferenceKind,
        lifetime: Lifetime,
        space: Space,
        access: Access,
        pointee: TypeId,
        nullability: Nullability,
    },
    /// Fixed-length array.
    FixedArray {
        element: TypeId,
        length: u64,
        copy: Copy,
    },
    /// Slice view.
    Slice {
        kind: ReferenceKind,
        lifetime: Lifetime,
        element: TypeId,
        space: Space,
        access: Access,
        nullability: Nullability,
    },
    /// Tuple of heterogeneous elements.
    Tuple {
        elements: Vec<TypeId>,
        copy: Copy,
    },
    /// Struct with named or positional fields.
    Struct {
        fields: Vec<LocalNodeId<Field>>,
        copy: Copy,
    },
    /// Nominal newtype wrapper.
    Newtype {
        inner: TypeId,
        copy: Copy,
    },
    /// Sum value.
    Variant {
        discriminant: TypeId,
        storage: TypeId,
        cases: Vec<VariantCase>,
        copy: Copy,
    },
    /// Fixed-width vector value.
    Vector {
        element: TypeId,
        lanes: u32,
        copy: Copy,
    },
    /// Tensor value type.
    Tensor {
        element: TypeId,
        shape: Vec<TensorDimension>,
        format: TensorFormat,
        sharding: TensorSharding,
        copy: Copy,
    },
    /// Tensor view type.
    TensorView {
        kind: ReferenceKind,
        lifetime: Lifetime,
        space: Space,
        access: Access,
        element: TypeId,
        shape: Vec<TensorDimension>,
        format: TensorViewFormat,
        sharding: TensorSharding,
        nullability: Nullability,
    },
    /// Bare function signature.
    FunctionSignature {
        lifetimes: Vec<crate::LifetimeParameter>,
        parameters: Vec<SignatureParameter>,
        result: TypeId,
    },
    /// Function pointer type.
    FunctionPointer {
        signature: TypeId,
    },
    /// Function value.
    Function {
        signature: TypeId,
        environment: TypeId,
    },
}

impl TypeKey {
    /// Create a key from a type definition.
    pub(super) fn from_type(ty: &Type) -> Self {
        match ty {
            Type::Error => TypeKey::Error,
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
            Type::Dynamic { constraint } => TypeKey::Dynamic {
                constraint: *constraint,
            },
            Type::WithLifetimes { base, lifetimes } => TypeKey::WithLifetimes {
                base: *base,
                lifetimes: lifetimes.clone(),
            },
            Type::Uninit { value } => TypeKey::Uninit { value: *value },
            Type::ManuallyDrop { value } => TypeKey::ManuallyDrop { value: *value },

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
                space: *space,
                access: *access,
                pointee: *pointee,
                nullability: *nullability,
            },

            Type::FixedArray {
                element,
                length,
                copy,
            } => TypeKey::FixedArray {
                element: *element,
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
                element: *element,
                space: *space,
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
                inner: *inner,
                copy: *copy,
            },
            Type::Variant {
                discriminant,
                storage,
                cases,
                copy,
            } => TypeKey::Variant {
                discriminant: *discriminant,
                storage: *storage,
                cases: cases.clone(),
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
                format,
                sharding,
                copy,
            } => TypeKey::Tensor {
                element: *element,
                shape: shape.clone(),
                format: *format,
                sharding: sharding.clone(),
                copy: *copy,
            },
            Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => TypeKey::TensorView {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: *space,
                access: *access,
                element: *element,
                shape: shape.clone(),
                format: *format,
                sharding: sharding.clone(),
                nullability: *nullability,
            },

            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } => TypeKey::FunctionSignature {
                lifetimes: lifetimes.clone(),
                parameters: parameters.clone(),
                result: *result,
            },
            Type::FunctionPointer { signature } => TypeKey::FunctionPointer {
                signature: *signature,
            },
            Type::Function {
                signature,
                environment,
            } => TypeKey::Function {
                signature: *signature,
                environment: *environment,
            },
        }
    }
}
