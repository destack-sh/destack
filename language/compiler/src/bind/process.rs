use dyst_dir::{DependencyEdge, Expression, LocalNodeId, ModuleId};

use crate::{BindError, BindResult, CompileTask, Compiler, ResolveTask};

/// Task to bind AST into DIR.
#[derive(Debug, Clone)]
pub enum BindTask {
    /// Bind a module.
    BindModule { module: ModuleId },
}

impl From<BindTask> for CompileTask {
    fn from(task: BindTask) -> Self {
        CompileTask::Bind(task)
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
            .session
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

        // bind dependencies
        let imports: Vec<DependencyEdge> = {
            let module = module.read();
            let mut tree = module.tree.write();
            let mut symbols = module.symbols.write();
            self.bind_dependency_edges(&module, module.scope, &mut tree, &mut symbols)
        };
        module.write().imports.extend(imports);

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());

        Ok(())
    }
}
