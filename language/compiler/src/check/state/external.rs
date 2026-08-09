use std::sync::Arc;

use destack_artifact::DirResolved;
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use rustc_hash::FxHashMap;

use super::{CheckState, Pass};
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one external module.
pub(in crate::check) struct CheckExternalModuleState {
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
    /// The committed member table, elaborated while checking.
    pub(in crate::check) members: dir::MemberTable<'static>,
    /// The modules the elaborated rows mention, empty while elaborating.
    pub(in crate::check) references: Vec<ModuleId>,
}

/// External module states keyed by module id.
#[derive(Default)]
pub(in crate::check) struct ExternalModuleTable {
    /// One state per loaded external module.
    slots: FxHashMap<ModuleId, CheckExternalModuleState>,
}

impl ExternalModuleTable {
    /// Return one loaded external module state.
    pub(in crate::check) fn get(&self, module: &ModuleId) -> Option<&CheckExternalModuleState> {
        self.slots.get(module)
    }

    /// Return whether one external module is loaded.
    pub(in crate::check) fn contains_key(&self, module: &ModuleId) -> bool {
        self.slots.contains_key(module)
    }

    /// Store one loaded external module state.
    pub(in crate::check) fn insert(&mut self, module: ModuleId, state: CheckExternalModuleState) {
        self.slots.insert(module, state);
    }
}

impl CheckState<'_> {
    /// Return loaded state for one external module.
    pub(in crate::check) fn external_module(&self, module: ModuleId) -> &CheckExternalModuleState {
        self.external_modules
            .get(&module)
            .unwrap_or_else(|| panic!("external module {module:?} was not loaded"))
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

        // read the resolve stage directly; it never depends on declared modules
        let resolved = self
            .artifacts
            .dir_resolved_content(module, self.profile)
            .map_err(CompilerError::from)?;
        self.external_resolved.insert(module, Arc::clone(&resolved));

        Ok(resolved)
    }

    /// Import and return state for one external module while checking.
    pub(in crate::check) fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Option<&CheckExternalModuleState>> {
        // skip external reads while declaring
        if self.is_declaration() {
            return Ok(None);
        }

        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;
            self.external_modules.insert(module, external);
        }

        Ok(Some(self.external_module(module)))
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
                None => {
                    // resolve import binders without a per-symbol target by their key
                    if let Some(target) = self.import_binder_target(current)?
                        && target != current
                    {
                        current = target;
                        continue;
                    }

                    return Ok(current);
                }
            }
        }
    }

    /// Return one import binder's resolved target symbol, when one exists.
    ///
    /// Binder symbols carry no declarations of their own; their targets live
    /// in the module's import resolutions or its resolved global names.
    pub(in crate::check) fn import_binder_target(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // read the binder's kind and key before releasing the table
        let table = self.binding_table(symbol.module_id);
        let binding = table.get_symbol(symbol.local_id);
        let kind = binding.kind;
        let key = binding.key;
        drop(table);

        if kind != dir::SymbolKind::Import {
            return Ok(None);
        }

        // follow the module's own import resolutions first
        let resolved = if self.is_own_module(symbol.module_id) {
            Arc::clone(&self.module(symbol.module_id).resolved)
        } else {
            self.external_resolved(symbol.module_id)?
        };
        if let Some(dir::ImportResolution::Resolved(dir::ImportTarget::Symbol(target))) =
            resolved.imports.symbol_resolution(symbol.local_id)
        {
            return Ok(Some(*target));
        }

        // follow resolved global names by the binder's key
        if let Some(key) = key
            && let Some([dir::ImportTarget::Symbol(target)]) = resolved
                .imports
                .global_target_by_key
                .get(&key)
                .map(Vec::as_slice)
        {
            return Ok(Some(*target));
        }

        Ok(None)
    }

    /// Import external modules and record the direct imports' visibility.
    ///
    /// Elaborated member bindings embed types from their module's own
    /// imports, so the load walks the import closure to a fixpoint.
    pub(in crate::check) fn import_external_modules(&mut self) -> CompilerResult<()> {
        // record the direct imports' visibility in every pass
        let module = self.module_id;
        let visible = self.external_module_ids(module);
        self.module_mut(module)
            .external_modules
            .extend(visible.iter().copied());

        // skip external reads while declaring
        if self.is_declaration() {
            return Ok(());
        }

        // load the modules the loaded tables mention, to a fixpoint
        let mut queue = visible.iter().copied().collect::<Vec<_>>();
        for external in &visible {
            self.import_external_module(*external)?;
        }
        while let Some(loaded) = queue.pop() {
            for referenced in self.external_module(loaded).references.clone() {
                if referenced == self.module_id || self.external_modules.contains_key(&referenced) {
                    continue;
                }
                self.import_external_module(referenced)?;
                queue.push(referenced);
            }
        }

        Ok(())
    }

    /// Return external modules that can be named from the checked module.
    fn external_module_ids(&self, module: ModuleId) -> FxIndexSet<ModuleId> {
        let mut external_modules = FxIndexSet::default();
        let imports = &self.module(module).resolved.imports;

        // collect resolved target modules outside the checked module
        for external_module in imports.target_modules() {
            if !self.is_own_module(external_module) {
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
        let bound = self
            .artifacts
            .dir_bound_content(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .dir_expanded_content(module, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .dir_resolved_content(module, self.profile)
            .map_err(CompilerError::from)?;

        // read the module's declared artifact
        let declared = self
            .artifacts
            .dir_declared_projected(module, self.profile)
            .map_err(CompilerError::from)?;

        // read stored member bindings while checking; elaborate reads none
        let elaborated = match self.pass {
            Pass::Check => Some(
                self.artifacts
                    .dir_elaborated(module, self.profile)
                    .map_err(CompilerError::from)?,
            ),
            _ => None,
        };
        let members = match &elaborated {
            Some(elaborated) => elaborated.member_table(),
            None => declared.member_table(),
        };
        let mut references = declared.references.clone();
        if let Some(elaborated) = &elaborated {
            references.extend(elaborated.references.iter().copied());
        }
        let types = match &elaborated {
            Some(elaborated) => elaborated.type_table(bound.as_ref(), expanded.as_ref(), &declared),
            None => declared.type_table(bound.as_ref(), expanded.as_ref()),
        };
        let bindings = match &elaborated {
            Some(elaborated) => {
                elaborated.binding_table(bound.as_ref(), expanded.as_ref(), &declared)
            }
            None => declared.binding_table(bound.as_ref(), expanded.as_ref()),
        };

        Ok(CheckExternalModuleState {
            bindings,
            types,
            statics: match &elaborated {
                Some(elaborated) => {
                    elaborated.static_table(bound.as_ref(), expanded.as_ref(), &declared)
                }
                None => declared.static_table(bound.as_ref(), expanded.as_ref()),
            },
            generics: match &elaborated {
                Some(elaborated) => elaborated.generic_table(&declared),
                None => declared.generic_table(),
            },
            definitions: match &elaborated {
                Some(elaborated) => elaborated.definition_table(),
                None => declared.definition_table(),
            },
            members,
            references,
            resolved,
        })
    }
}
