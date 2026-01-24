use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, TaskDependencyError, TaskResultCollector,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

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

    /// Phase 2: Build export inference summaries.
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

        // ensure local declarations are ready
        self.require_analyze_module_declare(module_id, profile)?;
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

        // read the module dir tables for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
        let exported_symbols = dir.exported_symbols.read();
        let binding_exports = dir.module_binding_exports.read();
        let mut collector = TaskResultCollector::new();

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

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }
}
