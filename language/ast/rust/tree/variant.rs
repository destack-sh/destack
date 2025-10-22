use dyst_source::StringId;

use crate::{Expression, Mutability, Name, Node, NodeId, NodeType, Visibility};

/// The style of a variant (tuple or struct).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VariantStyle {
    /// A tuple struct with explicit representation.
    Tuple,
    /// A struct with explicit representation.
    Struct,
}

/// A VariantField is a field declaration in some type.
///
/// Examples:
/// ```
/// bar: int32
/// baz: T
/// public T
/// readonly bar: int32
/// baz?: T // shorthand for baz: T?
/// "Content-Type": string
/// [x: string]: any
/// [string]: woof
/// [T] = "hello"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum VariantField {
    /// Named field.
    Named {
        /// The visibility of the field.
        visibility: Option<Visibility>,
        /// The mutability of the field.
        mutability: Option<Mutability>,
        /// The name of the field.
        name: Name,
        /// The type of the field.
        ty: NodeId<Expression>,
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
        ty: NodeId<Expression>,
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
        ty: NodeId<Expression>,
        /// The key type of the field.
        key: NodeId<Expression>,
        /// The default value of the field.
        default: Option<NodeId<Expression>>,
    },
}

// nocheckin TODO #Incomplete: getter/setter functions for Struct/Union/Interface/...Fields?
//  (useful for SOA-style struct views?)
//  (how does this interact with interfaces and unions?)
//  (how does this relate with Entities?)
//  (how does this relate to $ dynamicness?)

impl Node for VariantField {
    const KIND: NodeType = NodeType::VariantField;
}

impl VariantField {
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
            VariantField::Named { name, .. } => Some(name.string()),
            VariantField::Positional { .. } => None,
            VariantField::Dynamic { name, .. } => *name,
        }
    }

    /// Get the type of the field.
    #[inline]
    pub fn ty(&self) -> NodeId<Expression> {
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
