use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::{ArtifactKey, MemoryCacheStore};
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_session::Session;
use destack_source::{FileContent, FileSystem, MemoryFileSystem, ModuleId, ProfileId, TargetId};
use destack_workspace::{
    AmbientSnapshot, Change, Edit, FormatterOptions, LinterOptions, Ref, Repository, Revision,
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
        let repository = Arc::new(
            Repository::open_root_from_fs(
                root.clone(),
                fs.clone(),
                AmbientSnapshot::capture_process(),
            )
                .expect("failed to import repository from core workspace fs")
                .with_cache(Arc::new(MemoryCacheStore::new())),
        );
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

    let repository = Arc::new(
        Repository::open_root_from_fs(root, fs, AmbientSnapshot::capture_process())
            .expect("failed to import repository from core workspace fs")
            .with_cache(Arc::new(MemoryCacheStore::new())),
    );
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

    repository
        .apply(&reference, Change::set_text("destack.json", content))
        .expect("failed to materialize workspace test config");
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
        "quoteStyle": match formatter.quote_style {
            destack_workspace::QuoteStyle::Double => "double",
            destack_workspace::QuoteStyle::Single => "single",
            destack_workspace::QuoteStyle::Semantic => "semantic",
        },
        "trailingComma": match formatter.trailing_comma {
            destack_workspace::TrailingComma::All => "all",
            destack_workspace::TrailingComma::Es5 => "es5",
            destack_workspace::TrailingComma::None => "none",
        },
        "bracketSpacing": formatter.bracket_spacing,
        "arrowParens": match formatter.arrow_parentheses {
            destack_workspace::ArrowParentheses::Always => "always",
            destack_workspace::ArrowParentheses::Avoid => "avoid",
        },
        "quoteProps": match formatter.quote_property {
            destack_workspace::QuoteProperty::AsNeeded => "as-needed",
            destack_workspace::QuoteProperty::Consistent => "consistent",
            destack_workspace::QuoteProperty::Preserve => "preserve",
        },
        "bracketSameLine": formatter.bracket_same_line,
        "singleAttributePerLine": formatter.single_attribute_per_line,
        "organizeImports": match formatter.organize_imports {
            destack_workspace::OrganizeImports::On => "on",
            destack_workspace::OrganizeImports::Off => "off",
        },
        "importSortOrder": match formatter.import_sort_order {
            destack_workspace::ImportSortOrder::Natural => "natural",
            destack_workspace::ImportSortOrder::Alphabetical => "alphabetical",
        },
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

/// Apply one full file write to the current workspace revision.
pub fn write_workspace_file(
    repository: &Repository,
    path: &Path,
    content: FileContent,
) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());
    let logical_path = repository.normalize_workspace_path(path);
    let change = Change::single(Edit::SetFile {
        logical_path,
        content,
    });

    repository
        .apply(&reference, change)
        .expect("failed to apply workspace file change")
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
        .default_profile_id_for_module(revision, module_id)
        .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"))
}

/// Return the target profile id for one module, or the default profile.
pub fn profile_id_for_target_or_default(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
) -> ProfileId {
    repository
        .profile_id_for_target_or_default(revision, module_id, target_id)
        .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"))
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
    let session = Session::new(
        root.clone(),
        root,
        repository,
        head,
        None,
        compiler,
        linter,
        None,
        None,
    )
    .unwrap_or_else(|error| panic!("failed to initialize workspace session: {error}"));

    session
        .provide(artifact_keys)
        .unwrap_or_else(|error| panic!("failed to provide workspace artifacts: {error}"));

    session.revision()
}

/// Materialize one workspace root through one session driven reload.
fn materialize_workspace_root(repository: Arc<Repository>, root: &Path) {
    let head = Ref::for_workspace_root(root);
    let compiler = Arc::new(Compiler::new(repository.clone(), Default::default()));
    let linter = Arc::new(Linter::new(repository.clone()));
    let session = Session::new(
        root.to_path_buf(),
        root.to_path_buf(),
        repository,
        head,
        None,
        compiler,
        linter,
        None,
        None,
    )
    .unwrap_or_else(|error| panic!("failed to initialize workspace session: {error}"));

    session
        .scan_filesystem(true)
        .unwrap_or_else(|error| panic!("failed to materialize workspace root: {error}"));
}
