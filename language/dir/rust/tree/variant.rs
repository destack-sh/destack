use crate::{Expression, Mutability, Node, NodeId, NodeType, StringId, Type, Visibility};

/// A VariantField is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field ().
    Named {
        /// The visibility of the field.
        visibility: Option<Visibility>,
        /// The mutability of the field.
        mutability: Option<Mutability>,
        /// The name of the field.
        name: StringId,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Positional field.
    Positional {
        /// The visibility of the field.
        visibility: Option<Visibility>,
        /// The mutability of the field.
        mutability: Option<Mutability>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field.
    Dynamic {
        /// The visibility of the field.
        visibility: Option<Visibility>,
        /// The mutability of the field.
        mutability: Option<Mutability>,
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

impl Node for VariantField {
    const KIND: NodeType = NodeType::VariantField;
}

impl VariantField {
    /// Get the visibility of the field.
    #[inline]
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            VariantField::Named { visibility, .. } => *visibility,
            VariantField::Positional { visibility, .. } => *visibility,
            VariantField::Dynamic { visibility, .. } => *visibility,
        }
    }

    /// Get the mutability of the field.
    #[inline]
    pub fn mutability(&self) -> Option<Mutability> {
        match self {
            VariantField::Named { mutability, .. } => *mutability,
            VariantField::Positional { mutability, .. } => *mutability,
            VariantField::Dynamic { mutability, .. } => *mutability,
        }
    }

    /// Get the name of the field.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            VariantField::Named { name, .. } => Some(*name),
            VariantField::Positional { .. } => None,
            VariantField::Dynamic { name, .. } => *name,
        }
    }

    /// Get the type of the field.
    #[inline]
    pub fn ty(&self) -> NodeId<Type> {
        match self {
            VariantField::Named { ty, .. } => *ty,
            VariantField::Positional { ty, .. } => *ty,
            VariantField::Dynamic { ty, .. } => *ty,
        }
    }

    /// Get the default value of the field.
    #[inline]
    pub fn default(&self) -> Option<NodeId<Expression>> {
        match self {
            VariantField::Named { default, .. } => *default,
            VariantField::Positional { default, .. } => *default,
            VariantField::Dynamic { default, .. } => *default,
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
        fields: Vec<NodeId<VariantField>>,
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
        fields: Vec<NodeId<VariantField>>,
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
    const KIND: NodeType = NodeType::Variant;
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
