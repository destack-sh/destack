use std::slice::from_ref;
use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed, DirResolved};
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::{ModuleId, Span};

use super::{CheckExternalComponent, CheckState};
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one out-of-component external module.
pub(in crate::check) struct CheckExternalModuleState {
    /// The parsed external module, kept for diagnostic source spans.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded external module, kept for diagnostic source spans.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The resolved external module carrying import alias targets.
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

            let resolution = if self.is_component_module(current.module_id) {
                self.module(current.module_id)
                    .resolved
                    .imports
                    .symbol_resolution(current.local_id)
            } else {
                self.import_external_module(current.module_id)?
                    .resolved
                    .imports
                    .symbol_resolution(current.local_id)
            };
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

    /// Import committed external modules for the whole import closure.
    pub(in crate::check) fn import_component_external_modules(&mut self) -> CompilerResult<()> {
        // load every reachable external module's committed tables; inherent
        //  extension modules wait for their first extension lookup
        let external_modules = self
            .external_components
            .keys()
            .copied()
            .filter(|module| {
                let inherent = self.inherent_externals.as_ref();

                !inherent.is_some_and(|modules| modules.contains(module))
            })
            .collect::<Vec<_>>();
        for external_module in external_modules {
            self.import_external_module(external_module)?;
        }

        // record per-module visibility for name and member lookups
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        for module in modules {
            let visible = self.external_module_ids(module);
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
            if !self.is_component_module(external_module) {
                external_modules.insert(external_module);
            }
        }

        external_modules
    }

    /// Import external module state from committed artifacts.
    fn import_external_module_state(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<CheckExternalModuleState> {
        let artifact = self
            .external_components
            .get(&module)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check external module {module:?} has no component artifact"),
            })?;
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
        let (bindings, types, statics, generics, definitions) = match artifact {
            CheckExternalComponent::Checked(component) => {
                let checked_component = self
                    .artifacts
                    .dir_checked_component(component, self.profile)
                    .map_err(CompilerError::from)?;

                let checked =
                    checked_component
                        .module(module)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "checked component {component} does not contain external module \
                                 {module:?}"
                            ),
                        })?;

                (
                    checked.binding_table(bound.as_ref(), expanded.as_ref()),
                    checked.type_table(bound.as_ref(), expanded.as_ref()),
                    checked.static_table(bound.as_ref(), expanded.as_ref()),
                    checked.generic_table(),
                    checked.definition_table(),
                )
            }
            CheckExternalComponent::Declared(component) => {
                let declared = self
                    .artifacts
                    .dir_declared_component(component, self.profile)
                    .map_err(CompilerError::from)?;

                let declared = declared
                    .module(module)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!(
                            "declared component {component} does not contain external module \
                             {module:?}"
                        ),
                    })?;

                (
                    declared.binding_table(bound.as_ref(), expanded.as_ref()),
                    declared.type_table(bound.as_ref(), expanded.as_ref()),
                    declared.static_table(bound.as_ref(), expanded.as_ref()),
                    declared.generic_table(),
                    declared.definition_table(),
                )
            }
        };

        Ok(CheckExternalModuleState {
            parsed,
            expanded: Arc::clone(&expanded),
            resolved,
            bindings,
            types,
            statics,
            generics,
            definitions,
        })
    }
}
