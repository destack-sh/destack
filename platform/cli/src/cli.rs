// NOTE #Architecture: CLI should be implemented via NAPI on top of library/tui stuff

use clap::Parser;

#[cfg(feature = "dev")]
use crate::command::DevCommand;
use crate::command::{
    BuildArgs, CheckArgs, CleanArgs, FmtArgs, InitArgs, LintArgs, LspArgs, RunArgs,
};
use crate::common::TracingArgs;

#[derive(Parser, Debug)]
#[command(name = "destack", version, about = "Destack CLI", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub tracing: TracingArgs,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Check source files for type errors and lint issues.
    Check(CheckArgs),

    /// Compile source files.
    Build(BuildArgs),

    /// Compile and run a source file.
    Run(RunArgs),

    /// Lint source files (alias for check).
    Lint(LintArgs),

    /// Format source files.
    #[command(alias = "fmt")]
    Format(FmtArgs),

    /// Initialize a new project.
    Init(InitArgs),

    /// Remove build artifacts.
    Clean(CleanArgs),

    /// Start the language server (for editor integration).
    Lsp(LspArgs),

    /// Developer commands (compiler inspection, version management).
    #[cfg(feature = "dev")]
    #[command(subcommand)]
    Dev(DevCommand),
}
