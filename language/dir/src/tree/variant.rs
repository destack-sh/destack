use crate::{Expression, Mutability, Node, NodeId, NodeType, StringId, Type, Visibility};

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingKind {
    /// Definite binding (like `x: int32`).
    Must,
    /// Maybe binding (like `x?: int32` or just `T?`).
    Maybe,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind>,
    /// The mutability of the field.
    pub mutability: Option<Mutability>,
    /// The visibility of the field.
    pub visibility: Option<Visibility>,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator>,
}

/// A Field is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Named field (like `x: int32`).
    Named {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The name of the field.
        name: StringId,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Positional field (like `4`).
    Positional {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field (like `[x: string]: any`).
    Dynamic {
        /// The modifiers of the field.
        modifiers: Option<BindingModifier>,
        /// The name of the field (if any).
        name: Option<StringId>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The key type of the field.
        key: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

impl Field {
    /// Get the modifiers of the field.
    #[inline]
    pub fn modifiers(&self) -> Option<BindingModifier> {
        match self {
            Field::Named { modifiers, .. } => *modifiers,
            Field::Positional { modifiers, .. } => *modifiers,
            Field::Dynamic { modifiers, .. } => *modifiers,
        }
    }

    /// Get the name of the field.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Field::Named { name, .. } => Some(*name),
            Field::Positional { .. } => None,
            Field::Dynamic { name, .. } => *name,
        }
    }

    /// Get the type of the field.
    #[inline]
    pub fn ty(&self) -> NodeId<Type> {
        match self {
            Field::Named { ty, .. } => *ty,
            Field::Positional { ty, .. } => *ty,
            Field::Dynamic { ty, .. } => *ty,
        }
    }

    /// Get the default value of the field.
    #[inline]
    pub fn default(&self) -> Option<NodeId<Expression>> {
        match self {
            Field::Named { default, .. } => *default,
            Field::Positional { default, .. } => *default,
            Field::Dynamic { default, .. } => *default,
        }
    }
}

/// A Variant is a concrete structured data type.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Struct type.
    Struct {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Fields of the variant.
        fields: Vec<NodeId<Field>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
    /// Tuple type.
    Tuple {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Fields of the variant.
        fields: Vec<NodeId<Field>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
    /// Unit / "void" type.
    Unit {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        ty: Option<NodeId<Type>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
}

impl Node for Variant {
    const TYPE: NodeType = NodeType::Variant;
}

impl Variant {
    /// Get the name of the variant.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            Variant::Struct { name, .. } => *name,
            Variant::Tuple { name, .. } => *name,
            Variant::Unit { name, .. } => *name,
        }
    }

    /// Get the representation type of the variant.
    #[inline]
    pub fn ty(&self) -> Option<NodeId<Type>> {
        match self {
            Variant::Struct { ty, .. } => *ty,
            Variant::Tuple { ty, .. } => *ty,
            Variant::Unit { ty, .. } => *ty,
        }
    }
}
