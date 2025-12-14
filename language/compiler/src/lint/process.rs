use destack_linter::{LintLevel, LintRunner};
use destack_source::{ModuleId, PackageId};
use destack_workspace::Program;

use crate::{
    Compiler, LintError, LintResult, Task, TaskDebug, TaskDependencyError, TaskOutput,
    TaskResultCollector,
};

/// Task to lint something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LintTask {
    /// Lint a module.
    LintModule { module: ModuleId },

    /// Lint a package
    LintPackage { package: PackageId },
}

impl LintTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::LintModule { .. } => 1,
            Self::LintPackage { .. } => 2,
        }
    }
}

impl TaskDebug for LintTask {
    fn name(&self) -> &'static str {
        match self {
            Self::LintModule { .. } => "module",
            Self::LintPackage { .. } => "package",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::LintModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            Self::LintPackage { package } => {
                let package = program.packages.get(*package);
                let uri = package.read().uri.clone().to_string();
                format!(r#"package="{uri}""#)
            }
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
pub struct LintOutput {}

impl From<LintOutput> for TaskOutput {
    fn from(output: LintOutput) -> Self {
        TaskOutput::Lint(output)
    }
}

impl Compiler {
    /// Process a lint task.
    pub fn process_lint(&self, task: LintTask) -> LintResult<LintOutput> {
        match task {
            LintTask::LintModule { module } => {
                self.require_analyze_module(module)?;
                self.lint_module(module)?
            }
            LintTask::LintPackage { package } => self.lint_package(package)?,
        };
        Ok(LintOutput {})
    }

    /// Ensure a module has been analyzed.
    pub fn require_lint_module(&self, module_id: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintModule { module: module_id })
    }

    /// Ensure a package has been linted.
    pub fn require_lint_package(&self, package_id: PackageId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LintTask::LintPackage {
            package: package_id,
        })
    }

    /// Lint a module.
    fn lint_module(&self, module_id: ModuleId) -> LintResult<()> {
        let options = self.program.get_linter_options(module_id);
        if !options.enabled {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let runner = LintRunner::from_options(&options);

        // run at each IR level
        let ast_diagnostics = runner.lint_module(
            self.program.clone(),
            module.clone(),
            &options,
            LintLevel::Ast,
        );
        let dir_diagnostics = runner.lint_module(
            self.program.clone(),
            module.clone(),
            &options,
            LintLevel::Dir,
        );

        // collect diagnostics
        for diagnostic in ast_diagnostics.into_iter().chain(dir_diagnostics) {
            self.program
                .diagnostics
                .insert(diagnostic.into_diagnostic());
        }

        // #Incomplete: run comptime/user-defined lints?

        Ok(())
    }

    /// Lint a package.
    fn lint_package(&self, package_id: PackageId) -> LintResult<()> {
        // collect all modules in the package
        let modules = self
            .program
            .modules
            .iter()
            .filter(|module| module.read().package_id == package_id);

        // lint each module
        let mut collector = TaskResultCollector::new();
        for module in modules {
            let result = self.require_lint_module(module.read().id);
            collector.try_collect(result);
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(LintError::Yield { dependency });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_lint_module_detects_debugger() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
debugger;
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC001");
    }

    #[test]
    fn test_lint_module_detects_empty_block() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
{}
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC002");
    }

    #[test]
    fn test_lint_module_detects_constant_condition() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
if (true) { foo(); }
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC003");
    }

    #[test]
    fn test_lint_module_detects_self_compare() {
        let test = TestProgram::memory_sequential();
        let module = test.add_module(
            "test.ds",
            r#"
let x = 1;
x == x;
"#,
        );
        test.lint_module(module);
        test.compile();
        test.check_has_diagnostic("LC004");
    }
}
