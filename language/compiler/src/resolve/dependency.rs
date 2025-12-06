use destack_ast::StringId;
use destack_dir::{
    DependencyItem, DependencyMode, DependencySource, GlobalNodeIdAny, LocalNodeId, LocalScopeMark,
    NodeTree, StaticKey, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::Module;

use crate::{
    Compiler, ImportOutput, ImportTask, ResolveError, ResolveResult, TaskDependency, TaskOutput,
};

impl Compiler {
    /// Whether the target is a relative import.
    pub(super) fn is_import_relative(&self, target: StringId) -> bool {
        let target_str = self.program.strings.get(target);
        target_str.starts_with("./") || target_str.starts_with("../")
    }

    /// Try to resolve an import of some target specifier.
    /// Returns the resolved module id if successful, otherwise returns the yield "error".
    pub(super) fn resolve_import(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        symbols: &mut SymbolTable,
    ) -> ResolveResult<ModuleId> {
        let is_relative = self.is_import_relative(target);
        let relative_module = if is_relative { Some(module.id) } else { None };

        // get locally resolved import
        if let Some(remote_module_id) = symbols.get_resolved_import(relative_module, target) {
            return Ok(remote_module_id);
        }

        // get globally resolved import
        if !is_relative {
            let global_module = self.program.modules.get(self.program.root_module_id);
            let global_module = global_module.read();
            let global_symbols = global_module.dir.symbols.read();
            if let Some(remote_module_id) = global_symbols.get_resolved_import(None, target) {
                symbols.resolve_import(None, target, remote_module_id);
                return Ok(remote_module_id);
            }
        }

        // not resolved yet, prepare import task
        let import_task = ImportTask::ImportModuleFromSpecifier {
            source,
            target,
            ty: None,
            module: relative_module,
        };

        // skip yield if import task now has an output
        if let Some(output) = self.get_output(import_task.clone())
            && let TaskOutput::Import(ImportOutput {
                module: remote_module_id,
            }) = output
        {
            symbols.resolve_import(relative_module, target, remote_module_id);
            return Ok(remote_module_id);
        }

        // yield to import task
        let error = ResolveError::UnresolvedModule { node, target };
        let dependency = TaskDependency::Complete {
            node,
            task: import_task.into(),
            error: Some(Box::new(error.into())),
        };
        Err(ResolveError::Yield { dependency })
    }

    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let item = tree.get(item_id);
        let resolved_item: DependencyItem = match item {
            DependencyItem::UnresolvedRemote {
                source,
                name,
                mode,
                kind,
                alias,
                target,
                target_module: _,
                symbol,
            } => {
                let remote_module_id = self.resolve_import(
                    module,
                    item_id.into_global_any(module.id),
                    *source,
                    *target,
                    symbols,
                )?;
                let remote_module = self.program.modules.get(remote_module_id);
                let remote_module = remote_module.read();
                let remote_symbols = remote_module.dir.symbols.read();
                let target_symbol_id = match mode {
                    DependencyMode::Item => {
                        // resolve symbol in the remote module for item mode
                        let remote_scope_id = remote_module.dir.namespace_scope;
                        let remote_scope = remote_symbols.get_scope_by_id(remote_scope_id);
                        let key = name.map(StaticKey::Name).ok_or(
                            ResolveError::UnsupportedConstruct {
                                node: item_id.into_global_any(module.id),
                            },
                        )?;
                        self.resolve_absolute_symbol(
                            module,
                            item_id.into_global_any(module.id),
                            (remote_scope_id, remote_scope, LocalScopeMark::end()),
                            key,
                            &remote_symbols,
                        )
                        .map_err(|_| ResolveError::MissingSymbol {
                            node: item_id.into_global_any(module.id),
                            scope: remote_scope_id.into_global(remote_module_id),
                            via_module: Some(remote_module_id),
                            key,
                        })?
                    }
                    DependencyMode::Default => remote_module.dir.default_symbol,
                    DependencyMode::Namespace => remote_module.dir.namespace_symbol,
                };
                DependencyItem::Remote {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    target: *target,
                    target_module: remote_module_id,
                    symbol: *symbol,
                    target_symbol: target_symbol_id.into_global(remote_module_id),
                }
            }
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol,
            } => {
                let (scope_id, scope, mark) = symbols.get_scope(item_id, tree);
                let target_symbol_id = self.resolve_absolute_symbol(
                    module,
                    item_id.into_global_any(module.id),
                    (scope_id, scope, mark),
                    StaticKey::Name(*name),
                    symbols,
                )?;
                DependencyItem::Local {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    symbol: *symbol,
                    target_symbol: target_symbol_id.into_global(module.id),
                }
            }

            _ => {
                // nothing to do
                return Ok(());
            }
        };

        // update the target symbol of our symbol
        if let Some(symbol_id) = resolved_item.symbol()
            && let Some(target_symbol) = resolved_item.target_symbol()
        {
            symbols.get_symbol_mut(symbol_id).resolve_to(target_symbol);
        }

        *tree.get_mut(item_id) = resolved_item;
        Ok(())
    }
}
