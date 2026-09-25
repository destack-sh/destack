use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::executor::block_on;
use tspp_artifact::{ArtifactCache, BuildId};
use tspp_core::Blob;
use tspp_dir as dir;
use tspp_repository::{
    Change, Commit, DestackLayoutOverride, Environment, Execution, Host, Repository, Revision,
    Settings, TraceLevel,
};
use tspp_session::Executor;
use tspp_source::{
    DiagnosticLabel, DiagnosticTarget, Edit, FileId, FileMetadata, FileSystem, PhysicalFileSystem,
    Span, TemporaryPhysicalFileSystem, Uri,
};

use crate::command::{
    CheckInput, CommandInput, CommandOptions, CommandRevision, QueryInput, QueryOutput,
    RewriteInput, RewriteMode, RewriteOutput,
};
use crate::{CheckOutput, DiagnosticsRequest, Error, FileDiagnostics, Workspace};

/// Test harness for workspace integration tests.
#[derive(Debug)]
pub(super) struct TestWorkspace {
    /// Workspace under test.
    pub workspace: Workspace,
    /// Temporary filesystem root retained until the workspace closes.
    pub fs: TemporaryPhysicalFileSystem,
    /// Canonical workspace root.
    pub root: PathBuf,
    /// Persistent artifact cache directory when enabled.
    artifact_cache: Option<PathBuf>,
}

/// One mutable workspace branch under test.
pub(super) struct TestBranch<'a> {
    /// Workspace that owns this branch.
    workspace: &'a Workspace,
    /// Branch name.
    name: String,
    /// Current branch revision.
    revision: Revision,
}

/// One authored file in a workspace test.
#[derive(Debug, Clone)]
pub(super) struct TestFile {
    /// Absolute file path.
    path: PathBuf,
    /// Logical file identity.
    id: FileId,
    /// Current source text.
    source: String,
}

impl TestWorkspace {
    /// Create a new harness rooted at a temporary source root.
    pub(super) fn new(prefix: &str) -> Self {
        Self::build(prefix, |_| Arc::new(PhysicalFileSystem::new()), false)
    }

    /// Create a new harness with one selected entry file.
    pub(super) fn with_entry(prefix: &str, entry: &str) -> Self {
        let test = Self::new(prefix);
        let manifest = format!(
            r#"{{
  "packageManager": "tspp@2026.9.0",
  "name": "test",
  "targets": {{
    "default": {{
      "entry": ["{entry}"]
    }}
  }},
  "defaultTarget": "default"
}}
"#
        );
        let _ = test.file("package.json", &manifest);

        test
    }

    /// Create a new harness with persistent artifact storage.
    pub(super) fn persistent(prefix: &str) -> Self {
        Self::build(prefix, |_| Arc::new(PhysicalFileSystem::new()), true)
    }

    /// Create a new harness whose first write to one path fails.
    pub(super) fn new_with_write_failure(prefix: &str, path: &str, content: &str) -> Self {
        let content = content.as_bytes().to_vec();

        Self::build(
            prefix,
            |root| Arc::new(FailingFileSystem::fail_once(root.join(path), content)),
            false,
        )
    }

    /// Create one harness over an explicit repository filesystem.
    fn build(
        prefix: &str,
        file_system: impl FnOnce(&Path) -> Arc<dyn FileSystem>,
        is_persistent: bool,
    ) -> Self {
        // create a temporary filesystem root for test files
        let fs = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let root = fs.root().to_path_buf();
        let file_system = file_system(&root);

        // name the package before opening the repository
        let config = root.join("package.json");
        fs.write_text(
            &config,
            "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\" }\n",
        )
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", config.display()));

        // create a repository over the selected physical filesystem
        let artifact_cache = is_persistent.then(|| root.join(".tspp/artifacts"));
        let workspace = Self::open(&root, file_system, artifact_cache.as_deref());

        Self {
            workspace,
            fs,
            root,
            artifact_cache,
        }
    }

    /// Open one complete workspace generation over the retained physical root.
    fn open(
        root: &Path,
        file_system: Arc<dyn FileSystem>,
        artifact_cache: Option<&Path>,
    ) -> Workspace {
        let mut host = Host::new(BuildId::test(), Environment::capture_process(), file_system);
        let settings = Settings::default();
        if let Some(directory) = artifact_cache {
            let cache =
                ArtifactCache::open(BuildId::test(), directory, settings.cache.maximum_bytes)
                    .map(Arc::new)
                    .expect("open artifact cache");
            host = host.with_artifact_cache(cache, 4);
        }
        let (repository, physical) = Repository::open(
            root.to_path_buf(),
            host,
            settings,
            DestackLayoutOverride::default(),
        )
        .expect("failed to import repository from physical fs");
        let repository = Arc::new(repository);
        let executor = Executor::new(Execution::Threaded, 1).expect("create executor");

        // restore cached artifacts valid at this revision
        repository
            .restore_artifacts(physical, executor.worker_count())
            .expect("restore artifact cache");

        Workspace::new(repository, physical, executor).expect("expected workspace")
    }

    /// Restart every live toolchain owner while retaining source and cache files.
    pub(super) fn restart(self) -> Self {
        let Self {
            workspace,
            fs,
            root,
            artifact_cache,
        } = self;
        drop(workspace);
        let file_system = Arc::new(PhysicalFileSystem::new());
        let workspace = Self::open(&root, file_system, artifact_cache.as_deref());

        Self {
            workspace,
            fs,
            root,
            artifact_cache,
        }
    }

    /// Persist the current physical artifact selection.
    pub(super) fn save_artifacts(&self) {
        let revision = self.workspace.revision().expect("read physical revision");

        self.workspace
            .persist_artifacts(revision)
            .expect("schedule physical artifacts");
        self.workspace
            .flush_artifact_cache()
            .expect("flush physical artifacts");
    }

    /// Return the persistent artifact cache directory.
    pub(super) fn artifact_cache(&self) -> &Path {
        self.artifact_cache
            .as_ref()
            .expect("persistent artifact cache")
    }

    /// Run one fully traced check over an authored entry file and revision.
    pub(super) fn check(&self, main: PathBuf, revision: CommandRevision) -> CheckOutput {
        let mut input = CheckInput::from((
            revision,
            CommandOptions {
                inputs: vec![CommandInput::File { path: main }],
                ..CommandOptions::default()
            },
        ));
        input.lint = false;
        input.trace = Some(tspp_repository::TraceView::Detailed);

        block_on(self.workspace.check(input, None)).expect("workspace check")
    }

    /// Resolve a path under the temporary root.
    pub(super) fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.join(path.as_ref())
    }

    /// Build a source URI for one path.
    pub(super) fn uri_for_path(&self, path: &Path) -> Uri {
        Uri::from_path(path)
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

    /// Write and publish one authored file.
    pub(super) fn file(&self, path: &str, source: &str) -> TestFile {
        let absolute = self.write_text(path, source);
        let _ = self.apply_text(&absolute, source);

        TestFile {
            path: absolute,
            id: FileId::from_logical_str(path),
            source: source.to_string(),
        }
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

    /// Create one mutable branch at the physical revision.
    pub(super) fn create_branch(&self, name: &str) -> TestBranch<'_> {
        let revision = self.workspace.revision().expect("read physical revision");
        self.workspace
            .create_branch(name.to_string(), revision)
            .expect("create test branch");

        TestBranch {
            workspace: &self.workspace,
            name: name.to_string(),
            revision,
        }
    }
}

impl TestBranch<'_> {
    /// Return the current branch revision.
    pub(super) fn revision(&self) -> Revision {
        self.revision
    }

    /// Apply source edits and advance this branch.
    pub(super) fn edit(&mut self, edits: impl IntoIterator<Item = Edit>) {
        let trace = self.workspace.start_trace(TraceLevel::Disabled);
        let commit = self
            .workspace
            .edit_branch(
                &self.name,
                self.revision,
                edits.into_iter().collect(),
                trace.as_ref(),
            )
            .expect("edit test branch");

        self.revision = commit.after;
    }

    /// Read diagnostics for one file at the current branch revision.
    pub(super) fn diagnose(&self, file: &TestFile) -> FileDiagnostics {
        let diagnostics = block_on(
            self.workspace
                .diagnose(self.revision, DiagnosticsRequest::File(file.path.clone())),
        )
        .expect("read branch diagnostics");
        let [diagnostics] = diagnostics.as_slice() else {
            panic!("branch diagnostics returned {} files", diagnostics.len());
        };

        diagnostics.clone()
    }
}

impl TestFile {
    /// Return one exact source label selected by text.
    pub(super) fn label(&self, text: &str) -> DiagnosticLabel {
        let start = self
            .source
            .find(text)
            .unwrap_or_else(|| panic!("{} does not contain {text:?}", self.path.display()));
        let span = Span::at(self.id, start as u32, text.len() as u32);

        DiagnosticLabel::new(
            Blob::for_bytes(self.source.as_bytes()),
            DiagnosticTarget::Span(span),
        )
    }

    /// Return one exact source label with a message.
    pub(super) fn message(&self, text: &str, message: &str) -> DiagnosticLabel {
        let mut label = self.label(text);
        label.message = Some(message.to_string());

        label
    }

    /// Replace the first matching source fragment.
    pub(super) fn replace(&mut self, before: &str, after: &str) -> Edit {
        let Some(start) = self.source.find(before) else {
            panic!("{} does not contain {before:?}", self.path.display());
        };
        let end = start + before.len();
        self.source.replace_range(start..end, after);

        Edit::SetText {
            path: self.path.clone(),
            text: self.source.clone(),
        }
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
