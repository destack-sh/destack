use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactVersion};
use destack_source::{File, FileContentId, FileId, ModuleId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{Module, Package, Workspace};

impl Repository {
    /// Pin one revision for the lifetime of the returned guard.
    pub fn pin(self: &Arc<Self>, revision: Revision) -> Result<RevisionPin, RepositoryError> {
        let Some(entry) = self.revisions.get(&revision) else {
            return Err(RepositoryError::MissingRevision { revision });
        };
        entry.pin();

        drop(entry);

        Ok(RevisionPin::new(revision, Arc::clone(self)))
    }

    /// Increment one revision pin count.
    pub(crate) fn increment_revision_pin(&self, revision: Revision) {
        if let Some(entry) = self.revisions.get(&revision) {
            entry.pin();
        }
    }

    /// Decrement one revision pin count.
    pub(crate) fn decrement_revision_pin(&self, revision: Revision) {
        if let Some(entry) = self.revisions.get(&revision) {
            entry.unpin();
        }
    }

    /// Prune file revisions and file contents that are no longer reachable.
    pub fn prune_unreachable(&self) {
        let reachable_revisions = self.reachable_file_revisions();
        let reachable_file_contents = self.reachable_file_content_ids(&reachable_revisions);

        self.revisions
            .retain(|revision, _| reachable_revisions.contains(revision));
        self.artifact_versions
            .retain(|(revision, _), _| reachable_revisions.contains(revision));
        self.file_cache
            .retain_file_contents(&reachable_file_contents);
        self.files.retain_reachable(&reachable_file_contents);
    }

    /// Collect all file revisions reachable from refs and revision pins.
    fn reachable_file_revisions(&self) -> HashSet<Revision> {
        let mut reachable = HashSet::new();

        for revision in self.retained_revisions() {
            reachable.insert(revision);
        }

        reachable
    }

    /// Collect all file content ids reachable from one revision set.
    fn reachable_file_content_ids(
        &self,
        reachable_revisions: &HashSet<Revision>,
    ) -> HashSet<FileContentId> {
        let mut reachable = HashSet::new();

        for revision in reachable_revisions {
            let Some(revision_entry) = self.revisions.get(revision) else {
                continue;
            };
            let revision_state = revision_entry.state();

            for entry in revision_state.files.values() {
                reachable.insert(entry.content_id);
            }
        }

        reachable
    }

    /// Collect all retained revisions.
    fn retained_revisions(&self) -> Vec<Revision> {
        let mut retained_revisions = self
            .refs
            .iter()
            .map(|entry| *entry.value())
            .collect::<Vec<_>>();

        // revision pins
        retained_revisions.extend(
            self.revisions
                .iter()
                .filter(|entry| entry.value().is_pinned())
                .map(|entry| *entry.key()),
        );

        retained_revisions
    }
}

/// One pinned repository revision.
#[derive(Debug)]
pub struct RevisionPin {
    /// The pinned revision identity.
    revision: Revision,
    /// The shared repository owner.
    repository: Arc<Repository>,
}

impl RevisionPin {
    /// Build one revision pin from explicit parts.
    pub(crate) fn new(revision: Revision, repository: Arc<Repository>) -> Self {
        Self {
            revision,
            repository,
        }
    }

    /// Return the pinned revision identity.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the shared repository owner.
    pub fn repository(&self) -> &Repository {
        self.repository.as_ref()
    }

    /// Return one file for one file id.
    pub fn file(&self, file_id: FileId) -> Result<Option<Arc<File>>, RepositoryError> {
        self.repository.file(self.revision, file_id)
    }

    /// Return workspace metadata.
    pub fn workspace(&self) -> Result<Arc<Workspace>, RepositoryError> {
        self.repository.workspace(self.revision)
    }

    /// Return one package for one package id.
    pub fn package(&self, package_id: PackageId) -> Result<Option<Arc<Package>>, RepositoryError> {
        self.repository.package(self.revision, package_id)
    }

    /// Return one module for one module id.
    pub fn module(&self, module_id: ModuleId) -> Result<Option<Arc<Module>>, RepositoryError> {
        self.repository.module(self.revision, module_id)
    }

    /// Return the published artifact version for one artifact key in this pinned revision.
    pub fn artifact_version(
        &self,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        self.repository
            .artifact_version(self.revision, artifact_key)
    }
}

impl Clone for RevisionPin {
    /// Clone this revision pin and retain the underlying revision once more.
    fn clone(&self) -> Self {
        self.repository.increment_revision_pin(self.revision);

        Self {
            revision: self.revision,
            repository: Arc::clone(&self.repository),
        }
    }
}

impl Drop for RevisionPin {
    fn drop(&mut self) {
        // release one revision pin
        self.repository.decrement_revision_pin(self.revision);
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

    use crate::repository::{Edit, HostEnvironment, Ref, Repository, Revision};

    /// Keep one anonymous revision alive while it is pinned.
    #[test]
    fn test_keep_anonymous_revision_alive_while_pinned() {
        let root = unique_test_root("repository-pin");
        fs::create_dir_all(&root).expect("repository pin test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Arc::new(Repository::new(
            root.clone(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        ));
        let reference = Ref::for_workspace_root(&root);
        let base_revision = repository
            .current(&reference)
            .expect("workspace root ref should exist");
        let anonymous_revision = repository
            .fork_with_edits(
                base_revision,
                [Edit::add_text("src/example.ts", "export const value = 1")],
            )
            .expect("anonymous revision should publish");
        let revision_pin = repository
            .pin(anonymous_revision)
            .expect("anonymous revision should pin");

        // move the named ref elsewhere and prune anonymous state
        publish_edits(
            repository.as_ref(),
            &reference,
            [Edit::add_text("src/other.ts", "export const other = 2")],
        );

        // pinned revision
        assert!(repository.revision(anonymous_revision).is_ok());

        // after the last pin drops, the anonymous revision becomes collectible
        drop(revision_pin);
        repository.prune_unreachable();
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

    /// Prune old unpinned revisions after a ref moves.
    #[test]
    fn test_prune_unpinned_revision_after_ref_moves() {
        let root = unique_test_root("repository-prune");
        fs::create_dir_all(&root).expect("repository history test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Arc::new(Repository::new(
            root.clone(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        ));
        let reference = Ref::for_workspace_root(&root);

        let revision_1 = publish_edits(
            repository.as_ref(),
            &reference,
            [Edit::add_text("src/example.ts", "export const value = 1")],
        );
        let revision_2 = publish_edits(
            repository.as_ref(),
            &reference,
            [Edit::set_text("src/example.ts", "export const value = 2")],
        );
        let file_id = repository.file_id(&root.join("src/example.ts"));

        // current ref state
        let file = repository
            .file(revision_2, file_id)
            .expect("retained file lookup should succeed")
            .expect("retained file should exist");
        assert_eq!(file.text(), "export const value = 2");

        // old ref state
        assert!(repository.revision(revision_1).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Publish repository edits to one ref.
    fn publish_edits<I>(repository: &Repository, reference: &Ref, edits: I) -> Revision
    where
        I: IntoIterator<Item = Edit>,
    {
        let revision = repository
            .current(reference)
            .expect("repository ref should have a current revision");
        let revision = repository
            .fork_with_edits(revision, edits)
            .expect("repository edits should fork");

        repository
            .set_ref(reference, revision)
            .expect("repository ref should advance")
    }
}
