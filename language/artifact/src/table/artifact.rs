use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use destack_core::StringPool;
use destack_program::Program;
use destack_source::{ContentId, DiagnosticCollection};
use parking_lot::RwLock;
use rustc_hash::FxBuildHasher;

use super::entry::{
    ArtifactBinding, ArtifactBindingId, ArtifactEntry, ArtifactId, ArtifactOutcome,
    ArtifactRetention, ArtifactSidecar,
};
use super::pin::ArtifactBindingPin;
use crate::{
    ArtifactDependency, ArtifactError, ArtifactFailure, ArtifactInput, ArtifactKey,
    ArtifactPayload, ArtifactProjection, ArtifactProjectionFingerprint, ArtifactRecord,
    ArtifactResultRecord, ArtifactVersion, Asset, Build, Bundle, ComponentGraph, Data, DirBound,
    DirChecked, DirCheckedComponent, DirDeclaredComponent, DirExpanded, DirExported, DirImported,
    DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, InferenceComponentIndex,
    MirAnalyzed, MirElaborated, MirLowered, MirOptimized, MirVerified, ModuleIndex, ModuleLinted,
    Object, PackageGraph, Product, ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
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

/// Table of published artifacts and input bindings.
#[derive(Debug, Default)]
pub struct ArtifactTable {
    /// The exact artifact version entries.
    entries: DashMap<ArtifactVersion, ArtifactEntry, FxBuildHasher>,
    /// Dense ids by artifact key.
    artifact_ids: DashMap<ArtifactKey, ArtifactId, FxBuildHasher>,
    /// The next dense artifact id.
    next_artifact_id: AtomicU32,
    /// Immutable live bindings by compact id.
    bindings: DashMap<ArtifactBindingId, ArtifactBinding, FxBuildHasher>,
    /// The next immutable binding id.
    next_binding_id: AtomicU32,
    /// Binding ids by artifact input.
    inputs: DashMap<ArtifactInput, ArtifactBindingId, FxBuildHasher>,
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
        if !self.bindings.contains_key(&binding) {
            unreachable!("artifact pins can only retain published bindings: {binding:?}");
        }

        let mut retain_count = self.retained_bindings.entry(binding).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact binding with RAII release on drop.
    pub fn pin_binding(self: &Arc<Self>, binding: ArtifactBindingId) -> Option<ArtifactBindingPin> {
        let _retention = self.retention.read();
        if !self.bindings.contains_key(&binding) {
            return None;
        }

        self.retain_binding(binding);

        Some(ArtifactBindingPin::new(Arc::clone(self), binding))
    }

    /// Decrease the live reference count for one exact artifact binding.
    pub(crate) fn release_binding(&self, binding: ArtifactBindingId) {
        let Some(mut retain_count) = self.retained_bindings.get_mut(&binding) else {
            unreachable!("artifact pins can only release retained bindings: {binding:?}");
        };

        if *retain_count == 1 {
            drop(retain_count);
            self.retained_bindings.remove(&binding);
        } else {
            *retain_count -= 1;
        }
    }

    /// Retain selected and explicitly pinned artifact bindings.
    pub fn retain_bindings(
        &self,
        selected: &HashSet<ArtifactBindingId>,
    ) -> Result<ArtifactRetention, ArtifactError> {
        let _retention = self.retention.write();
        let pinned = self
            .retained_bindings
            .iter()
            .map(|entry| *entry.key())
            .collect::<HashSet<_>>();
        let mut retained = selected.clone();
        retained.extend(pinned);

        // collect the exact records and deduplicated payloads to retain
        let mut inputs = HashSet::with_capacity(retained.len());
        let mut versions = HashSet::with_capacity(retained.len());
        for binding_id in &retained {
            let binding = self.binding(*binding_id).ok_or(ArtifactError::Invalid(
                "retained artifact binding is missing",
            ))?;
            inputs.insert(binding.input);
            versions.insert(binding.version);
        }

        // release unreachable binding rows and input identities
        self.inputs
            .retain(|_input, binding| retained.contains(binding));
        self.bindings
            .retain(|binding, _entry| retained.contains(binding));

        // release payloads not produced by any retained binding
        self.entries.retain(|version, _| versions.contains(version));

        Ok(ArtifactRetention { inputs, versions })
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
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
        self.bindings.get(&id).map(|entry| entry.value().clone())
    }

    /// Return the binding id for one artifact input.
    pub fn binding_id(&self, input: &ArtifactInput) -> Option<ArtifactBindingId> {
        self.inputs.get(input).map(|entry| *entry)
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
        input: &ArtifactInput,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, crate::ArtifactError> {
        let Some(binding) = self.binding_id(input) else {
            return Ok(None);
        };
        let binding = self.binding(binding).ok_or(ArtifactError::Invalid(
            "artifact input references a missing binding",
        ))?;
        let Some(entry) = self
            .entries
            .get(&binding.version)
            .map(|entry| entry.clone())
        else {
            return Err(ArtifactError::Invalid("artifact input has no result entry"));
        };
        let Some(payload) = entry.result.payload() else {
            return Err(ArtifactError::Invalid(
                "failed artifact input cannot be persisted as a ready record",
            ));
        };

        let dependencies = binding.dependencies.iter().cloned().collect();
        let diagnostics = entry.diagnostics.as_ref().clone();
        let sidecars = entry.sidecars.iter().cloned().collect();
        let record = ArtifactRecord::new(
            *input,
            payload.as_ref(),
            strings,
            dependencies,
            diagnostics,
            sidecars,
        )?;
        if record.version() != binding.version {
            return Err(ArtifactError::VersionMismatch {
                expected: Box::new(binding.version),
                found: Box::new(record.version()),
            });
        }
        Ok(Some(record))
    }

    /// Publish one ready result without an input binding.
    pub fn publish_result(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) -> Result<(), ArtifactError> {
        if !payload.matches_key(&version.key) {
            return Err(ArtifactError::Invalid(
                "artifact payload does not match its version key",
            ));
        }

        let diagnostics = diagnostics.into();
        let sidecars = sidecars.into();
        let produced = ArtifactResultRecord::result_version(
            version.key,
            payload.as_ref(),
            diagnostics.as_ref(),
            sidecars.as_ref(),
        )?;
        if produced != version {
            return Err(ArtifactError::VersionMismatch {
                expected: Box::new(version),
                found: Box::new(produced),
            });
        }

        self.insert_result(version, payload, diagnostics, sidecars);

        Ok(())
    }

    /// Publish one ready artifact.
    pub fn publish(
        self: &Arc<Self>,
        input: ArtifactInput,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) -> Result<(ArtifactVersion, ArtifactBindingPin), ArtifactError> {
        let dependencies = dependencies.into();
        let diagnostics = diagnostics.into();
        let sidecars = sidecars.into();
        let version = ArtifactResultRecord::result_version(
            input.key,
            payload.as_ref(),
            diagnostics.as_ref(),
            sidecars.as_ref(),
        )?;

        let _retention = self.retention.read();
        self.insert_result(version, payload, diagnostics, sidecars);
        let binding = self.record_input(input, version, dependencies)?;
        self.retain_binding(binding);

        let pin = ArtifactBindingPin::new(Arc::clone(self), binding);

        Ok((version, pin))
    }

    /// Fail one artifact.
    pub fn fail(
        self: &Arc<Self>,
        input: ArtifactInput,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) -> Result<(ArtifactVersion, ArtifactBindingPin), ArtifactError> {
        let dependencies = dependencies.into();
        let diagnostics = diagnostics.into();
        let sidecars = sidecars.into();
        let version = ArtifactResultRecord::failure_version(
            input.key,
            &failure,
            diagnostics.as_ref(),
            sidecars.as_ref(),
        )?;

        let _retention = self.retention.read();
        self.entries
            .entry(version)
            .or_insert_with(|| ArtifactEntry::failed(diagnostics, sidecars, failure));
        let binding = self.record_input(input, version, dependencies)?;
        self.retain_binding(binding);

        let pin = ArtifactBindingPin::new(Arc::clone(self), binding);

        Ok((version, pin))
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
        diagnostics: Arc<DiagnosticCollection>,
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

        let entry = self.entries.get(version)?;
        let payload = entry.result.payload()?;

        payload.as_ref().fingerprint_projection(projection.key)
    }

    /// Record one deterministic artifact input result.
    fn record_input(
        &self,
        input: ArtifactInput,
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
    ) -> Result<ArtifactBindingId, ArtifactError> {
        if input.key != version.key {
            return Err(ArtifactError::Invalid(
                "artifact input key does not match its result version",
            ));
        }
        // intern the binding key and every artifact dependency owner
        self.intern_artifact_key(input.key);
        for dependency in dependencies.iter() {
            if let Some(key) = dependency.artifact_key() {
                self.intern_artifact_key(key);
            }
        }

        match self.inputs.entry(input) {
            Entry::Vacant(entry) => {
                let id = self.next_binding_id.fetch_add(1, Ordering::Relaxed);
                let id = ArtifactBindingId(id);
                self.bindings.insert(
                    id,
                    ArtifactBinding {
                        input,
                        version,
                        dependencies,
                    },
                );
                entry.insert(id);

                Ok(id)
            }
            Entry::Occupied(entry) => {
                let id = *entry.get();
                let binding = self.binding(id).ok_or(ArtifactError::Invalid(
                    "artifact input references a missing binding",
                ))?;
                if binding.version == version {
                    Ok(id)
                } else {
                    Err(ArtifactError::Nondeterministic {
                        input: Box::new(input),
                        existing: Box::new(binding.version),
                        produced: Box::new(version),
                    })
                }
            }
        }
    }

    artifact_getter!(global_environment, GlobalEnvironment, GlobalEnvironment);
    artifact_getter!(package_graph, PackageGraph, PackageGraph);
    artifact_getter!(component_graph, ComponentGraph, ComponentGraph);
    artifact_getter!(program_analysis, ProgramAnalysis, ProgramAnalysis);
    artifact_getter!(dir_parsed, DirParsed, DirParsed);
    artifact_getter!(data, Data, Data);
    artifact_getter!(dir_bound, DirBound, DirBound);
    artifact_getter!(dir_imported, DirImported, DirImported);
    artifact_getter!(dir_expanded, DirExpanded, DirExpanded);
    artifact_getter!(dir_exported, DirExported, DirExported);
    artifact_getter!(dir_resolved, DirResolved, DirResolved);
    artifact_getter!(
        dir_declared_component,
        DirDeclaredComponent,
        DirDeclaredComponent
    );
    artifact_getter!(
        dir_checked_component,
        DirCheckedComponent,
        DirCheckedComponent
    );
    artifact_getter!(dir_checked, DirChecked, DirChecked);
    artifact_getter!(dir_materialized, DirMaterialized, DirMaterialized);
    artifact_getter!(mir_lowered, MirLowered, MirLowered);
    artifact_getter!(mir_verified, MirVerified, MirVerified);
    artifact_getter!(mir_elaborated, MirElaborated, MirElaborated);
    artifact_getter!(mir_analyzed, MirAnalyzed, MirAnalyzed);
    artifact_getter!(mir_optimized, MirOptimized, MirOptimized);
    artifact_getter!(module_index, ModuleIndex, ModuleIndex);
    artifact_getter!(
        inference_component_index,
        InferenceComponentIndex,
        InferenceComponentIndex
    );
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
