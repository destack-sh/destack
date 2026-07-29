use std::sync::Arc;

use crate::ArtifactBindingId;

use super::ArtifactTable;

/// One retained exact artifact binding.
#[derive(Debug)]
pub struct ArtifactBindingPin {
    /// The shared artifact table that owns this binding.
    table: Arc<ArtifactTable>,
    /// The retained artifact binding.
    binding: ArtifactBindingId,
}

impl ArtifactBindingPin {
    /// Build one exact artifact binding pin.
    pub(crate) fn new(table: Arc<ArtifactTable>, binding: ArtifactBindingId) -> Self {
        Self { table, binding }
    }

    /// Return the retained artifact binding.
    pub const fn binding(&self) -> ArtifactBindingId {
        self.binding
    }
}

impl Drop for ArtifactBindingPin {
    fn drop(&mut self) {
        self.table.release_binding(self.binding);
    }
}
