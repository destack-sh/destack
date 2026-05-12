use serde::{Deserialize, Serialize};

use crate::{
    Expression, FunctionSignature, GenericParameter, Key, LocalNodeId, LocalSymbolId, Mutability,
    Node, NodeType, StaticExpression, StringId, TypeExpression, Visibility, WhereClause,
};

/// Static property in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node, so that we can pass it around directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticProperty {
    /// Unevaluated property.
    Unevaluated { node: LocalNodeId<Property> },

    /// Evaluated static field.
    Field {
        key: Key,
        value: StaticExpression,
        symbol: LocalSymbolId,
    },
    /// Evaluated static member function.
    Method {
        key: Option<Key>,
        signature: FunctionSignature,
        body: StaticExpression,
        symbol: LocalSymbolId,
    },
    /// Evaluated static spread.
    Spread {
        value: StaticExpression,
        symbol: LocalSymbolId,
    },
}

impl StaticProperty {
    /// Check if the static property and its values have been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticProperty::Unevaluated { .. } => false,
            StaticProperty::Field { value, .. } => value.is_evaluated(),
            StaticProperty::Method { body, .. } => body.is_evaluated(),
            StaticProperty::Spread { value, .. } => value.is_evaluated(),
        }
    }
}

/// Variance annotation for generic parameters.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarianceModifier {
    /// Contravariant parameter.
    In,
    /// Covariant parameter.
    Out,
    /// Invariant parameter.
    InOut,
}

/// The special role of a function.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionRole {
    /// Getter function.
    Getter,
    /// Setter function.
    Setter,
    /// Constructor function.
    Constructor,
    /// New type function.
    New,
    /// Implicit call function.
    Call,
}

/// The abstraction mode of a class method.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MethodAbstraction {
    /// A non-overridable concrete method.
    #[default]
    Concrete,
    /// A concrete method that subclasses may override.
    Virtual,
    /// A method that subclasses must implement.
    Abstract,
}

impl MethodAbstraction {
    /// Return whether the method is abstract.
    #[inline]
    pub const fn is_abstract(self) -> bool {
        matches!(self, Self::Abstract)
    }
}

/// A property of an object-like literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Property {
    /// Named field.
    Field {
        key: Key,
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
        is_shorthand: bool,
    },
    /// Named member function.
    Method {
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        symbol: LocalSymbolId,
    },
    /// Spread property.
    Spread {
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },
    /// Malformed property slot.
    Error { symbol: LocalSymbolId },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

impl Property {
    /// Get the key of the property when one exists.
    pub fn key(&self) -> Option<&Key> {
        match self {
            Property::Field { key, .. } => Some(key),
            Property::Method { key, .. } => key.as_ref(),
            Property::Spread { .. } | Property::Error { .. } => None,
        }
    }

    /// Get the function signature of the property when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            Property::Method { signature, .. } => Some(signature),
            Property::Field { .. } | Property::Spread { .. } | Property::Error { .. } => None,
        }
    }

    /// Get the symbol of the property.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Property::Field { symbol, .. }
            | Property::Method { symbol, .. }
            | Property::Spread { symbol, .. }
            | Property::Error { symbol } => *symbol,
        }
    }
}

/// A member of a declaration body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Member {
    /// Associated type alias.
    AssociatedType {
        name: StringId,
        generic_parameters: Vec<LocalNodeId<GenericParameter>>,
        where_clauses: Vec<LocalNodeId<WhereClause>>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_ambient: bool,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
    },
    /// Associated compile-time constant.
    AssociatedConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_ambient: bool,
        is_static: bool,
    },
    /// Named field.
    Field {
        key: Key,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        mutability: Option<Mutability>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_optional: bool,
        is_readonly: bool,
        is_ambient: bool,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
        is_accessor: bool,
    },
    /// Named member function.
    Method {
        key: Option<Key>,
        signature: FunctionSignature,
        abstraction: MethodAbstraction,
        body: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        symbol: LocalSymbolId,
        is_optional: bool,
        is_ambient: bool,
        is_override: bool,
        is_static: bool,
        is_accessor: bool,
    },
    /// Static initialization block.
    StaticBlock {
        body: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },
    /// Comptime block.
    ComptimeBlock {
        body: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },
    /// Malformed member slot.
    Error { symbol: LocalSymbolId },
}

impl Member {
    /// Get the declared name of the member when one exists.
    pub fn name(&self) -> Option<StringId> {
        match self {
            Member::AssociatedType { name, .. } | Member::AssociatedConst { name, .. } => {
                Some(*name)
            }
            _ => None,
        }
    }

    /// Get the symbol of the member.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Member::AssociatedType { symbol, .. }
            | Member::AssociatedConst { symbol, .. }
            | Member::Field { symbol, .. }
            | Member::Method { symbol, .. }
            | Member::StaticBlock { symbol, .. }
            | Member::ComptimeBlock { symbol, .. }
            | Member::Error { symbol } => *symbol,
        }
    }

    /// Get the key of the member when one exists.
    pub fn key(&self) -> Option<&Key> {
        match self {
            Member::Field { key, .. } => Some(key),
            Member::Method { key, .. } => key.as_ref(),
            _ => None,
        }
    }

    /// Get the function signature of the member when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            Member::Method { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Check whether the member is static.
    pub fn is_static(&self) -> bool {
        match self {
            Member::AssociatedType { is_static, .. }
            | Member::AssociatedConst { is_static, .. }
            | Member::Field { is_static, .. }
            | Member::Method { is_static, .. } => *is_static,
            Member::StaticBlock { .. } => true,
            Member::ComptimeBlock { .. } | Member::Error { .. } => false,
        }
    }
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
