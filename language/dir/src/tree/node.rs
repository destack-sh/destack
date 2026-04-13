use destack_source::AdaptImage;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use destack_source::{ModuleId, ProfileId};
use serde::{Deserialize, Serialize};

/// The type of a node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub enum NodeType {
    Expression,
    TypeExpression,
    Block,
    Declaration,
    Declarator,
    Property,
    TypeProperty,
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
            NodeType::Declaration => "declaration",
            NodeType::Declarator => "declarator",
            NodeType::Property => "property",
            NodeType::TypeProperty => "type property",
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
            NodeType::Decorator => "decorator",
        }
    }
}

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage)]
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
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage)]
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

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
impl<T: Clone + Node> Copy for LocalNodeId<T> {}

impl<T: Node> LocalNodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// Global node id across modules.
/// NOTE #Architecture: should we make GlobalNodeId/GlobalSymbolId/GlobalTypeId/.. carry ProfileId?
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage)]
#[serde(bound = "")]
pub struct GlobalNodeId<T: Node> {
    /// The module id of the global node.
    pub module_id: ModuleId,
    /// The local id of the global node.
    pub local_id: LocalNodeId<T>,
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage)]
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
    pub fn into_type<T: Node>(self) -> GlobalNodeId<T> {
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

/// Anchored global node id with profile provenance.
///
/// DIR nodes can come from different sources:
/// - Base DIR (shared, profile_id = None) - from Import phase
/// - Profile-specific DIR (profile_id = Some) - from Resolve/Analyze/Elaborate phases
///
/// This type tracks the provenance so diagnostics can find the correct source location.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
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

    /// Whether this node is lexically resolved (ignoring child nodes).
    ///
    /// Resolution is name resolution: binding names to symbols. A node is resolved
    /// when all its name references have been bound to their target symbols.
    /// For example, `UnresolvedPath` becomes `LocalReference` or `Member` after resolution.
    fn is_resolved(&self) -> bool {
        true // most nodes don't need resolution
    }

    /// Whether this node has been fully evaluated (ignoring child nodes).
    ///
    /// Evaluation is type-dependent transformation that happens after resolution.
    fn is_evaluated(&self) -> bool {
        true // most nodes don't need evaluation
    }
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Visibility {
    /// Public to everything.
    Public,
    /// Protected to derived constructs.
    Protected,
    /// Private to the closest module scope.
    Private,
}

/// The asynchrony of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Asynchrony {
    /// Synchronous function.
    Sync,
    /// Asynchronous function.
    Async,
}

/// Whether syntax is ambient or concrete.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum Ambientness {
    /// Ambient syntax declared with `declare`.
    Ambient,
    /// Concrete syntax with a body or emitted value.
    Concrete,
}

impl Ambientness {
    /// Return whether the syntax is ambient.
    #[inline]
    pub fn is_ambient(self) -> bool {
        matches!(self, Self::Ambient)
    }
}

/// The reference type of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ReferenceType {
    /// A value reference (like `^T`).
    Value,
    /// A reference to a mutable binding (like `&T`).
    Reference,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified (incl. inner if they are also mutable).
    Mutable,
}
