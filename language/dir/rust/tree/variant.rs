use crate::{Expression, Node, NodeId, NodeType, StringId, Type};

/// A Variant is structured data type.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Struct type.
    Struct {
        /// Name of the variant.
        name: Option<StringId>,
        /// Representation type of the variant.
        representation_type: Option<NodeId<Type>>,
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
        representation_type: Option<NodeId<Type>>,
        /// Fields of the variant.
        fields: Vec<NodeId<VariantField>>,
        /// Discriminator value.
        value: Option<NodeId<Expression>>,
    },
    /// Unit / "void" type.
    Unit {
        /// Name of the variant.
        name: Option<StringId>,
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
    pub fn representation_type(&self) -> Option<NodeId<Type>> {
        match self {
            Variant::Struct {
                representation_type,
                ..
            } => *representation_type,
            Variant::Tuple {
                representation_type,
                ..
            } => *representation_type,
            Variant::Unit { .. } => None,
        }
    }
}

/// A VariantField is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field.
    Named { name: StringId, ty: NodeId<Type> },
    /// Positional field.
    Positional { ty: NodeId<Type> },
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
}
