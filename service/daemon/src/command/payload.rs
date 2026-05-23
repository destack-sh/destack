use serde::{Deserialize, Serialize};

use super::bench::CommandBenchOptions;
use super::build::CommandBuildOptions;
use super::cache::CommandCacheOptions;
use super::check::{CommandCheckOptions, CommandLintOptions};
use super::clean::CommandCleanOptions;
use super::doc::CommandDocOptions;
use super::doctor::CommandDoctorOptions;
use super::format::CommandFormatOptions;
use super::info::CommandInfoOptions;
use super::manifest::CommandManifestOptions;
use super::repl::CommandReplOptions;
use super::run::CommandRunOptions;
use super::settings::CommandSettingsOptions;
use super::targets::CommandTargetsOptions;
use super::task::CommandTaskOptions;
use super::test::CommandTestOptions;

/// Payload for command execution requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandPayload {
    /// Typecheck without emitting artifacts.
    Check(CommandCheckOptions),
    /// Lint code.
    Lint(CommandLintOptions),
    /// Build artifacts.
    Build(CommandBuildOptions),
    /// Run the compiled output.
    Run(CommandRunOptions),
    /// Run tests.
    Test(CommandTestOptions),
    /// Format code.
    Format(CommandFormatOptions),
    /// Generate documentation.
    Doc(CommandDocOptions),
    /// Run benchmarks.
    Bench(CommandBenchOptions),
    /// Show workspace and target information.
    Info(CommandInfoOptions),
    /// Show resolved manifest.
    Manifest(CommandManifestOptions),
    /// List configured build targets.
    Targets(CommandTargetsOptions),
    /// Show cache directory locations.
    Cache(CommandCacheOptions),
    /// Show resolved machine and workspace settings.
    Settings(CommandSettingsOptions),
    /// Show environment and workspace diagnostics.
    Doctor(CommandDoctorOptions),
    /// Run workspace tasks.
    Task(CommandTaskOptions),
    /// Start a REPL session.
    Repl(CommandReplOptions),
    /// Remove build artifacts and caches.
    Clean(CommandCleanOptions),
}
