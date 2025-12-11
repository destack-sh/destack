use destack_dir::{Expression, GlobalNodeIdAny, LocalNodeId, LocalSymbolId, StaticKey, SymbolKind};
use destack_source::ModuleId;

use crate::{BindError, BindResult, Compiler, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_workspace::{Module, Program};

/// Task to bind AST into DIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BindTask {
    /// Bind a module's AST into DIR.
    BindModule { module: ModuleId },
}

impl BindTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            BindTask::BindModule { .. } => 1,
        }
    }
}

impl TaskDebug for BindTask {
    fn name(&self) -> &'static str {
        match self {
            BindTask::BindModule { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            BindTask::BindModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
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
        match task {
            BindTask::BindModule { module } => {
                self.require_import_module(module)?;
                let module_arc = self.program.modules.get(module);
                let mut module_guard = module_arc.write();
                self.bind_module(&mut module_guard);
            }
        }
        Ok(BindOutput {})
    }

    /// Ensure a module has been bound.
    pub fn require_bind_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModule { module })
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

        tracing::debug!(?module_uri, "bind.module.complete");
    }

    /// Bind the AST root expressions for a module.
    fn bind_module_roots(&self, module: &mut Module) {
        let mut tree = module.dir.tree.write();
        let mut symbols = module.dir.symbols.write();
        let mut types = module.dir.types.write();
        let roots: Vec<LocalNodeId<Expression>> = module
            .ast
            .roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    (
                        module.dir.namespace_scope,
                        symbols.get_scope_mark(module.dir.namespace_scope),
                    ),
                    *expression,
                    None,
                    &mut tree,
                    &mut symbols,
                    &mut types,
                )
            })
            .collect();
        module.dir.roots.extend(roots);
    }

    /// Bind module exports and resolve conflicts.
    fn bind_module_exports(&self, module: &mut Module) {
        let symbols = module.dir.symbols.read();

        // collect exported symbols
        let root_scope = symbols.get_scope_by_id(module.dir.namespace_scope);
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
        let mut exported = module.dir.exported_symbols.write();
        for (_, symbol_id) in exported_symbols.iter() {
            let symbol = symbols.get_symbol(*symbol_id);
            let space = symbol.space;
            let Some(key) = symbol.name() else {
                continue; // should have a name but fine
            };
            let key = StaticKey::Name(key);
            // override if already exported (we error conflicting exports in a separate check)
            exported.insert((space, key), *symbol_id);
        }
    }

    /// Check for conflicting item symbols in module scopes and report errors.
    fn bind_check_scopes(&self, module: &mut Module) {
        let symbols = module.dir.symbols.read();
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
