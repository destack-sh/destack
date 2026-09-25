use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_repository::TraceView;
use tspp_serde::Reflect;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandMessagePayload, CommandOptions, CommandRevision,
    CommandTargetOverrides, ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Options for the test command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct TestOptions {}

/// Request to run workspace tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TestInput {
    /// Revision selected for this test run.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
}

impl_command_input_options!(TestInput {});

impl CommandContext<'_> {
    /// Execute a test command.
    pub(crate) fn run_test_command(
        &mut self,
        _options: &TestOptions,
    ) -> CommandResult<CommandOutcome<CommandMessagePayload>> {
        self.unimplemented_command("test runner is not implemented yet")
    }
}
