use crate::analyze::common::TypeTablesContext;
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, Task, TaskDependencyError,
    TaskResultCollector,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module's interface summary has been computed.
    pub fn require_analyze_module_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        // avoid self dependency when already analyzing this module interface
        if let Some(Task::Analyze(AnalyzeTask::AnalyzeModuleInterface {
            module: current_module,
            profile: current_profile,
        })) = self.current_task()
            && current_module.id == module
            && current_profile.id == profile
        {
            return Ok(());
        }

        // avoid same-component self cycles while solving one interface component
        if let Some(Task::Analyze(AnalyzeTask::AnalyzeInterfaceComponent {
            module: component_anchor,
            profile: component_profile,
            ..
        })) = self.current_task()
            && component_profile.id == profile
        {
            if self.interface_modules_share_component(profile, component_anchor.id, module) {
                return Ok(());
            }
        }

        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInterface { module, profile })
    }

    /// Phase 2: Build interface summaries.
    pub(crate) fn analyze_module_interface_inner(
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_INTERFACE);

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
        let options = self.analyze_context_options_for_module(module_id);
        let mut type_tables =
            TypeTablesContext::new(&module, profile, &options, &tree, &symbols, &mut types);

        {
            let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_VALUES);

            // declare exported value types with local-only inference
            self.collect(
                &mut collector,
                self.infer_interface_value_types(
                    &mut type_tables.reborrow(),
                    &exported_symbols,
                    false,
                ),
            );

            // declare exported value types for module bindings
            for binding in binding_exports.values() {
                self.collect(
                    &mut collector,
                    self.infer_interface_value_types(
                        &mut type_tables.reborrow(),
                        &binding.exports,
                        false,
                    ),
                );
            }
        }

        {
            let _timing = self.timing_scope(tags::ANALYZE_INTERFACE_NAMESPACE);

            // declare the module namespace value type from exports
            self.collect(
                &mut collector,
                self.collect_module_namespace_value_type(
                    type_tables.module,
                    type_tables.profile,
                    &exported_symbols,
                    type_tables.tree,
                    type_tables.types,
                ),
            );
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }
}
