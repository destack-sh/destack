use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactVersion, MemoryBlobStore};
use destack_repository::{
    DestackLayoutOverride, Environment, Execution, Host, Ref, Repository, Revision, Settings,
    TraceSnapshot, TraceView, open_repository,
};
use destack_source::{Edit, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};

use crate::{Change, Commit, PreparedCommit, Session, SessionError};

const DEFAULT_ROOT: &str = "/workspace";

/// In-memory session exercise harness.
pub(crate) struct TestSession {
    /// The source root selected by session opening.
    root: PathBuf,
    /// The memory filesystem backing the session.
    fs: Arc<MemoryFileSystem>,
    /// The repository imported by the session.
    repository: Arc<Repository>,
    /// The live session under test.
    session: Session,
}

impl TestSession {
    /// Open one test session from files below the default root.
    pub(crate) fn open(files: &[(&str, &str)]) -> Result<Self, SessionError> {
        Self::open_with_root(DEFAULT_ROOT, files, 1)
    }

    /// Open one test session from files below the default root with explicit worker count.
    pub(crate) fn open_with_workers(
        files: &[(&str, &str)],
        worker_count: usize,
    ) -> Result<Self, SessionError> {
        Self::open_with_root(DEFAULT_ROOT, files, worker_count)
    }

    /// Open one test session from a specific input path.
    pub(crate) fn open_from(
        input: impl AsRef<Path>,
        files: &[(&str, &str)],
    ) -> Result<Self, SessionError> {
        Self::open_with_root(input, files, 1)
    }

    /// Open one test session from a specific input path and worker count.
    fn open_with_root(
        input: impl AsRef<Path>,
        files: &[(&str, &str)],
        worker_count: usize,
    ) -> Result<Self, SessionError> {
        let execution = if worker_count == 1 {
            Execution::Inline
        } else {
            Execution::Threaded
        };

        Self::create(input, files, worker_count, execution)
    }

    /// Create one test session with explicit executor behavior.
    fn create(
        input: impl AsRef<Path>,
        files: &[(&str, &str)],
        worker_count: usize,
        execution: Execution,
    ) -> Result<Self, SessionError> {
        let root = PathBuf::from(DEFAULT_ROOT);
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("test root directory should be created");

        for (path, content) in files {
            fs.write_string(&root.join(path), content)
                .expect("test file should write");
        }

        let host = Host::new(
            Environment::default(),
            fs.clone(),
            Arc::new(MemoryBlobStore::new()),
        )
        .with_execution(execution);
        let repository = open_repository(
            PathBuf::from(input.as_ref()),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;
        let repository = Arc::new(repository);
        let root = repository.path().to_path_buf();
        let head = Ref::for_root(&root);
        let session = Session::new(
            root.clone(),
            root.clone(),
            repository.clone(),
            head,
            worker_count,
            None,
        )?;

        Ok(Self {
            root,
            fs,
            repository,
            session,
        })
    }

    /// Write one file below the selected source root.
    pub(crate) fn write(&self, path: &str, content: &str) {
        self.fs
            .write_string(&self.root.join(path), content)
            .expect("test file should write");
    }

    /// Remove one file below the selected source root.
    pub(crate) fn remove(&self, path: &str) {
        self.fs
            .remove_file(&self.root.join(path))
            .expect("test file should remove");
    }

    /// Reload the session from its memory filesystem.
    pub(crate) fn reload(&self) -> Vec<Change> {
        self.session
            .reload_from_fs(&self.head())
            .expect("test session should reload")
    }

    /// Edit files through the session.
    pub(crate) fn edit(&self, edits: Vec<Edit>) -> Commit {
        self.session
            .edit(&self.head(), edits)
            .expect("test session should edit")
    }

    /// Replace one source file through the session.
    pub(crate) fn edit_text(&self, path: &str, text: &str) -> Commit {
        let edit = Edit::SetText {
            path: path.into(),
            text: text.into(),
        };

        self.edit(vec![edit])
    }

    /// Prepare one source file replacement without publishing it.
    pub(crate) fn prepare_text(&self, path: &str, text: &str) -> PreparedCommit<'_> {
        let edit = Edit::SetText {
            path: path.into(),
            text: text.into(),
        };
        let revision = self.revision();

        self.session
            .prepare_edit(&self.head(), revision, vec![edit])
            .expect("test session should prepare edit")
    }

    /// Load one module from the memory filesystem.
    pub(crate) fn load_module(&self, path: &str) {
        let path = self.root.join(path);
        self.session
            .load_module_from_fs(&self.head(), &path)
            .expect("test module should load");
    }

    /// Check one module target.
    pub(crate) fn check(&self, path: &str, target: &str) -> (ArtifactVersion, TraceSnapshot) {
        let revision = self.revision();
        let module = self.module_id(path, revision);
        let profile = self.profile_id(revision, module, target);
        let key = ArtifactKey::dir_checked(module, profile);
        let version = self
            .session
            .require(revision, key)
            .expect("test artifact should be required");
        let trace = self.trace();

        (version, trace)
    }

    /// Assert the selected source root.
    pub(crate) fn assert_root(&self, expected: &str) {
        assert_eq!(self.root, PathBuf::from(DEFAULT_ROOT).join(expected));
    }

    /// Assert the editable repository files visible at the current head.
    pub(crate) fn assert_files(&self, expected: &[&str]) {
        let actual = self.files();
        let mut expected = expected
            .iter()
            .map(|path| path.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(actual, expected);
    }

    /// Assert the number of editable repository files visible at the current head.
    pub(crate) fn assert_file_count(&self, expected: usize) {
        assert_eq!(self.files().len(), expected);
    }

    /// Assert update paths relative to the selected source root.
    pub(crate) fn assert_update_paths(&self, updates: &[Change], expected: &[&str]) {
        let mut actual = updates
            .iter()
            .map(|update| self.update_path(update))
            .collect::<Vec<_>>();
        actual.sort();

        let mut expected = expected
            .iter()
            .map(|path| path.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(actual, expected);
    }

    /// Assert that every update removed a file.
    pub(crate) fn assert_removed_updates(&self, updates: &[Change]) {
        assert!(
            updates.iter().all(Change::is_removed),
            "expected only removed updates, got {updates:#?}",
        );
    }

    /// Assert that no updates were emitted.
    pub(crate) fn assert_no_updates(&self, updates: &[Change]) {
        assert!(updates.is_empty(), "expected no updates, got {updates:#?}");
    }

    /// Assert file commit paths relative to the selected source root.
    pub(crate) fn assert_commit_paths(&self, commit: &Commit, expected: &[&str]) {
        self.assert_update_paths(&commit.changes, expected);
    }

    /// Assert that reloading changes the head revision.
    pub(crate) fn assert_revision_changed(&self, before: Revision) {
        assert_ne!(self.revision(), before);
    }

    /// Return the current head revision.
    pub(crate) fn revision(&self) -> Revision {
        self.repository
            .current(&self.head())
            .expect("test revision should exist")
    }

    /// Return the default session ref.
    fn head(&self) -> Ref {
        Ref::for_root(&self.root)
    }

    /// Return one module id at one revision.
    fn module_id(&self, path: &str, revision: Revision) -> ModuleId {
        self.repository
            .module_id_for_path(revision, &self.root.join(path))
            .expect("test module should resolve")
            .expect("test module should exist")
    }

    /// Return one profile id at one revision.
    fn profile_id(&self, revision: Revision, module: ModuleId, target: &str) -> ProfileId {
        let module = self
            .repository
            .module(revision, module)
            .expect("test module should load")
            .expect("test module should exist");
        let target = TargetId::new(module.package_id, target);
        let profile = self
            .repository
            .profile_for_module_target(revision, module.id, target)
            .expect("test profile should resolve");

        profile.id()
    }

    /// Return the latest detailed trace snapshot.
    fn trace(&self) -> TraceSnapshot {
        let trace = self
            .session
            .last_trace()
            .expect("test trace should be recorded");

        trace
            .snapshot(
                TraceView::Detailed,
                |_| Ok::<_, ()>(None),
                |_| Ok::<_, ()>(None),
            )
            .unwrap()
    }

    /// Return editable repository files at the current head.
    fn files(&self) -> Vec<String> {
        let revision = self.revision();
        let mut files = self
            .repository
            .editable_file_logical_paths(revision)
            .expect("test file paths should load")
            .into_iter()
            .map(|(_, path)| {
                let path = self.repository.string_pool().get(path);

                path.to_string()
            })
            .collect::<Vec<_>>();
        files.sort();

        files
    }

    /// Return one tracked text file at the current head.
    pub(crate) fn text(&self, path: &str) -> String {
        let revision = self.revision();
        let file_id = destack_source::FileId::from_logical_str(path);
        let file = self
            .repository
            .file(revision, file_id)
            .expect("test file should load")
            .expect("test file should exist");

        file.text().to_string()
    }

    /// Return one update path relative to the selected source root.
    fn update_path(&self, update: &Change) -> String {
        let path = update
            .uri()
            .to_path_buf()
            .expect("test update uri should be a path");
        let path = path.strip_prefix(&self.root).unwrap_or(&path);

        path.to_string_lossy().replace('\\', "/")
    }
}
