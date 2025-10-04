use crate::{Node, NodeId, NodeType, StringId, Type};

/// A Variant is structured type.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Struct type.
    Struct {
        name: Option<StringId>,
        fields: Vec<NodeId<VariantField>>,
    },
    /// Tuple type.
    Tuple {
        name: Option<StringId>,
        fields: Vec<NodeId<VariantField>>,
    },
    /// Unit / void type.
    Unit { name: Option<StringId> },
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
