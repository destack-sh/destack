use clap::Subcommand;

use super::{ReleaseArgs, StatsArgs, VersionCommands};

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
