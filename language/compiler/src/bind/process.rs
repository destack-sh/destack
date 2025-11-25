use dyst_dir::{Expression, LocalNodeId, ModuleId, Program};

use crate::{BindError, BindResult, CompileOutput, CompileTask, Compiler, ResolveTask};

/// Task to bind AST into DIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BindTask {
    /// Bind a module.
    BindModule { module: ModuleId },
}

impl BindTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            BindTask::BindModule { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            BindTask::BindModule { module } => {
                format!("bind module '{module:?}'")
            }
        }
    }
}

impl From<BindTask> for CompileTask {
    fn from(task: BindTask) -> Self {
        CompileTask::Bind(task)
    }
}

/// Output of a bind task.
#[derive(Debug, Clone)]
pub struct BindOutput {}

impl From<BindOutput> for CompileOutput {
    fn from(output: BindOutput) -> Self {
        CompileOutput::Bind(output)
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => self.bind_module(module),
        }
    }

    /// Bind a module.
    pub fn bind_module(&self, module: ModuleId) -> BindResult<()> {
        let module = self
            .program
            .modules
            .get(module)
            .ok_or(BindError::ModuleNotFound { module })?;
        let module_id = module.read().id;

        // bind roots
        let roots: Vec<LocalNodeId<Expression>> = {
            let module = module.read();
            let mut tree = module.tree.write();
            let mut symbols = module.symbols.write();
            let mut types = module.types.write();
            module
                .ast_roots
                .iter()
                .map(|expression| {
                    self.bind_expression(
                        &module,
                        module.scope,
                        *expression,
                        &mut tree,
                        &mut symbols,
                        &mut types,
                    )
                })
                .collect()
        };
        module.write().roots.extend(roots);

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());

        Ok(())
    }
}
