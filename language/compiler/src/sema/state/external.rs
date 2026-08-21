use std::sync::Arc;

use destack_artifact::{
    ArtifactProjectionKey, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded,
    DirParsed, DirResolved,
};
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use super::{CheckState, Pass};
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one external module.
pub(in crate::sema) struct CheckExternalModuleState {
    /// The parsed module tree.
    pub(in crate::sema) parsed: Arc<DirParsed>,
    /// The resolved external module holding the import alias targets.
    pub(in crate::sema) resolved: Arc<DirResolved>,
    /// The committed binding table.
    pub(in crate::sema) bindings: dir::BindingTable<'static>,
    /// The committed type table.
    pub(in crate::sema) types: dir::TypeTable<'static>,
    /// The committed static table.
    pub(in crate::sema) statics: dir::StaticTable<'static>,
    /// The committed generic table.
    pub(in crate::sema) generics: dir::GenericTable<'static>,
    /// The committed decision table, populated while materializing.
    pub(in crate::sema) decisions: dir::DecisionTable<'static>,
    /// The committed coercion table, populated while materializing.
    pub(in crate::sema) coercions: dir::CoercionTable<'static>,
    /// The committed definition table.
    pub(in crate::sema) definitions: dir::DefinitionTable<'static>,
    /// The committed member table, elaborated while checking.
    pub(in crate::sema) members: dir::MemberTable<'static>,
    /// The decided conformances and winners, carried once elaborate ran.
    pub(in crate::sema) auto: Option<Arc<dir::AutoSegment>>,
    /// The modules the loaded entries mention, empty while elaborating.
    pub(in crate::sema) references: Vec<ModuleId>,
}

/// External module states keyed by module id.
#[derive(Default)]
pub(in crate::sema) struct ExternalModuleTable {
    /// One state per loaded external module.
    slots: FxHashMap<ModuleId, CheckExternalModuleState>,
}

impl ExternalModuleTable {
    /// Return one loaded external module state.
    pub(in crate::sema) fn get(&self, module: &ModuleId) -> Option<&CheckExternalModuleState> {
        self.slots.get(module)
    }

    /// Return whether one external module is loaded.
    pub(in crate::sema) fn contains_key(&self, module: &ModuleId) -> bool {
        self.slots.contains_key(module)
    }

    /// Store one loaded external module state.
    pub(in crate::sema) fn insert(&mut self, module: ModuleId, state: CheckExternalModuleState) {
        self.slots.insert(module, state);
    }
}

impl CheckState<'_> {
    /// Return loaded state for one external module.
    pub(in crate::sema) fn external_module(&self, module: ModuleId) -> &CheckExternalModuleState {
        self.external_modules
            .get(&module)
            .unwrap_or_else(|| panic!("external module {module:?} was not loaded"))
    }

    /// Read one external module's resolved import targets.
    pub(in crate::sema) fn external_resolved(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Arc<DirResolved>> {
        if let Some(state) = self.external_modules.get(&module) {
            return Ok(Arc::clone(&state.resolved));
        }
        if let Some(resolved) = self.external_resolved.get(&module) {
            return Ok(Arc::clone(resolved));
        }

        // read the resolve stage directly
        let resolved = self
            .artifacts
            .read::<DirResolved>((module, self.profile))
            .map_err(CompilerError::from)?;
        self.external_resolved.insert(module, Arc::clone(&resolved));

        Ok(resolved)
    }

    /// Return the implementations of one interface declared across the program with their roots.
    pub(in crate::sema) fn program_implementations(
        &mut self,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::GlobalSymbolId, Option<dir::GlobalSymbolId>); 4]>> {
        // skip program reads while declaring
        if self.is_declaration() {
            return Ok(SmallVec::new());
        }

        // read the graph's per interface projection, depending on that interface alone
        let graph = self
            .artifacts
            .module_graph_reader(self.profile)
            .map_err(CompilerError::from)?;
        let symbols = graph
            .interface_implementations(interface)
            .map_err(CompilerError::from)?
            .iter()
            .map(|implementation| (implementation.symbol, implementation.root))
            .collect::<SmallVec<[_; 4]>>();

        Ok(symbols)
    }

    /// Import and return state for one external module while checking.
    pub(in crate::sema) fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Option<&CheckExternalModuleState>> {
        // skip external reads while declaring
        if self.is_declaration() {
            return Ok(None);
        }

        // load each external module once
        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;
            self.external_modules.insert(module, external);
        }

        Ok(Some(self.external_module(module)))
    }

    /// Resolve one symbol through import alias chains.
    pub(in crate::sema) fn resolve_symbol_alias(
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
                Some(dir::ImportResolution::Resolved(resolution))
                    if let dir::ExportTarget::Symbols(symbols) = &resolution.target
                        && let [target] = symbols.as_slice() =>
                {
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
    /// A binder's target lives in the module's import resolutions or in its
    /// resolved global names.
    pub(in crate::sema) fn import_binder_target(
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
        if let Some(dir::ImportResolution::Resolved(resolution)) =
            resolved.imports.symbol_resolution(symbol.local_id)
            && let dir::ExportTarget::Symbols(symbols) = &resolution.target
            && let [target] = symbols.as_slice()
        {
            return Ok(Some(*target));
        }

        // follow resolved global names by the binder's key
        if let Some(key) = key
            && let Some([resolution]) = resolved
                .imports
                .global_resolution_by_key
                .get(&key)
                .map(Vec::as_slice)
            && let dir::ExportTarget::Symbols(symbols) = &resolution.target
            && let [target] = symbols.as_slice()
        {
            return Ok(Some(*target));
        }

        Ok(None)
    }

    /// Import external modules and record the direct imports' visibility.
    ///
    /// Elaborated member bindings embed types from their module's own
    /// imports, so the load walks the import closure to a fixpoint.
    pub(in crate::sema) fn import_external_modules(&mut self) -> CompilerResult<()> {
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

        // seed with every module the own stage rows mention
        let mut visible = visible;
        if self.pass == Pass::Materialize {
            let state = self.module(module);
            let mentions = [
                state.declared.as_ref().map(|stage| &stage.references),
                state.elaborated.as_ref().map(|stage| &stage.references),
                state.checked.as_ref().map(|stage| &stage.references),
            ];
            for stage in mentions.into_iter().flatten() {
                visible.extend(
                    stage
                        .iter()
                        .copied()
                        .filter(|mentioned| *mentioned != module),
                );
            }
        }

        // load the modules the loaded tables mention, to a fixpoint
        let mut queue = visible.iter().copied().collect::<Vec<_>>();
        for external in &visible {
            self.import_external_module(*external)?;
        }

        // follow each loaded module's own references
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
        // read the stages every pass reaches
        let bound = self
            .artifacts
            .read_content::<DirBound>((module, self.profile))
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .read_content::<DirExpanded>((module, self.profile))
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .read_content::<DirResolved>((module, self.profile))
            .map_err(CompilerError::from)?;

        // read the parsed tree and the module's declared artifact
        let parsed = self
            .artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let declared = self
            .artifacts
            .read_projection::<DirDeclared>((module, self.profile), ArtifactProjectionKey::Declared)
            .map_err(CompilerError::from)?;

        // each pass loads external modules up to the stage its reads may reach
        match self.pass {
            Pass::Declare | Pass::Elaborate => Ok(Self::declared_external(
                module, parsed, bound, expanded, resolved, declared,
            )),
            Pass::Check => {
                let elaborated = self
                    .artifacts
                    .read::<DirElaborated>((module, self.profile))
                    .map_err(CompilerError::from)?;

                Ok(Self::elaborated_external(
                    module, parsed, bound, expanded, resolved, declared, elaborated,
                ))
            }
            Pass::Materialize => {
                let elaborated = self
                    .artifacts
                    .read::<DirElaborated>((module, self.profile))
                    .map_err(CompilerError::from)?;
                let checked = self
                    .artifacts
                    .read::<DirChecked>((module, self.profile))
                    .map_err(CompilerError::from)?;

                Ok(Self::checked_external(
                    parsed, bound, expanded, resolved, declared, elaborated, checked,
                ))
            }
        }
    }

    /// Build external state over declared faces, for the elaborating passes.
    fn declared_external(
        module: ModuleId,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
    ) -> CheckExternalModuleState {
        CheckExternalModuleState {
            bindings: declared.binding_table(bound.as_ref(), expanded.as_ref()),
            types: declared.type_table(bound.as_ref(), expanded.as_ref()),
            statics: declared.static_table(bound.as_ref(), expanded.as_ref()),
            generics: declared.generic_table(),
            decisions: Self::empty_decisions(module),
            coercions: Self::empty_coercions(module),
            definitions: declared.definition_table(),
            members: declared.member_table(),
            auto: None,
            references: declared.references.clone(),
            resolved,
            parsed,
        }
    }

    /// Return an empty decision table for the passes below materialize.
    fn empty_decisions(module: ModuleId) -> dir::DecisionTable<'static> {
        dir::DecisionTable::from_segment(Arc::new(dir::DecisionSegment::new(module)))
    }

    /// Return an empty coercion table for the passes below materialize.
    fn empty_coercions(module: ModuleId) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segment(Arc::new(dir::CoercionSegment::new(module)))
    }

    /// Build external state over elaborated faces, for the checking pass.
    fn elaborated_external(
        module: ModuleId,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
        elaborated: Arc<DirElaborated>,
    ) -> CheckExternalModuleState {
        let mut references = declared.references.clone();
        references.extend(elaborated.references.iter().copied());

        CheckExternalModuleState {
            bindings: elaborated.binding_table(bound.as_ref(), expanded.as_ref(), &declared),
            types: elaborated.type_table(bound.as_ref(), expanded.as_ref(), &declared),
            statics: elaborated.static_table(bound.as_ref(), expanded.as_ref(), &declared),
            generics: elaborated.generic_table(&declared),
            decisions: Self::empty_decisions(module),
            coercions: Self::empty_coercions(module),
            definitions: elaborated.definition_table(),
            members: elaborated.member_table(),
            auto: Some(Arc::clone(&elaborated.auto)),
            references,
            resolved,
            parsed,
        }
    }

    /// Build external state over checked bodies, for the materializing pass.
    fn checked_external(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
        elaborated: Arc<DirElaborated>,
        checked: Arc<DirChecked>,
    ) -> CheckExternalModuleState {
        let mut references = declared.references.clone();
        references.extend(elaborated.references.iter().copied());
        references.extend(checked.references.iter().copied());

        CheckExternalModuleState {
            bindings: checked.binding_table(
                bound.as_ref(),
                expanded.as_ref(),
                &declared,
                &elaborated,
            ),
            types: checked.type_table(bound.as_ref(), expanded.as_ref(), &declared, &elaborated),
            statics: checked.static_table(
                bound.as_ref(),
                expanded.as_ref(),
                &declared,
                &elaborated,
            ),
            generics: checked.generic_table(&declared, &elaborated),
            decisions: checked.decision_table(&declared, &elaborated),
            coercions: checked.coercion_table(),
            definitions: checked.definition_table(&elaborated),
            members: checked.member_table(&elaborated),
            auto: Some(Arc::clone(&elaborated.auto)),
            references,
            resolved,
            parsed,
        }
    }
}
