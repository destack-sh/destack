//! MIR node types and identifiers.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use destack_source::ModuleId;

/// The type of a MIR node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeType {
    /// A function definition.
    Function,
    /// A basic block.
    Block,
    /// An instruction.
    Instruction,
    /// A local variable (stack slot).
    Local,
    /// A type.
    Type,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Function => "function",
            NodeType::Block => "block",
            NodeType::Instruction => "instruction",
            NodeType::Local => "local",
            NodeType::Type => "type",
        }
    }
}

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        self.try_into_typed().unwrap()
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
pub struct LocalNodeId<T: Node> {
    pub id: u32,
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// A MIR Node.
pub trait Node: Sized {
    const TYPE: NodeType;
}
