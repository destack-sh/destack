use crate::{Compiler, ImportResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to import (load, parse, and bind) a module.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Import)]
pub enum ImportTask {
    /// Import a module completely (parse, bind, desugar, validate).
    #[task(code = 1, trace = "module={module}")]
    ImportModule { module: ModuleId },

    /// Parse a module (load file and parse into AST).
    #[task(code = 2, trace = "module={module}")]
    ImportModuleParse { module: ModuleId },

    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    #[task(code = 3, trace = "module={module}")]
    ImportModuleBind { module: ModuleId },

    /// Desugar the module syntactically.
    #[task(code = 4, trace = "module={module}")]
    ImportModuleDesugar { module: ModuleId },

    /// Validate module "syntactic" correctness.
    #[task(code = 5, trace = "module={module}")]
    ImportModuleValidate { module: ModuleId },
}

impl Compiler {
    /// Process an import task.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<()> {
        match task {
            ImportTask::ImportModule { module } => {
                self.require_import_module_validate(module)?;
            }
            ImportTask::ImportModuleParse { module } => {
                self.import_module_parse(module)?;
            }
            ImportTask::ImportModuleBind { module } => {
                self.import_module_bind(module)?;
                self.stats.record_bind();
            }
            ImportTask::ImportModuleDesugar { module } => {
                self.import_module_desugar_phase(module)?;
            }
            ImportTask::ImportModuleValidate { module } => {
                self.import_module_validate(module)?;
            }
        }
        Ok(())
    }

    /// Ensure a module has been fully imported (parsed, bound, desugared, validated).
    pub fn require_import_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModule { module })
    }

    /// Ensure a module has been parsed.
    pub fn require_import_module_parse(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModuleParse { module })
    }

    /// Ensure a module has been bound (DIR built).
    pub fn require_import_module_bind(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModuleBind { module })
    }

    /// Ensure a module has been desugared after binding.
    pub fn require_import_module_desugar(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModuleDesugar { module })
    }

    /// Ensure a module has been validated after binding.
    pub fn require_import_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModuleValidate { module })
    }
}
