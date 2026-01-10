use std::sync::Arc;

use destack_base::StringPool;
use destack_mir::{self as mir};
use destack_source::{ModuleId, ModuleVersion};
use parking_lot::RwLock;

use crate::TargetId;

/// MIR-level module data.
#[derive(Debug)]
pub struct ModuleMir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The target this is for.
    pub target: TargetId,
    /// The MIR of the Module (may be empty initially).
    pub tree: RwLock<mir::NodeTree>,
    /// The string pool of the Module's MIR stuff.
    pub strings: StringPool,
    /// Profile-guided optimization data for this module and target.
    pub profile: Option<Arc<mir::ProfileTable>>,
}

impl ModuleMir {
    /// Create a new ModuleMir.
    pub fn new(id: ModuleId, version: ModuleVersion, target: TargetId) -> Self {
        Self {
            id,
            version,

            target,
            tree: RwLock::new(mir::NodeTree::new()),
            strings: StringPool::new(),
            profile: None,
        }
    }

    /// Get the profile data, if any.
    pub fn profile(&self) -> Option<&mir::ProfileTable> {
        self.profile.as_deref()
    }

    /// Replace the profile data.
    pub fn set_profile(&mut self, profile: mir::ProfileTable) {
        self.profile = Some(Arc::new(profile));
    }

    /// Clear any profile data.
    pub fn clear_profile(&mut self) {
        self.profile = None;
    }
}
