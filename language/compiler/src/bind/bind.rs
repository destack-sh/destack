use dyst_dir::{ModuleId, NodeTree, ScopeKind, SymbolSpace};

use crate::{BindError, BindResult, Compiler, CompileTask, ResolveTask};

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

impl<'a> Compiler<'a> {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask, tree: &mut NodeTree) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => self.bind_module(module, tree),
        }
    }

    /// Bind a module.
    pub fn bind_module(&self, module: ModuleId, tree: &mut NodeTree) -> BindResult<()> {
        let module = self
            .session
            .modules
            .get(module)
            .ok_or(BindError::ModuleNotFound { module })?;
        let module_id = module.read().id;

        // create module symbol and scope
        let scope_id = tree.create_scope(ScopeKind::Namespace, self.session.root_scope_id, None);
        let symbol_id = tree.create_symbol(SymbolSpace::Value, None, scope_id);
        tree.get_scope_by_id_mut(scope_id).owner = Some(symbol_id);
        {
            let mut module = module.write();
            module.symbol = Some(symbol_id);
            module.scope = Some(scope_id);
        }

        // bind roots
        let roots: Vec<_> = module
            .read()
            .ast_roots
            .iter()
            .map(|expression| self.bind_expression(&module.read(), scope_id, *expression, tree))
            .collect();
        module.write().roots.extend(roots);

        // bind dependencies
        let imports = self.bind_dependency_edges(&module.read(), scope_id, tree);
        module.write().imports.extend(imports);

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());

        Ok(())
    }
}
