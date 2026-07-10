use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactVersion};
use destack_source::{ContentId, File, FileId, ModuleId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{Module, Package, Root};

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
    pub fn prune_unreachable(&self) -> Result<(), RepositoryError> {
        let reachable_revisions = self.reachable_file_revisions();
        let reachable_artifacts = self.reachable_artifact_versions(&reachable_revisions);
        let reachable_artifacts = self.artifact_table().reachable_closure(reachable_artifacts);
        let reachable_contents =
            self.reachable_content_ids(&reachable_revisions, &reachable_artifacts);

        self.revisions
            .retain(|revision, _| reachable_revisions.contains(revision));
        self.compact_revision_trees(&reachable_revisions);
        self.artifacts
            .versions
            .retain(|(revision, _key), _version| reachable_revisions.contains(revision));
        self.artifact_table().retain_reachable(&reachable_artifacts);
        self.artifact_store()
            .retain(&reachable_artifacts, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;
        self.files.cache.retain_file_contents(&reachable_contents);
        self.content_pool.retain_reachable(&reachable_contents);
        self.content_store()
            .retain_reachable(&reachable_contents)
            .map_err(|error| RepositoryError::ContentStore {
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Collect all file revisions reachable from refs and revision pins.
    fn reachable_file_revisions(&self) -> HashSet<Revision> {
        self.retained_revisions().into_iter().collect()
    }

    /// Collect all artifact versions reachable from revisions and artifact pins.
    fn reachable_artifact_versions(
        &self,
        reachable_revisions: &HashSet<Revision>,
    ) -> HashSet<ArtifactVersion> {
        let mut reachable = HashSet::new();

        for entry in self.artifacts.versions.iter() {
            let ((revision, _key), version) = entry.pair();
            if reachable_revisions.contains(revision) {
                reachable.insert(*version);
            }
        }

        for artifact_version in self.artifact_table().retained_versions() {
            reachable.insert(artifact_version);
        }

        reachable
    }

    /// Collect all content ids reachable from one revision set.
    fn reachable_content_ids(
        &self,
        reachable_revisions: &HashSet<Revision>,
        reachable_artifacts: &HashSet<ArtifactVersion>,
    ) -> HashSet<ContentId> {
        let mut reachable = HashSet::new();
        let roots = reachable_revisions
            .iter()
            .filter_map(|revision| self.revisions.get(revision))
            .map(|entry| entry.state().files())
            .collect::<Vec<_>>();

        // collect source file contents from shared tree nodes once
        for entry in self.files.entries.unique_values(roots) {
            reachable.insert(entry.content_id);
        }

        // artifact output contents
        for artifact_version in reachable_artifacts {
            for content in self.artifact_table().content_ids(artifact_version) {
                reachable.insert(content);
            }
        }

        reachable
    }

    /// Compact revision tree storage around reachable revision roots.
    fn compact_revision_trees(&self, reachable_revisions: &HashSet<Revision>) {
        let revisions = reachable_revisions
            .iter()
            .filter_map(|revision| self.revisions.get(revision))
            .map(|entry| entry.state())
            .collect::<Vec<_>>();

        // compact file roots
        let mut file_guards = revisions
            .iter()
            .map(|revision| revision.write_files())
            .collect::<Vec<_>>();
        let mut files = file_guards.iter().map(|files| **files).collect::<Vec<_>>();
        self.files.entries.compact(&mut files);
        for (guard, files) in file_guards.iter_mut().zip(files) {
            **guard = files;
        }
        drop(file_guards);
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

    /// Return root metadata.
    pub fn root(&self) -> Result<Arc<Root>, RepositoryError> {
        self.repository.root(self.revision)
    }

    /// Return one package for one package id.
    pub fn package(&self, package_id: PackageId) -> Result<Option<Arc<Package>>, RepositoryError> {
        self.repository.package(self.revision, package_id)
    }

    /// Return one module for one module id.
    pub fn module(&self, module_id: ModuleId) -> Result<Option<Arc<Module>>, RepositoryError> {
        self.repository.module(self.revision, module_id)
    }

    /// Return the artifact version valid for one artifact key in this pinned revision.
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use destack_artifact::{
        ArtifactKey, ArtifactVersion, Bundle, BundleFile, BundleMode, BundleSection, DiskBlobStore,
        EmitFormat, GlobalEnvironment, LanguageEnvironment,
    };
    use destack_dir::{GlobalSymbolId, LocalSymbolId};
    use destack_source::{
        Content, DiagnosticCollection, FileSystem, FileType, ModuleId, PackageId,
        PhysicalFileSystem, ProfileId, TargetId, Uri,
    };
    use indexmap::IndexMap;

    use crate::repository::{Edit, Ref, Repository, Revision};
    use crate::{DestackLayout, DestackLayoutOverride, Environment, Host, Settings};

    /// Create one repository for a test root.
    fn test_repository(root: &Path) -> Repository {
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            root,
            root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );

        let host = Host::new(environment, file_system, Arc::new(DiskBlobStore::new()));

        Repository::new(root.to_path_buf(), host, Settings::default(), layout)
    }

    /// Keep one anonymous revision alive while it is pinned.
    #[test]
    fn test_keep_anonymous_revision_alive_while_pinned() {
        let root = unique_test_root("repository-pin");
        fs::create_dir_all(&root).expect("repository pin test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let host = Host::new(environment, file_system, Arc::new(DiskBlobStore::new()));
        let repository = Arc::new(Repository::new(
            root.clone(),
            host,
            Settings::default(),
            layout,
        ));
        let reference = Ref::for_root(&root);
        let base_revision = repository
            .current(&reference)
            .expect("root ref should exist");
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
        repository
            .prune_unreachable()
            .expect("repository should prune");
        assert!(repository.revision(anonymous_revision).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Load content from the persistent store into a fresh repository.
    #[test]
    fn test_load_content_from_store() {
        let root = unique_test_root("repository-content-load");
        fs::create_dir_all(&root).expect("repository content load test root should exist");

        let repository = test_repository(&root);
        let content = Content::Text {
            content: "cached content\n".to_string(),
        };
        let content_id = repository
            .intern_content(content.clone())
            .expect("content should intern");

        let repository = test_repository(&root);
        let loaded = repository
            .content(content_id)
            .expect("content should load from store");

        assert_eq!(loaded.payload(), &content);

        let _ = fs::remove_dir_all(&root);
    }

    /// Load an artifact record from the persistent store into a fresh repository.
    #[test]
    fn test_load_artifact_from_store() {
        let root = unique_test_root("repository-artifact-load");
        fs::create_dir_all(&root).expect("repository artifact load test root should exist");

        let repository = test_repository(&root);
        let revision = repository
            .current(&Ref::for_root(&root))
            .expect("root ref should exist");
        let package = PackageId::new(1);
        let target = TargetId::new(package, "browser");
        let content = repository
            .intern_content(Content::Text {
                content: "console.log('loaded')\n".to_string(),
            })
            .expect("artifact content should intern");
        let output = Bundle::new(
            EmitFormat::Js,
            BundleMode::SingleFile,
            vec![BundleFile::new(
                BundleSection::Entry,
                Uri::from_string("memory:/out.js"),
                FileType::Script,
                content,
                None,
            )],
        );
        let key = ArtifactKey::bundle(package, target);
        let version = ArtifactVersion::new(key, repository.build_fingerprint(), None, []);

        repository
            .complete_artifact(
                revision,
                version,
                None,
                output.into(),
                Vec::new(),
                DiagnosticCollection::new(),
                Vec::new(),
            )
            .expect("bundle should publish");
        repository
            .flush_artifacts()
            .expect("artifact store should flush");

        let repository = test_repository(&root);
        let revision = repository
            .current(&Ref::for_root(&root))
            .expect("root ref should exist");
        let loaded = repository
            .load_artifact(revision, version)
            .expect("artifact should load from store");

        assert!(loaded);
        assert_eq!(
            repository
                .artifact_version(revision, &key)
                .expect("artifact binding should load"),
            Some(version)
        );
        assert!(repository.artifact_table().bundle(&version).is_some());
        assert!(repository.content(content).is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    /// Load artifact strings from the persistent store into a fresh repository.
    #[test]
    fn test_load_artifact_strings_from_store() {
        let root = unique_test_root("repository-artifact-strings-load");
        fs::create_dir_all(&root).expect("repository artifact strings load test root should exist");

        let repository = test_repository(&root);
        let revision = repository
            .current(&Ref::for_root(&root))
            .expect("root ref should exist");
        let package = PackageId::new(1);
        let module = ModuleId::new(package, 1);
        let profile = ProfileId::new(1);
        let string = repository.string_pool().intern("CachedSymbol");
        let symbol = GlobalSymbolId::new(module, LocalSymbolId::new(1));
        let mut symbols = IndexMap::new();
        symbols.insert(string, symbol);

        let output = GlobalEnvironment {
            language: LanguageEnvironment {
                symbol_by_item: IndexMap::new(),
                items_by_symbol: IndexMap::new(),
                symbols,
            },
            globals: vec![module],
            global_targets_by_key: IndexMap::new(),
        };
        let key = ArtifactKey::global_environment(profile);
        let version = ArtifactVersion::new(key, repository.build_fingerprint(), None, []);

        repository
            .complete_artifact(
                revision,
                version,
                None,
                output.into(),
                Vec::new(),
                DiagnosticCollection::new(),
                Vec::new(),
            )
            .expect("environment should publish");
        repository
            .flush_artifacts()
            .expect("artifact store should flush");

        let repository = test_repository(&root);
        let revision = repository
            .current(&Ref::for_root(&root))
            .expect("root ref should exist");
        let loaded = repository
            .load_artifact(revision, version)
            .expect("artifact should load from store");

        assert!(loaded);
        assert_eq!(
            repository.string_pool().get_maybe(string),
            Some("CachedSymbol")
        );
        assert_eq!(
            repository
                .artifact_table()
                .global_environment(&version)
                .expect("environment should load")
                .language
                .symbol_by_name("CachedSymbol"),
            Some(symbol)
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// Build one unique root for one repository test.
    fn unique_test_root(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let process_id = std::process::id();

        std::env::temp_dir().join(format!("destack-{prefix}-{process_id}-{timestamp}"))
    }

    /// Prune old unpinned revisions during explicit retention.
    #[test]
    fn test_prune_unpinned_revision_during_explicit_retention() {
        let root = unique_test_root("repository-prune");
        fs::create_dir_all(&root).expect("repository history test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let host = Host::new(environment, file_system, Arc::new(DiskBlobStore::new()));
        let repository = Arc::new(Repository::new(
            root.clone(),
            host,
            Settings::default(),
            layout,
        ));
        let reference = Ref::for_root(&root);

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

        // edit history remains available until retention runs
        assert!(repository.revision(revision_1).is_ok());

        repository
            .prune_unreachable()
            .expect("repository should prune");

        // old unpinned state
        assert!(repository.revision(revision_1).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Keep generated artifact contents while their artifact version is reachable.
    #[test]
    fn test_keep_generated_contents_while_artifact_reachable() {
        let root = unique_test_root("repository-generated-contents");
        fs::create_dir_all(&root).expect("repository generated contents test root should exist");

        let repository = test_repository(&root);
        let revision = repository
            .current(&Ref::for_root(&root))
            .expect("root ref should exist");
        let package = PackageId::new(1);
        let target = TargetId::new(package, "browser");
        let retained = repository
            .intern_content(Content::Text {
                content: "console.log('retained')\n".to_string(),
            })
            .expect("retained content should intern");
        let pruned = repository
            .intern_content(Content::Text {
                content: "console.log('pruned')\n".to_string(),
            })
            .expect("pruned content should intern");
        let output = Bundle::new(
            EmitFormat::Js,
            BundleMode::SingleFile,
            vec![BundleFile::new(
                BundleSection::Entry,
                Uri::from_string("memory:/out.js"),
                FileType::Script,
                retained,
                None,
            )],
        );
        let version = ArtifactVersion::new(
            ArtifactKey::bundle(package, target),
            repository.build_fingerprint(),
            None,
            [],
        );

        repository
            .complete_artifact(
                revision,
                version,
                None,
                output.into(),
                Vec::new(),
                DiagnosticCollection::new(),
                Vec::new(),
            )
            .expect("bundle should publish");
        repository
            .prune_unreachable()
            .expect("repository should prune");

        // reachable artifact contents
        assert!(repository.content(retained).is_ok());

        // unreferenced generated contents
        assert!(repository.content(pruned).is_err());

        let repository = test_repository(&root);

        // persistent reachable artifact contents
        assert!(repository.content(retained).is_ok());

        // persistent unreferenced generated contents
        assert!(repository.content(pruned).is_err());

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
