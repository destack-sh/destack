use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use clap::{Args, ValueEnum};
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{FileSystem, IndentStyle, LineEnding, PhysicalFileSystem};
use destack_workspace::{FormatterOptions, LinterOptions, Session, TsConfigRegistry};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
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

/// Arguments for configuring formatter options.
#[derive(Args, Debug, Clone, Default)]
pub struct FormatterOptionsArgs {
    /// The indent style (tab|space, default: tab).
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
    pub line_width: Option<u8>,
}

impl From<FormatterOptionsArgs> for FormatterOptions {
    fn from(args: FormatterOptionsArgs) -> Self {
        let mut options = FormatterOptions::default();
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
    #[arg(long = "cwd")]
    pub cwd: Option<PathBuf>,

    /// The number of worker threads to use (default: number of CPU cores).
    #[arg(long = "workers", short = 'j', default_value_t = default_workers())]
    pub workers: u16,

    /// The formatter options.
    #[command(flatten)]
    pub formatter: FormatterOptionsArgs,
}

impl ProgramArgs {
    /// Create a Session from these arguments.
    pub fn setup(&self) -> Arc<Session> {
        let cwd = self
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let formatter_options: FormatterOptions = self.formatter.clone().into();
        let linter_options = LinterOptions::default();
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());

        // create session (builtins are always loaded)
        let session = Session::new(cwd.clone())
            .with_fs(fs.clone())
            .with_formatter(formatter_options)
            .with_linter(linter_options);

        // create shared tsconfig registry for resolver
        let tsconfigs = Arc::new(TsConfigRegistry::new());

        // discover workspace
        let resolver = Resolver::new(
            session.fs.clone(),
            session.files.clone(),
            session.packages.clone(),
            tsconfigs.clone(),
            ResolveOptions::default(),
        );
        let workspace = resolver
            .discover_workspace(&cwd)
            .unwrap_or_else(|_| destack_workspace::Workspace::single_package(cwd.clone()));
        let root = workspace.root.clone();

        tracing::trace!(?cwd, ?root, workspace_kind = ?workspace.kind, workers = self.workers, "program.setup");

        // add the root to create a program
        session.add_root(root);

        Arc::new(session)
    }
}
