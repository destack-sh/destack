use indexmap::IndexMap;

use crate::{FlowBlockId, GlobalNodeIdAny, GlobalSymbolId, LocalTypeId};

/// Identify a stored flow environment.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowEnvironmentId(
    /// Store the environment index.
    pub u32,
);

/// Describe a flow environment with reachability and symbol narrowings.
#[derive(Debug, Clone)]
pub struct FlowEnvironment {
    /// Store narrowed types by symbol.
    pub bindings: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// Track whether the environment is reachable.
    pub is_reachable: bool,
}

/// Store flow environments indexed by block or node.
#[derive(Debug, Clone)]
pub struct FlowTable {
    /// Store all flow environments.
    pub environments: Vec<FlowEnvironment>,
    /// Map blocks to their entry environment.
    pub entry_environment_by_block: Vec<FlowEnvironmentId>,
    /// Map blocks to their exit environment.
    pub exit_environment_by_block: Vec<FlowEnvironmentId>,
    /// Map nodes to their environment.
    pub environment_by_node: IndexMap<GlobalNodeIdAny, FlowEnvironmentId>,
}

impl FlowEnvironment {
    /// Create an empty flow environment.
    pub fn new(is_reachable: bool) -> Self {
        Self {
            bindings: IndexMap::new(),
            is_reachable,
        }
    }
}

impl Default for FlowTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowTable {
    /// Create an empty flow table.
    pub fn new() -> Self {
        Self {
            environments: Vec::new(),
            entry_environment_by_block: Vec::new(),
            exit_environment_by_block: Vec::new(),
            environment_by_node: IndexMap::new(),
        }
    }

    /// Store a flow environment and return its identifier.
    pub fn push_environment(&mut self, environment: FlowEnvironment) -> FlowEnvironmentId {
        let environment_id = FlowEnvironmentId(self.environments.len() as u32);
        self.environments.push(environment);
        environment_id
    }

    /// Get a flow environment by identifier.
    pub fn environment(&self, environment_id: FlowEnvironmentId) -> Option<&FlowEnvironment> {
        self.environments.get(environment_id.0 as usize)
    }

    /// Get the entry environment identifier for a block.
    pub fn entry_environment_for_block(&self, block_id: FlowBlockId) -> Option<FlowEnvironmentId> {
        self.entry_environment_by_block
            .get(block_id.0 as usize)
            .copied()
    }

    /// Get the exit environment identifier for a block.
    pub fn exit_environment_for_block(&self, block_id: FlowBlockId) -> Option<FlowEnvironmentId> {
        self.exit_environment_by_block
            .get(block_id.0 as usize)
            .copied()
    }
}
