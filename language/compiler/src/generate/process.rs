use crate::{Compiler, GenerateResult, Task, TaskDebug, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to generate code for a module into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum GenerateTask {
    /// Generate a module for a specific target.
    GenerateModule {
        /// The module to generate.
        module: ModuleId,
        /// The target name (looked up on the module's package).
        target: String,
    },
}

impl GenerateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::GenerateModule { .. } => 1,
        }
    }
}

impl TaskDebug for GenerateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::GenerateModule { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::GenerateModule { module, target } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}" target="{target}""#)
            }
        }
    }
}

impl From<GenerateTask> for Task {
    fn from(task: GenerateTask) -> Self {
        Task::Generate(task)
    }
}

/// Output of a generate task.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateOutput {}

impl From<GenerateOutput> for TaskOutput {
    fn from(output: GenerateOutput) -> Self {
        TaskOutput::Generate(output)
    }
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<GenerateOutput> {
        match task {
            GenerateTask::GenerateModule { module, target } => {
                self.generate_module(module, &target)
            }
        }
    }

    /// Generate code for a module.
    fn generate_module(&self, module: ModuleId, target: &str) -> GenerateResult<GenerateOutput> {
        // TODO: implement generate_module
        // 1. look up target from module's package
        // 2. yield to Elaborate (JS/TS) or Optimize (Native/Wasm) depending on target.output
        // 3. dispatch to generate_js, generate_ts, generate_native, or generate_wasm
        // 4. store artifact in program.artifacts
        let _ = (module, target);
        todo!("generate_module")
    }
}
