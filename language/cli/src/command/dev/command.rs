use clap::Subcommand;

use super::{StatsArgs, VersionCommand};

/// Developer subcommands for compiler development and release management.
#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DevCommand {
    /// Codebase statistics (lines of code and tokens).
    Stats(StatsArgs),
    /// Version management commands.
    #[command(subcommand)]
    Version(VersionCommand),
}
