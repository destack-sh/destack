use std::collections::{HashMap, HashSet, VecDeque};

use indexmap::IndexMap;

use crate::analyze::common::TypeTablesContext;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, ModuleCheckOptions, Task,
    TaskDependencyError, TaskResultCollector,
};
use destack_builtin::BuiltinLibKind;
use destack_dir::{Export, GlobalSymbolId, LocalTypeId, StaticKey, SymbolSpace, SymbolType, Type};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{Module, ModuleSource, ProfileId};

impl Compiler {
    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        // avoid self dependency when already declaring this module
        if let Some(Task::Analyze(AnalyzeTask::AnalyzeModuleDeclare {
            module: current_module,
            profile: current_profile,
        })) = self.current_task()
            && current_module.id == module
            && current_profile.id == profile
        {
            return Ok(());
        }

        // skip ambient builtin declarations when libs are disabled
        if !self.options.load_libs {
            let module = self.program.modules.get(module);
            let module = module.read();
            if matches!(module.source, ModuleSource::Builtin(BuiltinLibKind::Lib)) {
                return Ok(());
            }
        }

        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleDeclare { module, profile })
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_DECLARE);

        self.require_resolve_module_canonical(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // load module state and dir tables
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // ensure builtins are resolved before declaring symbols
        self.require_resolve_builtins(profile)?;

        // ensure ambient libs are declared before user modules
        self.ensure_ambient_libs_declared(&module, profile)?;

        // snapshot the module dir tables for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
        let mut collector = TaskResultCollector::new();
        let module_checks = self.module_check_options_for_module(module.id);
        let options = self.analyze_context_options_for_module(module.id);
        let mut type_tables =
            TypeTablesContext::new(&module, profile, &options, &tree, &symbols, &mut types);

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECLARATIONS);
            self.collect(
                &mut collector,
                self.collect_module_declarations(&mut type_tables.reborrow()),
            );
        }

        // index declaration static parameter metadata
        self.index_static_parameter_metadata(&module, &tree, &symbols, type_tables.types);

        // yield after declaration metadata writes
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        let mut collector = TaskResultCollector::new();
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_TYPES);

            // evaluate remaining unevaluated types after declaration metadata is available
            if self.should_eager_evaluate_declared_types(&module, module_checks) {
                let has_dependency = self.evaluate_unevaluated_types_to_fixpoint(
                    &mut type_tables.reborrow(),
                    &mut collector,
                );
                if has_dependency {
                    if let Some(dependency) = collector.try_into_yield_any() {
                        return Err(AnalyzeError::Yield { dependency });
                    }

                    return Ok(());
                }
            }
        }

        // publish declared static parameter constraints
        let mut publish_collector = TaskResultCollector::new();
        {
            self.collect(
                &mut publish_collector,
                self.publish_static_parameter_constraints(&mut type_tables),
            );
        }

        // publish declare-owned static constant values for cross-module static evaluation
        self.collect(
            &mut publish_collector,
            self.publish_declared_static_constant_values_for_module(&mut type_tables.reborrow()),
        );

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_ALIASES);

            // publish exported alias targets from declared type metadata
            let exported_symbols = dir.exported_symbols.read();
            let binding_exports = dir.module_binding_exports.read();
            self.collect(
                &mut publish_collector,
                self.publish_declared_alias_targets(&mut type_tables.reborrow(), &exported_symbols),
            );
            for binding in binding_exports.values() {
                self.collect(
                    &mut publish_collector,
                    self.publish_declared_alias_targets(
                        &mut type_tables.reborrow(),
                        &binding.exports,
                    ),
                );
            }
        }

        // yield after static-constraint publication
        if let Some(dependency) = publish_collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // drop the read guard before taking a mutable lock for decorators
        drop(symbols);

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECORATORS);

            // attach well known decorator metadata to symbols
            let mut symbols = dir.symbols.write();
            let mut captures = dir.captures.write();
            self.collect(
                &mut collector,
                self.register_symbol_decorators(
                    &module,
                    profile,
                    &tree,
                    &mut symbols,
                    &mut captures,
                ),
            );
            drop(symbols);
            drop(captures);
        }

        // cache well-known intrinsics after decorator registration
        if module.is_user() {
            self.collect(
                &mut collector,
                self.ensure_well_known_intrinsics_for_profile(profile),
            );
        }
        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // return the collected result
        Ok(())
    }

    /// Ensure ambient libs are declared before analyzing a user module.
    fn ensure_ambient_libs_declared(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // collect dependency yields
        let mut collector = TaskResultCollector::new();

        if self.options.load_libs && module.is_user() {
            // resolve libs before declaring ambient modules
            if let Err(error) = self.require_resolve_libs(profile)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(AnalyzeError::from(error));
            }

            if let Some(builtins) = self.program.builtins.as_ref() {
                let profile_key = &self.program.profile(profile).key;
                if let Some(ambient_libs) = builtins.ambient_libs(profile_key) {
                    for lib_module_id in ambient_libs {
                        if lib_module_id == module.id {
                            continue;
                        }
                        if let Err(error) =
                            self.require_analyze_module_declare(lib_module_id, profile)
                            && let Some(error) = collector.try_collect::<(), _>(Err(error))
                        {
                            return Err(AnalyzeError::from(error));
                        }
                    }
                }
            }
        }

        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }

    /// Decide whether declared types should be eagerly evaluated for a module.
    fn should_eager_evaluate_declared_types(
        &self,
        _module: &Module,
        _module_checks: ModuleCheckOptions,
    ) -> bool {
        // declared type commitments are stage-owned outputs for cross-module reads
        true
    }

    /// Evaluate unevaluated types to a fixed point.
    ///
    /// Returns true if evaluation yielded dependencies.
    fn evaluate_unevaluated_types_to_fixpoint(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        collector: &mut TaskResultCollector,
    ) -> bool {
        // seed the worklist
        let mut pending: VecDeque<LocalTypeId> = (0..type_tables.types.type_count())
            .map(LocalTypeId::new)
            .filter(|ty_id| matches!(type_tables.types.get_type(*ty_id), Type::Unevaluated(_)))
            .collect();
        let mut did_change = true;

        while did_change && !pending.is_empty() {
            did_change = false;
            let mut next_pending = VecDeque::new();
            let type_count = type_tables.types.type_count();

            // evaluate pending types
            while let Some(ty_id) = pending.pop_front() {
                if !matches!(type_tables.types.get_type(ty_id), Type::Unevaluated(_)) {
                    continue;
                }

                self.collect(
                    collector,
                    self.resolve_declared_type(&mut type_tables.reborrow(), ty_id),
                );

                if collector.has_dependencies() {
                    return true;
                }

                if matches!(type_tables.types.get_type(ty_id), Type::Unevaluated(_)) {
                    next_pending.push_back(ty_id);
                } else {
                    did_change = true;
                }
            }

            // add newly created unevaluated types
            let new_type_count = type_tables.types.type_count();
            if new_type_count > type_count {
                for i in type_count..new_type_count {
                    let ty_id = LocalTypeId::new(i);
                    if matches!(type_tables.types.get_type(ty_id), Type::Unevaluated(_)) {
                        next_pending.push_back(ty_id);
                    }
                }
            }

            pending = next_pending;
        }

        false
    }

    /// Publish exported alias targets from declared local type metadata.
    fn publish_declared_alias_targets(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
    ) -> AnalyzeResult<()> {
        for export in exports.values() {
            // skip non-type exports
            if export.space != SymbolSpace::Type {
                continue;
            }

            // skip unresolved or remote exports
            let Some(export_symbol) = export.target.resolved() else {
                continue;
            };
            if export_symbol.module_id != type_tables.module.id {
                continue;
            }

            // skip non-alias symbols
            let symbol_entry = type_tables.symbols.get_symbol(export_symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                continue;
            }

            // load the declared alias target and ensure it is evaluated
            let typed_symbol = GlobalSymbolId::new(
                type_tables.module.id,
                export_symbol.local_id.with_type(symbol_entry.ty),
            );
            let Some(alias_target_id) = type_tables.types.get_alias_target_type_id(typed_symbol)
            else {
                continue;
            };
            self.ensure_type_evaluated(&mut type_tables.reborrow(), alias_target_id)?;

            // materialize value static arguments before publishing
            let needs_materialization = self.type_has_unevaluated_value_static_arguments(
                type_tables.module,
                type_tables.profile,
                alias_target_id,
                type_tables.tree,
                type_tables.symbols,
                type_tables.types,
                &mut HashSet::new(),
            );
            if !needs_materialization {
                continue;
            }

            let mut cache = HashMap::new();
            let materialized =
                self.materialize_static_arguments_in_type(type_tables, alias_target_id, &mut cache);
            if materialized != alias_target_id {
                type_tables
                    .types
                    .set_alias_target_type_id(typed_symbol, materialized);
            }
        }

        Ok(())
    }
}
