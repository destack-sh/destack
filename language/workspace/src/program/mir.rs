use destack_base::StringPool;
use destack_mir::{self as mir};
use destack_source::ModuleId;
use parking_lot::RwLock;

/// MIR-level module data.
#[derive(Debug)]
pub struct ModuleMir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The target this is for.
    // pub target: String, // nocheckin
    /// The MIR of the Module (may be empty initially).
    pub tree: RwLock<mir::NodeTree>,
    /// The string pool of the Module's MIR stuff.
    pub strings: StringPool,
}

impl ModuleMir {
    /// Create a new ModuleMir.
    pub fn new(id: ModuleId) -> Self {
        Self {
            id,
            tree: RwLock::new(mir::NodeTree::new()),
            strings: StringPool::new(),
        }
    }
}
