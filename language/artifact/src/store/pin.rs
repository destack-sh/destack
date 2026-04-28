use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ArtifactVersion;

use super::store::ArtifactStore;

/// One retained exact artifact version.
#[derive(Debug)]
pub struct ArtifactPin {
    /// The shared artifact store that owns this version.
    store: Arc<ArtifactStore>,
    /// The retained artifact version.
    version: ArtifactVersion,
}

impl ArtifactPin {
    /// Build one exact artifact pin.
    pub(crate) fn new(store: Arc<ArtifactStore>, version: ArtifactVersion) -> Self {
        Self { store, version }
    }

    /// Return the retained artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.version
    }
}

impl Drop for ArtifactPin {
    fn drop(&mut self) {
        self.store.decrease_ref_count(&self.version);
    }
}

/// One retained exact artifact version set with RAII release on drop.
#[derive(Debug)]
pub struct ArtifactPinSet {
    /// The shared artifact store that owns these pins.
    store: Arc<ArtifactStore>,
    /// The retained exact versions for one execution scope.
    pins: Mutex<HashMap<ArtifactVersion, ArtifactPin>>,
}

impl ArtifactPinSet {
    /// Build one empty pin set for one artifact store.
    pub fn new(store: Arc<ArtifactStore>) -> Self {
        Self {
            store,
            pins: Mutex::new(HashMap::new()),
        }
    }

    /// Return the shared artifact store.
    pub fn store(&self) -> &Arc<ArtifactStore> {
        &self.store
    }

    /// Return whether the pin set is empty.
    pub fn is_empty(&self) -> bool {
        self.pins
            .lock()
            .unwrap_or_else(|_| panic!("artifact pin set should not be poisoned"))
            .is_empty()
    }

    /// Retain one exact artifact version for this scope.
    pub fn pin(&self, version: ArtifactVersion) {
        let mut pins = self
            .pins
            .lock()
            .unwrap_or_else(|_| panic!("artifact pin set should not be poisoned"));

        if pins.contains_key(&version) {
            return;
        }

        let pin = self
            .store
            .pin(&version)
            .unwrap_or_else(|| panic!("missing artifact version to pin: {version:?}"));

        pins.insert(version, pin);
    }
}
