use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StaticArgument};

/// Unique identifier for generic instantiations.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalInstantiationId(pub u32);

impl LocalInstantiationId {
    /// Wrap an id as a local instantiation id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global instantiation id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstantiationId {
        GlobalInstantiationId {
            module_id,
            local_id: self,
        }
    }
}

/// Global instantiation id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalInstantiationId {
    /// The module id of the global instantiation.
    pub module_id: ModuleId,
    /// The local id of the global instantiation.
    pub local_id: LocalInstantiationId,
}

impl GlobalInstantiationId {
    /// Create a new global instantiation id.
    pub fn new(module_id: ModuleId, local_id: LocalInstantiationId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local instantiation id.
    pub fn into_local(self) -> LocalInstantiationId {
        self.local_id
    }
}

impl From<GlobalInstantiationId> for LocalInstantiationId {
    fn from(id: GlobalInstantiationId) -> Self {
        id.local_id
    }
}

impl Display for LocalInstantiationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A concrete application of static arguments to one generic symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instantiation {
    /// The symbol being instantiated.
    pub target: GlobalSymbolId,
    /// The static arguments in declaration order.
    pub arguments: Vec<StaticArgument>,
}

impl Instantiation {
    /// Create a generic instantiation.
    pub fn new(target: GlobalSymbolId, arguments: Vec<StaticArgument>) -> Self {
        Self { target, arguments }
    }
}
