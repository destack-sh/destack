use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, AnalyzeWarning, Compiler, TaskDependencyError,
};
use destack_dir::{
    Export, GlobalSymbolId, NodeTree, StaticKey, SymbolSpace, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{Module, ModuleGraph, ModuleGraphKey, ModuleGraphVersion, ProfileId};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

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
        self.require_interface_forward_closure(module, profile)?;

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
        self.report_interface_component_cycle_exports(
            profile,
            &component.component_modules,
            &component_set,
        )?;

        // report unknown interface exports after convergence
        self.report_interface_component_unknown_exports(profile, &component.component_modules)?;

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
        let graph_key = ModuleGraphKey::new(profile);
        let graph =
            self.program
                .index
                .module_graphs
                .get(&graph_key)
                .ok_or(AnalyzeError::Internal {
                    message: format!(
                        "missing interface graph snapshot during component convergence: anchor={anchor_module_id:?}, profile={profile:?}"
                    ),
                })?;
        let dependents_by_module = self.interface_component_dependents(&graph, &component_set);
        let export_slot_count = component_modules
            .iter()
            .copied()
            .map(|module_id| {
                self.interface_module_value_snapshot(profile, module_id)
                    .len()
            })
            .sum::<usize>();
        let max_steps =
            self.interface_component_max_steps(export_slot_count, component_modules.len());

        let mut pending = VecDeque::new();
        let mut pending_set = FxHashSet::default();
        for module_id in component_modules.iter().copied() {
            pending.push_back(module_id);
            pending_set.insert(module_id);
        }

        let mut snapshots_by_module: FxHashMap<ModuleId, Vec<InterfaceValueSnapshot>> =
            FxHashMap::default();
        let mut step_count = 0usize;

        while let Some(component_module_id) = pending.pop_front() {
            pending_set.remove(&component_module_id);

            step_count = step_count.saturating_add(1);
            if step_count > max_steps {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "interface component did not converge: anchor={anchor_module_id:?}, profile={profile:?}, modules={}, export_slots={export_slot_count}, max_steps={max_steps}, executed_steps={step_count}",
                        component_modules.len(),
                    ),
                });
            }

            let module_version = self.module_version(component_module_id);
            let profile_version = self.profile_version(profile);
            self.analyze_module_interface_inner(
                component_module_id,
                profile,
                module_version,
                profile_version,
            )?;

            let next_snapshot = self.interface_module_value_snapshot(profile, component_module_id);
            let changed = snapshots_by_module
                .get(&component_module_id)
                .is_none_or(|previous_snapshot| previous_snapshot != &next_snapshot);
            if !changed {
                continue;
            }
            snapshots_by_module.insert(component_module_id, next_snapshot);

            let Some(dependents) = dependents_by_module.get(&component_module_id) else {
                continue;
            };
            for dependent_module_id in dependents.iter().copied() {
                if pending_set.insert(dependent_module_id) {
                    pending.push_back(dependent_module_id);
                }
            }
        }

        Ok(component_set)
    }

    /// Compute the fixed-point worklist cap for one component.
    fn interface_component_max_steps(
        &self,
        export_slot_count: usize,
        module_count: usize,
    ) -> usize {
        let module_count = module_count.max(1);
        (export_slot_count.max(1) * module_count)
            .saturating_mul(module_count)
            .saturating_mul(4)
    }

    /// Collect one deterministic component-local dependent map.
    fn interface_component_dependents(
        &self,
        graph: &ModuleGraph,
        component_set: &FxHashSet<ModuleId>,
    ) -> FxHashMap<ModuleId, Vec<ModuleId>> {
        let mut dependents_by_module = FxHashMap::default();
        for module_id in component_set.iter().copied() {
            let mut dependents = graph
                .dependents_for(module_id)
                .into_iter()
                .filter(|dependent_module_id| component_set.contains(dependent_module_id))
                .collect::<Vec<_>>();
            dependents.sort_unstable();
            dependents_by_module.insert(module_id, dependents);
        }

        dependents_by_module
    }

    /// Snapshot one interface module value state vector.
    fn interface_module_value_snapshot(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
    ) -> Vec<InterfaceValueSnapshot> {
        let mut snapshot = Vec::new();
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let types = dir.types.read();
        let exported_symbols = dir.exported_symbols.read();
        self.interface_value_snapshot_for_exports(
            module.id,
            &symbols,
            &types,
            &exported_symbols,
            &mut snapshot,
        );

        let binding_exports = dir.module_binding_exports.read();
        for binding in binding_exports.values() {
            self.interface_value_snapshot_for_exports(
                module.id,
                &symbols,
                &types,
                &binding.exports,
                &mut snapshot,
            );
        }

        snapshot.sort_unstable();
        snapshot.dedup_by(|left, right| left.symbol == right.symbol);
        snapshot
    }

    /// Append interface value states for one export table.
    fn interface_value_snapshot_for_exports(
        &self,
        module_id: ModuleId,
        symbols: &SymbolTable,
        types: &TypeTable,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
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
        types: &TypeTable,
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
    fn report_interface_component_cycle_exports(
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
            let exported_symbols = dir.exported_symbols.read();
            self.report_interface_cycle_exports_for_table(
                &module,
                profile,
                &tree,
                &symbols,
                &mut types,
                &exported_symbols,
                component_set,
            )?;

            let binding_exports = dir.module_binding_exports.read();
            for binding in binding_exports.values() {
                self.report_interface_cycle_exports_for_table(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &mut types,
                    &binding.exports,
                    component_set,
                )?;
            }
        }

        Ok(())
    }

    /// Report unresolved interface cycles for one export table.
    fn report_interface_cycle_exports_for_table(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
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
            if !self.interface_value_requires_solver(types, value_type_id)
                && !self.interface_value_is_semantic_unknown(types, value_type_id)
            {
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
    fn report_interface_component_unknown_exports(
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
            let exported_symbols = dir.exported_symbols.read();
            self.report_interface_unknown_exports_for_table(
                &module,
                profile,
                &tree,
                &symbols,
                &types,
                &exported_symbols,
                &mut warned_symbols,
            )?;

            let binding_exports = dir.module_binding_exports.read();
            for binding in binding_exports.values() {
                self.report_interface_unknown_exports_for_table(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &types,
                    &binding.exports,
                    &mut warned_symbols,
                )?;
            }
        }

        Ok(())
    }

    /// Report semantic-unknown interface exports for one export table.
    fn report_interface_unknown_exports_for_table(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
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
            if !self.interface_value_is_semantic_unknown(types, value_type_id) {
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
}
