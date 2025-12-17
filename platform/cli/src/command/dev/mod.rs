pub mod dir;
pub mod parse;
pub mod resolve;
pub mod version;

use clap::Subcommand;

pub use dir::DirArgs;
pub use parse::ParseArgs;
pub use resolve::ResolveArgs;
pub use version::VersionCommands;

/// Developer subcommands for compiler development and release management.
#[derive(Subcommand, Debug, Clone)]
pub enum DevCommand {
    /// Parse source into AST.
    Parse(ParseArgs),
    /// Compile and dump DIR (Destack IR).
    Dir(DirArgs),
    /// Resolve a module specifier.
    Resolve(ResolveArgs),
    /// Version management commands.
    #[command(subcommand)]
    Version(VersionCommands),
}
