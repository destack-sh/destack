use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::ArtifactVersion;

use super::ArtifactTable;

/// One retained exact artifact version.
#[derive(Debug)]
pub struct ArtifactPin {
    /// The shared artifact table that owns this version.
    table: Arc<ArtifactTable>,
    /// The retained artifact version.
    version: ArtifactVersion,
}

impl ArtifactPin {
    /// Build one exact artifact pin.
    pub(crate) fn new(table: Arc<ArtifactTable>, version: ArtifactVersion) -> Self {
        Self { table, version }
    }

    /// Return the retained artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.version
    }
}

impl Drop for ArtifactPin {
    fn drop(&mut self) {
        self.table.decrease_ref_count(&self.version);
    }
}

/// One retained exact artifact version set with RAII release on drop.
#[derive(Debug)]
pub struct ArtifactPinSet {
    /// The shared artifact table that owns these pins.
    table: Arc<ArtifactTable>,
    /// The retained exact versions for one execution scope.
    pins: Mutex<HashMap<ArtifactVersion, ArtifactPin>>,
}

impl ArtifactPinSet {
    /// Build one empty pin set for one artifact table.
    pub fn new(table: Arc<ArtifactTable>) -> Self {
        Self {
            table,
            pins: Mutex::new(HashMap::new()),
        }
    }

    /// Return the shared artifact table.
    pub fn table(&self) -> &Arc<ArtifactTable> {
        &self.table
    }

    /// Return whether the pin set is empty.
    pub fn is_empty(&self) -> bool {
        self.pins.lock().is_empty()
    }

    /// Retain one exact artifact version for this scope.
    pub fn pin(&self, version: ArtifactVersion) -> bool {
        let mut pins = self.pins.lock();
        if pins.contains_key(&version) {
            return true;
        }

        let Some(pin) = self.table.pin(&version) else {
            return false;
        };
        pins.insert(version, pin);

        true
    }
}
