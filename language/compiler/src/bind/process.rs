use dyst_dir::{
    Expression, GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalSymbolId, ModuleId, Program,
    SymbolKey,
};

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
    pub fn message(&self, _program: &Program) -> String {
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
impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => self.bind_module(module),
        }
    }

    /// Bind a module.
    pub(super) fn bind_module(&self, module: ModuleId) -> BindResult<()> {
        let module = self.program.modules.get(module);
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
                        (
                            module.namespace_scope,
                            symbols.get_scope_mark(module.namespace_scope),
                        ),
                        *expression,
                        &mut tree,
                        &mut symbols,
                        &mut types,
                    )
                })
                .collect()
        };
        module.write().roots.extend(roots);

        // bind exports
        {
            let module = module.read();
            let tree = module.tree.read();
            let mut symbols = module.symbols.write();

            // collect exported symbols
            let root_scope = symbols.get_scope_by_id(module.namespace_scope);
            let exported_symbols: Vec<(GlobalNodeIdAny, LocalSymbolId)> = root_scope
                .named_symbols
                .iter()
                .filter_map(|(key, symbol_id)| {
                    let symbol = symbols.get_symbol(*symbol_id);
                    if let Some(primary_declaration) = symbol.primary_declaration
                        && symbol.export.is_some()
                    {
                        Some((primary_declaration, *symbol_id))
                    } else {
                        None
                    }
                })
                .collect();

            // resolve exported symbols and check for conflicts
            for (node_id, symbol_id) in exported_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let space = symbol.space;
                let Some(key) = symbol.name() else {
                    continue; // should have a name but fine
                };
                let key = SymbolKey::Name(key);
                // error on conflicting export
                if let Some(existing_symbol_id) = symbols.get_exported_symbol((space, key)) {
                    let existing_symbol = symbols.get_symbol(existing_symbol_id);
                    let error = BindError::ConflictingExport {
                        node: *node_id,
                        other_node: existing_symbol.primary_declaration,
                        module: module_id,
                        name: Some(key),
                    };
                    self.error(error);
                }
                // resolve if not conflicting
                else {
                    symbols.resolve_export((space, key), *symbol_id);
                }
            }
        }

        // check for conflicting item symbols in scopes
        {
            // nocheckin: detect conflicting item symbols in scopes
        }

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id });

        Ok(())
    }
}
