pub mod resolve;
pub mod version;

use clap::Subcommand;

pub use resolve::ResolveArgs;
pub use version::VersionCommands;

/// Developer subcommands for compiler development and release management.
#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DevCommand {
    /// Resolve a module specifier.
    Resolve(ResolveArgs),
    /// Version management commands.
    #[command(subcommand)]
    Version(VersionCommands),
}
