use clap::Parser;

use crate::command::TracingArgs;
use crate::command::compile::CompileArgs;
use crate::command::format::FormatArgs;
use crate::command::lex::LexArgs;
use crate::command::parse::ParseArgs;
use crate::command::resolve::ResolveArgs;
use crate::command::transpile::TranspileArgs;
use crate::command::version::VersionCommands;

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
    /// Tokenize source into tokens.
    Lex(LexArgs),
    /// Parse source into AST (implicit module).
    Parse(ParseArgs),
    /// Resolve a module specifier.
    Resolve(ResolveArgs),
    /// Compile source into its final DIR.
    Compile(CompileArgs),
    /// Transpile source into its final JavaScript.
    Transpile(TranspileArgs),
    /// Format source files.
    #[command(alias = "fmt")]
    Format(FormatArgs),
    /// Mark new versions.
    Version {
        #[command(subcommand)]
        subcommand: VersionCommands,
    },
}
