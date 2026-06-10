use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use destack_source::{ModuleId, ProfileId};
use serde::{Deserialize, Serialize};

use crate::Access;

/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeType {
    Expression,
    TypeExpression,
    Block,
    Catch,
    Declaration,
    Declarator,
    Property,
    TypeMember,
    TypeMappedParameter,
    Member,
    EnumField,
    WhereClause,
    DependencyItem,
    GenericParameter,
    Parameter,
    GenericArgument,
    TupleElement,
    Argument,
    MatchCase,
    Pattern,
    PatternField,
    AssignPattern,
    AssignPatternField,
    Decorator,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Expression => "expression",
            NodeType::TypeExpression => "type expression",
            NodeType::Block => "block",
            NodeType::Catch => "catch",
            NodeType::Declaration => "declaration",
            NodeType::Declarator => "declarator",
            NodeType::Property => "property",
            NodeType::TypeMember => "type member",
            NodeType::TypeMappedParameter => "type mapped parameter",
            NodeType::Member => "member",
            NodeType::EnumField => "enum field",
            NodeType::WhereClause => "where clause",
            NodeType::DependencyItem => "dependency item",
            NodeType::GenericParameter => "generic parameter",
            NodeType::Parameter => "parameter",
            NodeType::GenericArgument => "generic argument",
            NodeType::TupleElement => "tuple element",
            NodeType::Argument => "argument",
            NodeType::MatchCase => "match case",
            NodeType::Pattern => "pattern",
            NodeType::PatternField => "pattern field",
            NodeType::AssignPattern => "assign pattern",
            NodeType::AssignPatternField => "assign pattern field",
            NodeType::Decorator => "decorator",
        }
    }
}

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalNodeIdAny {
    pub id: u32,
    pub ty: NodeType,
}

impl LocalNodeIdAny {
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }

    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }

    /// Turn into a typed local node id.
    #[inline]
    pub fn try_into_typed<T: Node>(self) -> Result<LocalNodeId<T>, String> {
        // reject sentinel node ids
        if self.id == u32::MAX {
            return Err(format!("invalid {} node id {}", T::TYPE.name(), self.id));
        }

        // reject mismatched node types
        if self.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}",
                T::TYPE.name(),
                self.ty.name(),
                self.id
            ));
        }
        Ok(LocalNodeId::new(self.id))
    }

    /// Turn into a typed local node id.
    pub fn into_typed<T: Node>(self) -> LocalNodeId<T> {
        self.try_into_typed().unwrap()
    }

    /// Turn into a GlobalNodeIdAny.
    #[inline]
    pub fn into_global(self, module_id: ModuleId) -> GlobalNodeIdAny {
        GlobalNodeIdAny {
            module_id,
            local_id: self,
        }
    }

    /// Return a cache key for this node id.
    #[inline]
    pub fn cache_key(self) -> u64 {
        (self.id as u64) | ((self.ty as u64) << 32)
    }

    /// Turn into an AnchoredGlobalNodeId.
    #[inline]
    pub fn into_anchored(
        self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
    ) -> AnchoredGlobalNodeId {
        AnchoredGlobalNodeId {
            node_id: self.into_global(module_id),
            profile_id,
        }
    }
}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny
where
    T: Node,
{
    fn from(id: LocalNodeId<T>) -> Self {
        Self {
            id: id.id,
            ty: T::TYPE,
        }
    }
}

impl Debug for LocalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeIdAny")
            .field("id", &self.id)
            .field("type", &self.ty)
            .finish()
    }
}

impl<T: Node> TryFrom<LocalNodeIdAny> for LocalNodeId<T> {
    type Error = String;

    fn try_from(id: LocalNodeIdAny) -> Result<Self, Self::Error> {
        if id.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}",
                T::TYPE.name(),
                id.ty.name(),
                id.id
            ));
        }
        Ok(Self {
            id: id.id,
            _ty: PhantomData,
        })
    }
}

/// Unique identifier for nodes in a local arena, parameterized by node type.
#[repr(transparent)]
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct LocalNodeId<T: Node> {
    pub id: u32,
    #[serde(skip)]
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> LocalNodeId<T> {
    /// Create a new node id.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Turn into a LocalNodeIdAny.
    #[inline]
    pub fn into_any(self) -> LocalNodeIdAny {
        LocalNodeIdAny {
            id: self.id,
            ty: T::TYPE,
        }
    }

    /// Turn into a GlobalNodeId.
    #[inline]
    pub fn into_global(self, module_id: ModuleId) -> GlobalNodeId<T> {
        GlobalNodeId {
            module_id,
            local_id: self,
        }
    }

    /// Turn into a GlobalNodeIdAny.
    #[inline]
    pub fn into_global_any(self, module_id: ModuleId) -> GlobalNodeIdAny {
        GlobalNodeIdAny {
            module_id,
            local_id: self.into_any(),
        }
    }
}

impl<T: Node> Clone for LocalNodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

impl<T: Node> Copy for LocalNodeId<T> {}

impl<T: Node> PartialEq for LocalNodeId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Node> Eq for LocalNodeId<T> {}

impl<T: Node> PartialOrd for LocalNodeId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Node> Ord for LocalNodeId<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Node> Hash for LocalNodeId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Node> LocalNodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// Global node id across modules.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct GlobalNodeId<T: Node> {
    /// The module id of the global node.
    pub module_id: ModuleId,
    /// The local id of the global node.
    pub local_id: LocalNodeId<T>,
}

impl<T: Node> Clone for GlobalNodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Node> Copy for GlobalNodeId<T> {}

impl<T: Node> PartialEq for GlobalNodeId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.module_id == other.module_id && self.local_id == other.local_id
    }
}

impl<T: Node> Eq for GlobalNodeId<T> {}

impl<T: Node> PartialOrd for GlobalNodeId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Node> Ord for GlobalNodeId<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.module_id
            .cmp(&other.module_id)
            .then_with(|| self.local_id.cmp(&other.local_id))
    }
}

impl<T: Node> Hash for GlobalNodeId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module_id.hash(state);
        self.local_id.hash(state);
    }
}

impl<T: Node> GlobalNodeId<T> {
    /// Create a new global node id.
    pub fn new(module_id: ModuleId, local_id: LocalNodeId<T>) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a GlobalNodeIdAny.
    #[inline]
    pub fn into_any(self) -> GlobalNodeIdAny {
        GlobalNodeIdAny {
            module_id: self.module_id,
            local_id: self.local_id.into_any(),
        }
    }
}

impl<T: Node> Debug for GlobalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalNodeId")
            .field("module_id", &self.module_id)
            .field("local_id", &self.local_id)
            .finish()
    }
}

impl<T: Node> From<GlobalNodeId<T>> for LocalNodeId<T> {
    fn from(id: GlobalNodeId<T>) -> Self {
        id.local_id
    }
}

/// Global node id across modules.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalNodeIdAny {
    /// The module id of the global node.
    pub module_id: ModuleId,
    /// The local id of the global node.
    pub local_id: LocalNodeIdAny,
}

impl GlobalNodeIdAny {
    /// Create a new global node id.
    pub fn new(module_id: ModuleId, local_id: LocalNodeIdAny) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a typed global node id.
    pub fn try_into_typed<T: Node>(self) -> Result<GlobalNodeId<T>, String> {
        if self.local_id.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}/{}",
                T::TYPE.name(),
                self.local_id.ty.name(),
                self.module_id,
                self.local_id.id
            ));
        }
        Ok(GlobalNodeId {
            module_id: self.module_id,
            local_id: LocalNodeId::new(self.local_id.id),
        })
    }

    /// Turn into a typed global node id.
    pub fn into_typed<T: Node>(self) -> GlobalNodeId<T> {
        self.try_into_typed().unwrap()
    }

    /// Turn into a typed local node id.
    pub fn try_into_local_typed<T: Node>(self) -> Result<LocalNodeId<T>, String> {
        if self.local_id.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}/{}",
                T::TYPE.name(),
                self.local_id.ty.name(),
                self.module_id,
                self.local_id.id
            ));
        }
        Ok(LocalNodeId::new(self.local_id.id))
    }

    /// Turn into a typed local node id.
    pub fn into_local_typed<T: Node>(self) -> LocalNodeId<T> {
        self.try_into_local_typed().unwrap()
    }

    /// Turn into an AnchoredGlobalNodeId.
    #[inline]
    pub fn into_anchored(self, profile_id: Option<ProfileId>) -> AnchoredGlobalNodeId {
        AnchoredGlobalNodeId {
            node_id: self,
            profile_id,
        }
    }
}

impl Debug for GlobalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalNodeIdAny")
            .field("module_id", &self.module_id)
            .field("local_id", &self.local_id)
            .finish()
    }
}

impl<T: Node> From<GlobalNodeId<T>> for GlobalNodeIdAny {
    fn from(id: GlobalNodeId<T>) -> Self {
        Self {
            module_id: id.module_id,
            local_id: id.local_id.into_any(),
        }
    }
}

impl<T: Node> TryFrom<GlobalNodeIdAny> for GlobalNodeId<T> {
    type Error = String;

    fn try_from(id: GlobalNodeIdAny) -> Result<Self, Self::Error> {
        if id.local_id.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}/{}",
                T::TYPE.name(),
                id.local_id.ty.name(),
                id.module_id,
                id.local_id.id
            ));
        }
        Ok(Self {
            module_id: id.module_id,
            local_id: LocalNodeId::new(id.local_id.id),
        })
    }
}

impl<T: Node> TryFrom<GlobalNodeIdAny> for LocalNodeId<T> {
    type Error = String;

    fn try_from(id: GlobalNodeIdAny) -> Result<Self, Self::Error> {
        id.local_id.try_into()
    }
}

impl From<GlobalNodeIdAny> for LocalNodeIdAny {
    fn from(id: GlobalNodeIdAny) -> Self {
        id.local_id
    }
}

/// Anchored global node id with profile identity.
///
/// Base DIR nodes have no profile.
/// Profile-scoped patch nodes carry the profile that produced them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchoredGlobalNodeId {
    /// The global node id.
    pub node_id: GlobalNodeIdAny,
    /// The profile this node belongs to (None for base DIR).
    pub profile_id: Option<ProfileId>,
}

impl AnchoredGlobalNodeId {
    /// Create a new anchored global node id.
    pub fn new(node_id: GlobalNodeIdAny, profile_id: Option<ProfileId>) -> Self {
        Self {
            node_id,
            profile_id,
        }
    }

    /// Create an anchored node id for the base DIR (no profile).
    pub fn base(node_id: GlobalNodeIdAny) -> Self {
        Self {
            node_id,
            profile_id: None,
        }
    }

    /// Create an anchored node id for a profile-specific DIR.
    pub fn profiled(node_id: GlobalNodeIdAny, profile_id: ProfileId) -> Self {
        Self {
            node_id,
            profile_id: Some(profile_id),
        }
    }

    /// Get the module id.
    pub fn module_id(&self) -> ModuleId {
        self.node_id.module_id
    }

    /// Get the local node id.
    pub fn local_id(&self) -> LocalNodeIdAny {
        self.node_id.local_id
    }
}

impl Debug for AnchoredGlobalNodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnchoredGlobalNodeId")
            .field("node_id", &self.node_id)
            .field("profile_id", &self.profile_id)
            .finish()
    }
}

impl From<GlobalNodeIdAny> for AnchoredGlobalNodeId {
    /// Convert from GlobalNodeIdAny, assuming base DIR (no profile).
    fn from(node_id: GlobalNodeIdAny) -> Self {
        Self::base(node_id)
    }
}

impl<T: Node> From<GlobalNodeId<T>> for AnchoredGlobalNodeId {
    /// Convert from typed GlobalNodeId, assuming base DIR (no profile).
    fn from(node_id: GlobalNodeId<T>) -> Self {
        Self::base(node_id.into_any())
    }
}

/// A Node.
pub trait Node: Sized {
    const TYPE: NodeType;
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    /// Public to everything.
    Public,
    /// Protected to derived constructs.
    Protected,
    /// Private to the closest module scope.
    Private,
}

/// The asynchrony of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Asynchrony {
    /// Synchronous function.
    Sync,
    /// Asynchronous function.
    Async,
}

/// A Mutability is a const, mutable, or exclusive access qualifier.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified when the target is mutable.
    Mutable,
    /// May be modified through exclusive access.
    Exclusive,
}

impl Mutability {
    /// Return the normalized access value for this mutability qualifier.
    pub fn access(self) -> Access {
        match self {
            Self::Immutable => Access::Readonly,
            Self::Mutable => Access::Mutable,
            Self::Exclusive => Access::Exclusive,
        }
    }
}
