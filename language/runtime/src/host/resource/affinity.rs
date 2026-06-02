use serde::{Deserialize, Serialize};

use crate::host::binding::BindingAffinity;

/// Stored resource-affinity requirement for one live resource entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceAffinity {
    /// Require the owning worker context.
    Worker,
    /// Require the process main execution context.
    Main,
}

impl ResourceAffinity {
    /// Build one resource-affinity requirement from one binding affinity.
    pub const fn from_binding_affinity(affinity: BindingAffinity) -> Option<Self> {
        match affinity {
            BindingAffinity::None => None,
            BindingAffinity::Worker => Some(Self::Worker),
            BindingAffinity::Main => Some(Self::Main),
        }
    }

    /// Return the binding-affinity class represented by this resource requirement.
    pub const fn binding_affinity(self) -> BindingAffinity {
        match self {
            Self::Worker => BindingAffinity::Worker,
            Self::Main => BindingAffinity::Main,
        }
    }
}
