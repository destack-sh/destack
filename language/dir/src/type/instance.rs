use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StaticArgument};

/// Unique identifier for generic instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalInstanceId(pub u32);

impl LocalInstanceId {
    /// Wrap an id as a local instance id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global instance id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstanceId {
        GlobalInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global instance id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    /// Turn into a local instance id.
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

/// A concrete application of static arguments to one generic symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    /// The symbol being instantiated.
    pub symbol: GlobalSymbolId,
    /// The static arguments in declaration order.
    pub arguments: Vec<StaticArgument>,
}

impl Instance {
    /// Create a generic instance.
    pub fn new(symbol: GlobalSymbolId, arguments: Vec<StaticArgument>) -> Self {
        Self { symbol, arguments }
    }
}
