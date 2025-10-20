use dyst_source::StringId;

use crate::{Expression, Mutability, Node, NodeId, NodeType, Visibility};

/// The style of a variant (tuple or struct).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VariantStyle {
    /// A tuple struct with explicit representation.
    Tuple,
    /// A struct with explicit representation.
    Struct,
}

/// A VariantField is a field declaration.
///
/// Examples:
/// ```
/// bar: int32
/// baz: T
/// public T
/// readonly bar: int32
/// baz?: T // shorthand for baz: T?
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VariantField {
    /// The mutability of the field.
    pub mutability: Option<Mutability>,
    /// The visibility of the field.
    pub visibility: Option<Visibility>,
    /// The name of the field (may be unset for tuple fields).
    pub name: Option<StringId>,
    /// The type of the field.
    pub ty: NodeId<Expression>,
    /// The default value of the field.
    pub default: Option<NodeId<Expression>>,
}

// TODO #Incomplete: getter/setter functions for Struct/Union/Interface/...Fields?
//  (useful for SOA-style struct views?)
//  (how does this interact with interfaces and unions?)
//  (how does this relate with Entities?)
//  (how does this relate to $ dynamicness?)

impl Node for VariantField {
    const KIND: NodeType = NodeType::VariantField;
}
