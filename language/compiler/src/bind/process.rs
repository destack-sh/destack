use destack_dir::{
    Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, Module, Program, SymbolKey, SymbolKind,
};

use crate::{BindError, BindResult, Compiler, Task, TaskDebug, TaskOutput};

/// Task to bind AST into DIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BindTask {/* Bind is conceptually a stage, but we do it as part of import. */}

impl BindTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        unreachable!("bind is not a real task {self:?}");
    }
}

impl TaskDebug for BindTask {
    fn name(&self) -> &'static str {
        unreachable!("bind is not a real task {self:?}");
    }

    fn trace_args(&self, _program: &Program) -> String {
        unreachable!("bind is not a real task {self:?}");
    }
}

impl From<BindTask> for Task {
    fn from(task: BindTask) -> Self {
        Task::Bind(task)
    }
}

/// Output of a bind task.
#[derive(Debug, Clone, PartialEq)]
pub struct BindOutput {}

impl From<BindOutput> for TaskOutput {
    fn from(output: BindOutput) -> Self {
        TaskOutput::Bind(output)
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<BindOutput> {
        unreachable!("bind is not a real task {task:?}");
    }

    /// Bind a module.
    #[tracing::instrument(name = "bind.module", skip(self), fields(module_id))]
    pub(crate) fn bind_module(&self, module: &mut Module) {
        let module_uri = module.uri.to_string();
        tracing::debug!(?module_uri, "bind.module.start");

        // bind AST into DIR
        self.bind_module_roots(module);

        // bind module exports
        self.bind_module_exports(module);

        // check for conflicting symbols in scopes
        self.bind_check_scopes(module);

        tracing::debug!(?module, "bind.module.complete");
    }

    /// Bind the AST root expressions for a module.
    fn bind_module_roots(&self, module: &mut Module) {
        let mut tree = module.tree.write();
        let mut symbols = module.symbols.write();
        let mut types = module.types.write();
        let roots: Vec<LocalNodeId<Expression>> = module
            .ast_roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    (
                        module.namespace_scope,
                        symbols.get_scope_mark(module.namespace_scope),
                    ),
                    *expression,
                    None,
                    &mut tree,
                    &mut symbols,
                    &mut types,
                )
            })
            .collect();
        module.roots.extend(roots);
    }

    /// Bind module exports and resolve conflicts.
    fn bind_module_exports(&self, module: &mut Module) {
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
    }

    /// Check for conflicting item symbols in module scopes and report errors.
    fn bind_check_scopes(&self, module: &mut Module) {
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
                                module: module.id,
                                name: Some(*key),
                            }
                        } else {
                            BindError::ConflictingBinding {
                                node: primary_declaration,
                                other_node: other_primary_declaration,
                                scope: symbol.scope.0.into_global(module.id),
                                name: Some(*key),
                            }
                        };
                        self.error(error);
                    }
                }
            }
        }
    }
}
