use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, AnalyzeWarning, Compiler, TaskDependencyError,
};
use destack_dir::{Export, GlobalSymbolId, StaticKey, SymbolSpace, Type, TypeLiteral};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleGraphVersion, ProfileId};
use rustc_hash::FxHashSet;

/// One classified interface value state for convergence checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InterfaceValueState {
    /// The export remains indeterminate in this iteration.
    Indeterminate,
    /// The export resolved to an explicit error type.
    Error,
    /// The export resolved to one concrete local type id.
    Concrete(u32),
}

/// One snapshot entry for one exported value symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InterfaceValueSnapshot {
    /// The exported value symbol.
    symbol: GlobalSymbolId,
    /// The classified value state.
    state: InterfaceValueState,
}

impl Compiler {
    /// Ensure one interface component task is scheduled for this module.
    pub(crate) fn require_analyze_interface_component(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        // ensure forward dependency edges are available for component discovery
        self.require_resolve_forward_closure_for_interface_component(module, profile)?;

        // schedule the canonical anchor task for this component
        let anchor_module_id = self.interface_component_anchor_module_id(module, profile);
        let module = self.module_stamp(anchor_module_id);
        let profile = self.profile_stamp(profile);
        let graph = self.module_graph_stamp(profile.id);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeInterfaceComponent {
            module,
            profile,
            graph,
        })
    }

    /// Analyze one interface component.
    pub(crate) fn analyze_interface_component(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        graph_version: ModuleGraphVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        self.ensure_module_graph_version_matches::<AnalyzeError>(profile, graph_version)?;

        // resolve the strict component plan from the graph snapshot
        let component = self.interface_component_plan(module_id, profile)?;

        // ensure declarations are available for all component modules
        for component_module_id in component.component_modules.iter().copied() {
            self.require_analyze_module_declare(component_module_id, profile)?;
        }

        // require already solved interface dependencies outside the component
        for dependency_module_id in component.dependency_modules.iter().copied() {
            if let Err(error) = self.require_analyze_module_interface(dependency_module_id, profile)
            {
                return Err(AnalyzeError::from(error));
            }
        }

        // iterate component-local interface equations to a fixed point
        let component_set =
            self.converge_interface_component(module_id, profile, &component.component_modules)?;

        // report unresolved interface cycle dependencies after convergence
        self.report_unresolved_interface_component_cycle_exports(
            profile,
            &component.component_modules,
            &component_set,
        )?;

        // report unknown interface exports after convergence
        self.report_semantic_unknown_interface_component_exports(
            profile,
            &component.component_modules,
        )?;

        Ok(())
    }

    /// Converge one interface component to a fixed point and return its module set.
    fn converge_interface_component(
        &self,
        anchor_module_id: ModuleId,
        profile: ProfileId,
        component_modules: &[ModuleId],
    ) -> AnalyzeResult<FxHashSet<ModuleId>> {
        let component_set: FxHashSet<_> = component_modules.iter().copied().collect();
        let export_slot_count =
            self.interface_component_value_slot_count(profile, component_modules);
        let max_iterations =
            self.interface_component_max_iterations(export_slot_count, component_modules.len());

        let mut previous_snapshot: Option<Vec<InterfaceValueSnapshot>> = None;
        for _ in 0..max_iterations {
            self.analyze_interface_component_iteration(profile, component_modules)?;

            let next_snapshot = self.interface_component_value_snapshot(profile, component_modules);
            if previous_snapshot.as_ref() == Some(&next_snapshot) {
                return Ok(component_set);
            }
            previous_snapshot = Some(next_snapshot);
        }

        Err(AnalyzeError::Internal {
            message: format!(
                "interface component did not converge: anchor={anchor_module_id:?}, profile={profile:?}, modules={}, export_slots={export_slot_count}, max_iterations={max_iterations}",
                component_modules.len(),
            ),
        })
    }

    /// Analyze one component iteration in deterministic module order.
    fn analyze_interface_component_iteration(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
    ) -> AnalyzeResult<()> {
        for component_module_id in component_modules.iter().copied() {
            let module_version = self.module_version(component_module_id);
            let profile_version = self.profile_version(profile);
            self.analyze_module_interface_inner(
                component_module_id,
                profile,
                module_version,
                profile_version,
            )?;
        }

        Ok(())
    }

    /// Compute the fixed-point iteration cap for one component.
    fn interface_component_max_iterations(
        &self,
        export_slot_count: usize,
        module_count: usize,
    ) -> usize {
        (export_slot_count.max(1) * module_count).saturating_mul(4)
    }

    /// Count value-export slots in one interface component.
    fn interface_component_value_slot_count(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
    ) -> usize {
        self.interface_component_value_snapshot(profile, component_modules)
            .len()
    }

    /// Snapshot one interface component value state vector.
    fn interface_component_value_snapshot(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
    ) -> Vec<InterfaceValueSnapshot> {
        let mut snapshot = Vec::new();
        for component_module_id in component_modules.iter().copied() {
            let module = self.program.modules.get(component_module_id);
            let module = module.read();
            let dir = module.dir(profile);
            let symbols = dir.symbols.read();
            let types = dir.types.read();
            self.for_each_interface_export_table(&module, profile, |exports| {
                self.interface_value_snapshot_for_exports(
                    module.id,
                    &symbols,
                    &types,
                    exports,
                    &mut snapshot,
                );
            });
        }

        snapshot.sort_unstable();
        snapshot.dedup_by(|left, right| left.symbol == right.symbol);
        snapshot
    }

    /// Append interface value states for one export table.
    fn interface_value_snapshot_for_exports(
        &self,
        module_id: ModuleId,
        symbols: &destack_dir::SymbolTable,
        types: &destack_dir::TypeTable,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        snapshot: &mut Vec<InterfaceValueSnapshot>,
    ) {
        for export in exports.values() {
            let Some((export_symbol, _)) =
                self.interface_value_symbol_for_export(symbols, module_id, export)
            else {
                continue;
            };

            let state = self.classify_interface_value_state(types, export_symbol);
            snapshot.push(InterfaceValueSnapshot {
                symbol: export_symbol,
                state,
            });
        }
    }

    /// Classify one export symbol value state.
    fn classify_interface_value_state(
        &self,
        types: &destack_dir::TypeTable,
        symbol_id: GlobalSymbolId,
    ) -> InterfaceValueState {
        let Some(type_id) = types.get_value_type_id(symbol_id) else {
            return InterfaceValueState::Indeterminate;
        };

        let ty = types.get_type(type_id);

        if matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            }
        ) || ty.is_infer()
        {
            return InterfaceValueState::Indeterminate;
        }

        if matches!(ty, Type::Error) {
            return InterfaceValueState::Error;
        }

        InterfaceValueState::Concrete(type_id.0)
    }

    /// Report unresolved interface cycles after component convergence.
    fn report_unresolved_interface_component_cycle_exports(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
        component_set: &FxHashSet<ModuleId>,
    ) -> AnalyzeResult<()> {
        for component_module_id in component_modules.iter().copied() {
            let module = self.program.modules.get(component_module_id);
            let module = module.read();
            let dir = module.dir(profile);
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let mut types = dir.types.write();
            self.try_for_each_interface_export_table(&module, profile, |exports| {
                self.report_unresolved_interface_cycle_exports_for_table(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &mut types,
                    exports,
                    component_set,
                )
            })?;
        }

        Ok(())
    }

    /// Report unresolved interface cycles for one export table.
    fn report_unresolved_interface_cycle_exports_for_table(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
        symbols: &destack_dir::SymbolTable,
        types: &mut destack_dir::TypeTable,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        component_set: &FxHashSet<ModuleId>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(symbols, module.id, export)
            else {
                continue;
            };

            let Some(value_type_id) = types.get_value_type_id(export_symbol) else {
                continue;
            };
            if !self.interface_value_type_requires_cycle_anchor(types, value_type_id) {
                continue;
            }

            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(module, value_symbol, tree, symbols)
            else {
                continue;
            };
            let declarator = tree.get(declarator_id);

            // explicit export contracts break cycle-inference requirements
            if declarator.ty.is_some() {
                continue;
            }

            let Some(value_id) = declarator.value else {
                continue;
            };

            let references = self.interface_value_references(module, tree, symbols, value_id);
            let has_component_dependency = references
                .iter()
                .any(|symbol_id| component_set.contains(&symbol_id.module_id));
            if !has_component_dependency {
                continue;
            }

            let error_node = value_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::InterfaceInferenceRequiresAnnotation { node: error_node });

            let error_type_id = types.insert_type_from_any(Type::Error, declarator_id.into());
            types.set_value_type(export_symbol, error_type_id);
        }

        Ok(())
    }

    /// Report semantic-unknown interface exports after component convergence.
    fn report_semantic_unknown_interface_component_exports(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
    ) -> AnalyzeResult<()> {
        let mut warned_symbols = FxHashSet::default();

        for component_module_id in component_modules.iter().copied() {
            let module = self.program.modules.get(component_module_id);
            let module = module.read();
            let dir = module.dir(profile);
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let types = dir.types.read();
            self.try_for_each_interface_export_table(&module, profile, |exports| {
                self.report_semantic_unknown_interface_exports_for_table(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &types,
                    exports,
                    &mut warned_symbols,
                )
            })?;
        }

        Ok(())
    }

    /// Report semantic-unknown interface exports for one export table.
    fn report_semantic_unknown_interface_exports_for_table(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
        symbols: &destack_dir::SymbolTable,
        types: &destack_dir::TypeTable,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        warned_symbols: &mut FxHashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(symbols, module.id, export)
            else {
                continue;
            };
            if !warned_symbols.insert(export_symbol) {
                continue;
            }

            let Some(value_type_id) = types.get_value_type_id(export_symbol) else {
                continue;
            };
            if !self.interface_value_type_is_semantic_unknown(types, value_type_id) {
                continue;
            }

            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(module, value_symbol, tree, symbols)
            else {
                continue;
            };

            let warning_node = declarator_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.warning(AnalyzeWarning::ExportTypeUnknown { node: warning_node });
        }

        Ok(())
    }

    /// Visit each interface export table for a module.
    fn for_each_interface_export_table(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        mut callback: impl FnMut(&indexmap::IndexMap<(SymbolSpace, StaticKey), Export>),
    ) {
        let dir = module.dir(profile);
        let exported_symbols = dir.exported_symbols.read();
        let binding_exports = dir.module_binding_exports.read();

        callback(&exported_symbols);
        for binding in binding_exports.values() {
            callback(&binding.exports);
        }
    }

    /// Visit each interface export table for a module and stop on first error.
    fn try_for_each_interface_export_table(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        mut callback: impl FnMut(
            &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        ) -> AnalyzeResult<()>,
    ) -> AnalyzeResult<()> {
        let mut first_error = None;
        self.for_each_interface_export_table(module, profile, |exports| {
            if first_error.is_some() {
                return;
            }

            if let Err(error) = callback(exports) {
                first_error = Some(error);
            }
        });

        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(())
    }
}
