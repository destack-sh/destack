use destack_base::StringId;

/// Identifier for a TBAA node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TbaaNodeId(u32);

impl TbaaNodeId {
    /// Create a node id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a TBAA tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TbaaTagId(u32);

impl TbaaTagId {
    /// Create a tag id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// TBAA node describing a type in the alias tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TbaaNode {
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
    /// Parent node in the TBAA tree.
    pub parent: Option<TbaaNodeId>,
    /// Whether this node represents immutable memory.
    pub is_constant: bool,
}

/// TBAA tag describing an access in the alias tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TbaaTag {
    /// Base type node for the access.
    pub base: TbaaNodeId,
    /// Access type node for the access.
    pub access: TbaaNodeId,
    /// Byte offset within the base type.
    pub offset: u64,
    /// Size of the access in bytes.
    pub size: u64,
    /// Whether this access is to immutable memory.
    pub is_immutable: bool,
}

/// Table of TBAA nodes and tags.
#[derive(Debug, Clone, Default)]
pub struct TbaaTable {
    /// Registered TBAA nodes.
    pub nodes: Vec<TbaaNode>,
    /// Registered TBAA tags.
    pub tags: Vec<TbaaTag>,
}

impl TbaaTable {
    /// Create a new empty TBAA table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new TBAA node.
    pub fn create_node(
        &mut self,
        name: Option<StringId>,
        parent: Option<TbaaNodeId>,
        is_constant: bool,
    ) -> TbaaNodeId {
        let id = TbaaNodeId::new(self.nodes.len() as u32);
        self.nodes.push(TbaaNode {
            name,
            parent,
            is_constant,
        });
        id
    }

    /// Create a new TBAA tag.
    pub fn create_tag(
        &mut self,
        base: TbaaNodeId,
        access: TbaaNodeId,
        offset: u64,
        size: u64,
        is_immutable: bool,
    ) -> TbaaTagId {
        let id = TbaaTagId::new(self.tags.len() as u32);
        self.tags.push(TbaaTag {
            base,
            access,
            offset,
            size,
            is_immutable,
        });
        id
    }

    /// Return the TBAA node for an id.
    pub fn node(&self, id: TbaaNodeId) -> &TbaaNode {
        &self.nodes[id.index()]
    }

    /// Return the TBAA tag for an id.
    pub fn tag(&self, id: TbaaTagId) -> &TbaaTag {
        &self.tags[id.index()]
    }
}
