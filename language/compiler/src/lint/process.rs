use destack_linter::{LintDiagnostic, LintLevel, LintRunner};
use destack_source::ModuleId;
use destack_workspace::{LinterOptions, Program};

use crate::{Compiler, LintResult, Task, TaskDebug, TaskOutput};

/// Task to lint something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LintTask {
    /// Lint a module at the AST level.
    LintModuleAst { module: ModuleId },

    /// Lint a module at the DIR level.
    LintModuleDir { module: ModuleId },

    /// Lint a module at the MIR level.
    LintModuleMir { module: ModuleId },

    /// Lint the entire program.
    LintProgram,
}

impl LintTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::LintModuleAst { .. } => 1,
            Self::LintModuleDir { .. } => 2,
            Self::LintModuleMir { .. } => 3,
            Self::LintProgram => 4,
        }
    }
}

impl TaskDebug for LintTask {
    fn name(&self) -> &'static str {
        match self {
            Self::LintModuleAst { .. } => "module_ast",
            Self::LintModuleDir { .. } => "module_dir",
            Self::LintModuleMir { .. } => "module_mir",
            Self::LintProgram => "program",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::LintModuleAst { module }
            | Self::LintModuleDir { module }
            | Self::LintModuleMir { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            Self::LintProgram => String::new(),
        }
    }
}

impl From<LintTask> for Task {
    fn from(task: LintTask) -> Self {
        Task::Lint(task)
    }
}

/// Output of a lint task.
#[derive(Debug, Clone, PartialEq)]
pub struct LintOutput {
    /// Number of lint diagnostics reported.
    pub diagnostic_count: usize,
}

impl From<LintOutput> for TaskOutput {
    fn from(output: LintOutput) -> Self {
        TaskOutput::Lint(output)
    }
}

impl Compiler {
    /// Process a lint task.
    pub fn process_lint(&self, task: LintTask) -> LintResult<LintOutput> {
        let diagnostic_count = match task {
            LintTask::LintModuleAst { module } => {
                // require module has been parsed (import phase complete)
                self.require_import_module(module)?;
                self.lint_module(module, LintLevel::Ast)?
            }
            LintTask::LintModuleDir { module } => {
                // require module has been analyzed
                self.require_analyze_module(module)?;
                self.lint_module(module, LintLevel::Dir)?
            }
            LintTask::LintModuleMir { module } => {
                // require module has been lowered and verified
                self.require_verify_module(module)?;
                self.lint_module(module, LintLevel::Mir)?
            }
            LintTask::LintProgram => self.lint_program()?,
        };
        Ok(LintOutput { diagnostic_count })
    }

    /// Lint a module at the specified level.
    fn lint_module(&self, module_id: ModuleId, level: LintLevel) -> LintResult<usize> {
        let module_arc = self.program.modules.get(module_id);
        let module = module_arc.read();

        // get linter options from dsconfig, or use defaults
        let default_options = LinterOptions::default();
        let linter_options = self
            .program
            .with_dsconfig_options(&module, |opts| opts.linter.clone())
            .unwrap_or(default_options);

        drop(module);

        let runner = LintRunner::with_recommended_rules();
        let diagnostics = runner.lint_module(&self.program, module_arc, &linter_options, level);

        let count = diagnostics.len();
        for diagnostic in diagnostics {
            self.report_lint_diagnostic(diagnostic);
        }

        Ok(count)
    }

    /// Lint the entire program.
    fn lint_program(&self) -> LintResult<usize> {
        // for program-level lints, use default options
        // TODO: could merge options from all packages
        let linter_options = LinterOptions::default();
        let runner = LintRunner::with_recommended_rules();
        let diagnostics = runner.lint_program(&self.program, &linter_options);

        let count = diagnostics.len();
        for diagnostic in diagnostics {
            self.report_lint_diagnostic(diagnostic);
        }

        Ok(count)
    }

    /// Report a lint diagnostic.
    fn report_lint_diagnostic(&self, diagnostic: LintDiagnostic) {
        let compiler_diagnostic = diagnostic.into_diagnostic();
        self.program.diagnostics.insert(compiler_diagnostic);
    }
}
