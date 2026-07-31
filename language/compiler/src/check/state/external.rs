use std::slice::from_ref;
use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed, DirResolved};
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::{ModuleId, Span};

use super::CheckState;
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one out-of-component external module.
pub(in crate::check) struct CheckExternalModuleState {
    /// The parsed external module, kept for diagnostic source spans.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded external module, kept for diagnostic source spans.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The resolved external module holding the import alias targets.
    pub(in crate::check) resolved: Arc<DirResolved>,
    /// The committed binding table.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// The committed type table.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The committed static table.
    pub(in crate::check) statics: dir::StaticTable<'static>,
    /// The committed generic table.
    pub(in crate::check) generics: dir::GenericTable<'static>,
    /// The committed definition table.
    pub(in crate::check) definitions: dir::DefinitionTable<'static>,
}

impl CheckExternalModuleState {
    /// Return the post-expansion DIR tree view of this module.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, from_ref(&self.expanded.patch))
    }

    /// Return the authored diagnostic span of one visible node.
    pub(in crate::check) fn diagnostic_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let view = self.view();
        let source = view.get_source_any(node);

        // prefer the authored node that produced the visible node
        if let Some(span) = self.parsed.tree.get_main_span_by_id(source) {
            return Some(span);
        }
        if let Some(span) = self.parsed.tree.get_span_by_id(source) {
            return Some(span);
        }

        view.get_span_by_id(node.id)
    }
}

impl CheckState<'_> {
    /// Return loaded state for one external module.
    pub(in crate::check) fn external_module(&self, module: ModuleId) -> &CheckExternalModuleState {
        self.external_modules
            .get(&module)
            .unwrap_or_else(|| unreachable!("external module {module:?} was not loaded"))
    }

    /// Read one external module's resolved import targets.
    pub(in crate::check) fn external_resolved(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Arc<DirResolved>> {
        if let Some(state) = self.external_modules.get(&module) {
            return Ok(Arc::clone(&state.resolved));
        }
        if let Some(resolved) = self.external_resolved.get(&module) {
            return Ok(Arc::clone(resolved));
        }

        // read the resolve stage directly, it never depends on declared modules
        let resolved = self
            .artifacts
            .dir_resolved(module, self.profile)
            .map_err(CompilerError::from)?;
        self.external_resolved.insert(module, Arc::clone(&resolved));

        Ok(resolved)
    }

    /// Import and return state for one external module.
    pub(in crate::check) fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckExternalModuleState> {
        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;

            self.external_modules.insert(module, external);
        }

        Ok(self.external_module(module))
    }

    /// Resolve one symbol through import alias chains.
    pub(in crate::check) fn resolve_symbol_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let mut current = symbol;
        let mut visited = FxIndexSet::default();

        // hop alias targets until a declaring symbol appears
        loop {
            if !visited.insert(current) {
                return Err(CompilerError::Internal {
                    message: format!("symbol alias {symbol:?} forwards in a cycle"),
                });
            }

            let resolved = if self.is_own_module(current.module_id) {
                Arc::clone(&self.module(current.module_id).resolved)
            } else {
                self.external_resolved(current.module_id)?
            };
            let resolution = resolved.imports.symbol_resolution(current.local_id);
            match resolution {
                Some(dir::ImportResolution::Resolved(dir::ImportTarget::Symbol(target))) => {
                    current = *target;
                }
                Some(resolution) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "symbol alias {current:?} has no exact symbol target: {resolution:?}"
                        ),
                    });
                }
                None => return Ok(current),
            }
        }
    }

    /// Import directly imported external modules and record their visibility.
    pub(in crate::check) fn import_external_modules(&mut self) -> CompilerResult<()> {
        let modules = vec![self.module_id];
        for module in modules {
            let visible = self.external_module_ids(module);

            // load foreign modules only when checking
            if self.is_checking {
                for external in &visible {
                    self.import_external_module(*external)?;
                }
            }
            self.module_mut(module).external_modules.extend(visible);
        }

        Ok(())
    }

    /// Return external modules that can be named from one component module.
    fn external_module_ids(&self, module: ModuleId) -> FxIndexSet<ModuleId> {
        let mut external_modules = FxIndexSet::default();
        let imports = &self.module(module).resolved.imports;

        // collect resolved target modules outside the component
        for external_module in imports.target_modules() {
            if !self.is_own_module(external_module) {
                external_modules.insert(external_module);
            }
        }

        external_modules
    }

    /// Return one foreign symbol's binder kind while declaring.
    ///
    /// Kinds are binder facts, so declaring reads a foreign module's bound tables without
    /// touching its declared types.
    /// Bound and expanded artifacts precede every declared artifact, so this read keeps the
    /// declared artifact graph acyclic.
    pub(in crate::check) fn external_binder_kind(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::SymbolKind> {
        if !self.external_binders.contains_key(&symbol.module_id) {
            let bound = self
                .artifacts
                .dir_bound(symbol.module_id, self.profile)
                .map_err(CompilerError::from)?;
            let expanded = self
                .artifacts
                .dir_expanded(symbol.module_id, self.profile)
                .map_err(CompilerError::from)?;
            self.external_binders
                .insert(symbol.module_id, expanded.binding_table(bound.as_ref()));
        }

        Ok(self.external_binders[&symbol.module_id]
            .get_symbol(symbol.local_id)
            .kind)
    }

    /// Import external module state from committed artifacts.
    fn import_external_module_state(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<CheckExternalModuleState> {
        let parsed = self
            .artifacts
            .dir_parsed(module)
            .map_err(CompilerError::from)?;
        let bound = self
            .artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .dir_resolved(module, self.profile)
            .map_err(CompilerError::from)?;

        // read the module's sealed declared module
        let declared = self
            .artifacts
            .dir_declared(module, self.profile)
            .map_err(CompilerError::from)?;

        Ok(CheckExternalModuleState {
            bindings: declared.binding_table(bound.as_ref(), expanded.as_ref()),
            types: declared.type_table(bound.as_ref(), expanded.as_ref()),
            statics: declared.static_table(bound.as_ref(), expanded.as_ref()),
            generics: declared.generic_table(),
            definitions: declared.definition_table(),
            parsed,
            expanded: Arc::clone(&expanded),
            resolved,
        })
    }
}
