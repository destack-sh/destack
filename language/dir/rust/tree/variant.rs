use crate::{StringId, Type};

use super::NodeId;

/// A Variant is structured type.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Struct type.
    Struct,
    /// Tuple type.
    Tuple,
	/// Unit / void type.
    Unit,
}

/// A VariantField is a field of a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field.
    Named {
        name: StringId,
        r#type: NodeId<Type>,
    },
    /// Positional field.
    Positional { r#type: NodeId<Type> },
}
