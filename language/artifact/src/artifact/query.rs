use destack_qir::QueryIndex;
use serde::{Deserialize, Serialize};

/// Query index for one module profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleQueryIndex {
    /// The indexed query surface.
    pub index: QueryIndex,
}

/// Query index for one workspace profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceQueryIndex {
    /// The indexed query surface.
    pub index: QueryIndex,
}
