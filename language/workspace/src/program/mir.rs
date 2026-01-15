use std::sync::Arc;

use destack_base::StringPool;
use destack_mir::{self as mir};
use destack_source::{ModuleId, ModuleVersion};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

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

/// Serializable snapshot of ModuleMir data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMirData {
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The target this is for.
    pub target: TargetId,
    /// The MIR of the Module (may be empty initially).
    pub tree: mir::NodeTree,
    /// The string pool of the Module's MIR stuff.
    pub strings: StringPool,
    /// Profile-guided optimization data for this module and target.
    pub profile: Option<mir::ProfileTable>,
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

    /// Create a serializable snapshot of this module mir.
    pub fn to_data(&self) -> ModuleMirData {
        // snapshot module mir state
        ModuleMirData {
            id: self.id,
            version: self.version,
            target: self.target.clone(),
            tree: self.tree.read().clone(),
            strings: self.strings.clone(),
            profile: self
                .profile
                .as_ref()
                .map(|profile| profile.as_ref().clone()),
        }
    }

    /// Rebuild a ModuleMir from serialized data.
    pub fn from_data(data: ModuleMirData) -> Self {
        // rebuild module mir from snapshot
        Self {
            id: data.id,
            version: data.version,
            target: data.target,
            tree: RwLock::new(data.tree),
            strings: data.strings,
            profile: data.profile.map(Arc::new),
        }
    }
}

impl Serialize for ModuleMir {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_data().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModuleMir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = ModuleMirData::deserialize(deserializer)?;
        Ok(ModuleMir::from_data(data))
    }
}
