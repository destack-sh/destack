use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, LintTask};
use destack_source::ModuleId;
use destack_workspace::Program;
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
    ) -> Result<CommandOutcome, String> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;

        // enqueue analysis or lint tasks
        self.reset_diagnostics();
        let use_lint_tasks = should_run_lint_tasks(options.lint, &options.lint_options);
        enqueue_check_tasks(&self.program, &self.compiler, &modules, use_lint_tasks);

        // compile and collect diagnostics
        self.compiler.compile();
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
    ) -> Result<CommandOutcome, String> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;

        // enqueue lint tasks
        self.reset_diagnostics();
        let use_lint_tasks = should_run_lint_tasks(true, options);
        enqueue_check_tasks(&self.program, &self.compiler, &modules, use_lint_tasks);

        // compile and collect diagnostics
        self.compiler.compile();
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

/// Enqueue analysis or lint tasks for the provided modules.
fn enqueue_check_tasks(
    program: &Arc<Program>,
    compiler: &Arc<Compiler>,
    modules: &[ModuleId],
    lint: bool,
) {
    for module_id in modules {
        let profile = program.default_profile_id_for_module(*module_id);
        let module = compiler.module_stamp(*module_id);
        let profile = compiler.profile_stamp(profile);
        if lint {
            compiler.enqueue(LintTask::LintModule { module, profile });
        } else {
            compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
        }
    }
}

/// Determine if lint tasks should run.
fn should_run_lint_tasks(lint_enabled: bool, lint_options: &CommandLintOptions) -> bool {
    lint_enabled && !lint_options.fix && !lint_options.diff
}
