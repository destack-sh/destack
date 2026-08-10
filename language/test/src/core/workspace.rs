use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::{ArtifactKey, BuildId};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Execution, FormatterOptions, Host,
    LinterOptions, MemoryBlobStore, Ref, Repository, Revision, Settings,
};
use destack_session::{ArtifactPriority, Executor, Session};
use destack_source::{
    DiagnosticCollection, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId,
};
use destack_workspace::Workspace;
use futures::executor::block_on;
use serde_json::{Value, json};

/// One shared in memory workspace for suite execution.
#[derive(Debug)]
pub struct SharedMemoryWorkspace {
    /// The shared repository.
    repository: Arc<Repository>,
    /// The shared in memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The root directory prefix for allocated cases.
    root: PathBuf,
    /// The next unique case id.
    next_id: AtomicUsize,
}

impl SharedMemoryWorkspace {
    /// Create one shared in memory workspace rooted at the given path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let fs = Arc::new(MemoryFileSystem::new());
        fs.create_dir_all(&root)
            .expect("failed to create core workspace root");
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, fs.clone())
            .with_blob_store(Arc::new(MemoryBlobStore::new()));
        let repository = Arc::new(Repository::new(
            root.clone(),
            host,
            Settings::default(),
            layout,
        ));
        materialize_workspace(repository.clone());

        Self {
            repository,
            fs,
            root,
            next_id: AtomicUsize::new(0),
        }
    }

    /// Return the shared repository.
    pub fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the shared in memory file system.
    pub fn fs(&self) -> Arc<MemoryFileSystem> {
        self.fs.clone()
    }

    /// Return the workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Allocate one unique case root under the workspace root.
    pub fn allocate_root(&self, label: &str) -> PathBuf {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.root.join(format!("{label}-{id}"))
    }
}

/// Open one repository over one workspace root with explicit formatter and linter options.
pub fn open_repository_with_options(
    root: PathBuf,
    fs: Arc<dyn FileSystem>,
    formatter: FormatterOptions,
    linter: LinterOptions,
) -> Arc<Repository> {
    // materialize the workspace root before importing it
    fs.create_dir_all(&root)
        .expect("failed to create core workspace root");

    let environment = Environment::capture_process();
    let layout = DestackLayout::resolve(
        &root,
        &root,
        &environment,
        &Settings::default(),
        &DestackLayoutOverride::default(),
        None,
    );
    let host = Host::new(BuildId::test(), environment, fs)
        .with_blob_store(Arc::new(MemoryBlobStore::new()));
    let repository = Arc::new(Repository::new(root, host, Settings::default(), layout));
    materialize_workspace(repository.clone());
    materialize_workspace_options(repository.as_ref(), formatter, linter);

    repository
}

/// Materialize formatter and linter options as root config truth.
fn materialize_workspace_options(
    repository: &Repository,
    formatter: FormatterOptions,
    linter: LinterOptions,
) {
    let reference = Ref::for_root(repository.path());
    let json = json!({
        "formatter": formatter_json_value(formatter),
        "linter": linter_json_value(&linter),
    });
    let content =
        serde_json::to_string_pretty(&json).expect("workspace test config should serialize");
    let content = format!("{content}\n");
    let blob = repository
        .put_blob(content.as_bytes())
        .expect("workspace configuration Blob should store");

    let revision = repository
        .current(&reference)
        .expect("failed to read workspace test revision");
    let revision = repository
        .edit(revision, [Edit::set_file("destack.json", blob)])
        .expect("failed to materialize workspace test config")
        .after;

    repository
        .set_ref(&reference, revision)
        .expect("failed to publish workspace test config");
}

/// Convert formatter options to one config json value.
fn formatter_json_value(formatter: FormatterOptions) -> Value {
    json!({
        "lineEnding": match formatter.line_ending {
            destack_source::LineEnding::LineFeed => "lf",
            destack_source::LineEnding::CarriageReturnLineFeed => "crlf",
            destack_source::LineEnding::CarriageReturn => "cr",
        },
        "indentStyle": match formatter.indent_style {
            destack_source::IndentStyle::Tab => "tab",
            destack_source::IndentStyle::Space => "space",
        },
        "indentWidth": formatter.indent_width,
        "lineWidth": formatter.line_width,
    })
}

/// Convert linter options to one config json value.
fn linter_json_value(linter: &LinterOptions) -> Value {
    serde_json::to_value(linter).expect("failed to serialize linter options")
}

/// Return the current workspace revision for one repository.
pub fn current_workspace_revision(repository: &Repository) -> Revision {
    let reference = Ref::for_root(repository.path());

    repository
        .current(&reference)
        .expect("missing current workspace revision")
}

/// Return the module id for one workspace path.
pub fn module_id_for_path(repository: &Repository, revision: Revision, path: &Path) -> ModuleId {
    repository
        .module_id_for_path(revision, path)
        .unwrap_or_else(|error| panic!("failed to resolve module path: {error}"))
        .unwrap_or_else(|| panic!("missing module for path {}", path.display()))
}

/// Apply one text file write to the current workspace revision.
pub fn write_workspace_text_file(repository: &Repository, path: &Path, content: &str) -> Revision {
    let reference = Ref::for_root(repository.path());
    let logical_path = repository.logical_path(path);
    let blob = repository
        .put_blob(content.as_bytes())
        .expect("workspace source Blob should store");
    let edit = Edit::set_file(logical_path, blob);

    let revision = repository
        .current(&reference)
        .expect("failed to read workspace revision");
    let revision = repository
        .edit(revision, [edit])
        .expect("failed to apply workspace file change")
        .after;

    repository
        .set_ref(&reference, revision)
        .expect("failed to publish workspace file change")
}

/// Return the explicit built-in default target profile id for one module.
pub fn profile_id_for_builtin_default_target(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> ProfileId {
    let module = repository
        .module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to resolve module: {error}"))
        .unwrap_or_else(|| panic!("missing module {module_id:?}"));
    let target_id = TargetId::new(module.package_id, "default");

    profile_id_for_target(repository, revision, module_id, &target_id)
}

/// Return the explicit target profile id for one module.
pub fn profile_id_for_target(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
) -> ProfileId {
    let profile = repository
        .profile_for_module_target(revision, module_id, *target_id)
        .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"));

    profile.id()
}

/// Return diagnostics for one module compile slice.
pub fn module_artifact_diagnostics(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
) -> DiagnosticCollection {
    let mut diagnostics = DiagnosticCollection::new();
    let keys = [
        ArtifactKey::dir_parsed(module_id),
        ArtifactKey::dir_bound(module_id, profile),
        ArtifactKey::dir_imported(module_id, profile),
        ArtifactKey::dir_expanded(module_id, profile),
        ArtifactKey::dir_exported(module_id, profile),
        ArtifactKey::dir_resolved(module_id, profile),
        ArtifactKey::dir_checked(module_id, profile),
    ];

    // gather the module scoped diagnostics currently published for this revision
    for key in keys {
        let artifact_diagnostics = repository
            .diagnostics(revision, Some(key))
            .unwrap_or_else(|error| panic!("failed to read artifact diagnostics: {error}"));
        diagnostics.merge_from(&artifact_diagnostics);
    }

    diagnostics
}

/// Return diagnostics for one module target compile slice.
pub fn module_target_artifact_diagnostics(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
    target_id: TargetId,
) -> DiagnosticCollection {
    let mut diagnostics = module_artifact_diagnostics(repository, revision, module_id, profile);
    let keys = [
        ArtifactKey::mir_lowered(module_id, profile, target_id),
        ArtifactKey::mir_optimized(module_id, profile, target_id),
        ArtifactKey::script(module_id, target_id),
        ArtifactKey::object(module_id, target_id),
        ArtifactKey::asset(module_id, target_id),
    ];

    // gather target scoped diagnostics after the profile scoped surface
    for key in keys {
        let artifact_diagnostics = repository
            .diagnostics(revision, Some(key))
            .unwrap_or_else(|error| panic!("failed to read artifact diagnostics: {error}"));
        diagnostics.merge_from(&artifact_diagnostics);
    }

    diagnostics
}

/// Provide one root artifact slice through a workspace-root session.
pub fn provide_workspace_artifacts(
    repository: Arc<Repository>,
    artifact_keys: &[ArtifactKey],
) -> Revision {
    let root = repository.path().to_path_buf();
    let head = Ref::for_root(&root);
    let revision = repository
        .current(&head)
        .unwrap_or_else(|error| panic!("failed to read workspace revision: {error}"));
    let executor = executor(1);
    let session = Session::new(repository, executor).expect("failed to create workspace session");
    let run = session.provide(revision, artifact_keys, ArtifactPriority::Foreground);
    block_on(run.wait())
        .unwrap_or_else(|error| panic!("failed to provide workspace artifacts: {error}"));

    revision
}

/// Materialize one repository through its workspace owner.
fn materialize_workspace(repository: Arc<Repository>) {
    let executor = executor(1);
    let _workspace = Workspace::new(repository, None, executor)
        .unwrap_or_else(|error| panic!("failed to materialize workspace root: {error}"));
}

/// Create one threaded session executor for test work.
fn executor(worker_count: usize) -> Arc<Executor> {
    Executor::new(Execution::Threaded, worker_count).expect("failed to create artifact executor")
}
