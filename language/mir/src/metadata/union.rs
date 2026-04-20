use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, LocalNodeId, Type};

/// Payload storage strategy for a union layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnionPayloadKind {
    /// Store the payload inline inside the union struct.
    Inline,
    /// Store the payload as a managed box.
    Boxed,
}

/// Canonical discriminant values for tagged unions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnionDiscriminantValue {
    /// Null literal value.
    Null,
    /// Undefined literal value.
    Undefined,
    /// Boolean literal value.
    Boolean(bool),
    /// Number literal value stored as f64 bits.
    Number { bits: u64 },
    /// Bigint literal value.
    Bigint(i64),
    /// String literal value.
    String(StringId),
    /// Unique symbol literal value.
    UniqueSymbol,
}

/// Discriminant values for a field in tag order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionDiscriminantField {
    /// The discriminant field for each union element in tag order.
    pub field_by_element: Vec<LocalNodeId<Field>>,
    /// The field name shared by all union variants.
    pub field_name: StringId,
    /// Literal values ordered by tag value.
    pub values: Vec<UnionDiscriminantValue>,
}

/// Discriminant metadata for a tagged union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionDiscriminant {
    /// The primary discriminant field index in `fields`.
    pub primary_field_index: u32,
    /// Discriminant fields indexed by field name.
    pub fields: Vec<UnionDiscriminantField>,
}

/// Layout metadata for a lowered union type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionLayout {
    /// The tag field type.
    pub tag_type: LocalNodeId<Type>,
    /// The payload field type.
    pub payload_type: LocalNodeId<Type>,
    /// The payload storage strategy.
    pub payload_kind: UnionPayloadKind,
    /// The union element type ids in tag order.
    pub element_types: Vec<LocalNodeId<Type>>,
    /// The tag field name in the lowered layout.
    pub tag_field_name: StringId,
    /// The payload field name in the lowered layout.
    pub payload_field_name: StringId,
    /// Discriminant field metadata when present.
    pub discriminant: Option<UnionDiscriminant>,
}
