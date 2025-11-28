use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use clap::{Args, ValueEnum};
use dyst_dir::Program;
use dyst_source::{
    FileRegistry, FileSystem, FormattingOptions, IndentStyle, LanguageMode, LanguageOptions,
    LineEnding, MemoryFileSystem, PhysicalFileSystem,
};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
}

/// The file system type to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum FileSystemArg {
    /// Use the physical file system.
    #[default]
    Physical,
    /// Use an in-memory file system.
    Memory,
}

/// The language mode to use.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum LanguageModeArg {
    /// Lenient mode with relaxed checking.
    #[default]
    Lenient,
    /// Strict mode with explicit typing.
    Strict,
}

impl From<LanguageModeArg> for LanguageMode {
    fn from(mode: LanguageModeArg) -> Self {
        match mode {
            LanguageModeArg::Lenient => LanguageMode::Lenient,
            LanguageModeArg::Strict => LanguageMode::Strict,
        }
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

/// Arguments for configuring language options.
#[derive(Args, Debug, Clone)]
pub struct LanguageOptionsArgs {
    /// The language mode (lenient|strict, default: lenient).
    #[arg(long = "mode", value_enum)]
    pub language_mode: Option<LanguageModeArg>,

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

impl From<LanguageOptionsArgs> for LanguageOptions {
    fn from(args: LanguageOptionsArgs) -> Self {
        let mut options = LanguageOptions::default();

        if let Some(mode) = args.language_mode {
            options.mode = mode.into();
        }

        let mut formatting = FormattingOptions::default();
        if let Some(style) = args.indent_style {
            formatting.indent_style = style.into();
        }
        if let Some(width) = args.indent_width {
            formatting.indent_width = width;
        }
        if let Some(ending) = args.line_ending {
            formatting.line_ending = ending.into();
        }
        if let Some(width) = args.line_width {
            formatting.line_width = width;
        }
        options.formatting = formatting;

        options
    }
}

/// Arguments for configuring program setup.
#[derive(Args, Debug, Clone)]
pub struct ProgramArgs {
    /// The working directory (default: current directory).
    #[arg(long = "cwd")]
    pub cwd: Option<PathBuf>,

    /// The file system type to use (physical|memory, default: physical).
    #[arg(long = "fs", value_enum)]
    pub file_system: Option<FileSystemArg>,

    /// The number of worker threads to use (default: number of CPU cores).
    #[arg(long = "workers", short = 'j', default_value_t = default_workers())]
    pub workers: u16,

    /// The language options.
    #[command(flatten)]
    pub language: LanguageOptionsArgs,
}

impl ProgramArgs {
    /// Create a Program from these arguments.
    pub fn setup(&self) -> Arc<Program> {
        let cwd = self
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let language_options: LanguageOptions = self.language.clone().into();
        let fs_type = self.file_system.unwrap_or_default();
        let files = Arc::new(FileRegistry::new());
        let fs: Arc<dyn FileSystem> = match fs_type {
            FileSystemArg::Physical => Arc::new(PhysicalFileSystem::new()),
            FileSystemArg::Memory => Arc::new(MemoryFileSystem::new()),
        };
        tracing::trace!(?cwd, ?fs_type, workers = self.workers, "program.setup");
        let program = Program::new(language_options, cwd, fs, files);
        Arc::new(program)
    }
}
