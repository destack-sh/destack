use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use destack_core::StringPool;
use destack_program::Program;
use destack_source::ContentId;

use crate::DiagnosticRecord;
use parking_lot::RwLock;
use rustc_hash::FxBuildHasher;

use super::binding::ArtifactBindingIndex;
use super::dependency::{ArtifactDependencyOwner, ArtifactDependent};
use super::entry::{
    ArtifactBinding, ArtifactBindingId, ArtifactEntry, ArtifactId, ArtifactOutcome, ArtifactSidecar,
};
use super::pin::ArtifactBindingPin;
use crate::{
    ArtifactDependency, ArtifactError, ArtifactFailure, ArtifactKey, ArtifactPayload,
    ArtifactProjection, ArtifactProjectionFingerprint, ArtifactProjectionKey, ArtifactRecord,
    ArtifactVersion, Asset, Build, Bundle, Data, DirBound, DirChecked, DirDeclared, DirExpanded,
    DirExported, DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment,
    MirAnalyzed, MirElaborated, MirLowered, MirOptimized, MirVerified, ModuleGraph, ModuleIndex,
    ModuleLinted, Object, Product, ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
};

macro_rules! artifact_getter {
    ($name:ident, $type:ty, $variant:ident) => {
        /// Get one typed artifact payload.
        pub fn $name(&self, version: &ArtifactVersion) -> Option<Arc<$type>> {
            match self.payload(version)? {
                ArtifactPayload::$variant(payload) => Some(payload),
                _ => None,
            }
        }
    };
}

/// Table of published artifact results and exact bindings.
#[derive(Debug, Default)]
pub struct ArtifactTable {
    /// Reusable artifact results by version.
    entries: DashMap<ArtifactVersion, ArtifactEntry, FxBuildHasher>,
    /// Dense ids by artifact key.
    artifact_ids: DashMap<ArtifactKey, ArtifactId, FxBuildHasher>,
    /// The next dense artifact id.
    next_artifact_id: AtomicU32,

    /// Immutable live bindings by compact id.
    bindings: ArtifactBindingIndex,
    /// Exact binding ids grouped by reusable artifact version.
    bindings_by_version: DashMap<ArtifactVersion, Vec<ArtifactBindingId>, FxBuildHasher>,
    /// Every published binding of one artifact key, in publish order.
    bindings_by_artifact: DashMap<ArtifactId, Vec<ArtifactBindingId>, FxBuildHasher>,

    /// Immutable binding dependencies indexed by their owners.
    dependents: DashMap<ArtifactDependencyOwner, Vec<ArtifactDependent>, FxBuildHasher>,

    /// Projection fingerprints calculated for observed artifact values.
    projections: DashMap<
        (ArtifactVersion, ArtifactProjectionKey),
        ArtifactProjectionFingerprint,
        FxBuildHasher,
    >,

    /// The live retain count for each exact artifact binding.
    retained_bindings: DashMap<ArtifactBindingId, usize, FxBuildHasher>,
    /// Synchronizes binding retention with pruning.
    retention: RwLock<()>,
}

impl ArtifactTable {
    /// Create an artifact table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase the live reference count for one exact artifact binding.
    fn retain_binding(&self, binding: ArtifactBindingId) {
        if !self.bindings.contains(binding) {
            unreachable!("artifact pins can only retain published bindings: {binding:?}");
        }

        let mut retain_count = self.retained_bindings.entry(binding).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact binding with RAII release on drop.
    pub fn pin_binding(self: &Arc<Self>, binding: ArtifactBindingId) -> Option<ArtifactBindingPin> {
        let _retention = self.retention.read();
        if !self.bindings.contains(binding) {
            return None;
        }

        self.retain_binding(binding);

        Some(ArtifactBindingPin::new(Arc::clone(self), binding))
    }

    /// Decrease the live reference count for one exact artifact binding.
    pub(crate) fn release_binding(&self, binding: ArtifactBindingId) {
        // drop or decrement the retain count under the entry lock
        match self.retained_bindings.entry(binding) {
            Entry::Occupied(mut retained) => {
                if *retained.get() == 1 {
                    retained.remove();
                } else {
                    *retained.get_mut() -= 1;
                }
            }
            Entry::Vacant(_) => {
                unreachable!("artifact pins can only release retained bindings: {binding:?}")
            }
        }
    }

    /// Retain selected and explicitly pinned artifact bindings.
    pub fn retain_bindings(
        &self,
        selected: &HashSet<ArtifactBindingId>,
    ) -> Result<HashSet<ArtifactVersion>, ArtifactError> {
        let _retention = self.retention.write();
        let pinned = self
            .retained_bindings
            .iter()
            .map(|entry| *entry.key())
            .collect::<HashSet<_>>();
        let mut retained = selected.clone();
        retained.extend(pinned);

        // collect the exact artifact versions to retain
        let mut versions = HashSet::with_capacity(retained.len());
        for binding_id in &retained {
            let binding = self.binding(*binding_id).ok_or(ArtifactError::Invalid(
                "retained artifact binding is missing",
            ))?;
            versions.insert(binding.version);
        }

        // release unreachable binding rows and version indexes
        self.bindings_by_version.retain(|_version, bindings| {
            bindings.retain(|binding| retained.contains(binding));

            !bindings.is_empty()
        });
        self.bindings.retain(&retained);
        self.dependents.retain(|_owner, entries| {
            entries.retain(|dependent| retained.contains(&dependent.binding));

            !entries.is_empty()
        });

        // release payloads not produced by any retained binding
        self.entries.retain(|version, _| versions.contains(version));
        self.projections
            .retain(|(version, _key), _fingerprint| versions.contains(version));

        Ok(versions)
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<[DiagnosticRecord]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Intern one artifact key into a dense process-local id.
    pub fn intern_artifact_key(&self, key: ArtifactKey) -> ArtifactId {
        match self.artifact_ids.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let id = self.next_artifact_id.fetch_add(1, Ordering::Relaxed);
                let id = ArtifactId(id);
                entry.insert(id);

                id
            }
        }
    }

    /// Return the dense id for one interned artifact key.
    pub fn artifact_id(&self, key: ArtifactKey) -> Option<ArtifactId> {
        self.artifact_ids.get(&key).map(|entry| *entry)
    }

    /// Return one immutable artifact binding.
    pub fn binding(&self, id: ArtifactBindingId) -> Option<ArtifactBinding> {
        self.bindings.get(id)
    }

    /// Return binding dependencies that observe one owner.
    pub fn dependents(&self, owner: ArtifactDependencyOwner) -> Vec<ArtifactDependent> {
        match self.dependents.get(&owner) {
            Some(dependents) => dependents.clone(),
            None => Vec::new(),
        }
    }

    /// Return the recorded sidecars for one exact artifact version.
    pub fn sidecars(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactSidecar]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.sidecars))
    }

    /// Return the exact terminal outcome for one artifact version.
    pub fn outcome(&self, version: &ArtifactVersion) -> Option<ArtifactOutcome> {
        self.entries
            .get(version)
            .map(|entry| entry.result.outcome())
    }

    /// Return one self-contained artifact record.
    pub fn record(
        &self,
        version: ArtifactVersion,
        dependencies: &[ArtifactDependency],
        strings: &StringPool,
    ) -> Result<ArtifactRecord, ArtifactError> {
        let Some(entry) = self.entries.get(&version).map(|entry| entry.clone()) else {
            return Err(ArtifactError::Invalid(
                "artifact record has no result entry",
            ));
        };
        let Some(payload) = entry.result.payload() else {
            return Err(ArtifactError::Invalid(
                "failed artifact cannot be persisted as a ready record",
            ));
        };

        let dependencies = dependencies.to_vec();
        let diagnostics = entry.diagnostics.to_vec();
        let sidecars = entry.sidecars.iter().cloned().collect();
        let record = ArtifactRecord::new(
            version,
            payload.as_ref(),
            strings,
            dependencies,
            diagnostics,
            sidecars,
        )?;

        Ok(record)
    }

    /// Publish one ready result and its exact binding.
    pub fn publish(
        self: &Arc<Self>,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) -> Result<ArtifactBindingPin, ArtifactError> {
        if !payload.matches_key(&version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its result key",
            ));
        }

        let dependencies = dependencies.into();
        let diagnostics = diagnostics.into();
        let sidecars = sidecars.into();
        let _retention = self.retention.read();
        self.insert_result(version, payload, diagnostics, sidecars);
        let binding = self.intern_binding(version, dependencies)?;
        self.retain_binding(binding);

        Ok(ArtifactBindingPin::new(Arc::clone(self), binding))
    }

    /// Publish one exact binding for an existing result.
    pub fn publish_binding(
        self: &Arc<Self>,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
    ) -> Result<ArtifactBindingPin, ArtifactError> {
        let _retention = self.retention.read();
        if self.outcome(&version).is_none() {
            return Err(ArtifactError::Invalid(
                "artifact binding references a missing result",
            ));
        }

        let dependencies = dependencies.into();
        let binding = self.intern_binding(version, dependencies)?;
        self.retain_binding(binding);

        let pin = ArtifactBindingPin::new(Arc::clone(self), binding);

        Ok(pin)
    }

    /// Fail one artifact.
    pub fn fail(
        self: &Arc<Self>,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) -> Result<ArtifactBindingPin, ArtifactError> {
        let dependencies = dependencies.into();
        let diagnostics = diagnostics.into();
        let sidecars = sidecars.into();
        let _retention = self.retention.read();
        self.entries
            .entry(version)
            .or_insert_with(|| ArtifactEntry::failed(diagnostics, sidecars, failure));
        let binding = self.intern_binding(version, dependencies)?;
        self.retain_binding(binding);

        let pin = ArtifactBindingPin::new(Arc::clone(self), binding);

        Ok(pin)
    }

    /// Return the successful payload for one artifact version.
    pub fn payload(&self, version: &ArtifactVersion) -> Option<ArtifactPayload> {
        let entry = self.entries.get(version)?;

        entry.result.payload()
    }

    /// Insert one already fingerprinted ready result.
    fn insert_result(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        diagnostics: Arc<[DiagnosticRecord]>,
        sidecars: Arc<[ArtifactSidecar]>,
    ) {
        self.entries
            .entry(version)
            .or_insert_with(|| ArtifactEntry::ok(payload, diagnostics, sidecars));
    }

    /// Return the stable fingerprint of one projected artifact value.
    pub fn projection_fingerprint(
        &self,
        version: &ArtifactVersion,
        projection: &ArtifactProjection,
    ) -> Option<ArtifactProjectionFingerprint> {
        if version.key != projection.artifact {
            return None;
        }

        let identity = (*version, projection.key);
        if let Some(fingerprint) = self.projections.get(&identity) {
            return Some(*fingerprint);
        }

        // calculate each requested projection at most once
        let entry = self.entries.get(version)?;
        let payload = entry.result.payload()?;
        let fingerprint = payload.as_ref().fingerprint_projection(projection.key)?;
        let fingerprint = self.projections.entry(identity).or_insert(fingerprint);

        Some(*fingerprint)
    }

    /// Return every published binding of one artifact, newest first.
    pub fn artifact_bindings(&self, artifact: ArtifactId) -> Vec<ArtifactBindingId> {
        let Some(bindings) = self.bindings_by_artifact.get(&artifact) else {
            return Vec::new();
        };

        bindings.iter().rev().copied().collect()
    }

    /// Return the newest published binding of one artifact.
    pub fn latest_artifact_binding(&self, artifact: ArtifactId) -> Option<ArtifactBindingId> {
        let bindings = self.bindings_by_artifact.get(&artifact)?;

        bindings.last().copied()
    }

    /// Intern one exact artifact binding.
    fn intern_binding(
        &self,
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
    ) -> Result<ArtifactBindingId, ArtifactError> {
        // intern the artifact selected by this binding
        let artifact = self.intern_artifact_key(version.key);
        let mut bindings = self.bindings_by_version.entry(version).or_default();

        // reuse only a binding with the same exact dependency observations
        for binding_id in bindings.iter().copied() {
            let binding = self.binding(binding_id).ok_or(ArtifactError::Invalid(
                "artifact version references a missing binding",
            ))?;
            if binding.dependencies.as_ref() == dependencies.as_ref() {
                return Ok(binding_id);
            }
        }

        // publish one new exact binding and its reverse dependency edges
        let binding = ArtifactBinding {
            version,
            dependencies,
        };
        let id = self.bindings.insert(binding.clone())?;
        self.bindings_by_artifact
            .entry(artifact)
            .or_default()
            .push(id);
        self.record_dependents(artifact, id, &binding);
        bindings.push(id);

        Ok(id)
    }

    /// Index every dependency of one immutable artifact binding.
    fn record_dependents(
        &self,
        artifact: ArtifactId,
        binding_id: ArtifactBindingId,
        binding: &ArtifactBinding,
    ) {
        let mut first = 0;

        // append adjacent edges for one owner under one shard lock
        while first < binding.dependencies.len() {
            let owner = ArtifactDependencyOwner::from(&binding.dependencies[first]);
            let mut last = first + 1;
            while last < binding.dependencies.len()
                && ArtifactDependencyOwner::from(&binding.dependencies[last]) == owner
            {
                last += 1;
            }

            let mut dependents = self.dependents.entry(owner).or_default();
            dependents.extend((first..last).map(|dependency| ArtifactDependent {
                artifact,
                binding: binding_id,
                dependency: dependency as u32,
            }));
            first = last;
        }
    }

    artifact_getter!(global_environment, GlobalEnvironment, GlobalEnvironment);
    artifact_getter!(module_graph, ModuleGraph, ModuleGraph);
    artifact_getter!(program_analysis, ProgramAnalysis, ProgramAnalysis);
    artifact_getter!(dir_parsed, DirParsed, DirParsed);
    artifact_getter!(data, Data, Data);
    artifact_getter!(dir_bound, DirBound, DirBound);
    artifact_getter!(dir_imported, DirImported, DirImported);
    artifact_getter!(dir_expanded, DirExpanded, DirExpanded);
    artifact_getter!(dir_exported, DirExported, DirExported);
    artifact_getter!(dir_resolved, DirResolved, DirResolved);
    artifact_getter!(dir_declared, DirDeclared, DirDeclared);
    artifact_getter!(dir_checked, DirChecked, DirChecked);
    artifact_getter!(dir_materialized, DirMaterialized, DirMaterialized);
    artifact_getter!(mir_lowered, MirLowered, MirLowered);
    artifact_getter!(mir_verified, MirVerified, MirVerified);
    artifact_getter!(mir_elaborated, MirElaborated, MirElaborated);
    artifact_getter!(mir_analyzed, MirAnalyzed, MirAnalyzed);
    artifact_getter!(mir_optimized, MirOptimized, MirOptimized);
    artifact_getter!(module_index, ModuleIndex, ModuleIndex);
    artifact_getter!(program_index, ProgramIndex, ProgramIndex);
    artifact_getter!(script, Script, Script);
    artifact_getter!(object, Object, Object);
    artifact_getter!(asset, Asset, Asset);
    artifact_getter!(build, Build, Build);
    artifact_getter!(bundle, Bundle, Bundle);
    artifact_getter!(program, Program, Program);
    artifact_getter!(product, Product, Product);
    artifact_getter!(module_linted, ModuleLinted, ModuleLinted);
    artifact_getter!(program_linted, ProgramLinted, ProgramLinted);

    /// Return content ids referenced by one exact artifact payload.
    pub fn content_ids(&self, version: &ArtifactVersion) -> Option<Vec<ContentId>> {
        let entry = self.entries.get(version)?;
        let contents = match entry.result.payload() {
            Some(payload) => payload.content_ids(),
            None => Vec::new(),
        };

        Some(contents)
    }
}
