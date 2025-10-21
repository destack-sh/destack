use crate::{Expression, Mutability, Node, NodeId, NodeType, StringId, Type};

/// A VariantField is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field.
    Named {
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
        /// The mutability of the field.
        mutability: Option<Mutability>,
        /// The type of the field.
        ty: NodeId<Type>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
}

impl Node for VariantField {
    const KIND: NodeType = NodeType::VariantField;
}

impl VariantField {
    /// Get the name of the field.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self {
            VariantField::Named { name, .. } => Some(*name),
            VariantField::Positional { .. } => None,
        }
    }

    /// Get the default value of the field.
    #[inline]
    pub fn default(&self) -> Option<NodeId<Expression>> {
        match self {
            VariantField::Named { default, .. } => *default,
            VariantField::Positional { default, .. } => *default,
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
