use std::collections::{HashMap, HashSet};

use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, TaskDependencyError, TaskResultCollector,
};
use destack_dir::{
    Export, GlobalSymbolId, NodeTree, StaticKey, SymbolSpace, SymbolTable, SymbolType, TypeTable,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Ensure a module's export inference summary has been computed.
    pub fn require_analyze_module_export(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleExport { module, profile })
    }

    /// Phase 2: Build export inference "summaries".
    pub(crate) fn analyze_module_export_inner(
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_EXPORT);

        // ensure local declarations are ready
        self.require_analyze_module_declare(module_id, profile)?;

        // load module state and dir tables
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // read the module dir tables for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
        let mut collector = TaskResultCollector::new();

        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let exported_symbols = dir.exported_symbols.read();
        let binding_exports = dir.module_binding_exports.read();

        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPORT_VALUES);

            // declare exported value types with local-only inference
            self.collect(
                &mut collector,
                self.declare_exported_value_types(
                    &module,
                    profile,
                    &exported_symbols,
                    &tree,
                    &symbols,
                    &mut types,
                ),
            );

            // declare exported value types for module bindings
            for binding in binding_exports.values() {
                self.collect(
                    &mut collector,
                    self.declare_exported_value_types(
                        &module,
                        profile,
                        &binding.exports,
                        &tree,
                        &symbols,
                        &mut types,
                    ),
                );
            }
        }

        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPORT_ALIASES);

            // materialize alias targets for exported type symbols
            self.collect(
                &mut collector,
                self.materialize_exported_alias_targets(
                    &module,
                    profile,
                    &exported_symbols,
                    &tree,
                    &symbols,
                    &mut types,
                ),
            );

            for binding in binding_exports.values() {
                self.collect(
                    &mut collector,
                    self.materialize_exported_alias_targets(
                        &module,
                        profile,
                        &binding.exports,
                        &tree,
                        &symbols,
                        &mut types,
                    ),
                );
            }
        }

        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPORT_NAMESPACE);

            // declare the module namespace value type from exports
            self.collect(
                &mut collector,
                self.declare_module_namespace_value_type(
                    &module,
                    profile,
                    &exported_symbols,
                    &tree,
                    &mut types,
                ),
            );
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }

    /// Ensure exported alias targets are fully materialized in the local TypeTable.
    fn materialize_exported_alias_targets(
        &self,
        module: &Module,
        profile: ProfileId,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
            if export_symbol.module_id != module.id {
                continue;
            }

            // require alias symbols that own an alias target
            let symbol_entry = symbols.get_symbol(export_symbol.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                continue;
            }
            let typed_symbol =
                GlobalSymbolId::new(module.id, export_symbol.local_id.with_type(symbol_entry.ty));
            let Some(alias_target_id) = types.get_alias_target_type_id(typed_symbol) else {
                continue;
            };

            // evaluate unevaluated alias targets in place
            self.ensure_type_evaluated(module, profile, alias_target_id, tree, symbols, types)?;

            // materialize value static arguments when needed
            let needs_materialization = self.type_contains_unevaluated_value_static_arguments(
                module,
                profile,
                alias_target_id,
                tree,
                symbols,
                types,
                &mut HashSet::new(),
            );
            if !needs_materialization {
                continue;
            }

            let mut cache = HashMap::new();
            let materialized = self.materialize_static_arguments_in_type(
                module,
                profile,
                alias_target_id,
                tree,
                symbols,
                types,
                &mut cache,
            );
            if materialized != alias_target_id {
                types.set_alias_target_type_id(typed_symbol, materialized);
            }
        }

        Ok(())
    }
}
