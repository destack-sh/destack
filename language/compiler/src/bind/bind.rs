use dyst_dir::{DependencyEdge, Expression, LocalNodeId, ModuleId, ScopeKind, SymbolSpace};

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

        // create module symbol and scope
        let (scope_id, _symbol_id) = {
            let module = module.read();
            let mut symbols = module.symbols.write();
            let scope_id =
                symbols.create_scope(ScopeKind::Namespace, self.session.root_scope_id, None);
            let symbol_id = symbols.create_symbol(SymbolSpace::Value, None, scope_id);
            symbols.get_scope_by_id_mut(scope_id).owner = Some(symbol_id.into_global(module_id));
            (scope_id, symbol_id)
        };

        // bind roots
        let roots: Vec<LocalNodeId<Expression>> = {
            let module = module.read();
            let mut tree = module.tree.write();
            let mut symbols = module.symbols.write();
            module
                .ast_roots
                .iter()
                .map(|expression| {
                    self.bind_expression(&module, scope_id, *expression, &mut tree, &mut symbols)
                })
                .collect()
        };
        module.write().roots.extend(roots);

        // bind dependencies
        let imports: Vec<DependencyEdge> = {
            let module = module.read();
            let mut tree = module.tree.write();
            let mut symbols = module.symbols.write();
            self.bind_dependency_edges(&module, scope_id, &mut tree, &mut symbols)
        };
        module.write().imports.extend(imports);

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id }.into());

        Ok(())
    }
}
