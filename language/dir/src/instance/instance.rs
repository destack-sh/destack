use std::fmt::Display;

use crate::{GlobalSymbolId, LocalNodeId, ModuleId, StaticArgument};

/// Unique identifier for Instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalInstanceId(pub u32);

impl LocalInstanceId {
    /// Wrap an id as a InstanceId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalInstanceId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstanceId {
        GlobalInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global instance id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalInstanceId {
    /// The module id of the global instance.
    pub module_id: ModuleId,
    /// The local id of the global instance.
    pub local_id: LocalInstanceId,
}

impl GlobalInstanceId {
    /// Create a new global instance id.
    pub fn new(module_id: ModuleId, local_id: LocalInstanceId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalInstanceId.
    pub fn into_local(self) -> LocalInstanceId {
        self.local_id
    }
}

impl From<GlobalInstanceId> for LocalInstanceId {
    fn from(id: GlobalInstanceId) -> Self {
        id.local_id
    }
}

impl Display for LocalInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// An Instance is an instantiation of a statically parameterized type.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// The symbol we're instantiating.
    pub symbol_id: GlobalSymbolId,
    /// The static arguments to the instance.
    pub static_arguments: Vec<LocalNodeId<StaticArgument>>,
}
