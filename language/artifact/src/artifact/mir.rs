use destack_core::StringPool;
use destack_mir::{self as mir};
use destack_source::{ModuleId, TargetId};
use serde::{Deserialize, Serialize};

/// Base MIR payload before optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mir {
    /// The module id.
    pub id: ModuleId,

    /// The target id.
    pub target: TargetId,
    /// The MIR tree.
    pub tree: mir::Tree,
    /// The MIR string pool.
    pub strings: StringPool,
    /// Profile-guided optimization data for this module and target.
    pub profile: Option<mir::ProfileTable>,
}

impl Mir {
    /// Create a new base MIR payload.
    pub fn new(id: ModuleId, target: TargetId) -> Self {
        Self {
            id,
            target,
            tree: mir::Tree::new(),
            strings: StringPool::new(),
            profile: None,
        }
    }

    /// Get the profile data, if any.
    pub fn profile(&self) -> Option<&mir::ProfileTable> {
        self.profile.as_ref()
    }

    /// Replace the profile data.
    pub fn set_profile(&mut self, profile: mir::ProfileTable) {
        self.profile = Some(profile);
    }

    /// Clear any profile data.
    pub fn clear_profile(&mut self) {
        self.profile = None;
    }
}

/// Optimized MIR payload after pipeline transforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirOptimized {
    /// The module id.
    pub id: ModuleId,

    /// The target id.
    pub target: TargetId,
    /// The optimized MIR tree.
    pub tree: mir::Tree,
    /// The MIR string pool.
    pub strings: StringPool,
    /// Profile-guided optimization data for this module and target.
    pub profile: Option<mir::ProfileTable>,
}

impl MirOptimized {
    /// Create a new optimized MIR payload.
    pub fn new(id: ModuleId, target: TargetId) -> Self {
        Self {
            id,
            target,
            tree: mir::Tree::new(),
            strings: StringPool::new(),
            profile: None,
        }
    }

    /// Get the profile data, if any.
    pub fn profile(&self) -> Option<&mir::ProfileTable> {
        self.profile.as_ref()
    }

    /// Replace the profile data.
    pub fn set_profile(&mut self, profile: mir::ProfileTable) {
        self.profile = Some(profile);
    }

    /// Clear any profile data.
    pub fn clear_profile(&mut self) {
        self.profile = None;
    }
}
