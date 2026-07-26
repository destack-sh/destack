use std::collections::BTreeMap;
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactFlush, ArtifactKey, ArtifactPayload,
    ArtifactProjectionDependency, ArtifactSidecar, ArtifactStore, ArtifactTable, ArtifactVersion,
    ModuleSetFingerprint, PackageSetFingerprint, SourceDependency,
};
use destack_source::{DiagnosticCollection, FileId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ArtifactBase;
use crate::artifact::ArtifactReader;
use crate::repository::{Repository, RepositoryError, Revision};

/// Repository-owned artifact state tables and persistent store.
#[derive(Debug)]
pub(crate) struct Artifacts {
    /// Derived artifact versions already resolved for revision/key pairs.
    pub(crate) versions: DashMap<(Revision, ArtifactKey), ArtifactVersion>,
    /// Shared typed derived artifact table.
    pub(crate) table: Arc<ArtifactTable>,
    /// Persistent artifact record store.
    pub(crate) store: Arc<dyn ArtifactStore>,
}

impl Artifacts {
    /// Create empty artifact state tables over one persistent store.
    pub(crate) fn new(store: Arc<dyn ArtifactStore>) -> Self {
        Self {
            versions: DashMap::new(),
            table: Arc::new(ArtifactTable::default()),
            store,
        }
    }

    /// Replace the persistent artifact store.
    pub(crate) fn set_store(&mut self, store: Arc<dyn ArtifactStore>) {
        self.store = store;
    }
}

impl Repository {
    /// Return a read-only artifact reader for one pinned revision.
    pub fn artifact_reader(&self, revision: Revision) -> ArtifactReader<'_> {
        ArtifactReader::new(self, revision)
    }

    /// Return the fresh version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let versions = self.artifact_versions(revision, std::slice::from_ref(artifact_key))?;

        Ok(versions.into_iter().next().flatten())
    }

    /// Return the exact stored binding for one revision-scoped artifact key.
    pub fn artifact_binding(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let _revision = self.revision(revision)?;

        Ok(self.exact_artifact_binding(revision, artifact_key))
    }

    /// Return fresh versions for revision-scoped artifact keys.
    pub fn artifact_versions(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<Option<ArtifactVersion>>, RepositoryError> {
        let _revision = self.revision(revision)?;
        let mut versions = Vec::with_capacity(artifact_keys.len());
        let mut missing = FxHashSet::default();

        // read exact revision bindings
        for artifact_key in artifact_keys {
            if let Some(version) = self.exact_artifact_binding(revision, artifact_key) {
                versions.push(Some(version));
            } else {
                versions.push(None);
                missing.insert(*artifact_key);
            }
        }

        if missing.is_empty() {
            return Ok(versions);
        }

        // reuse nearest fresh predecessor bindings
        let fresh = self.fresh_artifact_versions(revision, &missing)?;

        // fill missing entries from fresh predecessors
        for (index, artifact_key) in artifact_keys.iter().enumerate() {
            if versions[index].is_none()
                && let Some(version) = fresh.get(artifact_key).copied()
            {
                versions[index] = Some(version);
            }
        }

        Ok(versions)
    }

    /// Return the nearest same-key predecessor artifact visible through revision bases.
    pub fn artifact_base(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<ArtifactBase>, RepositoryError> {
        let _revision = self.revision(revision)?;

        // nearest bases are tried first, preserving base order
        for ancestor in self.revision_ancestors(revision)? {
            if let Some(version) = self.exact_artifact_binding(ancestor, &artifact_key) {
                return Ok(Some(ArtifactBase::new(ancestor, version)));
            }
        }

        Ok(None)
    }

    /// Bind one already-stored artifact version to one revision.
    pub fn bind_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // require a terminal outcome before binding the version
        if self.artifact_table().outcome(&version).is_none() {
            return Err(RepositoryError::MissingArtifact { version });
        }

        self.bind_artifact_version(revision, version);

        Ok(())
    }

    /// Load one ready artifact from the persistent artifact store when present.
    pub fn load_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<bool, RepositoryError> {
        let Some(record) = self
            .artifact_store()
            .load(&version, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Ok(false);
        };
        let payload = record
            .decode_payload()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        self.publish_ready_artifact(
            revision,
            record.version,
            record.base,
            payload,
            record.dependencies,
            record.sources,
            record.diagnostics,
            record.sidecars,
        )?;

        Ok(true)
    }

    /// Complete one ready artifact by publishing and storing it.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        self.publish_artifact(
            revision,
            version,
            base,
            payload,
            dependencies,
            diagnostics,
            sidecars,
        )?;
        self.store_artifact(version)?;

        Ok(())
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn publish_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let sources = self.artifact_sources(base, &dependencies)?;

        self.publish_ready_artifact(
            revision,
            version,
            base,
            payload,
            dependencies,
            sources,
            diagnostics,
            sidecars,
        )
    }

    /// Store one ready artifact in the persistent artifact store.
    pub fn store_artifact(&self, version: ArtifactVersion) -> Result<(), RepositoryError> {
        self.artifact_store()
            .store(&version, self.artifact_table(), self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Flush pending artifacts into the persistent artifact store.
    pub fn flush_artifacts(&self) -> Result<ArtifactFlush, RepositoryError> {
        let flush = self
            .artifact_store()
            .flush(self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(flush)
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;
        let sources = self.artifact_sources(base, &dependencies)?;

        // store failure before exposing the revision binding
        self.artifact_table().fail(
            version,
            base,
            dependencies,
            sources,
            diagnostics,
            sidecars,
            failure,
        );
        self.bind_artifact_version(revision, version);

        Ok(())
    }

    /// Return diagnostics for one revision, optionally filtered to one artifact key.
    pub fn diagnostics(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let _revision = self.revision(revision)?;
        let mut diagnostics = DiagnosticCollection::new();

        // exact artifact
        if let Some(artifact_key) = artifact_key {
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                return Ok(diagnostics);
            };
            let artifact_diagnostics = self.artifact_diagnostics(version)?;
            diagnostics.merge_from(artifact_diagnostics.as_ref());

            return Ok(diagnostics);
        }

        let versions = self
            .artifacts
            .versions
            .iter()
            .filter_map(|entry| {
                let ((entry_revision, _key), version) = entry.pair();

                (*entry_revision == revision).then_some(*version)
            })
            .collect::<Vec<_>>();

        // all fresh artifacts in this revision
        for version in versions {
            let Some(fresh_version) = self.artifact_version(revision, &version.key)? else {
                continue;
            };
            if fresh_version != version {
                continue;
            }

            let artifact_diagnostics = self.artifact_diagnostics(version)?;
            diagnostics.merge_from(artifact_diagnostics.as_ref());
        }

        Ok(diagnostics)
    }

    /// Return diagnostics for requested roots and everything they were built from.
    ///
    /// Walks each root's dependency closure once, so diagnostics attached to
    /// shared dependencies, like one checked component under its per-module
    /// facades, report exactly once.
    pub fn diagnostics_for_keys(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let _revision = self.revision(revision)?;
        let mut diagnostics = DiagnosticCollection::new();
        let mut visited = FxHashSet::default();
        let mut pending = Vec::new();

        for artifact_key in artifact_keys {
            if let Some(version) = self.artifact_version(revision, artifact_key)? {
                pending.push(version);
            }
        }

        while let Some(version) = pending.pop() {
            if !visited.insert(version) {
                continue;
            }
            let artifact_diagnostics = self.artifact_diagnostics(version)?;
            diagnostics.merge_from(artifact_diagnostics.as_ref());

            let Some(dependencies) = self.artifact_table().dependencies(&version) else {
                continue;
            };
            for dependency in dependencies.iter() {
                match dependency {
                    ArtifactDependency::Artifact(version) => pending.push(*version),
                    ArtifactDependency::Projection(projection) => {
                        let key = &projection.projection.artifact;
                        if let Some(version) = self.artifact_version(revision, key)? {
                            pending.push(version);
                        }
                    }
                    ArtifactDependency::Source(_) => {}
                }
            }
        }

        Ok(diagnostics)
    }

    /// Return sidecars for one revision-scoped artifact key.
    pub fn artifact_sidecars(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Arc<[ArtifactSidecar]>, RepositoryError> {
        let _revision = self.revision(revision)?;
        let Some(version) = self.artifact_version(revision, &artifact_key)? else {
            return Ok(Arc::from([]));
        };

        self.artifact_table()
            .sidecars(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }

    /// Return one sidecar by exact name and label set.
    pub fn artifact_sidecar(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        name: &str,
        labels: &BTreeMap<String, String>,
    ) -> Result<Option<ArtifactSidecar>, RepositoryError> {
        let sidecars = self.artifact_sidecars(revision, artifact_key)?;

        Ok(sidecars
            .iter()
            .find(|sidecar| sidecar.matches(name, labels))
            .cloned())
    }

    /// Return an exact revision binding when its version is still terminal.
    pub(crate) fn exact_artifact_binding(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<ArtifactVersion> {
        let version = self
            .artifacts
            .versions
            .get(&(revision, *artifact_key))
            .map(|version| *version)?;

        self.artifact_table().outcome(&version).map(|_| version)
    }

    /// Bind one exact terminal artifact version to one revision.
    pub(crate) fn bind_artifact_version(&self, revision: Revision, version: ArtifactVersion) {
        self.artifacts
            .versions
            .insert((revision, version.key), version);
    }

    /// Return nearest fresh predecessor artifacts for requested missing keys.
    fn fresh_artifact_versions(
        &self,
        revision: Revision,
        artifact_keys: &FxHashSet<ArtifactKey>,
    ) -> Result<FxHashMap<ArtifactKey, ArtifactVersion>, RepositoryError> {
        let mut fresh = FxHashMap::default();

        let mut artifact_keys = artifact_keys.iter().copied().collect::<Vec<_>>();
        artifact_keys.sort_unstable();

        // walk ancestor candidates nearest first
        for ancestor in self.revision_ancestors(revision)? {
            let mut candidates = Vec::new();

            for artifact_key in &artifact_keys {
                if fresh.contains_key(artifact_key) {
                    continue;
                }

                let Some(version) = self.exact_artifact_binding(ancestor, artifact_key) else {
                    continue;
                };

                candidates.push((*artifact_key, version));
            }

            if candidates.is_empty() {
                continue;
            }

            let source_delta = self.source_delta_between(revision, ancestor)?;

            // reuse only candidates unaffected by changed source files
            for (artifact_key, version) in candidates {
                if self.artifact_is_fresh(revision, version, source_delta.files())? {
                    fresh.insert(artifact_key, version);
                }
            }

            if fresh.len() == artifact_keys.len() {
                break;
            }
        }

        // expose fresh requested versions to this revision
        for version in fresh.values().copied() {
            self.bind_artifact_version(revision, version);
        }

        Ok(fresh)
    }

    /// Return whether one artifact version is fresh in one revision.
    fn artifact_is_fresh(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        changed_files: &[FileId],
    ) -> Result<bool, RepositoryError> {
        let sources = self
            .artifact_table()
            .sources(&version)
            .ok_or(RepositoryError::MissingArtifact { version })?;

        // reject artifacts whose source closure changed
        if source_sets_intersect(sources.as_ref(), changed_files) {
            return Ok(false);
        }

        let dependencies = self
            .artifact_table()
            .dependencies(&version)
            .ok_or(RepositoryError::MissingArtifact { version })?;

        // require every exact dependency observation to remain current
        for dependency in dependencies.iter() {
            let is_fresh = match dependency {
                ArtifactDependency::Artifact(version) => {
                    self.artifact_version(revision, &version.key)? == Some(*version)
                }
                ArtifactDependency::Projection(dependency) => {
                    self.projection_dependency_is_fresh(revision, dependency)?
                }
                ArtifactDependency::Source(dependency) => {
                    self.source_dependency_is_fresh(revision, dependency)?
                }
            };
            if !is_fresh {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one primitive source observation still matches the revision.
    fn source_dependency_is_fresh(
        &self,
        revision: Revision,
        dependency: &SourceDependency,
    ) -> Result<bool, RepositoryError> {
        match dependency {
            SourceDependency::FileContent { file, content } => {
                let current = self.file_content_id(revision, *file)?;

                Ok(current == Some(*content))
            }
            SourceDependency::Packages { fingerprint } => {
                let packages = self.package_ids(revision)?;
                let current = PackageSetFingerprint::new(&packages);

                Ok(&current == fingerprint)
            }
            SourceDependency::Modules { fingerprint } => {
                let modules = self.module_ids(revision)?;
                let current = ModuleSetFingerprint::new(&modules);

                Ok(&current == fingerprint)
            }
        }
    }

    /// Return whether one projection dependency still matches in the revision.
    fn projection_dependency_is_fresh(
        &self,
        revision: Revision,
        dependency: &ArtifactProjectionDependency,
    ) -> Result<bool, RepositoryError> {
        let Some(version) = self.artifact_version(revision, &dependency.projection.artifact)?
        else {
            return Ok(false);
        };
        let fingerprint = self
            .artifact_table()
            .projection_fingerprint(&version, &dependency.projection);

        Ok(fingerprint == Some(dependency.fingerprint))
    }

    /// Return the transitive source files for one artifact record.
    fn artifact_sources(
        &self,
        base: Option<ArtifactVersion>,
        dependencies: &[ArtifactDependency],
    ) -> Result<Vec<FileId>, RepositoryError> {
        let mut sources = Vec::new();

        // inherit source reachability from the incremental base
        if let Some(base) = base {
            let base_sources = self
                .artifact_table()
                .sources(&base)
                .ok_or(RepositoryError::MissingArtifact { version: base })?;
            sources.extend(base_sources.iter().copied());
        }

        // collect exact source reachability
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Artifact(version) => {
                    let dependency_sources = self
                        .artifact_table()
                        .sources(version)
                        .ok_or(RepositoryError::MissingArtifact { version: *version })?;
                    sources.extend(dependency_sources.iter().copied());
                }
                ArtifactDependency::Projection(dependency) => {
                    // projections depend on their fingerprint, not the full source closure
                    if !self.artifact_table().has(&dependency.version) {
                        return Err(RepositoryError::MissingArtifact {
                            version: dependency.version,
                        });
                    }
                }
                ArtifactDependency::Source(SourceDependency::FileContent { file, .. }) => {
                    sources.push(*file);
                }
                ArtifactDependency::Source(
                    SourceDependency::Packages { .. } | SourceDependency::Modules { .. },
                ) => {}
            }
        }

        sources.sort_unstable();
        sources.dedup();

        Ok(sources)
    }

    /// Publish one ready payload without writing the persistent store.
    fn publish_ready_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        sources: Vec<FileId>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        self.artifact_table().publish(
            version,
            base,
            payload,
            dependencies,
            sources,
            diagnostics,
            sidecars,
        );
        self.bind_artifact_version(revision, version);

        Ok(())
    }

    /// Load all content ids referenced by one artifact payload.
    fn load_artifact_contents(&self, payload: &ArtifactPayload) -> Result<(), RepositoryError> {
        for content in payload.content_ids() {
            let _ = self.content(content)?;
        }

        Ok(())
    }

    /// Return diagnostics for one exact artifact version.
    fn artifact_diagnostics(
        &self,
        version: ArtifactVersion,
    ) -> Result<Arc<DiagnosticCollection>, RepositoryError> {
        self.artifact_table()
            .diagnostics(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }
}

/// Return whether two sorted source file sets intersect.
fn source_sets_intersect(left: &[FileId], right: &[FileId]) -> bool {
    let mut left_index = 0;
    let mut right_index = 0;

    // walk both sorted sets once
    while left_index < left.len() && right_index < right.len() {
        let left_file = left[left_index];
        let right_file = right[right_index];

        if left_file == right_file {
            return true;
        } else if left_file < right_file {
            left_index += 1;
        } else {
            right_index += 1;
        }
    }

    false
}
