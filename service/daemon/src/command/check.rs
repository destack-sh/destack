use std::sync::Arc;

use destack_compiler::{BuildKey, Compiler};
use destack_linter::Linter;
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, Program};
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

        // enqueue analysis tasks
        self.reset_diagnostics();
        let lint_enabled = should_run_lint_tasks(options.lint, &options.lint_options);
        enqueue_check_tasks(&self.program, &self.compiler, &modules);

        // compile and collect diagnostics
        self.compiler.compile();
        if lint_enabled {
            run_module_lints(&self.program, &modules)?;
        }
        let raw_diagnostics = self.collect_raw_diagnostics();
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let stats = self
            .compiler
            .stats
            .snapshot_with_program(self.program.modules.len(), Some(&self.program));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            self.program.profiles.len(),
            0,
            Some(stats),
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

        // enqueue analysis tasks
        self.reset_diagnostics();
        let lint_enabled = should_run_lint_tasks(true, options);
        enqueue_check_tasks(&self.program, &self.compiler, &modules);

        // compile and collect diagnostics
        self.compiler.compile();
        if lint_enabled {
            run_module_lints(&self.program, &modules)?;
        }
        let raw_diagnostics = self.collect_raw_diagnostics();
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let stats = self
            .compiler
            .stats
            .snapshot_with_program(self.program.modules.len(), Some(&self.program));

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            self.program.profiles.len(),
            0,
            Some(stats),
        ))
    }
}

/// Enqueue analysis tasks for the provided modules.
fn enqueue_check_tasks(program: &Arc<Program>, compiler: &Arc<Compiler>, modules: &[ModuleId]) {
    for module_id in modules {
        let profile = program.default_profile_id_for_module(*module_id);
        compiler.enqueue(BuildKey::artifact(ArtifactKey::dir_analyzed(
            *module_id, profile,
        )));
    }
}

/// Run module scoped lints over already-built compiler products.
fn run_module_lints(
    program: &Arc<Program>,
    modules: &[ModuleId],
) -> Result<(), DaemonCommandError> {
    let linter = Linter::new(program.clone());

    // lint each requested module against its default profile
    for module_id in modules {
        let profile_id = program.default_profile_id_for_module(*module_id);
        linter
            .lint_module(*module_id, profile_id)
            .map_err(|error| DaemonCommandError::compiler(error.to_string()))?;
    }

    Ok(())
}

/// Determine if lint tasks should run.
fn should_run_lint_tasks(lint_enabled: bool, lint_options: &CommandLintOptions) -> bool {
    lint_enabled && !lint_options.fix && !lint_options.diff
}
