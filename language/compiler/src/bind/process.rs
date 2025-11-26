use dyst_dir::{
    Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, ModuleId, Program, SymbolKey,
    SymbolKind,
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
    pub(super) fn bind_module(&self, module_id: ModuleId) -> BindResult<()> {
        self.bind_module_roots(module_id)?;
        self.bind_module_exports(module_id)?;
        self.check_module_scopes(module_id)?;

        // next task: resolve module
        self.enqueue(ResolveTask::ResolveModule { module: module_id });

        Ok(())
    }

    /// Bind the AST root expressions for a module.
    fn bind_module_roots(&self, module_id: ModuleId) -> BindResult<()> {
        let module = self.program.modules.get(module_id);
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
        Ok(())
    }

    /// Bind module exports and resolve conflicts.
    fn bind_module_exports(&self, module_id: ModuleId) -> BindResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut symbols = module.symbols.write();

        // collect exported symbols
        let root_scope = symbols.get_scope_by_id(module.namespace_scope);
        let exported_symbols: Vec<(GlobalNodeIdAny, LocalSymbolId)> = root_scope
            .named_symbols
            .iter()
            .filter_map(|(_, symbol_id)| {
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
        for (_, symbol_id) in exported_symbols.iter() {
            let symbol = symbols.get_symbol(*symbol_id);
            let space = symbol.space;
            let Some(key) = symbol.name() else {
                continue; // should have a name but fine
            };
            let key = SymbolKey::Name(key);
            // override if already exported (we error conflicting exports in a separate check)
            symbols.resolve_export((space, key), *symbol_id);
        }
        Ok(())
    }

    /// Check for conflicting item symbols in module scopes and report errors.
    fn check_module_scopes(&self, module_id: ModuleId) -> BindResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let symbols = module.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                for (other_key, other_symbol_id) in scope.named_symbols.iter() {
                    if *other_key == *key && *other_symbol_id != *symbol_id {
                        let other_symbol = symbols.get_symbol(*other_symbol_id);

                        // local "conflicts" are fine
                        if symbol.kind == SymbolKind::Local
                            && other_symbol.kind == SymbolKind::Local
                        {
                            continue;
                        }

                        // conflicting binding, error appropriately
                        let Some(other_primary_declaration) = other_symbol.primary_declaration
                        else {
                            continue;
                        };
                        let error = if symbol.export.is_some() && other_symbol.export.is_some() {
                            BindError::ConflictingExport {
                                node: primary_declaration,
                                other_node: other_primary_declaration,
                                module: module_id,
                                name: Some(*key),
                            }
                        } else {
                            BindError::ConflictingBinding {
                                node: primary_declaration,
                                other_node: other_primary_declaration,
                                scope: scope.id.into_global(module_id),
                                name: Some(*key),
                            }
                        };
                        self.error(error);
                    }
                }
            }
        }
        Ok(())
    }
}
