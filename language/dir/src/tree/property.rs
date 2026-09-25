use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Expression, FunctionSignature, GenericParameter, Keyword, LocalNodeId, MemberSpace, Mutability,
    Name, Node, NodeFold, NodeType, ScopeKind, StaticKey, StringId, SymbolKind, TypeExpression,
    Visibility, WhereClause,
};

/// Variance annotation for generic parameters.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// One member slot within a namespace.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    /// Return the paired accessor role.
    #[inline]
    pub const fn accessor_counterpart(self) -> Option<Self> {
        match self {
            Self::Getter => Some(Self::Setter),
            Self::Setter => Some(Self::Getter),
            Self::Constructor | Self::New | Self::Call => None,
        }
    }

    /// Return the keyword for the function accessor.
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

impl TryFrom<FunctionRole> for MemberSlot {
    type Error = FunctionRole;

    fn try_from(role: FunctionRole) -> Result<Self, Self::Error> {
        match role {
            FunctionRole::Constructor => Ok(Self::Constructor),
            FunctionRole::New => Ok(Self::New),
            FunctionRole::Call => Ok(Self::Call),
            FunctionRole::Getter | FunctionRole::Setter => Err(role),
        }
    }
}

/// The abstraction mode of a class method.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum Property {
    /// Named field.
    Field {
        name: Name,
        value: LocalNodeId<Expression>,
        is_shorthand: bool,
    },
    /// Object-like member function.
    Method {
        name: Option<Name>,
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
    /// Return the member slot occupied by this property.
    pub fn slot(&self) -> Option<MemberSlot> {
        match self {
            Self::Field { name, .. } => Some(MemberSlot::Key((*name).into())),
            Self::Method {
                name, signature, ..
            } => signature
                .role
                .and_then(|role| role.try_into().ok())
                .or(name.map(|name| MemberSlot::Key(name.into()))),
            Self::Spread { .. } | Self::Error => None,
        }
    }

    /// Return the member namespace occupied by this property.
    pub fn space(&self) -> Option<MemberSpace> {
        match self {
            Self::Field { .. } | Self::Method { .. } => Some(MemberSpace::Instance),
            Self::Spread { .. } | Self::Error => None,
        }
    }

    /// Return the property name when one exists.
    pub fn name(&self) -> Option<Name> {
        match self {
            Property::Field { name, .. } => Some(*name),
            Property::Method { name, .. } => *name,
            Property::Spread { .. } | Property::Error => None,
        }
    }

    /// Return the function signature of the property when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            Property::Method { signature, .. } => Some(signature),
            Property::Field { .. } | Property::Spread { .. } | Property::Error => None,
        }
    }

    /// Return whether this property has a `static` modifier.
    pub const fn has_static_modifier(&self) -> bool {
        false
    }

    /// Return whether this property body has an implicit receiver.
    pub const fn has_implicit_receiver(&self) -> bool {
        matches!(self, Self::Method { .. })
    }

    /// Return whether this property belongs to the object surface.
    pub const fn is_instance_member(&self) -> bool {
        matches!(
            self,
            Self::Field { .. } | Self::Method { .. } | Self::Spread { .. }
        )
    }
}

/// A member of a declaration body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
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
    },
    /// Associated compile-time constant.
    AssociatedConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
        visibility: Option<Visibility>,
        is_ambient: bool,
        is_abstract: bool,
        is_override: bool,
    },
    /// Named field.
    Field {
        name: Name,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
        mutability: Option<Mutability>,
        visibility: Option<Visibility>,
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
        name: Option<Name>,
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
    /// Const evaluation block.
    ConstBlock { body: LocalNodeId<Expression> },
    /// Malformed member slot.
    Error,
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}

impl Member {
    /// Return this member's declared visibility.
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            Self::AssociatedType { visibility, .. }
            | Self::AssociatedConst { visibility, .. }
            | Self::Field { visibility, .. }
            | Self::Method { visibility, .. } => *visibility,
            Self::StaticBlock { .. } | Self::ConstBlock { .. } | Self::Error => None,
        }
    }

    /// Return the nominal slot occupied by this member.
    pub fn slot(&self) -> Option<MemberSlot> {
        match self {
            Self::AssociatedType { name, .. } | Self::AssociatedConst { name, .. } => {
                Some(MemberSlot::Key(StaticKey::Name(*name)))
            }
            Self::Field { name, .. } => Some(MemberSlot::Key((*name).into())),
            Self::Method {
                name, signature, ..
            } => signature
                .role
                .and_then(|role| role.try_into().ok())
                .or(name.map(|name| MemberSlot::Key(name.into()))),
            Self::StaticBlock { .. } | Self::ConstBlock { .. } | Self::Error => None,
        }
    }

    /// Return the member namespace.
    pub fn space(&self) -> Option<MemberSpace> {
        let space = match self {
            Self::AssociatedType { .. } | Self::AssociatedConst { .. } => MemberSpace::Static,
            Self::Field { is_static, .. } | Self::Method { is_static, .. } => match is_static {
                true => MemberSpace::Static,
                false => MemberSpace::Instance,
            },
            Self::StaticBlock { .. } | Self::ConstBlock { .. } | Self::Error => return None,
        };

        Some(space)
    }

    /// Return the symbol key introduced by this member.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::AssociatedType { name, .. } | Self::AssociatedConst { name, .. } => {
                Some(StaticKey::Name(*name))
            }
            Self::Field { name, .. } => Some((*name).into()),
            Self::Method {
                name: Some(name), ..
            } => Some((*name).into()),
            Self::Method { name: None, .. }
            | Self::StaticBlock { .. }
            | Self::ConstBlock { .. }
            | Self::Error => None,
        }
    }

    /// Return the symbol kind introduced by this member.
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Self::AssociatedType { .. } => Some(SymbolKind::AssociatedType),
            Self::AssociatedConst { .. } => Some(SymbolKind::AssociatedConst),
            Self::Field { .. } => Some(SymbolKind::Variable),
            Self::Method { name: Some(_), .. } => Some(SymbolKind::Function),
            Self::Method { name: None, .. }
            | Self::StaticBlock { .. }
            | Self::ConstBlock { .. }
            | Self::Error => None,
        }
    }

    /// Return whether this member sees the enclosing receiver scope.
    pub fn binds_receiver(&self) -> bool {
        match self {
            Self::Field { .. }
            | Self::Method { .. }
            | Self::AssociatedType { .. }
            | Self::AssociatedConst { .. } => true,
            Self::StaticBlock { .. } | Self::ConstBlock { .. } | Self::Error => false,
        }
    }

    /// Return the scope kind owned by this member symbol.
    pub fn symbol_scope_kind(&self) -> Option<ScopeKind> {
        match self {
            Self::AssociatedType { .. } => Some(ScopeKind::Type),
            Self::Method { name: Some(_), .. } => Some(ScopeKind::Function),
            _ => None,
        }
    }

    /// Return the authored member name when one exists.
    pub fn name(&self) -> Option<Name> {
        match self {
            Member::AssociatedType { name, .. } | Member::AssociatedConst { name, .. } => {
                Some(Name::Identifier(*name))
            }
            Member::Field { name, .. } => Some(*name),
            Member::Method { name, .. } => *name,
            _ => None,
        }
    }

    /// Return the function signature of the member when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            Member::Method { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Return whether this member has a `static` modifier.
    pub const fn has_static_modifier(&self) -> bool {
        match self {
            Member::Field { is_static, .. } | Member::Method { is_static, .. } => *is_static,
            Member::AssociatedType { .. }
            | Member::AssociatedConst { .. }
            | Member::StaticBlock { .. }
            | Member::ConstBlock { .. }
            | Member::Error => false,
        }
    }

    /// Return whether this member body has an implicit receiver.
    pub const fn has_implicit_receiver(&self) -> bool {
        match self {
            Member::Field { is_static, .. } | Member::Method { is_static, .. } => !*is_static,
            Member::AssociatedType { .. } | Member::AssociatedConst { .. } => true,
            Member::StaticBlock { .. } | Member::ConstBlock { .. } | Member::Error => false,
        }
    }

    /// Return whether this member belongs to the instance surface.
    pub const fn is_instance_member(&self) -> bool {
        match self {
            Member::Field { is_static, .. } | Member::Method { is_static, .. } => !*is_static,
            Member::AssociatedType { .. }
            | Member::AssociatedConst { .. }
            | Member::StaticBlock { .. }
            | Member::ConstBlock { .. }
            | Member::Error => false,
        }
    }
}
