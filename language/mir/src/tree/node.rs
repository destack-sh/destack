use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_source::{ModuleId, TargetId};

use crate::{Block, Function, Global, Local};

/// Compact block identity in one MIR function.
pub type BlockId = LocalNodeId<Block>;

/// Compact function identity in one MIR tree.
pub type FunctionId = LocalNodeId<Function>;

/// Compact local identity in one MIR function.
pub type LocalId = LocalNodeId<Local>;

/// Compact global identity in one MIR tree.
pub type GlobalId = LocalNodeId<Global>;

/// The type of a MIR node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum NodeType {
    /// A function definition.
    Function,
    /// A basic block.
    Block,
    /// An instruction.
    Instruction,
    /// A block terminator.
    Terminator,
    /// A local variable (stack slot).
    Local,
    /// A named type declaration.
    TypeDeclaration,
    /// A global variable or constant.
    Global,
}

/// Dense index entry for one MIR node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        assert!(
            local_id <= Self::LOCAL_ID_MASK,
            "MIR node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | (Self::node_type_tag(node_type) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Function,
            1 => NodeType::Block,
            2 => NodeType::Instruction,
            3 => NodeType::Terminator,
            4 => NodeType::Local,
            5 => NodeType::TypeDeclaration,
            6 => NodeType::Global,
            _ => unreachable!("invalid MIR node type tag in packed node index"),
        }
    }

    /// Return the stable packed tag for one node type.
    #[inline]
    fn node_type_tag(node_type: NodeType) -> u32 {
        match node_type {
            NodeType::Function => 0,
            NodeType::Block => 1,
            NodeType::Instruction => 2,
            NodeType::Terminator => 3,
            NodeType::Local => 4,
            NodeType::TypeDeclaration => 5,
            NodeType::Global => 6,
        }
    }
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Function => "function",
            NodeType::Block => "block",
            NodeType::Instruction => "instruction",
            NodeType::Terminator => "terminator",
            NodeType::Local => "local",
            NodeType::TypeDeclaration => "type_declaration",
            NodeType::Global => "global",
        }
    }
}

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
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
        match self.try_into_typed() {
            Ok(id) => id,
            Err(error) => unreachable!("{error}"),
        }
    }

    /// Turn into an AnchoredGlobalNodeId.
    #[inline]
    pub fn into_anchored(self, module_id: ModuleId, target_id: TargetId) -> AnchoredGlobalNodeId {
        AnchoredGlobalNodeId {
            node_id: GlobalNodeIdAny {
                module_id,
                local_id: self,
            },
            target_id,
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
#[derive(Serialize, Deserialize, Reflect)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct LocalNodeId<T: Node> {
    pub id: u32,
    #[serde(skip)]
    _ty: PhantomData<fn() -> T>,
}

// manual impls to avoid requiring T: Eq, T: Hash, etc.
impl<T: Node> Clone for LocalNodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Node> Copy for LocalNodeId<T> {}

impl<T: Node> PartialEq for LocalNodeId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Node> Eq for LocalNodeId<T> {}

impl<T: Node> std::hash::Hash for LocalNodeId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Node> PartialOrd for LocalNodeId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Node> Ord for LocalNodeId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
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
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

impl<T: Node> LocalNodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
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

/// Global node id across modules.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
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

/// Global node id across modules (untyped).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
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
        match self.try_into_typed() {
            Ok(id) => id,
            Err(error) => unreachable!("{error}"),
        }
    }

    /// Turn into an AnchoredGlobalNodeId.
    #[inline]
    pub fn into_anchored(self, target_id: TargetId) -> AnchoredGlobalNodeId {
        AnchoredGlobalNodeId {
            node_id: self,
            target_id,
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
        id.try_into_typed()
    }
}

impl From<GlobalNodeIdAny> for LocalNodeIdAny {
    fn from(id: GlobalNodeIdAny) -> Self {
        id.local_id
    }
}

/// Anchored global node id with target identity.
///
/// MIR is always generated per-target, so every MIR node has an associated target.
/// This type carries the target needed to resolve module-local ids.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AnchoredGlobalNodeId {
    /// The global node id.
    pub node_id: GlobalNodeIdAny,
    /// The target this node belongs to.
    pub target_id: TargetId,
}

impl AnchoredGlobalNodeId {
    /// Create a new anchored global node id.
    pub fn new(node_id: GlobalNodeIdAny, target_id: TargetId) -> Self {
        Self { node_id, target_id }
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
            .field("target_id", &self.target_id)
            .finish()
    }
}

/// A MIR Node.
pub trait Node: Sized {
    const TYPE: NodeType;
}
