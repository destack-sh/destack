use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_source::{DiagnosticCollection, ModuleId};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use super::context::CommandContext;
use super::dispatch::CommandOutcome;
use super::error::DaemonCommandError;

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
            artifact_keys.push(check_root_for_module(
                &self.repository,
                revision,
                *module_id,
                lint_enabled,
            )?);
        }

        // provide the requested roots
        let run_stats = self
            .session
            .provide(&artifact_keys)
            .map_err(|error| error.to_string())?;
        let revision = self.session.revision();
        let raw_diagnostics =
            collect_module_profile_diagnostics(&self.repository, revision, &modules)?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let module_count = self.module_count(revision)?;
        let profile_count = self.default_profile_count(revision, &modules)?;
        let stats = self
            .session
            .compiler()
            .stats
            .snapshot_with_repository(module_count, Some(&self.repository));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
            Some(stats),
        )
        .with_run_stats(run_stats))
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
            artifact_keys.push(check_root_for_module(
                &self.repository,
                revision,
                *module_id,
                lint_enabled,
            )?);
        }

        // provide the requested roots
        let run_stats = self
            .session
            .provide(&artifact_keys)
            .map_err(|error| error.to_string())?;
        let revision = self.session.revision();
        let raw_diagnostics =
            collect_module_profile_diagnostics(&self.repository, revision, &modules)?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let module_count = self.module_count(revision)?;
        let profile_count = self.default_profile_count(revision, &modules)?;
        let stats = self
            .session
            .compiler()
            .stats
            .snapshot_with_repository(module_count, Some(&self.repository));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            0,
            Some(stats),
        )
        .with_run_stats(run_stats))
    }
}

/// Build the requested check root for one module.
fn check_root_for_module(
    repository: &Arc<Repository>,
    revision: Revision,
    module_id: ModuleId,
    lint_enabled: bool,
) -> Result<ArtifactKey, DaemonCommandError> {
    let profile = repository
        .default_profile_id_for_module(revision, module_id)
        .map_err(|error| DaemonCommandError::compiler(error.to_string()))?;

    if lint_enabled {
        return Ok(ArtifactKey::module_linted(module_id, profile));
    }

    Ok(ArtifactKey::dir_analyzed(module_id, profile))
}

/// Collect diagnostics across the current artifact families for the requested modules.
fn collect_module_profile_diagnostics(
    repository: &Arc<Repository>,
    revision: Revision,
    modules: &[ModuleId],
) -> Result<DiagnosticCollection, DaemonCommandError> {
    let mut diagnostics = DiagnosticCollection::new();

    // current module families
    for module_id in modules {
        let profile_id = repository
            .default_profile_id_for_module(revision, *module_id)
            .map_err(|error| DaemonCommandError::compiler(error.to_string()))?;
        diagnostics
            .merge_from(&repository.module_artifact_diagnostics(revision, *module_id, profile_id));
    }

    Ok(diagnostics)
}

/// Determine if lint tasks should run.
fn should_run_lint_tasks(lint_enabled: bool, lint_options: &CommandLintOptions) -> bool {
    lint_enabled && !lint_options.fix && !lint_options.diff
}
