use std::collections::HashSet;
use std::sync::Arc;

use tspp_artifact::{ArtifactKey, ArtifactVersion};
use tspp_core::BlobId;
use tspp_source::{File, FileId, ModuleId, PackageId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{Module, Package, Root};

impl Repository {
    /// Pin one revision for the lifetime of the returned guard.
    pub fn pin(self: &Arc<Self>, revision: Revision) -> Result<RevisionPin, RepositoryError> {
        let Some(entry) = self.revisions.get(&revision) else {
            return Err(RepositoryError::MissingRevision { revision });
        };
        entry.pin();
        let state = entry.state();

        drop(entry);

        Ok(RevisionPin::new(revision, state, self.clone()))
    }

    /// Increment one revision pin count.
    pub(crate) fn increment_revision_pin(&self, revision: Revision) {
        let Some(entry) = self.revisions.get(&revision) else {
            unreachable!("cannot clone a pin for a missing repository revision");
        };

        entry.pin();
    }

    /// Decrement one revision pin count.
    pub(crate) fn decrement_revision_pin(&self, revision: Revision) {
        let Some(entry) = self.revisions.get(&revision) else {
            unreachable!("cannot release a pin for a missing repository revision");
        };

        entry.unpin();
    }

    /// Prune repository revisions, artifact results, and loaded files that are no longer reachable.
    pub fn prune_unreachable(&self) -> Result<(), RepositoryError> {
        let reachable_revisions = self.reachable_file_revisions();
        let reachable_blobs = self.reachable_file_blobs(&reachable_revisions);

        self.revisions
            .retain(|revision, _| reachable_revisions.contains(revision));
        self.compact_revision_trees(&reachable_revisions);
        self.artifact_table().prune();
        self.file_cache.retain(&reachable_blobs);

        Ok(())
    }

    /// Collect all file revisions reachable from revision pins.
    fn reachable_file_revisions(&self) -> HashSet<Revision> {
        self.retained_revisions().into_iter().collect()
    }

    /// Collect file BlobIds reachable from one revision set.
    fn reachable_file_blobs(&self, reachable_revisions: &HashSet<Revision>) -> HashSet<BlobId> {
        let mut reachable = HashSet::new();
        let roots = reachable_revisions
            .iter()
            .filter_map(|revision| self.revisions.get(revision))
            .map(|entry| entry.state().files())
            .collect::<Vec<_>>();

        // collect file Blobs from shared tree nodes once
        for entry in self.file_tree.unique_values(roots) {
            reachable.insert(entry.blob.id);
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
        self.file_tree.compact(&mut files);
        for (guard, files) in file_guards.iter_mut().zip(files) {
            **guard = files;
        }
        drop(file_guards);
    }

    /// Collect all retained revisions.
    fn retained_revisions(&self) -> Vec<Revision> {
        self.revisions
            .iter()
            .filter(|entry| entry.value().is_pinned())
            .map(|entry| *entry.key())
            .collect()
    }
}

/// One pinned repository revision.
#[derive(Debug)]
pub struct RevisionPin {
    /// The pinned revision identity.
    revision: Revision,
    /// The pinned immutable revision state.
    state: Arc<crate::repository::RevisionState>,
    /// The shared repository owner.
    repository: Arc<Repository>,
}

impl RevisionPin {
    /// Build one revision pin.
    pub(crate) fn new(
        revision: Revision,
        state: Arc<crate::repository::RevisionState>,
        repository: Arc<Repository>,
    ) -> Self {
        Self {
            revision,
            state,
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

    /// Return the retained immutable revision state.
    pub(crate) fn state(&self) -> &Arc<crate::repository::RevisionState> {
        &self.state
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
            state: self.state.clone(),
            repository: self.repository.clone(),
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, fs, process};

    use tspp_artifact::{
        ArtifactDependency, ArtifactKey, ArtifactProjection, ArtifactProjectionKey,
        ArtifactVersion, BuildId, Bundle, BundleFile, BundleMode, BundleSection, DirExported,
        EnvironmentBound, SourceDependency,
    };
    use tspp_dir::{ExportTable, GlobalTable};
    use tspp_source::{
        FileSystem, FileType, ModuleId, PackageId, PhysicalFileSystem, ProfileId, TargetId, Uri,
    };

    use crate::repository::{Change, Edit, Repository, RepositoryError, Revision};
    use crate::{Environment, Host, Settings, StorageLayout, StorageLayoutOverride};

    /// Create one repository for a test root.
    fn test_repository(root: &Path) -> (Repository, Revision) {
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let host = Host::new(BuildId::test(), environment, file_system);

        test_repository_with_host(root, host)
    }

    /// Create one repository under an existing host.
    fn test_repository_with_host(root: &Path, host: Host) -> (Repository, Revision) {
        let layout = StorageLayout::resolve(
            root,
            root,
            host.environment(),
            &Settings::default(),
            &StorageLayoutOverride::default(),
            None,
        );

        Repository::new(root.to_path_buf(), host, Settings::default(), layout)
    }

    /// Keep one revision alive only while it is pinned.
    #[test]
    fn test_prune_revision_after_pin_drops() {
        let root = unique_test_root("repository-pin");
        fs::create_dir_all(&root).expect("repository pin test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = StorageLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &StorageLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, file_system);
        let (repository, base_revision) =
            Repository::new(root.clone(), host, Settings::default(), layout);
        let repository = Arc::new(repository);
        let edited_revision = repository
            .edit(
                base_revision,
                [Edit::add_file(
                    "src/example.tspp",
                    repository
                        .retain_blob(b"export const value = 1")
                        .expect("source Blob should store"),
                )],
            )
            .expect("edited revision should publish")
            .after;
        let revision_pin = repository
            .pin(edited_revision)
            .expect("edited revision should pin");

        // pinned revision
        assert!(repository.revision(edited_revision).is_ok());

        // after the last pin drops, the edited revision becomes collectible
        drop(revision_pin);
        repository
            .prune_unreachable()
            .expect("repository should prune");
        assert!(repository.revision(edited_revision).is_err());

        // unpinned base revision
        assert!(repository.revision(base_revision).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    /// Apply source edits while package configuration is incomplete.
    #[test]
    fn test_edit_source_with_incomplete_configuration() {
        let root = unique_test_root("repository-incomplete-configuration");
        fs::create_dir_all(&root).unwrap();
        let (repository, revision) = test_repository(&root);
        let configuration = repository.retain_blob(b"{").unwrap();
        let source = repository.retain_blob(b"const value = 1;\n").unwrap();
        let revision = apply_edits(
            &repository,
            revision,
            [
                Edit::add_file("package.json", configuration),
                Edit::add_file("src/value.tspp", source),
            ],
        );

        // retain the source edit while configuration remains incomplete
        let replacement = repository.retain_blob(b"const value = 2;\n").unwrap();
        let result = repository.edit(revision, [Edit::set_file("src/value.tspp", replacement)]);
        fs::remove_dir_all(&root).unwrap();
        let commit = result.unwrap();
        assert_eq!(
            commit.changes,
            vec![Change {
                file: repository.file_id(Path::new("src/value.tspp")),
                path: "src/value.tspp".to_string(),
                before: Some(source),
                after: Some(replacement),
            }],
        );
    }

    /// Apply file edits and repair an invalid condition alias.
    #[test]
    fn test_repair_condition_alias() {
        let root = unique_test_root("repository-condition-alias");
        fs::create_dir_all(&root).unwrap();
        let (repository, revision) = test_repository(&root);
        let configuration = repository
            .retain_blob(
                br#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}
"#,
            )
            .unwrap();
        let source = repository.retain_blob(b"const value = 1;\n").unwrap();
        let revision = apply_edits(
            &repository,
            revision,
            [
                Edit::add_file("package.json", configuration),
                Edit::add_file("main.tspp", source),
            ],
        );
        let modules = repository.module_ids(revision).unwrap();

        // report the invalid alias during module discovery
        let invalid = repository
            .retain_blob(
                br#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app",
  "conditions": { "aliases": { "custom": "missing" } }
}
"#,
            )
            .unwrap();
        let broken = apply_edits(
            &repository,
            revision,
            [Edit::set_file("package.json", invalid)],
        );
        repository.package_ids(broken).unwrap();
        let expected = RepositoryError::InvalidConfig {
            file: repository.file_id(Path::new("package.json")),
            message: "unknown condition 'missing'".to_string(),
        };
        assert_eq!(repository.module_ids(broken).unwrap_err(), expected);

        // accept path edits while module discovery reports the invalid alias
        let added = apply_edits(&repository, broken, [Edit::add_file("other.tspp", source)]);
        assert_eq!(repository.module_ids(added).unwrap_err(), expected);
        let removed = apply_edits(&repository, added, [Edit::remove_file("other.tspp")]);
        let repaired = apply_edits(
            &repository,
            removed,
            [Edit::set_file("package.json", configuration)],
        );
        assert_eq!(repository.module_ids(repaired).unwrap(), modules);

        fs::remove_dir_all(root).unwrap();
    }

    /// Apply file edits and restore a missing inherited configuration.
    #[test]
    fn test_restore_configuration_parent() {
        let root = unique_test_root("repository-configuration-parent");
        fs::create_dir_all(&root).unwrap();
        let (repository, revision) = test_repository(&root);
        let configuration = repository
            .retain_blob(
                br#"{
  "packageManager": "tspp@2026.9.0",
  "extends": "./base.json"
}
"#,
            )
            .unwrap();
        let parent = repository
            .retain_blob(
                br#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}
"#,
            )
            .unwrap();
        let revision = apply_edits(
            &repository,
            revision,
            [
                Edit::add_file("package.json", configuration),
                Edit::add_file("base.json", parent),
            ],
        );
        let packages = repository.package_ids(revision).unwrap();

        // report the missing parent while accepting further file changes
        let moved = apply_edits(
            &repository,
            revision,
            [Edit::move_file("base.json", "moved.json")],
        );
        let expected = RepositoryError::MissingFile {
            path: root.join("base.json").display().to_string(),
        };
        assert_eq!(repository.package_ids(moved).unwrap_err(), expected);
        let source = repository.retain_blob(b"const value = 1;\n").unwrap();
        let edited = apply_edits(&repository, moved, [Edit::add_file("main.tspp", source)]);
        assert_eq!(repository.package_ids(edited).unwrap_err(), expected);

        // resolve packages after restoring the referenced parent
        let repaired = apply_edits(
            &repository,
            edited,
            [Edit::move_file("moved.json", "base.json")],
        );
        assert_eq!(repository.package_ids(repaired).unwrap(), packages);

        fs::remove_dir_all(root).unwrap();
    }

    /// Preserve package artifacts across module additions and invalidate configuration changes.
    #[test]
    fn test_add_module_preserves_package_artifacts() {
        assert_package_configuration_edits(
            "package.json",
            &[(
                "package.json",
                r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}
"#,
            )],
        );
    }

    /// Invalidate package artifacts when an inherited configuration changes.
    #[test]
    fn test_edit_inherited_package_configuration() {
        assert_package_configuration_edits(
            "base.json",
            &[
                (
                    "package.json",
                    r#"{
  "packageManager": "tspp@2026.9.0",
  "extends": "./base.json"
}
"#,
                ),
                (
                    "base.json",
                    r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}
"#,
                ),
            ],
        );
    }

    /// Invalidate package artifacts when inherited workspace configuration changes.
    #[test]
    fn test_edit_inherited_workspace_configuration() {
        assert_package_configuration_edits(
            "workspace.base.json",
            &[
                (
                    "package.json",
                    r#"{
  "packageManager": "tspp@2026.9.0",
  "extends": "./workspace.base.json"
}
"#,
                ),
                (
                    "workspace.base.json",
                    r#"{
  "packageManager": "tspp@2026.9.0",
  "workspaces": ["packages/*"]
}
"#,
                ),
                (
                    "packages/app/package.json",
                    r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "app"
}
"#,
                ),
            ],
        );
    }

    /// Assert artifact selection across module additions and configuration edits, moves, and removals.
    fn assert_package_configuration_edits(configuration_path: &str, files: &[(&str, &str)]) {
        let root = unique_test_root("repository-package-module");
        fs::create_dir_all(&root).unwrap();
        let (repository, revision) = test_repository(&root);
        let edits = files.iter().map(|(path, source)| {
            let blob = repository.retain_blob(source.as_bytes()).unwrap();

            Edit::add_file(path, blob)
        });
        let revision = apply_edits(&repository, revision, edits);
        let temporary = repository.retain_blob(b"").unwrap();
        let revision = apply_edits(
            &repository,
            revision,
            [Edit::add_file("src/temporary.tspp", temporary)],
        );
        let package = repository
            .package_by_name(revision, "app")
            .unwrap()
            .unwrap();
        let dependency = repository.package_dependency(revision, package.id).unwrap();
        let key = ArtifactKey::bundle(package.id, TargetId::new(package.id, "browser"));

        // publish a result that observes package configuration and dependency resolution
        let dependencies = vec![ArtifactDependency::Source(dependency)];
        let version = repository.artifact_identity(key, &dependencies);
        repository
            .complete_artifact(
                revision,
                key,
                Bundle::new(BundleMode::SingleFile, Vec::new()).into(),
                dependencies,
                Vec::new(),
                None,
            )
            .unwrap();

        // return to an earlier file revision after evaluating its package configuration
        let revision = apply_edits(
            &repository,
            revision,
            [Edit::remove_file("src/temporary.tspp")],
        );

        // preserve the selected result across consecutive module additions
        let source = repository
            .retain_blob(b"export const value = 1;\n")
            .unwrap();
        let mut revision = revision;
        for path in ["src/value.tspp", "src/other.tspp"] {
            let path = package.path.as_ref().unwrap().join(path);
            revision = apply_edits(
                &repository,
                revision,
                [Edit::add_file(path.to_str().unwrap(), source)],
            );
            let selected = repository
                .revision(revision)
                .unwrap()
                .artifacts
                .read()
                .current(key);
            assert_eq!(selected.map(|entry| entry.version), Some(version));
        }
        let source_path = package.path.as_ref().unwrap().join("src/value.tspp");

        // invalidate the result when its package configuration changes
        let config = repository
            .retain_blob(
                br#"{
  "packageManager": "tspp@2026.9.0",
  "name": "renamed"
}
"#,
            )
            .unwrap();
        for edit in [
            Edit::set_file(configuration_path, config),
            Edit::move_file(configuration_path, "moved.json"),
            Edit::remove_file(configuration_path),
        ] {
            let after = apply_edits(&repository, revision, [edit]);
            let selected = repository
                .revision(after)
                .unwrap()
                .artifacts
                .read()
                .current(key);
            assert_eq!(selected.map(|entry| entry.version), None);
        }

        // report incomplete configuration when resolving an affected artifact
        let configuration_file = repository.file_id(Path::new(configuration_path));
        let configuration = repository
            .file_blob(revision, configuration_file)
            .unwrap()
            .unwrap();
        let incomplete = repository.retain_blob(b"{").unwrap();
        let broken = apply_edits(
            &repository,
            revision,
            [Edit::set_file(configuration_path, incomplete)],
        );
        let error = repository.resolve_artifact(broken, &key).unwrap_err();
        let expected = RepositoryError::InvalidConfig {
            file: configuration_file,
            message: "EOF while parsing an object at line 1 column 1".to_string(),
        };
        assert_eq!(error, expected);

        // accept content and path edits while configuration is incomplete
        let replacement = repository
            .retain_blob(b"export const value = 2;\n")
            .unwrap();
        let source_path = source_path.to_str().unwrap();
        let edited = apply_edits(
            &repository,
            broken,
            [Edit::set_file(source_path, replacement)],
        );
        let moved = apply_edits(
            &repository,
            edited,
            [Edit::move_file(source_path, "moved.tspp")],
        );
        let removed = apply_edits(&repository, moved, [Edit::remove_file("moved.tspp")]);
        let added = apply_edits(
            &repository,
            removed,
            [Edit::add_file(source_path, replacement)],
        );
        assert_eq!(
            repository.resolve_artifact(added, &key).unwrap_err(),
            expected
        );

        // repair configuration and validate the retained artifact against its actual dependencies
        let repaired = apply_edits(
            &repository,
            added,
            [Edit::set_file(configuration_path, configuration)],
        );
        let expected = repository.resolve_artifact(revision, &key).unwrap();
        assert_eq!(
            repository.resolve_artifact(repaired, &key).unwrap(),
            expected
        );
        assert_eq!(
            repository
                .file_blob(repaired, repository.file_id(Path::new(source_path)))
                .unwrap(),
            Some(replacement)
        );

        fs::remove_dir_all(root).unwrap();
    }

    /// Reuse one live artifact result across repositories under the same host.
    #[test]
    fn test_reuse_artifact_across_hosted_repositories() {
        let first_root = unique_test_root("repository-artifact-host-first");
        let second_root = unique_test_root("repository-artifact-host-second");
        fs::create_dir_all(&first_root).expect("first repository root should exist");
        fs::create_dir_all(&second_root).expect("second repository root should exist");

        // create two repositories under one process host
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let host = Host::new(BuildId::test(), environment, file_system);
        let (first, first_revision) = test_repository_with_host(&first_root, host.clone());
        let (second, second_revision) = test_repository_with_host(&second_root, host);
        let first = Arc::new(first);
        let second = Arc::new(second);

        // publish one exact result in the first repository
        let package = PackageId::new(1);
        let target = TargetId::new(package, "browser");
        let key = ArtifactKey::bundle(package, target);
        let output = Bundle::new(BundleMode::SingleFile, Vec::new());
        first
            .complete_artifact(
                first_revision,
                key,
                output.into(),
                Vec::new(),
                Vec::new(),
                None,
            )
            .expect("first repository should publish artifact");
        let version = first.artifact_identity(key, &[]);

        // reuse the shared result as a base before selecting its exact version
        let base = second
            .artifact_base(second_revision, key)
            .expect("second repository should inspect shared bases")
            .expect("shared artifact base should exist");
        assert_eq!(base.version(), version);

        // select the shared result in the second repository
        assert!(
            second
                .select_artifact_version(second_revision, key, Vec::new())
                .expect("second repository should inspect shared artifact")
        );
        let second_pin = second
            .pin(second_revision)
            .expect("second repository revision should pin");

        // pruning the first repository must preserve the second selection
        first
            .prune_unreachable()
            .expect("first repository should prune");
        assert!(
            second
                .artifact_table()
                .payload(&version)
                .expect("read shared artifact payload")
                .is_some()
        );

        drop(second_pin);
        let _ = fs::remove_dir_all(&first_root);
        let _ = fs::remove_dir_all(&second_root);
    }

    /// Build one unique root for one repository test.
    fn unique_test_root(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock should be after unix epoch")
            .as_nanos();
        let process_id = process::id();

        env::temp_dir().join(format!("tspp-{prefix}-{process_id}-{timestamp}"))
    }

    /// Prune old unpinned revisions during explicit retention.
    #[test]
    fn test_prune_unpinned_revision_during_explicit_retention() {
        let root = unique_test_root("repository-prune");
        fs::create_dir_all(&root).expect("repository history test root should exist");

        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let environment = Environment::capture_process();
        let layout = StorageLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &StorageLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, file_system);
        let (repository, initial) =
            Repository::new(root.clone(), host, Settings::default(), layout);
        let repository = Arc::new(repository);
        let revision_1 = apply_edits(
            repository.as_ref(),
            initial,
            [Edit::add_file(
                "src/example.tspp",
                repository
                    .retain_blob(b"export const value = 1")
                    .expect("source Blob should store"),
            )],
        );
        let revision_2 = apply_edits(
            repository.as_ref(),
            revision_1,
            [Edit::set_file(
                "src/example.tspp",
                repository
                    .retain_blob(b"export const value = 2")
                    .expect("source Blob should store"),
            )],
        );
        let _revision_pin = repository.pin(revision_2).expect("pin retained revision");
        let file_id = repository.file_id(&root.join("src/example.tspp"));

        // retained revision
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

    /// Retain only the exact artifact version selected by a live revision.
    #[test]
    fn test_prune_unselected_artifact_version() {
        let root = unique_test_root("repository-artifact-version-prune");
        fs::create_dir_all(&root).expect("repository artifact version test root should exist");

        let (repository, initial) = test_repository(&root);
        let repository = Arc::new(repository);
        let first_revision = apply_edits(
            &repository,
            initial,
            [Edit::add_file(
                "src/input.tspp",
                repository
                    .retain_blob(b"export const value = 1;")
                    .expect("source Blob should store"),
            )],
        );
        let file_id = repository.file_id(&root.join("src/input.tspp"));
        let first_blob = repository
            .file_blob(first_revision, file_id)
            .expect("first source Blob should resolve")
            .expect("first source Blob should exist");
        let first_dependencies = vec![ArtifactDependency::Source(SourceDependency::file(
            file_id,
            first_blob.id,
        ))];

        // publish the first artifact version
        let package = PackageId::new(1);
        let target = TargetId::new(package, "browser");
        let output_blob = repository
            .retain_blob(b"console.log('same output')\n")
            .expect("artifact output should store");
        let output = Bundle::new(
            BundleMode::SingleFile,
            vec![BundleFile::new(
                BundleSection::Entry,
                Uri::from_string("memory:/out.js"),
                FileType::JavaScript,
                output_blob,
                None,
            )],
        );
        let key = ArtifactKey::bundle(package, target);
        let first_version = ArtifactVersion::new(
            key,
            repository.host().build_id(),
            first_dependencies.clone(),
        );
        repository
            .complete_artifact(
                first_revision,
                key,
                output.clone().into(),
                first_dependencies,
                Vec::new(),
                None,
            )
            .expect("first artifact version should publish");
        let first_pin = repository
            .pin(first_revision)
            .expect("first artifact revision should pin");

        // publish a different version with the same payload
        let second_revision = apply_edits(
            &repository,
            first_revision,
            [Edit::set_file(
                "src/input.tspp",
                repository
                    .retain_blob(b"export const value = 2;")
                    .expect("source Blob should store"),
            )],
        );
        let _second_pin = repository
            .pin(second_revision)
            .expect("second artifact revision should pin");
        let second_blob = repository
            .file_blob(second_revision, file_id)
            .expect("second source Blob should resolve")
            .expect("second source Blob should exist");
        let second_dependencies = vec![ArtifactDependency::Source(SourceDependency::file(
            file_id,
            second_blob.id,
        ))];
        let second_version = ArtifactVersion::new(
            key,
            repository.host().build_id(),
            second_dependencies.clone(),
        );
        repository
            .complete_artifact(
                second_revision,
                key,
                output.clone().into(),
                second_dependencies,
                Vec::new(),
                None,
            )
            .expect("second artifact version should publish");
        assert_ne!(first_version, second_version);

        // retain both exact versions while separate pins select them
        repository
            .prune_unreachable()
            .expect("repository should retain both revisions");
        assert!(
            repository
                .artifact_table()
                .payload(&first_version)
                .expect("read first artifact payload")
                .is_some()
        );
        assert!(
            repository
                .artifact_table()
                .payload(&second_version)
                .expect("read second artifact payload")
                .is_some()
        );

        // release the first revision and prune its exact version
        drop(first_pin);
        repository
            .prune_unreachable()
            .expect("repository should prune");
        assert!(
            repository
                .artifact_table()
                .payload(&first_version)
                .expect("read released artifact payload")
                .is_none()
        );
        assert!(
            repository
                .artifact_table()
                .payload(&second_version)
                .expect("read retained artifact payload")
                .is_some()
        );

        // continue from the retained revision after releasing the first result
        let third_revision = apply_edits(
            &repository,
            second_revision,
            [Edit::set_file(
                "src/input.tspp",
                repository
                    .retain_blob(b"export const value = 3;")
                    .expect("source Blob should store"),
            )],
        );
        let third_blob = repository
            .file_blob(third_revision, file_id)
            .expect("third source Blob should resolve")
            .expect("third source Blob should exist");
        let third_dependencies = vec![ArtifactDependency::Source(SourceDependency::file(
            file_id,
            third_blob.id,
        ))];
        let third_version = ArtifactVersion::new(
            key,
            repository.host().build_id(),
            third_dependencies.clone(),
        );
        repository
            .complete_artifact(
                third_revision,
                key,
                output.into(),
                third_dependencies,
                Vec::new(),
                None,
            )
            .expect("third artifact version should publish");
        assert!(
            repository
                .artifact_table()
                .payload(&second_version)
                .expect("read preceding artifact payload")
                .is_some()
        );
        assert!(
            repository
                .artifact_table()
                .payload(&third_version)
                .expect("read current artifact payload")
                .is_some()
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// Reuse one result across revisions with different exact projection owners.
    #[test]
    fn test_reuse_artifact_version_across_projection_owners() {
        let root = unique_test_root("repository-artifact-projection-revisions");
        fs::create_dir_all(&root).expect("repository projection test root should exist");

        let (repository, initial) = test_repository(&root);
        let repository = Arc::new(repository);
        let first_revision = apply_edits(
            &repository,
            initial,
            [Edit::add_file(
                "src/input.tspp",
                repository
                    .retain_blob(b"export const value = 1;")
                    .expect("source Blob should store"),
            )],
        );
        // build one projected owner and dependent result on the first revision
        let package = PackageId::new(1);
        let module = ModuleId::new(package, 1);
        let profile = ProfileId::new(1);
        let file = repository.file_id(&root.join("src/input.tspp"));
        let first_blob = repository
            .file_blob(first_revision, file)
            .expect("first source Blob should resolve")
            .expect("first source Blob should exist");
        let first_owner_dependencies = vec![ArtifactDependency::Source(SourceDependency::file(
            file,
            first_blob.id,
        ))];
        let owner = DirExported {
            exports: ExportTable::new(module),
            globals: GlobalTable::new(module),
            locals: Vec::new(),
        };
        let owner_key = ArtifactKey::dir_exported(module, profile);
        let first_owner = ArtifactVersion::new(
            owner_key,
            repository.host().build_id(),
            first_owner_dependencies.clone(),
        );
        repository
            .complete_artifact(
                first_revision,
                owner_key,
                owner.clone().into(),
                first_owner_dependencies,
                Vec::new(),
                None,
            )
            .expect("first component graph should publish");
        let projection = ArtifactProjection::new(owner_key, ArtifactProjectionKey::Payload);
        let fingerprint = repository
            .artifact_table()
            .projection_fingerprint(&first_owner, &projection)
            .expect("component projection should fingerprint")
            .expect("component projection should exist");
        let first_dependency =
            ArtifactDependency::projection(first_owner.key, projection.key, fingerprint);
        let dependent_key = ArtifactKey::environment_bound(profile);
        let dependent = ArtifactVersion::new(
            dependent_key,
            repository.host().build_id(),
            [first_dependency.clone()],
        );
        repository
            .complete_artifact(
                first_revision,
                dependent_key,
                EnvironmentBound::default().into(),
                vec![first_dependency],
                Vec::new(),
                None,
            )
            .expect("dependent result should publish");

        // change only the owner's exact source observation on the second revision
        let second_revision = apply_edits(
            &repository,
            first_revision,
            [Edit::set_file(
                "src/input.tspp",
                repository
                    .retain_blob(b"export const value = 2;")
                    .expect("source Blob should store"),
            )],
        );
        let second_blob = repository
            .file_blob(second_revision, file)
            .expect("second source Blob should resolve")
            .expect("second source Blob should exist");
        let second_owner_dependencies = vec![ArtifactDependency::Source(SourceDependency::file(
            file,
            second_blob.id,
        ))];
        let second_owner = ArtifactVersion::new(
            owner_key,
            repository.host().build_id(),
            second_owner_dependencies.clone(),
        );
        assert_ne!(first_owner, second_owner);
        repository
            .complete_artifact(
                second_revision,
                owner_key,
                owner.into(),
                second_owner_dependencies,
                Vec::new(),
                None,
            )
            .expect("second component graph should publish");

        // reuse the dependent version with each revision's exact projection owner
        let resolved = repository
            .artifact_version(second_revision, &dependent_key)
            .expect("dependent result should resolve");
        assert_eq!(resolved, Some(dependent));
        let first_base = repository
            .artifact_base(first_revision, dependent_key)
            .expect("first revision artifact should resolve")
            .expect("first revision artifact should exist");
        let second_base = repository
            .artifact_base(second_revision, dependent_key)
            .expect("second revision artifact should resolve")
            .expect("second revision artifact should exist");
        let [ArtifactDependency::Projection(first_projection)] = first_base.dependencies() else {
            panic!("first revision should select one projection dependency");
        };
        let [ArtifactDependency::Projection(second_projection)] = second_base.dependencies() else {
            panic!("second revision should select one projection dependency");
        };
        assert_eq!(first_base.version(), dependent);
        assert_eq!(second_base.version(), dependent);
        assert_eq!(first_projection.projection().artifact, first_owner.key);
        assert_eq!(second_projection.projection().artifact, second_owner.key);
        assert_eq!(
            first_projection.fingerprint(),
            second_projection.fingerprint()
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// Apply repository edits to one exact revision.
    fn apply_edits<I>(repository: &Repository, revision: Revision, edits: I) -> Revision
    where
        I: IntoIterator<Item = Edit>,
    {
        repository
            .edit(revision, edits)
            .expect("repository edits should apply")
            .after
    }
}
