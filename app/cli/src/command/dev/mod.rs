pub mod release;
pub mod stats;
pub mod version;

use clap::Subcommand;

pub use release::ReleaseArgs;
pub use stats::StatsArgs;
pub use version::VersionCommands;

/// Developer subcommands for compiler development and release management.
#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DevCommand {
    /// Codebase statistics (lines of code and tokens).
    Stats(StatsArgs),
    /// Integrated release flow for preflight and publish.
    Release(ReleaseArgs),
    /// Version management commands.
    #[command(subcommand)]
    Version(VersionCommands),
}
