use std::path::PathBuf;
use std::sync::Arc;

use clap::{Args, ValueEnum};
use destack_artifact::MemoryCacheStore;
use destack_daemon::protocol::ConfigPatch;
use destack_session::{Session, open_repository_from_fs};
use destack_source::{FileSystem, IndentStyle, LineEnding, PhysicalFileSystem};
use destack_workspace::{
    FormatterOptions, HostEnvironment, LintPreset, LintSeverity, LinterOptions, Ref, Repository,
};

use crate::pipeline::daemon::config_patches_from_program;

use crate::common::{ReportArgs, report_error};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    Session::default_worker_count() as u16
}

/// File system override for CLI testing.
#[derive(Clone)]
pub struct FileSystemOverride {
    /// The file system to use for setup.
    fs: Arc<dyn FileSystem>,
}

impl FileSystemOverride {
    /// Create a new file system override.
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Clone the underlying file system handle.
    pub fn fs(&self) -> Arc<dyn FileSystem> {
        self.fs.clone()
    }
}

impl std::fmt::Debug for FileSystemOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSystemOverride").finish()
    }
}

/// The indent style to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum IndentStyleArg {
    /// Use tabs for indentation.
    #[default]
    Tab,
    /// Use spaces for indentation.
    Space,
}

impl From<IndentStyleArg> for IndentStyle {
    fn from(style: IndentStyleArg) -> Self {
        match style {
            IndentStyleArg::Tab => IndentStyle::Tab,
            IndentStyleArg::Space => IndentStyle::Space,
        }
    }
}

/// The line ending style to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LineEndingArg {
    /// Unix-style line endings (LF).
    #[default]
    Lf,
    /// Windows-style line endings (CRLF).
    Crlf,
    /// Classic Mac-style line endings (CR).
    Cr,
}

impl From<LineEndingArg> for LineEnding {
    fn from(ending: LineEndingArg) -> Self {
        match ending {
            LineEndingArg::Lf => LineEnding::LineFeed,
            LineEndingArg::Crlf => LineEnding::CarriageReturnLineFeed,
            LineEndingArg::Cr => LineEnding::CarriageReturn,
        }
    }
}

/// Lint rule preset.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LintPresetArg {
    /// No rules enabled by default.
    None,
    /// Recommended rules enabled (default).
    #[default]
    Recommended,
    /// All rules enabled.
    All,
}

impl From<LintPresetArg> for LintPreset {
    fn from(value: LintPresetArg) -> Self {
        match value {
            LintPresetArg::None => LintPreset::None,
            LintPresetArg::Recommended => LintPreset::Recommended,
            LintPresetArg::All => LintPreset::All,
        }
    }
}

/// Arguments for configuring linter options.
#[derive(Args, Debug, Clone, Default)]
pub struct LinterOptionsArgs {
    /// Lint rule preset (none|recommended|all, default: recommended).
    #[arg(long = "lint-preset", value_enum)]
    pub preset: Option<LintPresetArg>,

    /// Allow specific lint rules (set to note severity).
    #[arg(long = "allow", value_name = "RULE")]
    pub allow: Vec<String>,

    /// Warn on specific lint rules (set to warning severity).
    #[arg(long = "warn", value_name = "RULE")]
    pub warn: Vec<String>,

    /// Deny specific lint rules (set to error severity).
    #[arg(long = "deny", value_name = "RULE")]
    pub deny: Vec<String>,

    /// Maximum cyclomatic complexity (default: 40).
    #[arg(long = "max-complexity")]
    pub max_complexity: Option<usize>,

    /// Maximum function parameters (default: 4).
    #[arg(long = "max-params")]
    pub max_params: Option<usize>,

    /// Maximum nesting depth (default: 4).
    #[arg(long = "max-depth")]
    pub max_depth: Option<usize>,

    /// Maximum lines per file (default: 500).
    #[arg(long = "max-lines")]
    pub max_lines: Option<usize>,
}

impl From<LinterOptionsArgs> for LinterOptions {
    fn from(args: LinterOptionsArgs) -> Self {
        let mut options = LinterOptions::default();
        if let Some(preset) = args.preset {
            options.preset = preset.into();
        }
        // apply rule overrides
        for rule in args.allow {
            options.overrides.insert(rule, LintSeverity::Note);
        }
        for rule in args.warn {
            options.overrides.insert(rule, LintSeverity::Warning);
        }
        for rule in args.deny {
            options.overrides.insert(rule, LintSeverity::Error);
        }
        // complexity thresholds
        if let Some(max_complexity) = args.max_complexity {
            options.complexity.max_cyclomatic_complexity = max_complexity;
        }
        if let Some(max_params) = args.max_params {
            options.complexity.max_params = max_params;
        }
        if let Some(max_depth) = args.max_depth {
            options.complexity.max_depth = max_depth;
        }
        if let Some(max_lines) = args.max_lines {
            options.complexity.max_lines = max_lines;
        }
        options
    }
}

/// Arguments for configuring formatter options.
#[derive(Args, Debug, Clone, Default)]
pub struct FormatterOptionsArgs {
    /// The indent style (tab|space, default: space).
    #[arg(long = "indent-style", value_enum)]
    pub indent_style: Option<IndentStyleArg>,

    /// The indent width in spaces (default: 4).
    #[arg(long = "indent-width")]
    pub indent_width: Option<u8>,

    /// The line ending style (lf|crlf|cr, default: lf).
    #[arg(long = "line-ending", value_enum)]
    pub line_ending: Option<LineEndingArg>,

    /// The maximum line width (default: 100).
    #[arg(long = "line-width")]
    pub line_width: Option<u16>,
}

impl From<FormatterOptionsArgs> for FormatterOptions {
    fn from(args: FormatterOptionsArgs) -> Self {
        let mut options = FormatterOptions::default();
        // layout
        if let Some(style) = args.indent_style {
            options.indent_style = style.into();
        }
        if let Some(width) = args.indent_width {
            options.indent_width = width;
        }
        if let Some(ending) = args.line_ending {
            options.line_ending = ending.into();
        }
        if let Some(width) = args.line_width {
            options.line_width = width;
        }
        options
    }
}

/// Arguments for configuring program setup.
#[derive(Args, Debug, Clone, Default)]
pub struct ProgramArgs {
    /// The working directory (default: current directory).
    #[arg(long = "cwd", global = true)]
    pub cwd: Option<PathBuf>,

    /// Path to one project directory or destack.json configuration file.
    #[arg(long = "config", short = 'c', global = true)]
    pub config: Option<PathBuf>,

    /// Workspace root directory (defaults to resolved workspace from cwd).
    #[arg(long = "workspace", global = true)]
    pub workspace: Option<PathBuf>,

    /// Cache directory override.
    #[arg(long = "cache-dir", global = true)]
    pub cache_dir: Option<PathBuf>,

    /// Test only file system override.
    #[arg(skip)]
    pub fs_override: Option<FileSystemOverride>,

    /// The number of worker threads to use (default: number of CPU cores).
    #[arg(
        long = "workers",
        short = 'j',
        default_value_t = default_workers(),
        global = true
    )]
    pub workers: u16,

    /// Enable watch mode for supported commands.
    #[arg(long = "watch", global = true)]
    pub watch: bool,

    /// Enable dev mode for supported commands.
    #[arg(long = "dev", global = true)]
    pub dev: bool,

    /// Emit profiling information where supported.
    #[arg(long = "profile", global = true)]
    pub profile: bool,

    /// The formatter options.
    #[command(flatten)]
    pub formatter: FormatterOptionsArgs,

    /// The linter options.
    #[command(flatten)]
    pub linter: LinterOptionsArgs,
}

impl ProgramArgs {
    /// Return the effective working directory for these arguments.
    pub fn effective_cwd(&self) -> PathBuf {
        self.cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    /// Build config patches from explicit CLI options.
    pub fn config_patches(&self) -> Vec<ConfigPatch> {
        config_patches_from_program(self)
    }

    /// Attach a file system override for testing.
    pub fn with_fs_override(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs_override = Some(FileSystemOverride::new(fs));
        self
    }

    /// Create a repository from these arguments.
    pub fn setup(&self) -> Arc<Repository> {
        self.setup_with_fs(self.fs_override.as_ref().map(FileSystemOverride::fs))
    }

    /// Create a repository using an explicit file system override.
    pub fn setup_with_fs(&self, fs_override: Option<Arc<dyn FileSystem>>) -> Arc<Repository> {
        let cwd = self.effective_cwd();
        let workspace_root = self.workspace.clone().unwrap_or_else(|| cwd.clone());
        let has_fs_override = fs_override.is_some();
        let fs: Arc<dyn FileSystem> = fs_override.unwrap_or_else(|| {
            let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
            fs
        });

        // discover and import the repository in one step
        let mut repository = open_repository_from_fs(
            workspace_root,
            fs.clone(),
            HostEnvironment::capture_process(),
        )
        .expect("failed to import repository from file system");

        // prefer in memory cache stores for test file systems
        if has_fs_override {
            repository = repository.with_cache(Arc::new(MemoryCacheStore::new()));
        }

        let repository = Arc::new(repository);
        let reference = Ref::for_workspace_root(repository.workspace_root());
        let revision = repository
            .current(&reference)
            .expect("failed to resolve current workspace revision");
        let workspace = repository
            .workspace(revision)
            .expect("failed to derive workspace view");
        let root = workspace.root.clone();
        let workspace_kind = workspace.kind;

        tracing::trace!(?cwd, ?root, workspace_kind = ?workspace_kind, workers = self.workers, "program.setup");

        repository
    }
}

/// Ensure that unsupported watch or dev flags are not set.
pub fn ensure_no_watch_or_dev(
    command: &str,
    program: &ProgramArgs,
    report: &ReportArgs,
) -> Option<i32> {
    // reject watch mode when not implemented
    if program.watch {
        return Some(report_error(
            command,
            report,
            "--watch is not implemented yet",
        ));
    }

    // reject dev mode when not implemented
    if program.dev {
        return Some(report_error(
            command,
            report,
            "--dev is not implemented yet",
        ));
    }

    None
}
