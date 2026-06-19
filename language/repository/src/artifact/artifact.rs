use std::collections::BTreeMap;
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactDirectoryEntry, ArtifactFailure, ArtifactFlush, ArtifactKey,
    ArtifactPathState, ArtifactPayload, ArtifactRecord, ArtifactSidecar, ArtifactStore,
    ArtifactTable, ArtifactVersion, SourceDependency,
};
use destack_source::{DiagnosticCollection, FileId, StringId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::artifact::ArtifactReader;
use crate::repository::{Repository, RepositoryError, Revision, normalize_logical_path};

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

/// Freshness of one predecessor artifact version for one revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactFreshness {
    /// The candidate dependency graph matches the requested revision.
    Fresh,
    /// The candidate dependency graph does not match the requested revision.
    Stale,
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

        // check predecessor candidates
        let candidates = self.artifact_candidates(revision, &missing)?;
        let fresh = self.fresh_artifact_candidates(revision, missing.len(), candidates)?;

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
    pub fn artifact_base_version(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let _revision = self.revision(revision)?;

        // nearest bases are tried first, preserving base order
        for ancestor in self.revision_ancestors(revision)? {
            if let Some(version) = self.exact_artifact_binding(ancestor, &artifact_key) {
                return Ok(Some(version));
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

        self.artifacts
            .versions
            .insert((revision, version.key), version);

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

        self.load_artifact_record(revision, record)?;

        Ok(true)
    }

    /// Publish one ready artifact and bind its exact version to one revision.
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
        self.publish_ready_artifact(
            revision,
            version,
            base,
            payload,
            dependencies,
            diagnostics,
            sidecars,
        )
    }

    /// Publish one loaded artifact record and bind its exact version to one revision.
    fn load_artifact_record(
        &self,
        revision: Revision,
        record: ArtifactRecord,
    ) -> Result<(), RepositoryError> {
        let payload = record
            .decode_payload()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        self.publish_ready_artifact(
            revision,
            record.version,
            record.base,
            payload,
            record.dependencies,
            record.diagnostics,
            record.sidecars,
        )
    }

    /// Publish one ready payload without writing the persistent store.
    fn publish_ready_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        self.artifact_table()
            .publish(version, base, payload, dependencies, diagnostics, sidecars);
        self.artifacts
            .versions
            .insert((revision, version.key), version);

        Ok(())
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

    /// Load all content ids referenced by one artifact payload.
    fn load_artifact_contents(&self, payload: &ArtifactPayload) -> Result<(), RepositoryError> {
        for content in payload.content_ids() {
            let _ = self.content(content)?;
        }

        Ok(())
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

        // store failure before exposing the revision binding
        self.artifact_table()
            .fail(version, base, dependencies, diagnostics, sidecars, failure);
        self.artifacts
            .versions
            .insert((revision, version.key), version);

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
            let artifact_diagnostics = self
                .artifact_table()
                .diagnostics(&version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version })?;

            diagnostics.merge_from(&artifact_diagnostics);

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

            let artifact_diagnostics = self
                .artifact_table()
                .diagnostics(&version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version })?;
            diagnostics.merge_from(&artifact_diagnostics);
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
    fn exact_artifact_binding(
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

    /// Return nearest ancestor candidates for the requested missing keys.
    fn artifact_candidates(
        &self,
        revision: Revision,
        artifact_keys: &FxHashSet<ArtifactKey>,
    ) -> Result<Vec<(ArtifactKey, ArtifactVersion)>, RepositoryError> {
        let mut candidates = Vec::new();

        let mut artifact_keys = artifact_keys.iter().copied().collect::<Vec<_>>();
        artifact_keys.sort_unstable();

        // walk bases nearest first
        for ancestor in self.revision_ancestors(revision)? {
            for artifact_key in &artifact_keys {
                if let Some(version) = self.exact_artifact_binding(ancestor, artifact_key) {
                    candidates.push((*artifact_key, version));
                }
            }
        }

        Ok(candidates)
    }

    /// Return ancestors of one revision in nearest-first deterministic order.
    fn revision_ancestors(&self, revision: Revision) -> Result<Vec<Revision>, RepositoryError> {
        let revision = self.revision(revision)?;
        let mut ancestors = Vec::new();
        let mut pending = revision.bases();
        let mut seen = FxHashSet::default();
        let mut index = 0;

        // walk base graph breadth first
        while index < pending.len() {
            let ancestor = pending[index];
            index += 1;

            // skip already visited bases
            if !seen.insert(ancestor) {
                continue;
            }

            // skip pruned bases
            let Some(ancestor_state) = self.revisions.get(&ancestor) else {
                continue;
            };

            let ancestor_state = ancestor_state.state();
            ancestors.push(ancestor);
            pending.extend(ancestor_state.bases());
        }

        Ok(ancestors)
    }

    /// Return predecessor artifacts whose exact dependency graph matches one revision.
    fn fresh_artifact_candidates(
        &self,
        revision: Revision,
        missing_key_count: usize,
        candidates: Vec<(ArtifactKey, ArtifactVersion)>,
    ) -> Result<FxHashMap<ArtifactKey, ArtifactVersion>, RepositoryError> {
        let mut states = FxHashMap::default();
        let mut fresh = FxHashMap::default();

        // check predecessors until every key has a fresh version
        for (artifact_key, version) in candidates {
            if fresh.contains_key(&artifact_key) {
                continue;
            }

            self.record_artifact_freshness(revision, version, &mut states)?;

            if states.get(&version) == Some(&ArtifactFreshness::Fresh) {
                fresh.insert(artifact_key, version);
            }

            if fresh.len() == missing_key_count {
                break;
            }
        }

        // expose every fresh version to this revision
        for (version, state) in states {
            if state == ArtifactFreshness::Fresh {
                self.artifacts
                    .versions
                    .insert((revision, version.key), version);
            }
        }

        Ok(fresh)
    }

    /// Record freshness for one artifact and the artifact dependencies it reaches.
    fn record_artifact_freshness(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        states: &mut FxHashMap<ArtifactVersion, ArtifactFreshness>,
    ) -> Result<(), RepositoryError> {
        let mut pending = vec![(version, false)];

        // walk artifact dependencies without recursion
        while let Some((version, is_expanded)) = pending.pop() {
            if states.contains_key(&version) {
                continue;
            }

            // use exact bindings as terminal freshness facts
            if let Some(binding) = self.exact_artifact_binding(revision, &version.key) {
                let state = if binding == version {
                    ArtifactFreshness::Fresh
                } else {
                    ArtifactFreshness::Stale
                };
                states.insert(version, state);

                continue;
            }

            // missing records cannot prove freshness
            let Some(dependencies) = self.artifact_table().dependencies(&version) else {
                states.insert(version, ArtifactFreshness::Stale);

                continue;
            };

            // decide once dependency freshness is known
            if is_expanded {
                let state = self.artifact_dependency_freshness(revision, &dependencies, states)?;
                states.insert(version, state);
            }
            // otherwise queue dependencies first
            else {
                let state = self.prepare_artifact_freshness(
                    revision,
                    version,
                    &dependencies,
                    states,
                    &mut pending,
                )?;
                if let Some(state) = state {
                    states.insert(version, state);
                }
            }
        }

        Ok(())
    }

    /// Return freshness for an artifact whose dependencies were already visited.
    fn artifact_dependency_freshness(
        &self,
        revision: Revision,
        dependencies: &[ArtifactDependency],
        states: &FxHashMap<ArtifactVersion, ArtifactFreshness>,
    ) -> Result<ArtifactFreshness, RepositoryError> {
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Artifact(dependency) => {
                    if states.get(dependency) != Some(&ArtifactFreshness::Fresh) {
                        return Ok(ArtifactFreshness::Stale);
                    }
                }
                ArtifactDependency::Source(dependency) => {
                    if !self.source_dependency_matches(revision, dependency)? {
                        return Ok(ArtifactFreshness::Stale);
                    }
                }
            }
        }

        Ok(ArtifactFreshness::Fresh)
    }

    /// Queue artifact dependencies before deciding one artifact's freshness.
    fn prepare_artifact_freshness(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: &[ArtifactDependency],
        states: &FxHashMap<ArtifactVersion, ArtifactFreshness>,
        pending: &mut Vec<(ArtifactVersion, bool)>,
    ) -> Result<Option<ArtifactFreshness>, RepositoryError> {
        let mut missing = Vec::new();

        // scan recorded dependencies for stale or unknown inputs
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Artifact(dependency) => match states.get(dependency) {
                    Some(ArtifactFreshness::Fresh) => {}
                    Some(ArtifactFreshness::Stale) => {
                        return Ok(Some(ArtifactFreshness::Stale));
                    }
                    None => missing.push(*dependency),
                },
                ArtifactDependency::Source(dependency) => {
                    if !self.source_dependency_matches(revision, dependency)? {
                        return Ok(Some(ArtifactFreshness::Stale));
                    }
                }
            }
        }

        // decide immediately when all dependencies are fresh
        if missing.is_empty() {
            Ok(Some(ArtifactFreshness::Fresh))
        }
        // otherwise revisit after missing dependencies
        else {
            pending.push((version, true));
            pending.extend(missing.into_iter().map(|version| (version, false)));

            Ok(None)
        }
    }

    /// Return whether one exact source observation still matches one revision.
    fn source_dependency_matches(
        &self,
        revision: Revision,
        dependency: &SourceDependency,
    ) -> Result<bool, RepositoryError> {
        match dependency {
            SourceDependency::FileContent { file, content } => {
                Ok(self.file_content_id(revision, *file)? == Some(*content))
            }
            SourceDependency::PathState { path, state } => {
                Ok(self.artifact_path_state(revision, *path)? == *state)
            }
            SourceDependency::DirectoryEntries { directory, entries } => {
                Ok(self.artifact_directory_entries(revision, *directory)? == *entries)
            }
        }
    }

    /// Return the current state of one observed artifact path.
    fn artifact_path_state(
        &self,
        revision: Revision,
        path: StringId,
    ) -> Result<ArtifactPathState, RepositoryError> {
        let path = normalize_logical_path(self.string_pool().get(path));
        let file = FileId::from_logical_str(&path);

        if self.file_content_id(revision, file)?.is_some() {
            return Ok(ArtifactPathState::File);
        } else if self.artifact_directory_exists(revision, &path)? {
            return Ok(ArtifactPathState::Directory);
        } else {
            Ok(ArtifactPathState::Missing)
        }
    }

    /// Return the current direct entries of one observed artifact directory.
    fn artifact_directory_entries(
        &self,
        revision: Revision,
        directory: StringId,
    ) -> Result<Vec<ArtifactDirectoryEntry>, RepositoryError> {
        let directory = normalize_logical_path(self.string_pool().get(directory));

        self.artifact_directory_entries_for_path(revision, &directory)
    }

    /// Add one direct child entry for one source path when it belongs to a directory.
    fn push_artifact_directory_entry(
        &self,
        directory: &str,
        path: &str,
        entries: &mut Vec<ArtifactDirectoryEntry>,
    ) {
        let path = normalize_logical_path(path);

        // compute the path relative to the requested directory
        let relative = if directory.is_empty() {
            path.as_str()
        } else {
            let prefix = format!("{directory}/");
            if let Some(relative) = path.strip_prefix(&prefix) {
                relative
            } else {
                return;
            }
        };

        // take the first path segment as the direct child
        let Some(first) = relative.split('/').next() else {
            return;
        };

        // rebuild the direct child path in repository coordinates
        let entry_path = if directory.is_empty() {
            first.to_string()
        } else {
            format!("{directory}/{first}")
        };

        // classify descendants as directory entries
        let state = if entry_path == path {
            ArtifactPathState::File
        } else {
            ArtifactPathState::Directory
        };

        let path = self.intern_logical_path(entry_path);

        entries.push(ArtifactDirectoryEntry::new(path, state));
    }

    /// Return whether one logical source directory exists in one revision.
    fn artifact_directory_exists(
        &self,
        revision: Revision,
        directory: &str,
    ) -> Result<bool, RepositoryError> {
        let entries = self.artifact_directory_entries_for_path(revision, directory)?;

        Ok(!entries.is_empty())
    }

    /// Return current direct entries for one logical directory path.
    fn artifact_directory_entries_for_path(
        &self,
        revision: Revision,
        directory: &str,
    ) -> Result<Vec<ArtifactDirectoryEntry>, RepositoryError> {
        let revision = self.revision(revision)?;
        let directory = normalize_logical_path(directory);
        let mut entries = Vec::new();

        // collect editable direct children
        self.files
            .entries
            .visit(revision.files(), &mut |_file_id, entry| {
                let path = self.logical_path_text(entry.logical_path);
                self.push_artifact_directory_entry(&directory, path, &mut entries);
            });

        // collect builtin direct children
        for builtin in self.builtin.files() {
            self.push_artifact_directory_entry(&directory, builtin.uri, &mut entries);
        }

        entries.sort_unstable();
        entries.dedup();

        Ok(entries)
    }
}
