use serde::{Deserialize, Serialize};

use crate::{
    Expression, FunctionSignature, GenericParameter, Key, Keyword, LocalNodeId, Mutability, Node,
    NodeType, ScopeKind, StaticKey, StringId, SymbolForm, SymbolSpace, TypeExpression, Visibility,
    WhereClause,
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

/// A nominal member slot.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemberSlot {
    /// Property keyed by a static key.
    Key(StaticKey),
    /// Constructor role member.
    Constructor,
    /// New role member.
    New,
    /// Callable role member.
    Call,
}

impl FunctionRole {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionRole::Getter => Some(Keyword::Get),
            FunctionRole::Setter => Some(Keyword::Set),
            FunctionRole::Constructor => Some(Keyword::Constructor),
            FunctionRole::New => Some(Keyword::New),
            FunctionRole::Call => None,
        }
    }
}

impl MemberSlot {
    /// Return the slot for one anonymous function role.
    #[inline]
    pub fn from_function_role(role: FunctionRole) -> Option<Self> {
        match role {
            FunctionRole::Constructor => Some(Self::Constructor),
            FunctionRole::New => Some(Self::New),
            FunctionRole::Call => Some(Self::Call),
            FunctionRole::Getter | FunctionRole::Setter => None,
        }
    }
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
        is_shorthand: bool,
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
        is_optional: bool,
        is_definite: bool,
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
        is_optional: bool,
        is_ambient: bool,
        is_override: bool,
        is_static: bool,
        is_accessor: bool,
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
    /// Return the nominal slot occupied by this member.
    pub fn slot(&self) -> Option<MemberSlot> {
        match self {
            Self::AssociatedType { name, .. } | Self::AssociatedConst { name, .. } => {
                Some(MemberSlot::Key(StaticKey::Name(*name)))
            }
            Self::Field { key, .. } => key.direct_static_key().map(MemberSlot::Key),
            Self::Method {
                key: Some(key), ..
            } => key.direct_static_key().map(MemberSlot::Key),
            Self::Method {
                key: None,
                signature,
                ..
            } => signature.role.and_then(MemberSlot::from_function_role),
            Self::StaticBlock { .. } | Self::ComptimeBlock { .. } | Self::Error => None,
        }
    }

    /// Return the symbol key introduced by this member.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::AssociatedType { name, .. } | Self::AssociatedConst { name, .. } => {
                Some(StaticKey::Name(*name))
            }
            Self::Field { key, .. } => key.direct_static_key(),
            Self::Method { key: Some(key), .. } => key.direct_static_key(),
            Self::Method { key: None, .. }
            | Self::StaticBlock { .. }
            | Self::ComptimeBlock { .. }
            | Self::Error => None,
        }
    }

    /// Return the symbol form introduced by this member.
    pub fn symbol_form(&self) -> Option<SymbolForm> {
        match self {
            Self::AssociatedType { .. } => Some(SymbolForm::TypeAlias),
            Self::AssociatedConst { .. } | Self::Field { .. } => Some(SymbolForm::Variable),
            Self::Method { key: Some(_), .. } => Some(SymbolForm::Function),
            Self::Method { key: None, .. }
            | Self::StaticBlock { .. }
            | Self::ComptimeBlock { .. }
            | Self::Error => None,
        }
    }

    /// Return the symbol space introduced by this member.
    pub fn symbol_space(&self) -> Option<SymbolSpace> {
        self.symbol_form().map(SymbolForm::symbol_space)
    }

    /// Return the scope kind owned by this member symbol.
    pub fn symbol_scope_kind(&self) -> Option<ScopeKind> {
        match self {
            Self::AssociatedType { .. } => Some(ScopeKind::Type),
            Self::Method { key: Some(_), .. } => Some(ScopeKind::Function),
            _ => None,
        }
    }

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
            | Member::Method { is_static, .. } => *is_static,
            Member::StaticBlock { .. } => true,
            Member::ComptimeBlock { .. } | Member::Error => false,
        }
    }
}
