use std::ops::Range;
use std::sync::{Arc, Weak};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use rustc_hash::FxBuildHasher;

use crate::{
    Artifact, ArtifactDependency, ArtifactEntry, ArtifactError, ArtifactFailure,
    ArtifactInvalidation, ArtifactKey, ArtifactOutcome, ArtifactPackRecord, ArtifactPayload,
    ArtifactProjection, ArtifactProjectionFingerprint, ArtifactProjectionKey, ArtifactResult,
    ArtifactVersion, DiagnosticRecord,
};

/// Shared immutable artifact results indexed by exact version.
#[derive(Debug, Default)]
pub struct ArtifactTable {
    /// Live artifact results by exact version.
    entries: DashMap<ArtifactVersion, Weak<ArtifactEntry>, FxBuildHasher>,
    /// Live artifact results grouped by key for incremental derivation.
    candidates: DashMap<ArtifactKey, Vec<Weak<ArtifactEntry>>, FxBuildHasher>,
    /// Live dependent ranges indexed by the values they observe.
    dependents: DashMap<ArtifactInvalidation, Vec<ArtifactDependent>, FxBuildHasher>,
    /// Calculated projection fingerprints by exact artifact version.
    projections: DashMap<
        (ArtifactVersion, ArtifactProjectionKey),
        ArtifactProjectionFingerprint,
        FxBuildHasher,
    >,
}

impl ArtifactTable {
    /// Create an empty artifact table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return one live artifact result.
    pub fn entry(&self, version: &ArtifactVersion) -> Option<Arc<ArtifactEntry>> {
        self.entries.get(version)?.upgrade()
    }

    /// Return retained candidates from newest to oldest.
    pub fn candidates(&self, key: ArtifactKey) -> Vec<Arc<ArtifactEntry>> {
        let Some(candidates) = self.candidates.get(&key) else {
            return Vec::new();
        };

        candidates.iter().rev().filter_map(Weak::upgrade).collect()
    }

    /// Return one live successful result available as an incremental base.
    pub fn base(&self, key: ArtifactKey) -> Option<Arc<ArtifactEntry>> {
        let candidates = self.candidates.get(&key)?;

        candidates.iter().rev().find_map(|candidate| {
            let candidate = candidate.upgrade()?;
            matches!(candidate.outcome(), ArtifactOutcome::Ok).then_some(candidate)
        })
    }

    /// Remove lookup slots for released artifact results.
    pub fn prune(&self) {
        self.entries
            .retain(|_version, entry| entry.strong_count() > 0);
        self.candidates.retain(|_key, candidates| {
            candidates.retain(|candidate| candidate.strong_count() > 0);

            !candidates.is_empty()
        });
        self.dependents.retain(|_owner, dependents| {
            dependents.retain(|dependent| dependent.entry.strong_count() > 0);

            !dependents.is_empty()
        });
        self.projections
            .retain(|(version, _projection), _fingerprint| self.entry(version).is_some());
    }

    /// Return live artifacts that observe one dependency owner.
    pub fn dependents(
        &self,
        invalidation: ArtifactInvalidation,
    ) -> Vec<(Arc<ArtifactEntry>, Range<u32>)> {
        let Some(dependents) = self.dependents.get(&invalidation) else {
            return Vec::new();
        };

        dependents
            .iter()
            .filter_map(|dependent| {
                let entry = dependent.entry.upgrade()?;

                Some((entry, dependent.dependencies.clone()))
            })
            .collect()
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<[DiagnosticRecord]>> {
        self.entry(version).map(|entry| entry.diagnostics())
    }

    /// Return whether one exact artifact version reported an error diagnostic.
    pub fn has_errors(&self, version: &ArtifactVersion) -> Option<bool> {
        self.entry(version).map(|entry| entry.has_errors())
    }

    /// Return the exact terminal outcome for one artifact version.
    pub fn outcome(&self, version: &ArtifactVersion) -> Option<ArtifactOutcome> {
        self.entry(version).map(|entry| entry.outcome())
    }

    /// Publish one ready artifact result.
    pub fn publish(
        &self,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        payload: ArtifactPayload,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
    ) -> Result<Arc<ArtifactEntry>, ArtifactError> {
        if !payload.matches_key(&version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its result key",
            ));
        }

        let entry = Arc::new(ArtifactEntry::ok(
            version,
            dependencies,
            payload,
            diagnostics,
        ));
        let entry = self.insert(entry);

        Ok(entry)
    }

    /// Publish one failed artifact result.
    pub fn fail(
        &self,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        failure: ArtifactFailure,
    ) -> Arc<ArtifactEntry> {
        let entry = Arc::new(ArtifactEntry::failed(
            version,
            dependencies,
            diagnostics,
            failure,
        ));

        self.insert(entry)
    }

    /// Return the successful payload for one artifact version.
    pub fn payload(
        &self,
        version: &ArtifactVersion,
    ) -> Result<Option<ArtifactPayload>, ArtifactError> {
        let Some(entry) = self.entry(version) else {
            return Ok(None);
        };
        let payload = match &entry.result {
            ArtifactResult::Ok(payload) => payload.clone(),
            ArtifactResult::Cached(record) => record.payload()?,
            ArtifactResult::Failed(_failure) => return Ok(None),
        };
        if !payload.matches_key(&version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its result key",
            ));
        }

        Ok(Some(payload))
    }

    /// Return one typed artifact payload.
    pub fn artifact<A: Artifact>(
        &self,
        version: &ArtifactVersion,
    ) -> Result<Option<Arc<A>>, ArtifactError> {
        let Some(payload) = self.payload(version)? else {
            return Ok(None);
        };

        Ok(A::from_payload(payload))
    }

    /// Return the stable fingerprint of one projected artifact value.
    pub fn projection_fingerprint(
        &self,
        version: &ArtifactVersion,
        projection: &ArtifactProjection,
    ) -> Result<Option<ArtifactProjectionFingerprint>, ArtifactError> {
        if version.key != projection.artifact {
            return Ok(None);
        }

        if self.entry(version).is_none() {
            return Ok(None);
        }
        let projection_key = (*version, projection.key);
        if let Some(fingerprint) = self.projections.get(&projection_key) {
            return Ok(Some(*fingerprint));
        }

        // calculate each requested projection at most once
        let Some(payload) = self.payload(version)? else {
            return Ok(None);
        };
        let Some(fingerprint) = payload.as_ref().fingerprint_projection(projection.key)? else {
            return Ok(None);
        };
        let fingerprint = *self
            .projections
            .entry(projection_key)
            .or_insert(fingerprint);

        Ok(Some(fingerprint))
    }

    /// Restore one encoded successful artifact record.
    pub(crate) fn restore(
        &self,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        record: ArtifactPackRecord,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
    ) -> Arc<ArtifactEntry> {
        let entry = Arc::new(ArtifactEntry::cached(
            version,
            dependencies,
            record,
            diagnostics,
        ));

        self.insert(entry)
    }

    /// Intern one exact immutable artifact result.
    fn insert(&self, entry: Arc<ArtifactEntry>) -> Arc<ArtifactEntry> {
        match self.entries.entry(entry.version) {
            Entry::Occupied(mut occupied) => {
                let Some(existing) = occupied.get().upgrade() else {
                    occupied.insert(Arc::downgrade(&entry));
                    self.index(&entry);

                    return entry;
                };

                existing
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Arc::downgrade(&entry));
                self.index(&entry);

                entry
            }
        }
    }

    /// Index one newly interned artifact result.
    fn index(&self, entry: &Arc<ArtifactEntry>) {
        self.candidates
            .entry(entry.version.key)
            .or_default()
            .push(Arc::downgrade(entry));
        self.record_dependents(entry);
    }

    /// Index adjacent dependency ranges by their invalidation owner.
    fn record_dependents(&self, entry: &Arc<ArtifactEntry>) {
        let mut first = 0;

        while first < entry.dependencies.len() {
            let invalidation = entry.dependencies[first].invalidation();
            let mut last = first + 1;
            while last < entry.dependencies.len()
                && entry.dependencies[last].invalidation() == invalidation
            {
                last += 1;
            }

            self.dependents
                .entry(invalidation)
                .or_default()
                .push(ArtifactDependent {
                    entry: Arc::downgrade(entry),
                    dependencies: first as u32..last as u32,
                });
            first = last;
        }
    }
}

/// One live artifact range observing the same dependency owner.
#[derive(Debug, Clone)]
struct ArtifactDependent {
    /// The dependent artifact result.
    entry: Weak<ArtifactEntry>,
    /// Adjacent dependency ordinals observing the owner.
    dependencies: Range<u32>,
}
