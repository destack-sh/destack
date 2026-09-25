use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Args, ValueEnum};
use tspp_artifact::ArtifactCache;
use tspp_daemon::{
    DaemonConnectOptions, DaemonConnection, DaemonEndpoint, DaemonLaunch, DaemonLaunchCommand,
    OpenWorkspaceRequest,
};
use tspp_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Execution, FormatterOptions, Host,
    Repository, Revision, Settings, SourceRoot,
};
use tspp_session::Executor;
use tspp_source::{FileSystem, IndentStyle, LineEnding, PhysicalFileSystem};
use tspp_workspace::{ManifestOverride, Workspace};

use crate::common::{ReportArgs, overrides_from_program, report_error};
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    Executor::default_worker_count() as u16
}

/// File system override for CLI testing.
#[derive(Clone)]
pub struct FileSystemOverride {
    /// The file system to use for setup.
    file_system: Arc<dyn FileSystem>,
}

impl FileSystemOverride {
    /// Create a file system override.
    pub fn new(file_system: Arc<dyn FileSystem>) -> Self {
        Self { file_system }
    }

    /// Clone the underlying file system handle.
    pub fn file_system(&self) -> Arc<dyn FileSystem> {
        self.file_system.clone()
    }
}

impl std::fmt::Debug for FileSystemOverride {
    /// Format the file system override without its trait object.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FileSystemOverride").finish()
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
    /// Run only these lint rules.
    #[arg(
        long = "only",
        value_name = "RULE",
        conflicts_with_all = ["allow", "warn", "deny"]
    )]
    pub only: Vec<String>,

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

    /// Cache directory override.
    #[arg(long = "cache-dir", global = true)]
    pub cache_dir: Option<PathBuf>,

    /// Test only file system override.
    #[arg(skip)]
    pub file_system_override: Option<FileSystemOverride>,

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
            file_system_override: None,
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
        // resolve the process directory once
        let process = std::env::current_dir()
            .map_err(|error| ConsoleError::message(format!("failed to read cwd: {error}")))?;

        // use the process directory when no override exists
        let Some(cwd) = self.cwd.as_deref() else {
            return Ok(process);
        };

        // preserve explicit absolute paths
        if cwd.is_absolute() {
            return Ok(cwd.to_path_buf());
        }

        Ok(process.join(cwd))
    }

    /// Return the explicit manifest path resolved against the working directory.
    pub fn manifest_path(&self) -> ConsoleResult<Option<PathBuf>> {
        self.manifest
            .as_deref()
            .map(|path| self.resolve_path(path))
            .transpose()
    }

    /// Return the source path used to discover the workspace root.
    fn workspace_path(&self) -> ConsoleResult<PathBuf> {
        // prefer an explicit workspace
        if let Some(workspace) = self.workspace.as_deref() {
            return self.resolve_path(workspace);
        }

        // discover from an explicit manifest
        if let Some(manifest) = self.manifest_path()? {
            return Ok(manifest);
        }

        self.effective_cwd()
    }

    /// Resolve one command path against the working directory.
    fn resolve_path(&self, path: &Path) -> ConsoleResult<PathBuf> {
        // preserve explicit absolute paths
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // resolve relative paths from the command directory
        let cwd = self.effective_cwd()?;

        Ok(cwd.join(path))
    }

    /// Build manifest overrides from explicit CLI options.
    pub fn overrides(&self) -> Vec<ManifestOverride> {
        overrides_from_program(self)
    }

    /// Attach a file system override for testing.
    pub fn with_file_system_override(mut self, file_system: Arc<dyn FileSystem>) -> Self {
        self.file_system_override = Some(FileSystemOverride::new(file_system));
        self
    }

    /// Resolve the machine artifact cache directory and size limit.
    pub(crate) fn resolve_artifact_cache(&self) -> ConsoleResult<(PathBuf, Option<u64>)> {
        let file_system = self.file_system();
        let cwd = self.effective_cwd()?;
        let mut environment = Environment::capture_process();
        environment.cwd = Some(cwd.clone());
        let (home, settings) = self.load_settings(file_system.as_ref(), &cwd, &environment)?;

        // resolve the final cache path from every machine override
        let directory = DestackLayout::resolve_cache(
            &cwd,
            &home,
            &environment,
            &settings,
            self.cache_dir.as_deref(),
        );

        Ok((directory, settings.cache.maximum_bytes))
    }

    /// Open a repository and its imported physical revision from these arguments.
    pub(crate) fn open_repository(&self) -> ConsoleResult<(Arc<Repository>, Revision)> {
        let file_system = self.file_system();
        let cwd = self.effective_cwd()?;
        let workspace_path = self.workspace_path()?;
        let mut environment = Environment::capture_process();
        environment.cwd = Some(cwd.clone());

        // resolve machine settings before opening the repository
        let layout_override = DestackLayoutOverride {
            home: self.home.clone(),
            packages: self.package_dir.clone(),
            cache: self.cache_dir.clone(),
        };
        let (home, settings) = self.load_settings(file_system.as_ref(), &cwd, &environment)?;

        // discover and import the repository in one step
        let build_id = Workspace::BUILD_ID;
        let artifact_cache = DestackLayout::resolve_cache(
            &cwd,
            &home,
            &environment,
            &settings,
            layout_override.cache.as_deref(),
        );
        let artifact_cache =
            ArtifactCache::open(build_id, artifact_cache, settings.cache.maximum_bytes)
                .map(Arc::new)
                .map_err(|error| {
                    ConsoleError::message(format!("failed to open artifact cache: {error}"))
                })?;
        let host = Host::new(build_id, environment, file_system)
            .with_artifact_cache(artifact_cache, self.workers as usize);
        let (repository, revision) =
            Repository::open(workspace_path, host, settings, layout_override).map_err(|error| {
                ConsoleError::message(format!("failed to open workspace: {error}"))
            })?;

        let repository = Arc::new(repository);
        repository
            .root(revision)
            .map_err(|error| ConsoleError::message(format!("failed to derive root: {error}")))?;

        // restore cached artifacts valid at this revision
        repository
            .restore_artifacts(revision, self.workers as usize)
            .map_err(|error| {
                ConsoleError::message(format!("artifact cache restore failed: {error}"))
            })?;

        Ok((repository, revision))
    }

    /// Open a daemon workspace from these arguments.
    pub(crate) fn daemon_workspace(&self) -> ConsoleResult<Workspace> {
        let (repository, physical) = self.open_repository()?;
        let executor = self.executor()?;

        Workspace::new(repository, physical, executor)
            .map_err(|error| ConsoleError::message(format!("workspace open failed: {error}")))
    }

    /// Return the selected host file system.
    fn file_system(&self) -> Arc<dyn FileSystem> {
        self.file_system_override
            .as_ref()
            .map(FileSystemOverride::file_system)
            .unwrap_or_else(|| Arc::new(PhysicalFileSystem::new()))
    }

    /// Load machine settings and return their resolved home directory.
    fn load_settings(
        &self,
        file_system: &dyn FileSystem,
        cwd: &Path,
        environment: &Environment,
    ) -> ConsoleResult<(PathBuf, Settings)> {
        let home = DestackLayout::resolve_home(cwd, environment, self.home.as_deref());
        let settings = Settings::load_from_home(file_system, &home).map_err(|error| {
            ConsoleError::message(format!(
                "failed to load Destack settings from {}: {error}",
                home.display()
            ))
        })?;

        Ok((home, settings))
    }

    /// Open a local workspace from these arguments.
    pub(crate) fn workspace(&self) -> ConsoleResult<Arc<Workspace>> {
        let (repository, physical) = self.open_repository()?;
        let executor = self.executor()?;
        let workspace = Workspace::new(repository, physical, executor)
            .map_err(|error| ConsoleError::message(format!("workspace open failed: {error}")))?;

        Ok(Arc::new(workspace))
    }

    /// Connect to the shared daemon and open this program's workspace root.
    pub(crate) fn connect_daemon(&self) -> ConsoleResult<(DaemonConnection, PathBuf)> {
        let root = self.workspace_root()?;
        let endpoint = self.daemon_endpoint()?;
        let launch = self.daemon_launch(&endpoint, root.clone())?;
        let connection = endpoint
            .connect(DaemonConnectOptions::default(), Some(launch))
            .map_err(|error| ConsoleError::message(format!("daemon connection failed: {error}")))?;
        let opened = connection
            .daemon()
            .open_workspace(OpenWorkspaceRequest { root })
            .map_err(|error| ConsoleError::message(format!("workspace open failed: {error}")))?;

        Ok((connection, opened.value.root))
    }

    /// Resolve the daemon endpoint selected by these arguments.
    pub(crate) fn daemon_endpoint(&self) -> ConsoleResult<DaemonEndpoint> {
        let cwd = self.effective_cwd()?;
        let environment = Environment::capture_process();
        let home = DestackLayout::resolve_home(&cwd, &environment, self.home.as_deref());

        Ok(DaemonEndpoint::new(home))
    }

    /// Discover the workspace root selected by these arguments.
    pub(crate) fn workspace_root(&self) -> ConsoleResult<PathBuf> {
        let file_system = self.file_system();
        let workspace_path = self.workspace_path()?;
        let root =
            SourceRoot::discover(file_system.as_ref(), &workspace_path).map_err(|error| {
                ConsoleError::message(format!("workspace discovery failed: {error}"))
            })?;

        Ok(root.into())
    }

    /// Create the session executor selected by these arguments.
    fn executor(&self) -> ConsoleResult<Arc<Executor>> {
        Executor::new(Execution::Threaded, self.workers as usize)
            .map_err(|error| ConsoleError::message(format!("executor start failed: {error}")))
    }

    /// Build the daemon process launch for this program.
    pub(crate) fn daemon_launch(
        &self,
        endpoint: &DaemonEndpoint,
        root: PathBuf,
    ) -> ConsoleResult<DaemonLaunch> {
        let cwd = self.effective_cwd()?;
        let cache_directory = self.cache_dir.as_ref().map(|directory| {
            if directory.is_absolute() {
                directory.clone()
            } else {
                cwd.join(directory)
            }
        });
        let mut command =
            DaemonLaunchCommand::current(vec![OsString::from("daemon"), OsString::from("serve")])
                .map_err(|error| ConsoleError::message(format!("daemon launch failed: {error}")))?;

        // preserve every machine and workspace selection in the daemon process
        command.home = self.home.clone();
        command.package_directory = self.package_dir.clone();
        command.cache_directory = cache_directory;
        command.manifest = self.manifest.clone();
        command.current_directory = Some(cwd);

        Ok(command.launch(endpoint, root))
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
