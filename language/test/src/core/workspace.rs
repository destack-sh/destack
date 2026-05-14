use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::{ArtifactKey, MemoryCacheStore};
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_session::Session;
use destack_source::{
    DiagnosticCollection, FileContent, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId,
};
use destack_workspace::{
    Edit, FormatterOptions, HostEnvironment, LinterOptions, Ref, Repository, Revision,
};
use serde_json::{Map, Value, json};

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
        let repository = Arc::new(Repository::new(
            root.clone(),
            Arc::new(MemoryCacheStore::new()),
            fs.clone(),
            HostEnvironment::capture_process(),
        ));
        materialize_workspace_root(repository.clone(), &root);

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

    let repository = Arc::new(Repository::new(
        root,
        Arc::new(MemoryCacheStore::new()),
        fs,
        HostEnvironment::capture_process(),
    ));
    let root = repository.workspace_root().to_path_buf();
    materialize_workspace_root(repository.clone(), &root);
    materialize_workspace_options(repository.as_ref(), formatter, linter);

    repository
}

/// Materialize formatter and linter options as root config truth.
fn materialize_workspace_options(
    repository: &Repository,
    formatter: FormatterOptions,
    linter: LinterOptions,
) {
    let reference = Ref::for_workspace_root(repository.workspace_root());
    let json = json!({
        "formatter": formatter_json_value(formatter),
        "linter": linter_json_value(&linter),
    });
    let content =
        serde_json::to_string_pretty(&json).expect("workspace test config should serialize");
    let content = format!("{content}\n");

    let revision = repository
        .current(&reference)
        .expect("failed to read workspace test revision");
    let revision = repository
        .fork_with_edits(revision, [Edit::set_text("destack.json", content)])
        .expect("failed to materialize workspace test config");

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
    let mut rules = Map::new();
    let mut categories = Map::new();
    let mut overrides = Map::new();

    // rules
    rules.insert(
        "preset".to_string(),
        json!(match linter.preset {
            destack_workspace::LintPreset::None => "none",
            destack_workspace::LintPreset::Recommended => "recommended",
            destack_workspace::LintPreset::Strict => "strict",
            destack_workspace::LintPreset::All => "all",
        }),
    );

    // category overrides
    for (category, severity) in &linter.categories {
        categories.insert(
            category.name().to_string(),
            lint_severity_json_value(*severity),
        );
    }
    if !categories.is_empty() {
        rules.insert("categories".to_string(), Value::Object(categories));
    }

    // rule overrides
    for (rule, severity) in &linter.overrides {
        overrides.insert(rule.clone(), lint_severity_json_value(*severity));
    }
    for (rule, severity) in overrides {
        rules.insert(rule, severity);
    }

    json!({
        "enabled": linter.enabled,
        "rules": Value::Object(rules),
        "complexity": {
            "maxCyclomaticComplexity": linter.complexity.max_cyclomatic_complexity,
            "maxParams": linter.complexity.max_params,
            "maxDepth": linter.complexity.max_depth,
            "maxLines": linter.complexity.max_lines,
        },
    })
}

/// Convert lint severity to one config json value.
fn lint_severity_json_value(severity: destack_workspace::LintSeverity) -> Value {
    json!(match severity {
        destack_workspace::LintSeverity::Off => "off",
        destack_workspace::LintSeverity::Note => "off",
        destack_workspace::LintSeverity::Warning => "warn",
        destack_workspace::LintSeverity::Error => "error",
    })
}

/// Return the current workspace revision for one repository.
pub fn current_workspace_revision(repository: &Repository) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());

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

/// Apply one full file write to the current workspace revision.
pub fn write_workspace_file(
    repository: &Repository,
    path: &Path,
    content: FileContent,
) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());
    let logical_path = repository.logical_path(path);
    let edit = Edit::SetFile {
        logical_path,
        content,
    };

    let revision = repository
        .current(&reference)
        .expect("failed to read workspace revision");
    let revision = repository
        .fork_with_edits(revision, [edit])
        .expect("failed to apply workspace file change");

    repository
        .set_ref(&reference, revision)
        .expect("failed to publish workspace file change")
}

/// Apply one text file write to the current workspace revision.
pub fn write_workspace_text_file(repository: &Repository, path: &Path, content: &str) -> Revision {
    write_workspace_file(
        repository,
        path,
        FileContent::Text {
            content: content.to_string(),
        },
    )
}

/// Return the default profile id for one module.
pub fn default_profile_id_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> ProfileId {
    repository
        .module_profile(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"))
        .id()
}

/// Return the target profile id for one module, or the default profile.
pub fn profile_id_for_target_or_default(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
) -> ProfileId {
    if let Some(profile) = repository
        .module_target_profile(revision, module_id, *target_id)
        .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"))
    {
        return profile.id();
    }

    default_profile_id_for_module(repository, revision, module_id)
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
        ArtifactKey::module_output(module_id, target_id),
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
    compiler: Arc<Compiler>,
    artifact_keys: &[ArtifactKey],
) -> Revision {
    let root = repository.workspace_root().to_path_buf();
    let head = Ref::for_workspace_root(&root);
    let linter = Arc::new(Linter::new(repository.clone()));
    let query = Arc::new(Query::new(repository.clone()));
    let session = Session::new(
        root.clone(),
        root,
        repository,
        head,
        compiler,
        linter,
        query,
        1,
        None,
    )
    .expect("failed to create workspace session");

    let revision = session
        .revision(session.head())
        .unwrap_or_else(|error| panic!("failed to read workspace revision: {error}"));
    session
        .provide(revision, artifact_keys)
        .unwrap_or_else(|error| panic!("failed to provide workspace artifacts: {error}"));

    session
        .revision(session.head())
        .unwrap_or_else(|error| panic!("failed to read workspace revision: {error}"))
}

/// Materialize one workspace root through one session driven reload.
fn materialize_workspace_root(repository: Arc<Repository>, root: &Path) {
    let head = Ref::for_workspace_root(root);
    let compiler = Arc::new(Compiler::new(repository.clone()));
    let linter = Arc::new(Linter::new(repository.clone()));
    let query = Arc::new(Query::new(repository.clone()));
    let session = Session::new(
        root.to_path_buf(),
        root.to_path_buf(),
        repository,
        head,
        compiler,
        linter,
        query,
        1,
        None,
    )
    .expect("failed to create workspace session");

    session
        .reload_from_fs(session.head())
        .unwrap_or_else(|error| panic!("failed to materialize workspace root: {error}"));
}
