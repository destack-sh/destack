use destack_artifact::ArtifactKey;
use destack_source::ModuleId;
use destack_workspace::Revision;
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Lint/fix options for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandLintOptions {
    /// Apply fixes.
    pub fix: bool,
    /// Include unsafe fixes.
    pub unsafe_fixes: bool,
    /// Show diff instead of applying fixes.
    pub diff: bool,
}

/// Options for the check command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCheckOptions {
    /// Whether linting should run when supported.
    pub lint: bool,
    /// Lint/fix options for the check command.
    pub lint_options: CommandLintOptions,
}

impl CommandContext<'_> {
    /// Execute a check command.
    pub(super) fn run_check_command(
        &mut self,
        options: &CommandCheckOptions,
    ) -> super::CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // collect the requested roots
        let lint_enabled = should_run_lint_tasks(options.lint, &options.lint_options);
        let mut artifact_keys = Vec::new();
        for module_id in &modules {
            artifact_keys.push(self.check_root_for_module(revision, *module_id, lint_enabled)?);
        }

        // provide the requested roots
        let revision = self.revision()?;
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let raw_diagnostics = self
            .repository
            .diagnostics(revision)
            .map_err(|error| error.to_string())?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.default_profile_count(revision, &modules)?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
        ))
    }

    /// Execute a lint command.
    pub(super) fn run_lint_command(
        &mut self,
        options: &CommandLintOptions,
    ) -> super::CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;

        // collect the requested roots
        let lint_enabled = should_run_lint_tasks(true, options);
        let mut artifact_keys = Vec::new();
        for module_id in &modules {
            artifact_keys.push(self.check_root_for_module(revision, *module_id, lint_enabled)?);
        }

        // provide the requested roots
        let revision = self.revision()?;
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let raw_diagnostics = self
            .repository
            .diagnostics(revision)
            .map_err(|error| error.to_string())?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let profile_count = self.default_profile_count(revision, &modules)?;

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
        ))
    }

    /// Build the requested check root for one module.
    fn check_root_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        lint_enabled: bool,
    ) -> super::CommandResult<ArtifactKey> {
        let profile = self.module_profile_id(revision, module_id)?;

        if lint_enabled {
            return Ok(ArtifactKey::module_linted(module_id, profile));
        }

        Ok(ArtifactKey::dir_checked(module_id, profile))
    }
}

/// Determine if lint tasks should run.
fn should_run_lint_tasks(lint_enabled: bool, lint_options: &CommandLintOptions) -> bool {
    lint_enabled && !lint_options.fix && !lint_options.diff
}
