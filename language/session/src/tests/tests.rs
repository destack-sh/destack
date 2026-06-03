use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_source::{FileSystem, MemoryFileSystem};
use destack_workspace::{DestackLayoutOverride, Environment, Ref, Repository, Revision, Settings};

use crate::{FileUpdate, Session, SessionError, open_repository_from_fs};

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
        Self::open_at(DEFAULT_ROOT, files)
    }

    /// Open one test session from a specific input path.
    pub(crate) fn open_at(
        input: impl AsRef<Path>,
        files: &[(&str, &str)],
    ) -> Result<Self, SessionError> {
        let root = PathBuf::from(DEFAULT_ROOT);
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("test root directory should be created");

        for (path, content) in files {
            fs.write_string(&root.join(path), content)
                .expect("test file should write");
        }

        let repository = open_repository_from_fs(
            PathBuf::from(input.as_ref()),
            fs.clone(),
            Environment::default(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;
        let repository = Arc::new(repository);
        let root = repository.workspace_root().to_path_buf();
        let head = Ref::for_workspace_root(&root);
        let compiler = Arc::new(Compiler::new(repository.clone()));
        let linter = Arc::new(Linter::new(repository.clone()));
        let query = Arc::new(Query::new(repository.clone()));
        let session = Session::new(
            root.clone(),
            root.clone(),
            repository.clone(),
            head,
            compiler,
            linter,
            query,
            1,
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
    pub(crate) fn reload(&self) -> Vec<FileUpdate> {
        self.session
            .reload_from_fs(&self.head())
            .expect("test session should reload")
    }

    /// Load one module from the memory filesystem.
    pub(crate) fn load_module(&self, path: &str) {
        let path = self.root.join(path);
        self.session
            .load_module_from_fs(&self.head(), &path)
            .expect("test module should load");
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
    pub(crate) fn assert_update_paths(&self, updates: &[FileUpdate], expected: &[&str]) {
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
    pub(crate) fn assert_removed_updates(&self, updates: &[FileUpdate]) {
        assert!(
            updates.iter().all(FileUpdate::is_removed),
            "expected only removed updates, got {updates:#?}",
        );
    }

    /// Assert that no updates were emitted.
    pub(crate) fn assert_no_updates(&self, updates: &[FileUpdate]) {
        assert!(updates.is_empty(), "expected no updates, got {updates:#?}");
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
        Ref::for_workspace_root(&self.root)
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

    /// Return one update path relative to the selected source root.
    fn update_path(&self, update: &FileUpdate) -> String {
        let path = update
            .uri()
            .to_path_buf()
            .expect("test update uri should be a path");
        let path = path.strip_prefix(&self.root).unwrap_or(&path);

        path.to_string_lossy().replace('\\', "/")
    }
}
