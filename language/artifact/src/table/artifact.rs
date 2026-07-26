use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use dashmap::DashMap;
use destack_core::StringPool;
use destack_program::Program;
use destack_source::{ContentId, DiagnosticCollection, FileId};
use rustc_hash::FxBuildHasher;

use super::entry::{ArtifactEntry, ArtifactOutcome, ArtifactSidecar};
use super::pin::ArtifactPin;
use crate::{
    ArtifactDependency, ArtifactFailure, ArtifactPayload, ArtifactProjection,
    ArtifactProjectionFingerprint, ArtifactRecord, ArtifactVersion, Asset, Build, Bundle,
    ComponentGraph, Data, DirBound, DirChecked, DirCheckedComponent, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, MirAnalyzed,
    MirElaborated, MirLowered, MirOptimized, MirVerified, ModuleIndex, ModuleLinted, Object,
    PackageGraph, Product, ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
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

/// Table of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactTable {
    /// The exact artifact version entries.
    entries: DashMap<ArtifactVersion, ArtifactEntry, FxBuildHasher>,
    /// The live retain count for each exact artifact version.
    retained_versions: DashMap<ArtifactVersion, usize, FxBuildHasher>,
}

impl ArtifactTable {
    /// Create a new semantic artifact table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase the live reference count for one exact artifact version.
    pub(crate) fn increase_ref_count(&self, version: &ArtifactVersion) {
        if !self.exists(version) {
            unreachable!("artifact pins can only retain published versions: {version:?}");
        }

        let mut retain_count = self.retained_versions.entry(*version).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact version with RAII release on drop.
    pub fn pin(self: &Arc<Self>, version: &ArtifactVersion) -> Option<ArtifactPin> {
        if !self.exists(version) {
            return None;
        }

        self.increase_ref_count(version);

        Some(ArtifactPin::new(Arc::clone(self), *version))
    }

    /// Decrease the live reference count for one exact artifact version.
    pub(crate) fn decrease_ref_count(&self, version: &ArtifactVersion) {
        let Some(mut retain_count) = self.retained_versions.get_mut(version) else {
            unreachable!("artifact pins can only release retained versions: {version:?}");
        };

        if *retain_count == 1 {
            drop(retain_count);
            self.retained_versions.remove(version);
        } else {
            *retain_count -= 1;
        }
    }

    /// Return all exact artifact versions retained by live pins.
    pub fn retained_versions(&self) -> Vec<ArtifactVersion> {
        self.retained_versions
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Retain reachable and explicitly pinned artifact versions.
    pub fn retain_reachable(&self, reachable: &HashSet<ArtifactVersion>) {
        let retained = self
            .retained_versions
            .iter()
            .map(|entry| *entry.key())
            .collect::<HashSet<_>>();
        let reachable = self.reachable_closure(reachable.iter().chain(retained.iter()).copied());

        // retain reachable and explicitly pinned entries
        self.entries
            .retain(|version, _| reachable.contains(version));
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Return the exact dependencies for one artifact version.
    pub fn dependencies(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactDependency]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.dependencies))
    }

    /// Return the transitive source files for one artifact version.
    pub fn sources(&self, version: &ArtifactVersion) -> Option<Arc<[FileId]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.sources))
    }

    /// Return the predecessor artifact for one exact artifact version.
    pub fn base(&self, version: &ArtifactVersion) -> Option<ArtifactVersion> {
        self.entries.get(version).and_then(|entry| entry.base)
    }

    /// Return roots plus artifact dependencies and base artifacts needed to load them.
    pub fn reachable_closure(
        &self,
        roots: impl IntoIterator<Item = ArtifactVersion>,
    ) -> HashSet<ArtifactVersion> {
        let mut reachable = HashSet::new();
        let mut pending = roots.into_iter().collect::<VecDeque<_>>();

        // walk artifact records that must remain loadable with each root
        while let Some(version) = pending.pop_front() {
            if !reachable.insert(version) {
                continue;
            }
            let Some(entry) = self.entries.get(&version) else {
                continue;
            };

            if let Some(base) = entry.base {
                pending.push_back(base);
            }
            for dependency in entry.dependencies.iter() {
                match dependency {
                    ArtifactDependency::Artifact(dependency) => {
                        pending.push_back(*dependency);
                    }
                    ArtifactDependency::Projection(dependency) => {
                        pending.push_back(dependency.version);
                    }
                    ArtifactDependency::Source(_dependency) => {}
                }
            }
        }

        reachable
    }

    /// Return the recorded sidecars for one exact artifact version.
    pub fn sidecars(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactSidecar]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.sidecars))
    }

    /// Return whether one exact artifact version entry exists.
    fn exists(&self, version: &ArtifactVersion) -> bool {
        self.entries.contains_key(version)
    }

    /// Return the exact terminal outcome for one artifact version.
    pub fn outcome(&self, version: &ArtifactVersion) -> Option<ArtifactOutcome> {
        self.entries
            .get(version)
            .map(|entry| entry.result.outcome())
    }

    /// Return the provider failure for one exact artifact version.
    pub fn failure(&self, version: &ArtifactVersion) -> Option<ArtifactFailure> {
        match self.outcome(version) {
            Some(ArtifactOutcome::Failed(failure)) => Some(failure),
            Some(ArtifactOutcome::Ok) | None => None,
        }
    }

    /// Return whether one exact artifact payload is published.
    pub fn has(&self, version: &ArtifactVersion) -> bool {
        self.payload(version).is_some()
    }

    /// Return one self-contained artifact record.
    pub fn record(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, crate::ArtifactStoreError> {
        let Some(entry) = self.entries.get(version).map(|entry| entry.clone()) else {
            return Ok(None);
        };
        let Some(payload) = entry.result.payload() else {
            return Ok(None);
        };

        let dependencies = entry.dependencies.iter().cloned().collect();
        let sources = entry.sources.iter().copied().collect();
        let diagnostics = entry.diagnostics.as_ref().clone();
        let sidecars = entry.sidecars.iter().cloned().collect();
        let record = ArtifactRecord::new(
            *version,
            entry.base,
            payload.as_ref(),
            strings,
            dependencies,
            sources,
            diagnostics,
            sidecars,
        )?;

        Ok(Some(record))
    }

    /// Publish one ready artifact.
    pub fn publish(
        &self,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        sources: impl Into<Arc<[FileId]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) {
        if !payload.matches_key(&version.key) {
            unreachable!(
                "artifact payload did not match key: key={:?} payload={}",
                version.key,
                payload.name()
            );
        }

        let dependencies = dependencies.into();
        let sources = sources.into();

        self.entries.insert(
            version,
            ArtifactEntry::ok(base, payload, dependencies, sources, diagnostics, sidecars),
        );
    }

    /// Fail one artifact.
    pub fn fail(
        &self,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        sources: impl Into<Arc<[FileId]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) {
        let dependencies = dependencies.into();
        let sources = sources.into();

        self.entries.insert(
            version,
            ArtifactEntry::failed(base, dependencies, sources, diagnostics, sidecars, failure),
        );
    }

    /// Return the successful payload for one artifact version.
    fn payload(&self, version: &ArtifactVersion) -> Option<ArtifactPayload> {
        let entry = self.entries.get(version)?;

        entry.result.payload()
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

        self.payload(version)?
            .projection_fingerprint(projection.key)
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
    pub fn content_ids(&self, version: &ArtifactVersion) -> Vec<ContentId> {
        self.payload(version)
            .map(|payload| payload.content_ids())
            .unwrap_or_default()
    }
}
