use destack_base::StringPool;
use destack_mir::{self as mir};
use destack_source::{ModuleId, ModuleVersion};
use parking_lot::RwLock;

/// MIR-level module data.
#[derive(Debug)]
pub struct ModuleMir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The target this is for.
    pub target: String,
    /// The MIR of the Module (may be empty initially).
    pub tree: RwLock<mir::NodeTree>,
    /// The string pool of the Module's MIR stuff.
    pub strings: StringPool,
}

impl ModuleMir {
    /// Create a new ModuleMir.
    pub fn new(id: ModuleId, version: ModuleVersion, target: String) -> Self {
        Self {
            id,
            version,

            target,
            tree: RwLock::new(mir::NodeTree::new()),
            strings: StringPool::new(),
        }
    }
}
