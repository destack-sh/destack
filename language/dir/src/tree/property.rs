use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{
    Ambientness, Expression, FunctionSignature, GenericParameter, Key, LocalNodeId, LocalSymbolId,
    Mutability, Node, NodeType, StaticExpression, StringId, TypeExpression, Visibility,
    WhereClause,
};

/// Static property in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node, so that we can pass it around directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum VarianceModifier {
    /// Contravariant parameter.
    In,
    /// Covariant parameter.
    Out,
    /// Invariant parameter.
    InOut,
}

/// The mode of a function.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub enum FunctionMode {
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

/// A property of an object-like literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Property {
    /// Named field.
    Field {
        key: Key,
        value: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
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

/// A member of a declaration body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Member {
    /// Associated type alias.
    Type {
        name: StringId,
        generic_parameters: Vec<LocalNodeId<GenericParameter>>,
        where_clauses: Vec<LocalNodeId<WhereClause>>,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
        symbol: LocalSymbolId,
    },
    /// Associated compile-time constant.
    ComptimeConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_static: bool,
        symbol: LocalSymbolId,
    },
    /// Named field.
    Field {
        key: Key,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        is_optional: bool,
        is_readonly: bool,
        mutability: Option<Mutability>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
        is_const_asserted: bool,
        is_accessor: bool,
        is_comptime: bool,
        symbol: LocalSymbolId,
    },
    /// Named member function.
    Method {
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
        is_accessor: bool,
        is_comptime: bool,
        symbol: LocalSymbolId,
    },
    /// Type embedding.
    Embed {
        value: LocalNodeId<TypeExpression>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_static: bool,
        symbol: LocalSymbolId,
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
    /// Get the symbol of the member.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Member::Type { symbol, .. }
            | Member::ComptimeConst { symbol, .. }
            | Member::Field { symbol, .. }
            | Member::Method { symbol, .. }
            | Member::Embed { symbol, .. }
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
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
