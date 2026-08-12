use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_artifact::BuildId;
use destack_core::Blob;
use destack_dir as dir;
use destack_repository::{
    Change, Commit, DestackLayoutOverride, Environment, Execution, Host, Repository, Revision,
    Settings,
};
use destack_session::Executor;
use destack_source::{
    Edit, FileId, FileMetadata, FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem, Uri,
};
use futures::executor::block_on;

use crate::command::{
    CommandInput, CommandOptions, CommandRevision, QueryInput, QueryOutput, RewriteInput,
    RewriteMode, RewriteOutput,
};
use crate::{Error, Workspace};

/// Test harness for workspace integration tests.
#[derive(Debug)]
pub(super) struct TestWorkspace {
    /// Temporary filesystem root.
    pub fs: TemporaryPhysicalFileSystem,
    /// Workspace under test.
    pub workspace: Workspace,
    /// Canonical workspace root.
    pub root: PathBuf,
}

impl TestWorkspace {
    /// Create a new harness rooted at a temporary source root.
    pub(super) fn new(prefix: &str) -> Self {
        Self::build(prefix, |_| Arc::new(PhysicalFileSystem::new()))
    }

    /// Create a new harness whose first write to one path fails.
    pub(super) fn new_with_write_failure(prefix: &str, path: &str, content: &str) -> Self {
        let content = content.as_bytes().to_vec();

        Self::build(prefix, |root| {
            Arc::new(FailingFileSystem::fail_once(root.join(path), content))
        })
    }

    /// Create one harness over an explicit repository filesystem.
    fn build(prefix: &str, file_system: impl FnOnce(&Path) -> Arc<dyn FileSystem>) -> Self {
        // create a temporary filesystem root for test files
        let fs = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let root = fs.root().to_path_buf();
        let file_system = file_system(&root);

        // name the package before opening the repository
        let config = root.join("destack.json");
        fs.write_text(&config, "{ \"name\": \"test\" }\n")
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", config.display()));

        // create a repository over the selected physical filesystem
        let host = Host::new(BuildId::test(), Environment::capture_process(), file_system);
        let (repository, physical) = Repository::open(
            root.clone(),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .expect("failed to import repository from physical fs");
        let repository = Arc::new(repository);
        let executor = Executor::new(Execution::Threaded, 1).expect("create executor");
        let workspace = Workspace::new(repository, physical, executor).expect("expected workspace");

        Self {
            fs,
            workspace,
            root,
        }
    }

    /// Resolve a path under the temporary root.
    pub(super) fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.join(path.as_ref())
    }

    /// Build a source URI for one path.
    pub(super) fn uri_for_path(&self, path: &Path) -> Uri {
        Uri::from_file_path(path)
    }

    /// Build one exact expected text change.
    pub(super) fn change(path: &str, before: Option<&str>, after: Option<&str>) -> Change {
        Change {
            file: FileId::from_logical_str(path),
            path: path.to_string(),
            before: before.map(|text| Blob::for_bytes(text.as_bytes())),
            after: after.map(|text| Blob::for_bytes(text.as_bytes())),
        }
    }

    /// Write text under the default root.
    pub(super) fn write_text(&self, path: impl AsRef<Path>, source: &str) -> PathBuf {
        let path = self.path_for(path);
        self.fs
            .write_text(&path, source)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
        path
    }

    /// Write one text file at the current physical revision.
    pub(super) fn apply_text(&self, path: &Path, source: &str) -> Commit {
        let edits = vec![Edit::SetText {
            path: path.to_path_buf(),
            text: source.to_string(),
        }];

        self.write(edits)
            .unwrap_or_else(|error| panic!("failed file update for {}: {error}", path.display()))
    }

    /// Write source edits against exact physical workspace state.
    pub(super) fn write(&self, edits: Vec<Edit>) -> Result<Commit, Error> {
        let revision = self.workspace.revision()?;

        self.workspace.edit(revision, edits)
    }
}

/// Workspace fixture for Pattern commands.
pub(super) struct TestPattern {
    /// Workspace harness.
    pub(super) harness: TestWorkspace,
    /// Selected command inputs.
    inputs: Vec<CommandInput>,
}

impl TestPattern {
    /// Create an empty Pattern command fixture.
    pub(super) fn new(name: &str) -> Self {
        Self {
            harness: TestWorkspace::new(name),
            inputs: Vec::new(),
        }
    }

    /// Add one authored file outside the selected command inputs.
    pub(super) fn file(self, path: &str, source: &str) -> Self {
        self.write(path, source);

        self
    }

    /// Add one authored file to the selected command inputs.
    pub(super) fn input(mut self, path: &str, source: &str) -> Self {
        let path = self.write(path, source);
        self.inputs.push(CommandInput::File { path });

        self
    }

    /// Build one structural Query.
    pub(super) fn query(&self, pattern: &str) -> TestQuery<'_> {
        let options = CommandOptions {
            inputs: self.inputs.clone(),
            ..CommandOptions::default()
        };
        let mut input = QueryInput::from((CommandRevision::Current, options));
        input.pattern = pattern.to_string();

        TestQuery {
            fixture: self,
            input,
        }
    }

    /// Build one structural Rewrite.
    pub(super) fn rewrite(&self, pattern: &str, replacement: &str) -> TestRewrite<'_> {
        let options = CommandOptions {
            inputs: self.inputs.clone(),
            ..CommandOptions::default()
        };
        let mut input = RewriteInput::from((CommandRevision::Current, options));
        input.pattern = pattern.to_string();
        input.replacement = replacement.to_string();

        TestRewrite {
            fixture: self,
            input,
        }
    }

    /// Return one authored file path.
    pub(super) fn path(&self, path: &str) -> PathBuf {
        self.harness.path_for(path)
    }

    /// Return one authored file URI.
    pub(super) fn uri(&self, path: &str) -> Uri {
        self.harness.uri_for_path(&self.path(path))
    }

    /// Read one authored file from disk.
    pub(super) fn source(&self, path: &str) -> String {
        let path = self.path(path);

        self.harness
            .fs
            .read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    }

    /// Return the current Workspace revision.
    pub(super) fn revision(&self) -> Revision {
        self.harness
            .workspace
            .revision()
            .expect("read Pattern test revision")
    }

    /// Write and publish one authored file.
    fn write(&self, path: &str, source: &str) -> PathBuf {
        let path = self.harness.write_text(path, source);
        self.harness.apply_text(&path, source);

        path
    }
}

/// One configurable Pattern Query fixture.
pub(super) struct TestQuery<'test> {
    /// Pattern fixture.
    fixture: &'test TestPattern,
    /// Query input.
    input: QueryInput,
}

impl TestQuery<'_> {
    /// Select one contextual DIR node type.
    pub(super) fn kind(mut self, kind: dir::NodeType) -> Self {
        self.input.kind = Some(kind);

        self
    }

    /// Add one semantic predicate.
    pub(super) fn where_(mut self, predicate: &str) -> Self {
        self.input.predicates.push(predicate.to_string());

        self
    }

    /// Retain matched source files in the response.
    pub(super) fn include_sources(mut self) -> Self {
        self.input.include_sources = true;

        self
    }

    /// Execute this Query.
    pub(super) fn run(self) -> QueryOutput {
        block_on(self.fixture.harness.workspace.query(self.input, None)).expect("run Pattern Query")
    }
}

/// One configurable Pattern Rewrite fixture.
pub(super) struct TestRewrite<'test> {
    /// Pattern fixture.
    fixture: &'test TestPattern,
    /// Rewrite input.
    input: RewriteInput,
}

impl TestRewrite<'_> {
    /// Select one contextual DIR node type.
    pub(super) fn kind(mut self, kind: dir::NodeType) -> Self {
        self.input.kind = Some(kind);

        self
    }

    /// Add one semantic predicate.
    pub(super) fn where_(mut self, predicate: &str) -> Self {
        self.input.predicates.push(predicate.to_string());

        self
    }

    /// Select Rewrite execution behavior.
    pub(super) fn mode(mut self, mode: RewriteMode) -> Self {
        self.input.mode = mode;

        self
    }

    /// Execute this Rewrite.
    pub(super) fn run(self) -> RewriteOutput {
        block_on(self.fixture.harness.workspace.rewrite(self.input, None))
            .expect("run Pattern Rewrite")
    }
}

/// Physical filesystem that fails one selected write exactly once.
#[derive(Debug)]
struct FailingFileSystem {
    /// The physical filesystem receiving operations.
    physical: PhysicalFileSystem,
    /// The path whose first write fails.
    path: PathBuf,
    /// Exact bytes whose first write fails.
    content: Vec<u8>,
    /// Whether the configured failure remains armed.
    is_armed: AtomicBool,
}

impl FailingFileSystem {
    /// Create one armed write failure.
    fn fail_once(path: PathBuf, content: Vec<u8>) -> Self {
        Self {
            physical: PhysicalFileSystem::new(),
            path,
            content,
            is_armed: AtomicBool::new(true),
        }
    }
}

impl FileSystem for FailingFileSystem {
    /// Create an unarmed physical filesystem wrapper.
    fn new() -> Self {
        Self {
            physical: PhysicalFileSystem::new(),
            path: PathBuf::new(),
            content: Vec::new(),
            is_armed: AtomicBool::new(false),
        }
    }

    /// Return whether one path exists.
    fn exists(&self, path: &Path) -> io::Result<bool> {
        self.physical.exists(path)
    }

    /// Return metadata for one path.
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.physical.metadata(path)
    }

    /// Resolve one symbolic link.
    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        self.physical.resolve_symlink(path)
    }

    /// Canonicalize one path.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.physical.canonicalize(path)
    }

    /// Open one file as a byte stream.
    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        self.physical.open(path)
    }

    /// Read one file.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.physical.read(path)
    }

    /// Read one directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        self.physical.read_dir(path)
    }

    /// Read one UTF-8 file.
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.physical.read_to_string(path)
    }

    /// Return metadata without following symbolic links.
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.physical.symlink_metadata(path)
    }

    /// Write one file unless its configured failure remains armed.
    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        let is_failure = path == self.path && content == self.content;
        if is_failure && self.is_armed.swap(false, Ordering::AcqRel) {
            return Err(io::Error::other("injected write failure"));
        }

        self.physical.write(path, content)
    }

    /// Create one directory.
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.physical.create_dir(path)
    }

    /// Create one directory tree.
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.physical.create_dir_all(path)
    }

    /// Remove one file.
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.physical.remove_file(path)
    }

    /// Remove one empty directory.
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.physical.remove_dir(path)
    }
}
