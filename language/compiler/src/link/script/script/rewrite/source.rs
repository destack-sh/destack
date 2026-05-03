use std::collections::HashMap;
use std::sync::Arc;

use destack_core::StringPool;
use destack_source::ModuleId;
use {destack_codegen_js as js, destack_dir as dir};

use super::super::linker::OutputModule;
use crate::{LinkError, LinkResult, ScriptLinker};

/// One linked output scope for identifier assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum OutputScopeId {
    /// The merged top level scope for one linked output.
    TopLevel,
    /// One source-backed nested scope.
    Source {
        /// The source module that owns the scope.
        module_id: ModuleId,
        /// The source-local scope id.
        scope_id: dir::LocalScopeId,
    },
}

/// One source-backed minify context.
#[derive(Debug, Clone)]
pub(super) struct MinifySourceContext {
    /// The source tree.
    pub(super) tree: Arc<dir::Tree>,
    /// The source strings.
    pub(super) strings: Arc<StringPool>,
    /// The source symbol table.
    pub(super) symbols: Arc<dir::SymbolTable>,
    /// The namespace scope for this module.
    pub(super) namespace_scope: dir::LocalScopeId,
}

/// One descending node visitor that records visited JS node ids.
#[derive(Debug, Clone, Default)]
struct ReferencedNodeCollector {
    /// The visited node ids in walk order.
    visited: Vec<u32>,
    /// The visitor options.
    options: js::NodeVisitorOptions,
}

impl js::NodeVisitor for ReferencedNodeCollector {
    fn options(&self) -> &js::NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &js::Tree, _ty: js::NodeType, id: u32) {
        self.visited.push(id);
    }
}

impl ScriptLinker<'_> {
    /// Load all source contexts needed for one linked output.
    pub(super) fn load_minify_source_contexts(
        &self,
        modules: &[OutputModule],
    ) -> LinkResult<HashMap<ModuleId, MinifySourceContext>> {
        let mut source_contexts = HashMap::new();

        // keep one source context per referenced source module
        for (module_id, module) in modules {
            self.insert_minify_source_context(*module_id, &mut source_contexts)?;
            self.insert_referenced_minify_source_contexts(module, &mut source_contexts)?;
        }

        Ok(source_contexts)
    }

    /// Load one source context into the minify cache when needed.
    fn insert_minify_source_context(
        &self,
        module_id: ModuleId,
        source_contexts: &mut HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<()> {
        if module_id == ModuleId::EPHEMERAL || source_contexts.contains_key(&module_id) {
            return Ok(());
        }

        let source_context = self
            .minify_source_context(module_id, source_contexts)?
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing source context for minify module {module_id:?}"),
            })?;

        source_contexts.insert(module_id, source_context);

        Ok(())
    }

    /// Return one source context for identifier minification.
    pub(super) fn minify_source_context(
        &self,
        module_id: ModuleId,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<Option<MinifySourceContext>> {
        // synthetic modules have no source context
        if module_id == ModuleId::EPHEMERAL {
            return Ok(None);
        }

        // reuse the cached context when possible
        if let Some(source_context) = source_contexts.get(&module_id) {
            return Ok(Some(source_context.clone()));
        }

        // fall back to the declared source context on demand
        self.ensure_module_profile(module_id)?;

        let profile_id = self.profile_id_for_module(module_id)?;
        let dir = self
            .compiler
            .dir_declared(self.context, module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing declared source context for minify module {:?} profile {:?}: {error:?}",
                    module_id, profile_id
                ),
            })?;

        Ok(Some(MinifySourceContext {
            tree: Arc::new(dir.tree.clone()),
            strings: Arc::new(dir.strings.clone()),
            symbols: Arc::new(dir.symbols.clone()),
            namespace_scope: dir.namespace_scope,
        }))
    }

    /// Load source contexts for every source-backed module referenced by one linked JS tree.
    fn insert_referenced_minify_source_contexts(
        &self,
        module: &js::Module,
        source_contexts: &mut HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<()> {
        let mut visitor = ReferencedNodeCollector::default();
        js::walk_roots(&mut visitor, &module.tree, &module.roots);

        // source locations and source-backed symbols
        for node_id in &visitor.visited {
            let (module_id, _) = module.tree.get_source(*node_id);
            self.insert_minify_source_context(module_id, source_contexts)?;

            let Some(js::ScriptSymbolId::Source(symbol_id)) = module.tree.symbol_by_id(*node_id)
            else {
                continue;
            };

            self.insert_minify_source_context(symbol_id.module_id, source_contexts)?;
        }

        Ok(())
    }

    /// Return the symbol that owns one lowered declaration name.
    pub(super) fn declaration_name_symbol(
        &self,
        module: &js::Module,
        declaration_id: js::LocalNodeId<js::Declaration>,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<Option<js::ScriptSymbolId>> {
        let declaration = module.tree.get(declaration_id);
        if !matches!(
            declaration,
            js::Declaration::Class(_) | js::Declaration::Function(_)
        ) {
            return Ok(module.tree.symbol(declaration_id));
        }

        let (module_id, source_id) = module.tree.get_source(declaration_id.id);
        if module_id == ModuleId::EPHEMERAL {
            return Ok(module.tree.symbol(declaration_id));
        }

        let source_context = self
            .minify_source_context(module_id, source_contexts)?
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing source context for declaration module {module_id:?}"),
            })?;
        let source_declaration_id = dir::LocalNodeId::<dir::Declaration>::new(source_id);
        let source_declaration = source_context.tree.get(source_declaration_id);
        let symbol_id = source_declaration.name_symbol().into_global(module_id);

        Ok(Some(js::ScriptSymbolId::Source(symbol_id)))
    }

    /// Return the original source name for one symbol when one exists.
    pub(super) fn source_symbol_name(
        &self,
        symbol_id: js::ScriptSymbolId,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<Option<String>> {
        match symbol_id {
            js::ScriptSymbolId::Source(symbol_id) => {
                let Some(source_context) =
                    self.minify_source_context(symbol_id.module_id, source_contexts)?
                else {
                    return Ok(None);
                };
                let symbol = source_context.symbols.get_symbol(symbol_id.local_id);
                let Some(name) = symbol.name() else {
                    return Ok(None);
                };

                Ok(Some(source_context.strings.get(name).to_string()))
            }
            js::ScriptSymbolId::ModuleDefault(_) => Ok(Some(js::MODULE_DEFAULT_NAME.to_string())),
        }
    }

    /// Return the output scope that owns one symbol.
    pub(super) fn symbol_scope_id(
        &self,
        symbol_id: js::ScriptSymbolId,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<OutputScopeId> {
        match symbol_id {
            js::ScriptSymbolId::Source(symbol_id) => {
                let source_context = self
                    .minify_source_context(symbol_id.module_id, source_contexts)?
                    .ok_or_else(|| LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "missing source context for symbol module {:?} profile lookup",
                            symbol_id.module_id
                        ),
                    })?;
                let symbol = source_context.symbols.get_symbol(symbol_id.local_id);
                let scope_id = symbol.scope.0;

                if scope_id == source_context.namespace_scope {
                    Ok(OutputScopeId::TopLevel)
                } else {
                    Ok(OutputScopeId::Source {
                        module_id: symbol_id.module_id,
                        scope_id,
                    })
                }
            }
            js::ScriptSymbolId::ModuleDefault(_) => Ok(OutputScopeId::TopLevel),
        }
    }

    /// Return the parent output scope when one exists.
    pub(super) fn parent_scope_id(
        &self,
        scope_id: OutputScopeId,
        source_contexts: &HashMap<ModuleId, MinifySourceContext>,
    ) -> LinkResult<Option<OutputScopeId>> {
        match scope_id {
            OutputScopeId::TopLevel => Ok(None),
            OutputScopeId::Source {
                module_id,
                scope_id,
            } => {
                let profile_id = self.profile_id_for_module(module_id)?;
                let source_context = self
                    .minify_source_context(module_id, source_contexts)?
                    .ok_or_else(|| LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "missing source context for scope module {:?} profile {:?}",
                            module_id, profile_id
                        ),
                    })?;
                let scope = source_context.symbols.get_scope_by_id(scope_id);
                let Some((parent_scope_id, _)) = scope.parent else {
                    return Ok(Some(OutputScopeId::TopLevel));
                };

                if parent_scope_id == source_context.namespace_scope {
                    Ok(Some(OutputScopeId::TopLevel))
                } else {
                    Ok(Some(OutputScopeId::Source {
                        module_id,
                        scope_id: parent_scope_id,
                    }))
                }
            }
        }
    }
}
