use std::collections::{HashSet, hash_map};
use std::mem;

use destack_artifact::{
    ArtifactError, ArtifactFlush, ArtifactRecord, ArtifactStore, ArtifactVersion,
};
use destack_core::StringPool;
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashMap;

/// Process-local artifact record storage.
#[derive(Debug, Default)]
pub(crate) struct MemoryStore {
    /// Artifact records waiting for publication.
    pending: Mutex<Vec<ArtifactRecord>>,
    /// Artifact records keyed by exact version.
    records: RwLock<FxHashMap<ArtifactVersion, ArtifactRecord>>,
}

impl MemoryStore {
    /// Create one empty memory store.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl ArtifactStore for MemoryStore {
    fn load(
        &self,
        version: &ArtifactVersion,
        _strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError> {
        Ok(self.records.read().get(version).cloned())
    }

    fn store(&self, record: ArtifactRecord) -> Result<(), ArtifactError> {
        self.pending.lock().push(record);

        Ok(())
    }

    fn flush(&self, _strings: &StringPool) -> Result<ArtifactFlush, ArtifactError> {
        let pending = mem::take(&mut *self.pending.lock());
        let mut records = self.records.write();
        let mut selected = FxHashMap::default();

        // verify every pending version before mutating published state
        let mut is_conflicting = false;
        for record in &pending {
            if records
                .get(&record.version)
                .is_some_and(|existing| existing != record)
            {
                is_conflicting = true;
                break;
            }

            // reject conflicts within this pending batch
            match selected.entry(record.version) {
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(record.clone());
                }
                hash_map::Entry::Occupied(entry) if entry.get() == record => {}
                hash_map::Entry::Occupied(_) => {
                    is_conflicting = true;
                    break;
                }
            }
        }

        // restore the complete batch after any conflict
        if is_conflicting {
            drop(records);
            self.pending.lock().extend(pending);

            return Err(ArtifactError::Invalid(
                "one artifact version has conflicting records",
            ));
        }

        // publish one record per verified version
        let before = records.len();
        for (version, record) in selected {
            records.entry(version).or_insert(record);
        }
        let artifacts = records.len() - before;

        Ok(ArtifactFlush {
            artifacts,
            ..ArtifactFlush::default()
        })
    }

    fn retain(
        &self,
        versions: &HashSet<ArtifactVersion>,
        _strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        self.pending
            .lock()
            .retain(|record| versions.contains(&record.version));
        self.records
            .write()
            .retain(|version, _record| versions.contains(version));

        Ok(())
    }
}
