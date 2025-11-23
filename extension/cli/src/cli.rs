use clap::Parser;

use crate::command::compile::CompileArgs;
use crate::command::lex::LexArgs;
use crate::command::parse::ParseArgs;
use crate::command::resolve::ResolveArgs;
use crate::command::transpile::TranspileArgs;
use crate::command::version::VersionCommands;

#[derive(Parser, Debug)]
#[command(name = "destack", version, about = "Destack CLI", long_about = None)]
pub enum Cli {
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
    /// Mark new versions.
    Version {
        #[command(subcommand)]
        subcommand: VersionCommands,
    },
}
