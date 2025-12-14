use destack_source::{ModuleId, PackageId};
use destack_workspace::Program;

use crate::{Compiler, LintResult, Task, TaskDebug, TaskOutput, TaskResultCollector};

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
pub struct LintOutput {
    /// Number of lint diagnostics reported.
    pub diagnostic_count: usize,
}

impl From<LintOutput> for TaskOutput {
    fn from(output: LintOutput) -> Self {
        TaskOutput::Lint(output)
    }
}

// nocheckin: implement lint in Compiler and hook it up

impl Compiler {
    /// Process a lint task.
    pub fn process_lint(&self, task: LintTask) -> LintResult<LintOutput> {
        let diagnostic_count = match task {
            LintTask::LintModule { module } => {
                self.require_analyze_module(module)?;
                self.lint_module(module)?
            }
            LintTask::LintPackage { package } => self.lint_package(package)?,
        };
        Ok(LintOutput { diagnostic_count })
    }

    /// Lint a module.
    fn lint_module(&self, _module_id: ModuleId) -> LintResult<usize> {
        // TODO: implement lint module
        Ok(0)
    }

    /// Lint a package.
    fn lint_package(&self, package_id: PackageId) -> LintResult<usize> {
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
    }
}
