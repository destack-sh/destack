use dyst_dir::{ModuleId, NodeTree, ScopeKind};

use crate::{BindResult, Compiler, CompilerTask, ResolveTask};

/// Task to bind AST into DIR.
#[derive(Debug, Clone)]
pub enum BindTask {
    /// Bind a module.
    BindModule { module: ModuleId },
}

impl From<BindTask> for CompilerTask {
    fn from(task: BindTask) -> Self {
        CompilerTask::Bind(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask, tree: &mut NodeTree) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => self.bind_module(module, tree),
        }
    }

    /// Bind a module.
    pub fn bind_module(&self, module: ModuleId, tree: &mut NodeTree) -> BindResult<()> {
        let module = self.session.modules.get_mut(module).unwrap();
        let module_id = module.id;

        // create scope
        let scope_id = tree.create_scope(ScopeKind::Module, None, None);
        module.scope = Some(scope_id);

        // bind expressions
        let expressions: Vec<_> = module
            .ast_roots
            .iter()
            .map(|expression| self.bind_expression(&module, scope_id, *expression, tree))
            .collect();
        module.roots.extend(expressions);

        // begin resolving
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());

        Ok(())
    }
}
