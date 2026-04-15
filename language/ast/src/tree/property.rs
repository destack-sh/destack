use serde::{Deserialize, Serialize};

use crate::{
    Ambientness, Expression, FunctionSignature, GenericParameter, Key, Keyword, LocalNodeId,
    Mutability, Node, NodeType, StringId, TypeExpression, Visibility, WhereClause,
};

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

impl VarianceModifier {
    /// Return the keyword string for the variance modifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            VarianceModifier::In => "in",
            VarianceModifier::Out => "out",
            VarianceModifier::InOut => "in out",
        }
    }
}

/// The mode of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

impl FunctionMode {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionMode::Getter => Some(Keyword::Get),
            FunctionMode::Setter => Some(Keyword::Set),
            FunctionMode::Constructor => Some(Keyword::Constructor),
            FunctionMode::New => Some(Keyword::New),
            FunctionMode::Call => None,
        }
    }
}

/// A property of an object-like literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Property {
    /// Named field.
    Field {
        key: Key,
        value: LocalNodeId<Expression>,
    },
    /// Object-like member function.
    Method {
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Spread property.
    Spread { value: LocalNodeId<Expression> },
    /// Malformed property slot.
    Error,
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
            Property::Spread { .. } | Property::Error => None,
        }
    }

    /// Get the function signature of the property when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            Property::Method { signature, .. } => Some(signature),
            Property::Field { .. } | Property::Spread { .. } | Property::Error => None,
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
        ambient: Ambientness,
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
        ambient: Ambientness,
        is_static: bool,
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
    },
    /// Type embedding.
    Embed {
        value: LocalNodeId<TypeExpression>,
        visibility: Option<Visibility>,
        ambient: Ambientness,
        is_static: bool,
    },
    /// Static initialization block.
    StaticBlock { body: LocalNodeId<Expression> },
    /// Comptime block.
    ComptimeBlock { body: LocalNodeId<Expression> },
    /// Malformed member slot.
    Error,
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
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
            | Member::Method { is_static, .. }
            | Member::Embed { is_static, .. } => *is_static,
            Member::StaticBlock { .. } => true,
            Member::ComptimeBlock { .. } | Member::Error => false,
        }
    }
}
