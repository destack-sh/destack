use std::path::PathBuf;
use std::sync::Arc;

use clap::{Args, ValueEnum};
use destack_artifact::MemoryBlobStore;
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, FormatterOptions, Host, Ref, Repository,
    Settings, default_blob_store, open_repository,
};
use destack_source::{FileSystem, FileWatcher, IndentStyle, LineEnding, PhysicalFileSystem};
use destack_workspace::{LocalWorkspace, ManifestOverride, Workspace};

use crate::common::overrides_from_program;

use crate::common::{ReportArgs, report_error};
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    LocalWorkspace::default_worker_count() as u16
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

/// Arguments for configuring linter options.
#[derive(Args, Debug, Clone, Default)]
pub struct LinterOptionsArgs {
    /// Disable specific lint rules.
    #[arg(long = "allow", value_name = "RULE")]
    pub allow: Vec<String>,

    /// Warn on specific lint rules.
    #[arg(long = "warn", value_name = "RULE")]
    pub warn: Vec<String>,

    /// Report specific lint rules as errors.
    #[arg(long = "deny", value_name = "RULE")]
    pub deny: Vec<String>,
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
#[derive(Args, Debug, Clone)]
pub struct ProgramArgs {
    /// The working directory (default: current directory).
    #[arg(long = "cwd", global = true)]
    pub cwd: Option<PathBuf>,

    /// Path to one project directory or destack.json manifest file.
    #[arg(long = "manifest", global = true)]
    pub manifest: Option<PathBuf>,

    /// Workspace root directory (defaults to resolved workspace from cwd).
    #[arg(long = "workspace", global = true)]
    pub workspace: Option<PathBuf>,

    /// Destack home directory override.
    #[arg(long = "home", global = true)]
    pub home: Option<PathBuf>,

    /// Package directory override.
    #[arg(long = "package-dir", global = true)]
    pub package_dir: Option<PathBuf>,

    /// Workspace cache directory override.
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

    /// Show command and artifact timings.
    #[arg(long, global = true)]
    pub timings: bool,

    /// The formatter options.
    #[command(flatten)]
    pub formatter: FormatterOptionsArgs,

    /// The linter options.
    #[command(flatten)]
    pub linter: LinterOptionsArgs,
}

impl Default for ProgramArgs {
    /// Return default program arguments.
    fn default() -> Self {
        Self {
            cwd: None,
            manifest: None,
            workspace: None,
            home: None,
            package_dir: None,
            cache_dir: None,
            fs_override: None,
            workers: default_workers(),
            watch: false,
            dev: false,
            timings: false,
            formatter: FormatterOptionsArgs::default(),
            linter: LinterOptionsArgs::default(),
        }
    }
}

impl ProgramArgs {
    /// Select a workspace root from one positional directory argument.
    pub fn select_workspace_root(&mut self, root: PathBuf) -> ConsoleResult<()> {
        if let Some(workspace) = self.workspace.as_ref()
            && *workspace != root
        {
            return Err(ConsoleError::message(format!(
                "directory argument {root:?} conflicts with --workspace {workspace:?}"
            )));
        }

        self.workspace = Some(root);

        Ok(())
    }

    /// Return the effective working directory for these arguments.
    pub fn effective_cwd(&self) -> ConsoleResult<PathBuf> {
        if let Some(cwd) = self.cwd.clone() {
            return Ok(cwd);
        }

        std::env::current_dir()
            .map_err(|error| ConsoleError::message(format!("failed to read cwd: {error}")))
    }

    /// Build manifest overrides from explicit CLI options.
    pub fn overrides(&self) -> Vec<ManifestOverride> {
        overrides_from_program(self)
    }

    /// Attach a file system override for testing.
    pub fn with_fs_override(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs_override = Some(FileSystemOverride::new(fs));
        self
    }

    /// Open a repository from these arguments.
    pub(crate) fn open_repository(&self) -> ConsoleResult<Arc<Repository>> {
        self.open_repository_with_fs(self.fs_override.as_ref().map(FileSystemOverride::fs))
    }

    /// Open a repository using an explicit file system override.
    pub(crate) fn open_repository_with_fs(
        &self,
        fs_override: Option<Arc<dyn FileSystem>>,
    ) -> ConsoleResult<Arc<Repository>> {
        let cwd = self.effective_cwd()?;
        let workspace_root = self.workspace.clone().unwrap_or_else(|| cwd.clone());
        let has_fs_override = fs_override.is_some();
        let fs: Arc<dyn FileSystem> = fs_override.unwrap_or_else(|| {
            let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
            fs
        });
        let mut environment = Environment::capture_process();
        environment.cwd = Some(cwd.clone());

        // resolve machine settings before opening the repository
        let layout_override = DestackLayoutOverride {
            home: self.home.clone(),
            packages: self.package_dir.clone(),
            workspace_cache: self.cache_dir.clone(),
        };
        let home = DestackLayout::resolve_home(&cwd, &environment, layout_override.home.as_deref());
        let settings = Settings::load_from_home(fs.as_ref(), &home).map_err(|error| {
            ConsoleError::message(format!(
                "failed to load Destack settings from {}: {error}",
                home.display()
            ))
        })?;

        // discover and import the repository in one step
        let blob_store = if has_fs_override {
            Arc::new(MemoryBlobStore::new()) as Arc<dyn destack_artifact::BlobStore>
        } else {
            default_blob_store()
        };
        let host = Host::new(environment, fs.clone(), blob_store);
        let repository = open_repository(workspace_root, host, settings, layout_override)
            .map_err(|error| ConsoleError::message(format!("failed to open workspace: {error}")))?;

        let repository = Arc::new(repository);
        let reference = Ref::for_root(repository.path());
        let revision = repository.current(&reference).map_err(|error| {
            ConsoleError::message(format!(
                "failed to resolve current workspace revision: {error}"
            ))
        })?;
        repository
            .root(revision)
            .map_err(|error| ConsoleError::message(format!("failed to derive root: {error}")))?;

        Ok(repository)
    }

    /// Open a local workspace from these arguments.
    pub(crate) fn workspace(
        &self,
        file_watcher: Option<Arc<dyn FileWatcher>>,
    ) -> ConsoleResult<(Arc<dyn Workspace>, Vec<PathBuf>)> {
        let repository = self.open_repository()?;
        let roots = vec![
            self.workspace
                .clone()
                .unwrap_or_else(|| repository.path().to_path_buf()),
        ];
        let workspace = LocalWorkspace::new(
            repository,
            None,
            file_watcher,
            roots.clone(),
            self.workers as usize,
            None,
        )
        .map_err(|error| ConsoleError::message(format!("workspace open failed: {error}")))?;

        Ok((Arc::new(workspace), roots))
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
