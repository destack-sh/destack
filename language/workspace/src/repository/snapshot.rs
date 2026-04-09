use std::sync::{Arc, Weak};

use destack_artifact::{ArtifactKey, ArtifactStamp, ArtifactVersion};
use destack_source::{File, FileId, ModuleId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision, RevisionState};
use crate::{Module, Package, Workspace};

impl Repository {
    /// Return one pinned repository snapshot for one revision.
    pub fn snapshot(
        self: &Arc<Self>,
        revision: Revision,
    ) -> Result<RepositorySnapshot, RepositoryError> {
        // verify and pin the revision
        let revision_state = self.revision(revision)?;
        if !revision_state.is_immutable() {
            return Err(RepositoryError::MutableRevision { revision });
        }

        self.pin_revision(revision);

        let pin = Arc::new(RevisionPin::new(revision, Arc::downgrade(self)));

        Ok(RepositorySnapshot::new(
            revision,
            Arc::clone(self),
            Arc::clone(&revision_state),
            pin,
        ))
    }

    /// Retain one anonymous revision pin.
    pub(crate) fn pin_revision(&self, revision: Revision) {
        self.pinned_revisions
            .entry(revision)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Release one anonymous revision pin.
    pub(crate) fn unpin_revision(&self, revision: Revision) {
        let Some(mut entry) = self.pinned_revisions.get_mut(&revision) else {
            return;
        };

        if *entry > 1 {
            *entry -= 1;
            return;
        }

        drop(entry);
        self.pinned_revisions.remove(&revision);
    }
}

/// One pinned repository view at one retained revision.
#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    /// The retained revision identity.
    revision: Revision,
    /// The shared repository owner.
    repository: Arc<Repository>,
    /// The retained immutable revision state.
    revision_state: Arc<RevisionState>,
    /// The retained revision pin.
    _pin: Arc<RevisionPin>,
}

impl RepositorySnapshot {
    /// Build one pinned snapshot from explicit parts.
    pub(crate) fn new(
        revision: Revision,
        repository: Arc<Repository>,
        revision_state: Arc<RevisionState>,
        pin: Arc<RevisionPin>,
    ) -> Self {
        Self {
            revision,
            repository,
            revision_state,
            _pin: pin,
        }
    }

    /// Return the retained revision identity.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the shared repository owner.
    pub fn repository(&self) -> &Repository {
        self.repository.as_ref()
    }

    /// Return the retained immutable revision state.
    pub(crate) fn revision_state(&self) -> &RevisionState {
        self.revision_state.as_ref()
    }

    /// Return one file snapshot for one file id.
    pub fn file(&self, file_id: FileId) -> Result<Option<Arc<File>>, RepositoryError> {
        self.repository.file(self.revision, file_id)
    }

    /// Return one workspace snapshot.
    pub fn workspace(&self) -> Result<Arc<Workspace>, RepositoryError> {
        self.repository.workspace(self.revision)
    }

    /// Return one package snapshot for one package id.
    pub fn package(&self, package_id: PackageId) -> Result<Option<Arc<Package>>, RepositoryError> {
        self.repository.package(self.revision, package_id)
    }

    /// Return one module snapshot for one module id.
    pub fn module(&self, module_id: ModuleId) -> Result<Option<Arc<Module>>, RepositoryError> {
        self.repository.module(self.revision, module_id)
    }

    /// Return the current artifact stamp for one artifact key in this pinned revision.
    pub fn artifact_stamp(&self, artifact_key: &ArtifactKey) -> ArtifactStamp {
        self.repository.artifact_stamp_for_revision_state(
            self.revision,
            self.revision_state(),
            artifact_key,
        )
    }

    /// Return the exact artifact version for one artifact key in this pinned revision.
    pub fn artifact_version(&self, artifact_key: &ArtifactKey) -> ArtifactVersion {
        ArtifactVersion::new(*artifact_key, self.artifact_stamp(artifact_key))
    }
}

/// One retained repository revision pin.
#[derive(Debug)]
pub(crate) struct RevisionPin {
    /// The retained revision identity.
    revision: Revision,
    /// The repository that owns the pinned revision graph.
    repository: Weak<Repository>,
}

impl RevisionPin {
    /// Build one revision pin from explicit parts.
    pub(crate) fn new(revision: Revision, repository: Weak<Repository>) -> Self {
        Self {
            revision,
            repository,
        }
    }
}

impl Drop for RevisionPin {
    fn drop(&mut self) {
        // drop one retained revision pin
        let Some(repository) = self.repository.upgrade() else {
            return;
        };

        repository.unpin_revision(self.revision);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use destack_artifact::DiskCacheStore;
    use destack_source::{FileSystem, PhysicalFileSystem};

    use crate::repository::{AmbientSnapshot, Change, Ref, Repository};

    /// Keep one anonymous revision alive while one snapshot is pinned.
    #[test]
    fn test_keep_anonymous_revision_alive_while_snapshot_is_pinned() {
        let root = unique_test_root("repository-snapshot");
        fs::create_dir_all(&root).expect("repository snapshot test root should exist");

        let repository = Arc::new(Repository::open_root(
            root.clone(),
            AmbientSnapshot::capture_process(),
        ));
        let reference = Ref::for_workspace_root(&root);
        let base_revision = repository
            .current(&reference)
            .expect("workspace root ref should exist");
        let anonymous_revision = repository
            .apply_to_revision(
                base_revision,
                Change::add_text("src/example.ts", "export const value = 1"),
            )
            .expect("anonymous revision should publish");
        let snapshot = repository
            .snapshot(anonymous_revision)
            .expect("anonymous revision snapshot should pin");

        // move the named ref elsewhere and prune anonymous state
        repository
            .apply(
                &reference,
                Change::add_text("src/other.ts", "export const other = 2"),
            )
            .expect("workspace ref should advance");

        // pinned snapshot
        assert!(repository.revision(anonymous_revision).is_ok());

        // after the last snapshot drops, the anonymous revision becomes collectible
        drop(snapshot);
        repository.prune_unreachable_file_state();
        assert!(repository.revision(anonymous_revision).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Build one unique workspace root for one repository test.
    fn unique_test_root(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let process_id = std::process::id();

        std::env::temp_dir().join(format!("destack-{prefix}-{process_id}-{timestamp}"))
    }

    /// Retain recent pinned history while compacting older ancestry.
    #[test]
    fn test_compact_pinned_file_history() {
        let root = unique_test_root("repository-history");
        fs::create_dir_all(&root).expect("repository history test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Arc::new(Repository::new(
            root.clone(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            AmbientSnapshot::capture_process(),
        ));
        let reference = Ref::for_workspace_root(&root);

        repository
            .seed_root_source(&reference)
            .expect("repository root source should seed");

        let mut revisions = Vec::new();

        for value in 1..=33 {
            let change = if value == 1 {
                Change::add_text("src/example.ts", format!("export const value = {value}"))
            } else {
                Change::set_text("src/example.ts", format!("export const value = {value}"))
            };
            let revision = repository
                .apply(&reference, change)
                .expect("revision should publish");

            revisions.push(revision);
        }

        let revision_1 = revisions[0];
        let revision_2 = revisions[1];
        let revision_3 = *revisions.last().expect("latest revision should exist");
        let file_id = repository.file_id_for_logical_path("src/example.ts");

        // retained head state
        let file = repository
            .file(revision_3, file_id)
            .expect("retained file lookup should succeed")
            .expect("retained file should exist");
        assert_eq!(file.text(), "export const value = 33");

        // bounded history
        assert!(repository.revision(revision_1).is_err());
        assert!(repository.revision(revision_2).is_ok());
        assert!(repository.revision(revision_3).is_ok());
        assert!(
            repository
                .revision(revision_2)
                .expect("retained boundary revision should exist")
                .parents
                .is_empty()
        );

        let _ = fs::remove_dir_all(&root);
    }
}
