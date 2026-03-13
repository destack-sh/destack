use crate::analyze::DirReadBoundary;
use crate::analyze::common::{SymbolTypeView, TreeSymbolView, TypeView};
use crate::{AnalyzeError, AnalyzeResult, AnalyzeWarning, Compiler};
use destack_dir::{
    Declarator, Export, Expression, GlobalSymbolId, LocalNodeId, StaticKey, SymbolSpace, Type,
    TypeLiteral, TypeTable,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{
    ModuleDir, ModuleDirData, ModuleGraph, ModuleGraphKey, ModuleGraphVersion, ModuleSource,
    ProfileId,
};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::sync::Arc;

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

/// One exported value that participates in interface cycle reporting.
#[derive(Debug, Clone)]
struct InterfaceCycleCandidate {
    /// The module that owns the export.
    module_id: ModuleId,
    /// The exported symbol to report and overwrite on error.
    export_symbol: GlobalSymbolId,
    /// The local value symbol behind the export.
    value_symbol: GlobalSymbolId,
    /// The declarator that owns the export value.
    declarator_id: LocalNodeId<Declarator>,
    /// The initializer expression for the export value.
    value_id: LocalNodeId<Expression>,
    /// Whether the export has an explicit declared contract.
    has_annotation: bool,
    /// Referenced exported values inside the same interface component.
    dependency_symbols: Vec<GlobalSymbolId>,
}

/// Return the cycle-graph identity for one symbol ignoring its view-specific type tag.
fn interface_cycle_symbol_identity(symbol: GlobalSymbolId) -> (ModuleId, u32) {
    (symbol.module_id, symbol.local_id.id)
}

impl Compiler {
    /// Analyze one interface component.
    pub(crate) fn analyze_interface_component(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        graph_version: ModuleGraphVersion,
    ) -> AnalyzeResult<Vec<(ModuleId, ProfileId, ModuleDirData)>> {
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
            self.require_dir_declared(component_module_id, profile)?;
        }

        // require already solved interface dependencies outside the component
        for dependency_module_id in component.dependency_modules.iter().copied() {
            if let Err(error) = self.require_dir_interface(dependency_module_id, profile) {
                return Err(AnalyzeError::from(error));
            }
        }

        // iterate component-local interface equations to a fixed point
        let mut component_dirs = FxHashMap::default();
        let component_set = self.converge_interface_component(
            module_id,
            profile,
            &component.component_modules,
            &mut component_dirs,
        )?;

        // report unresolved interface cycle dependencies after convergence
        self.report_interface_component_cycle_exports(
            profile,
            &component.component_modules,
            &component_set,
            &mut component_dirs,
        )?;

        // report unknown interface exports after convergence
        self.report_interface_component_unknown_exports(
            profile,
            &component.component_modules,
            &component_dirs,
        )?;

        let mut entries = Vec::with_capacity(component.component_modules.len());
        for component_module_id in component.component_modules.iter().copied() {
            let Some(dir) = component_dirs.get(&component_module_id) else {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing converged interface dir: module={component_module_id:?}, profile={profile:?}"
                    ),
                });
            };

            entries.push((component_module_id, profile, dir.to_data()));
        }

        Ok(entries)
    }

    /// Converge one interface component to a fixed point and return its module set.
    fn converge_interface_component(
        &self,
        anchor_module_id: ModuleId,
        profile: ProfileId,
        component_modules: &[ModuleId],
        component_dirs: &mut FxHashMap<ModuleId, Arc<ModuleDir>>,
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
                let dir = self
                    .require_artifact_dir_for_boundary(
                        module_id,
                        profile,
                        DirReadBoundary::Declared,
                    )
                    .map_err(AnalyzeError::from)?;
                let dir = self.transient_dir_for_artifact(
                    module_id,
                    profile,
                    DirReadBoundary::Interface,
                    dir,
                );

                Ok::<usize, AnalyzeError>(
                    self.interface_module_value_snapshot(module_id, profile, dir.as_ref())
                        .len(),
                )
            })
            .try_fold(0usize, |sum, count| count.map(|count| sum + count))?;
        let max_steps =
            self.interface_component_max_steps(export_slot_count, component_modules.len());

        let mut pending = VecDeque::new();
        let mut pending_set = FxHashSet::default();
        for module_id in component_modules.iter().copied() {
            let dir = self
                .require_artifact_dir_for_boundary(module_id, profile, DirReadBoundary::Declared)
                .map_err(AnalyzeError::from)?;
            let dir = self.transient_dir_for_artifact(
                module_id,
                profile,
                DirReadBoundary::Interface,
                dir,
            );
            self.set_active_dir_frame(module_id, profile, DirReadBoundary::Interface, &dir);
            component_dirs.insert(module_id, dir);
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
            let dir = Arc::new(self.analyze_module_interface_inner(
                component_module_id,
                profile,
                module_version,
                profile_version,
            )?);
            let next_snapshot =
                self.interface_module_value_snapshot(component_module_id, profile, dir.as_ref());
            self.set_active_dir_frame(
                component_module_id,
                profile,
                DirReadBoundary::Interface,
                &dir,
            );
            component_dirs.insert(component_module_id, dir);

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
        module_id: ModuleId,
        profile: ProfileId,
        dir: &ModuleDir,
    ) -> Vec<InterfaceValueSnapshot> {
        let mut snapshot = Vec::new();
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();
        let exported_symbols = dir.exported_symbols.read();
        self.interface_value_snapshot_for_exports(
            SymbolTypeView::new(&module, profile, &symbols, &types),
            &exported_symbols,
            &mut snapshot,
        );

        let binding_exports = dir.module_binding_exports.read();
        for binding in binding_exports.values() {
            self.interface_value_snapshot_for_exports(
                SymbolTypeView::new(&module, profile, &symbols, &types),
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
        ctx: SymbolTypeView<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        snapshot: &mut Vec<InterfaceValueSnapshot>,
    ) {
        for export in exports.values() {
            let Some((export_symbol, _)) =
                self.interface_value_symbol_for_export(ctx.symbols, ctx.module.id, export)
            else {
                continue;
            };

            let state = self.classify_interface_value_state(ctx.types, export_symbol);
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
        component_dirs: &mut FxHashMap<ModuleId, Arc<ModuleDir>>,
    ) -> AnalyzeResult<()> {
        let cycle_candidates = self.collect_interface_cycle_candidates(
            profile,
            component_modules,
            component_set,
            component_dirs,
        )?;
        let cycle_candidates = self.unanchored_interface_cycle_candidates(cycle_candidates);

        if cycle_candidates.is_empty() {
            return Ok(());
        }

        let mut candidates_by_module =
            FxHashMap::<ModuleId, Vec<InterfaceCycleCandidate>>::default();
        for candidate in cycle_candidates {
            candidates_by_module
                .entry(candidate.module_id)
                .or_default()
                .push(candidate);
        }

        for component_module_id in component_modules.iter().copied() {
            let Some(cycle_candidates) = candidates_by_module.get(&component_module_id) else {
                continue;
            };

            let Some(dir) = component_dirs.get(&component_module_id) else {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing local interface dir for cycle reporting: module={component_module_id:?}, profile={profile:?}"
                    ),
                });
            };
            let mut types = dir.types.write();

            for candidate in cycle_candidates {
                let error_node = candidate
                    .value_id
                    .into_global_any(candidate.module_id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InterfaceInferenceRequiresAnnotation { node: error_node });

                let error_type_id =
                    types.insert_type_from_any(Type::Error, candidate.declarator_id.into());
                types.set_value_type(candidate.export_symbol, error_type_id);
                if candidate.value_symbol != candidate.export_symbol {
                    types.set_value_type(candidate.value_symbol, error_type_id);
                }
            }
        }

        Ok(())
    }

    /// Collect exported values that participate in this interface component.
    fn collect_interface_cycle_candidates(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
        component_set: &FxHashSet<ModuleId>,
        component_dirs: &FxHashMap<ModuleId, Arc<ModuleDir>>,
    ) -> AnalyzeResult<Vec<InterfaceCycleCandidate>> {
        let mut cycle_candidates = Vec::new();
        let mut seen_exports = FxHashSet::default();
        let mut cycle_candidate_by_symbol = FxHashMap::default();

        for component_module_id in component_modules.iter().copied() {
            let module = self.program.modules.get(component_module_id);
            let module = module.read();
            let Some(dir) = component_dirs.get(&component_module_id) else {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing local interface dir for cycle candidate collection: module={component_module_id:?}, profile={profile:?}"
                    ),
                });
            };
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let exported_symbols = dir.exported_symbols.read();
            let binding_exports = dir.module_binding_exports.read();

            self.collect_interface_cycle_candidates_for_table(
                &module,
                profile,
                &tree,
                &symbols,
                &exported_symbols,
                &mut seen_exports,
                &mut cycle_candidates,
                &mut cycle_candidate_by_symbol,
            )?;

            for binding in binding_exports.values() {
                self.collect_interface_cycle_candidates_for_table(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &binding.exports,
                    &mut seen_exports,
                    &mut cycle_candidates,
                    &mut cycle_candidate_by_symbol,
                )?;
            }
        }

        for candidate_index in 0..cycle_candidates.len() {
            let references = cycle_candidates[candidate_index].dependency_symbols.clone();
            let mut dependency_symbols = FxHashSet::default();

            for reference_symbol in references {
                if !component_set.contains(&reference_symbol.module_id) {
                    continue;
                }

                let dependency_key = interface_cycle_symbol_identity(reference_symbol);
                let Some(dependency_index) =
                    cycle_candidate_by_symbol.get(&dependency_key).copied()
                else {
                    continue;
                };
                let dependency_candidate = &cycle_candidates[dependency_index];
                dependency_symbols.insert(dependency_candidate.export_symbol);
            }

            let candidate = &mut cycle_candidates[candidate_index];
            candidate.dependency_symbols = dependency_symbols.into_iter().collect();
            candidate.dependency_symbols.sort_unstable();
        }

        Ok(cycle_candidates)
    }

    /// Collect exported values from one export table for interface cycle reporting.
    fn collect_interface_cycle_candidates_for_table(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
        symbols: &destack_dir::SymbolTable,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        seen_exports: &mut FxHashSet<GlobalSymbolId>,
        cycle_candidates: &mut Vec<InterfaceCycleCandidate>,
        cycle_candidate_by_symbol: &mut FxHashMap<(ModuleId, u32), usize>,
    ) -> AnalyzeResult<()> {
        let tree_symbol_view = TreeSymbolView::new(module, profile, tree, symbols);

        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(symbols, module.id, export)
            else {
                continue;
            };
            if !seen_exports.insert(export_symbol) {
                continue;
            }

            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(tree_symbol_view, value_symbol)
            else {
                continue;
            };
            let declarator = tree.get(declarator_id);
            let Some(value_id) = declarator.value else {
                continue;
            };

            let cycle_candidate_index = cycle_candidates.len();
            cycle_candidate_by_symbol.insert(
                interface_cycle_symbol_identity(export_symbol),
                cycle_candidate_index,
            );
            cycle_candidate_by_symbol.insert(
                interface_cycle_symbol_identity(value_symbol),
                cycle_candidate_index,
            );
            cycle_candidates.push(InterfaceCycleCandidate {
                module_id: module.id,
                export_symbol,
                value_symbol,
                declarator_id,
                value_id,
                has_annotation: declarator.ty.is_some(),
                dependency_symbols: export.dependencies.clone(),
            });
        }

        Ok(())
    }

    /// Return the unanchored cycle candidates that still require interface diagnostics.
    fn unanchored_interface_cycle_candidates(
        &self,
        cycle_candidates: Vec<InterfaceCycleCandidate>,
    ) -> Vec<InterfaceCycleCandidate> {
        let candidate_count = cycle_candidates.len();
        let mut adjacency = vec![Vec::new(); candidate_count];
        let mut reverse_adjacency = vec![Vec::new(); candidate_count];
        let mut export_symbol_to_index = FxHashMap::default();

        for (candidate_index, candidate) in cycle_candidates.iter().enumerate() {
            export_symbol_to_index.insert(candidate.export_symbol, candidate_index);
        }

        for (candidate_index, candidate) in cycle_candidates.iter().enumerate() {
            for dependency_symbol in &candidate.dependency_symbols {
                let Some(dependency_index) = export_symbol_to_index.get(dependency_symbol).copied()
                else {
                    continue;
                };
                adjacency[candidate_index].push(dependency_index);
                reverse_adjacency[dependency_index].push(candidate_index);
            }
        }

        let mut order = Vec::with_capacity(candidate_count);
        let mut visited = vec![false; candidate_count];
        for candidate_index in 0..candidate_count {
            self.visit_interface_cycle_order(candidate_index, &adjacency, &mut visited, &mut order);
        }

        let mut component_ids = vec![usize::MAX; candidate_count];
        let mut component_count = 0usize;
        for candidate_index in order.into_iter().rev() {
            if component_ids[candidate_index] != usize::MAX {
                continue;
            }
            self.assign_interface_cycle_component(
                candidate_index,
                component_count,
                &reverse_adjacency,
                &mut component_ids,
            );
            component_count += 1;
        }

        let mut component_members = vec![Vec::new(); component_count];
        for (candidate_index, component_id) in component_ids.iter().copied().enumerate() {
            component_members[component_id].push(candidate_index);
        }

        let mut rejected_candidates = FxHashSet::default();
        for component_members in component_members {
            let has_cycle = component_members.len() > 1
                || component_members
                    .iter()
                    .copied()
                    .any(|candidate_index| adjacency[candidate_index].contains(&candidate_index));
            if !has_cycle {
                continue;
            }

            let has_annotation = component_members
                .iter()
                .copied()
                .any(|candidate_index| cycle_candidates[candidate_index].has_annotation);
            if has_annotation {
                continue;
            }

            for candidate_index in component_members {
                rejected_candidates.insert(candidate_index);
            }
        }

        cycle_candidates
            .into_iter()
            .enumerate()
            .filter_map(|(candidate_index, candidate)| {
                rejected_candidates
                    .contains(&candidate_index)
                    .then_some(candidate)
            })
            .collect()
    }

    /// Visit one candidate for SCC order construction.
    fn visit_interface_cycle_order(
        &self,
        candidate_index: usize,
        adjacency: &[Vec<usize>],
        visited: &mut [bool],
        order: &mut Vec<usize>,
    ) {
        if visited[candidate_index] {
            return;
        }
        visited[candidate_index] = true;

        for dependency_index in adjacency[candidate_index].iter().copied() {
            self.visit_interface_cycle_order(dependency_index, adjacency, visited, order);
        }

        order.push(candidate_index);
    }

    /// Assign one SCC id through the reverse graph.
    fn assign_interface_cycle_component(
        &self,
        candidate_index: usize,
        component_id: usize,
        reverse_adjacency: &[Vec<usize>],
        component_ids: &mut [usize],
    ) {
        if component_ids[candidate_index] != usize::MAX {
            return;
        }
        component_ids[candidate_index] = component_id;

        for dependency_index in reverse_adjacency[candidate_index].iter().copied() {
            self.assign_interface_cycle_component(
                dependency_index,
                component_id,
                reverse_adjacency,
                component_ids,
            );
        }
    }

    /// Report semantic-unknown interface exports after component convergence.
    fn report_interface_component_unknown_exports(
        &self,
        profile: ProfileId,
        component_modules: &[ModuleId],
        component_dirs: &FxHashMap<ModuleId, Arc<ModuleDir>>,
    ) -> AnalyzeResult<()> {
        let mut warned_symbols = FxHashSet::default();

        for component_module_id in component_modules.iter().copied() {
            let module = self.program.modules.get(component_module_id);
            let module = module.read();
            let module_checks = self.module_check_options_for_module(component_module_id);
            let skip_declaration_unknown_warnings = module.language_type.is_declaration()
                && (module_checks.skip_lib_check
                    || (matches!(module.source, ModuleSource::Builtin(_))
                        && !self.options.validate_builtin_libs));
            if skip_declaration_unknown_warnings {
                continue;
            }

            let Some(dir) = component_dirs.get(&component_module_id) else {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing local interface dir for unknown export reporting: module={component_module_id:?}, profile={profile:?}"
                    ),
                });
            };
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let types = dir.types.read();
            let exported_symbols = dir.exported_symbols.read();
            self.report_interface_unknown_exports_for_table(
                TypeView::new(&module, profile, &tree, &symbols, &types),
                &exported_symbols,
                &mut warned_symbols,
            )?;

            let binding_exports = dir.module_binding_exports.read();
            for binding in binding_exports.values() {
                self.report_interface_unknown_exports_for_table(
                    TypeView::new(&module, profile, &tree, &symbols, &types),
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
        ctx: TypeView<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        warned_symbols: &mut FxHashSet<GlobalSymbolId>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            let Some((export_symbol, value_symbol)) =
                self.interface_value_symbol_for_export(ctx.symbols, ctx.module.id, export)
            else {
                continue;
            };
            if !warned_symbols.insert(export_symbol) {
                continue;
            }

            let Some(value_type_id) = ctx.types.get_value_type_id(export_symbol) else {
                continue;
            };
            if !self.interface_value_is_semantic_unknown(ctx.types, value_type_id) {
                continue;
            }

            let Some(declarator_id) =
                self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), value_symbol)
            else {
                continue;
            };

            let warning_node = declarator_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.warning(AnalyzeWarning::ExportTypeUnknown { node: warning_node });
        }

        Ok(())
    }
}
